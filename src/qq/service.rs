use std::sync::Arc;

use async_trait::async_trait;
use hmac::{Hmac, Mac};
use salvo::prelude::*;
use sha1::Sha1;
use tracing::{debug, error, warn};

use super::types::{NormalizedQQMessageEvent, OneBotEvent};

type HmacSha1 = Hmac<Sha1>;

#[async_trait]
pub trait QQEventBridge: Send + Sync {
    async fn handle_qq_event(&self, event: NormalizedQQMessageEvent) -> anyhow::Result<()>;
}

pub struct QQWebhookService {
    event_path: String,
    listen_secret: Option<String>,
    bridge: Arc<dyn QQEventBridge>,
}

impl QQWebhookService {
    pub fn new(
        event_path: impl Into<String>,
        listen_secret: Option<String>,
        bridge: Arc<dyn QQEventBridge>,
    ) -> Self {
        Self {
            event_path: event_path.into(),
            listen_secret,
            bridge,
        }
    }

    pub fn router(self: Arc<Self>) -> Router {
        let route = self.event_path.trim_start_matches('/').to_string();
        Router::new().push(Router::with_path(route).post(QQEventHandler { service: self }))
    }

    fn verify_signature(&self, payload: &[u8], req: &Request) -> bool {
        let Some(secret) = &self.listen_secret else {
            return true;
        };

        let Some(signature) = req.header::<String>("X-Signature") else {
            return false;
        };
        let Some(signature_hex) = signature.strip_prefix("sha1=") else {
            return false;
        };

        let mut mac = match HmacSha1::new_from_slice(secret.as_bytes()) {
            Ok(mac) => mac,
            Err(_) => return false,
        };
        mac.update(payload);
        let expected = hex::encode(mac.finalize().into_bytes());

        expected.eq_ignore_ascii_case(signature_hex)
    }
}

struct QQEventHandler {
    service: Arc<QQWebhookService>,
}

#[handler]
impl QQEventHandler {
    async fn handle(&self, req: &mut Request, res: &mut Response) {
        let payload = match req.payload().await {
            Ok(payload) => payload.clone(),
            Err(err) => {
                warn!("failed to read qq event payload: {err}");
                res.status_code(StatusCode::BAD_REQUEST);
                res.render(Json(serde_json::json!({ "error": "invalid payload" })));
                return;
            }
        };

        if !self.service.verify_signature(&payload, req) {
            res.status_code(StatusCode::UNAUTHORIZED);
            res.render(Json(serde_json::json!({ "error": "invalid signature" })));
            return;
        }

        let event: OneBotEvent = match serde_json::from_slice(&payload) {
            Ok(event) => event,
            Err(err) => {
                warn!("failed to parse qq event json: {err}");
                res.render(Json(serde_json::json!({ "ok": true })));
                return;
            }
        };

        let Some(normalized) = event.to_normalized_message_event() else {
            debug!("ignored non-message qq event");
            res.render(Json(serde_json::json!({ "ok": true })));
            return;
        };

        if let Err(err) = self.service.bridge.handle_qq_event(normalized).await {
            error!("failed to handle qq event: {err}");
        }

        res.render(Json(serde_json::json!({ "ok": true })));
    }
}
