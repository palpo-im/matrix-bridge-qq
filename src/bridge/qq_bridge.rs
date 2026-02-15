use std::sync::Arc;

use async_trait::async_trait;
use salvo::conn::TcpListener;
use salvo::prelude::*;
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::database::{Database, Portal, QQUser};
use crate::matrix::{extract_text_content, AppService, AppServiceBridge, MatrixClient, RoomEvent};
use crate::qq::{NormalizedQQMessageEvent, QQClient, QQEventBridge, QQWebhookService};

pub struct QQBridge {
    config: Config,
    db: Database,
    matrix_client: MatrixClient,
    qq_client: QQClient,
}

impl QQBridge {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        let db = Database::connect(
            &config.appservice.database.r#type,
            &config.appservice.database.uri,
            config.appservice.database.max_open_conns,
            config.appservice.database.max_idle_conns,
        )
        .await?;
        db.run_migrations().await?;

        let matrix_client = MatrixClient::new(
            config.homeserver.address.clone(),
            config.appservice.as_token.clone(),
        );
        let qq_client = QQClient::new(
            config.bridge.onebot.api_base.clone(),
            config.bridge.onebot.access_token.clone(),
        );

        Ok(Self {
            config,
            db,
            matrix_client,
            qq_client,
        })
    }

    pub async fn start(self: Arc<Self>) -> anyhow::Result<()> {
        let appservice = Arc::new(AppService::new(
            self.config.appservice.hs_token.clone(),
            self.clone() as Arc<dyn AppServiceBridge>,
        ));
        let qq_service = Arc::new(QQWebhookService::new(
            self.config.bridge.onebot.event_path.clone(),
            self.config.bridge.onebot.listen_secret.clone(),
            self.clone() as Arc<dyn QQEventBridge>,
        ));

        let addr = format!(
            "{}:{}",
            self.config.appservice.hostname, self.config.appservice.port
        );

        info!("starting matrix-bridge-qq on {addr}");
        let portals = self.db.count_portals().await.unwrap_or(0);
        info!("loaded {portals} portals from database");

        let router = Router::new()
            .push(Router::with_path("healthz").get(HealthHandler))
            .push(appservice.router())
            .push(qq_service.router());

        let listener = TcpListener::new(addr).bind().await;
        Server::new(listener).serve(router).await;

        Ok(())
    }

    pub async fn stop(&self) {
        info!("stop requested");
    }

    async fn ensure_portal(&self, chat_type: &str, chat_id: &str) -> anyhow::Result<Portal> {
        if let Some(portal) = self.db.get_portal_by_chat(chat_type, chat_id).await? {
            return Ok(portal);
        }

        let is_direct = chat_type == "private";
        let name = if is_direct {
            self.config.bridge.render_private_room_name(chat_id)
        } else {
            self.config.bridge.render_group_room_name(chat_id)
        };

        let room_id = self
            .matrix_client
            .create_private_room(&name, is_direct)
            .await?;

        let portal = Portal {
            chat_type: chat_type.to_string(),
            chat_id: chat_id.to_string(),
            room_id,
            name,
        };

        self.db.upsert_portal(&portal).await?;
        Ok(portal)
    }

    async fn ensure_puppet_joined(&self, room_id: &str, puppet_mxid: &str) {
        if let Err(err) = self.matrix_client.invite_user(room_id, puppet_mxid).await {
            debug!(
                "invite puppet {} to {} ignored: {}",
                puppet_mxid, room_id, err
            );
        }
        if let Err(err) = self.matrix_client.join_room_as(room_id, puppet_mxid).await {
            warn!("join puppet {} to {} failed: {}", puppet_mxid, room_id, err);
        }
    }

    async fn handle_matrix_message_event(&self, event: &RoomEvent) -> anyhow::Result<()> {
        if event.event_type != "m.room.message" {
            return Ok(());
        }

        let Some(room_id) = event.room_id.as_deref() else {
            return Ok(());
        };
        let Some(event_id) = event.event_id.as_deref() else {
            return Ok(());
        };
        let Some(sender) = event.sender.as_deref() else {
            return Ok(());
        };

        if sender == self.config.bot_mxid() || self.config.is_qq_namespace_user(sender) {
            return Ok(());
        }

        let Some(text) = extract_text_content(&event.content) else {
            return Ok(());
        };

        if text.trim().is_empty() {
            return Ok(());
        }

        if text.trim() == format!("{} ping", self.config.bridge.command_prefix) {
            let txn_id = format!("notice-{}", chrono::Utc::now().timestamp_millis());
            if let Err(err) = self
                .matrix_client
                .send_notice(room_id, "pong", &txn_id)
                .await
            {
                warn!("failed to send command response notice: {}", err);
            }
            return Ok(());
        }

        let Some(portal) = self.db.get_portal_by_room(room_id).await? else {
            debug!("matrix room {} is not mapped to qq portal", room_id);
            return Ok(());
        };

        let inserted = self
            .db
            .insert_message_if_absent(
                "matrix",
                event_id,
                room_id,
                &portal.chat_type,
                &portal.chat_id,
            )
            .await?;
        if !inserted {
            debug!("duplicate matrix event {} ignored", event_id);
            return Ok(());
        }

        let qq_message_id = match portal.chat_type.as_str() {
            "private" => {
                self.qq_client
                    .send_private_msg(&portal.chat_id, &text)
                    .await?
            }
            "group" => {
                self.qq_client
                    .send_group_msg(&portal.chat_id, &text)
                    .await?
            }
            _ => {
                warn!(
                    "unknown chat type {} for room {}",
                    portal.chat_type, room_id
                );
                return Ok(());
            }
        };

        if !qq_message_id.is_empty() {
            self.db
                .update_qq_message_id("matrix", event_id, &qq_message_id)
                .await?;
        }

        Ok(())
    }

    fn should_ignore_qq_event(&self, event: &NormalizedQQMessageEvent) -> bool {
        if !self.config.bridge.onebot.ignore_own_messages {
            return false;
        }

        if let Some(config_self_id) = &self.config.bridge.onebot.self_id {
            if &event.sender_id == config_self_id {
                return true;
            }
        }

        if let Some(runtime_self_id) = &event.self_id {
            if &event.sender_id == runtime_self_id {
                return true;
            }
        }

        false
    }

    async fn bridge_qq_to_matrix(&self, event: NormalizedQQMessageEvent) -> anyhow::Result<()> {
        if self.should_ignore_qq_event(&event) {
            debug!("ignore own qq message {}", event.message_id);
            return Ok(());
        }

        let chat_type = event.chat_type.as_str();
        let portal = self.ensure_portal(chat_type, &event.chat_id).await?;

        let inserted = self
            .db
            .insert_message_if_absent(
                "qq",
                &event.message_id,
                &portal.room_id,
                &portal.chat_type,
                &portal.chat_id,
            )
            .await?;

        if !inserted {
            debug!("duplicate qq message {} ignored", event.message_id);
            return Ok(());
        }

        let puppet_mxid = self.config.format_qq_mxid(&event.sender_id);
        let displayname = format!("{} (QQ)", event.sender_name);

        self.db
            .upsert_qq_user(&QQUser {
                qq_user_id: event.sender_id.clone(),
                mxid: puppet_mxid.clone(),
                displayname: displayname.clone(),
                avatar_url: None,
            })
            .await?;

        self.matrix_client
            .ensure_user_profile(&puppet_mxid, &displayname, None)
            .await?;
        self.ensure_puppet_joined(&portal.room_id, &puppet_mxid)
            .await;

        let txn_id = format!("qq-{}", event.message_id);
        let event_id = self
            .matrix_client
            .send_text_as(&portal.room_id, &puppet_mxid, &event.text, &txn_id)
            .await?;

        self.db
            .update_matrix_event_id("qq", &event.message_id, &event_id)
            .await?;

        Ok(())
    }
}

#[async_trait]
impl AppServiceBridge for QQBridge {
    async fn handle_transaction(&self, txn_id: &str, events: Vec<RoomEvent>) -> anyhow::Result<()> {
        let inserted = self.db.mark_transaction_processed(txn_id).await?;
        if !inserted {
            debug!("duplicate matrix transaction {} ignored", txn_id);
            return Ok(());
        }

        for event in events {
            if let Err(err) = self.handle_matrix_message_event(&event).await {
                error!(
                    "failed to process matrix event {:?}: {}",
                    event.event_id, err
                );
            }
        }
        Ok(())
    }

    fn is_user_in_namespace(&self, mxid: &str) -> bool {
        mxid == self.config.bot_mxid() || self.config.is_qq_namespace_user(mxid)
    }
}

#[async_trait]
impl QQEventBridge for QQBridge {
    async fn handle_qq_event(&self, event: NormalizedQQMessageEvent) -> anyhow::Result<()> {
        self.bridge_qq_to_matrix(event).await
    }
}

struct HealthHandler;

#[handler]
impl HealthHandler {
    async fn handle(&self, res: &mut Response) {
        res.render(Json(serde_json::json!({ "ok": true })));
    }
}
