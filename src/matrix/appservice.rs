use std::sync::Arc;

use async_trait::async_trait;
use salvo::prelude::*;
use tracing::{debug, error, warn};

use super::types::{RoomEvent, Transaction};

#[async_trait]
pub trait AppServiceBridge: Send + Sync {
    async fn handle_transaction(&self, txn_id: &str, events: Vec<RoomEvent>) -> anyhow::Result<()>;
    fn is_user_in_namespace(&self, mxid: &str) -> bool;
}

pub struct AppService {
    hs_token: String,
    bridge: Arc<dyn AppServiceBridge>,
}

impl AppService {
    pub fn new(hs_token: impl Into<String>, bridge: Arc<dyn AppServiceBridge>) -> Self {
        Self {
            hs_token: hs_token.into(),
            bridge,
        }
    }

    pub fn router(self: Arc<Self>) -> Router {
        Router::new()
            .push(
                Router::with_path("/_matrix/app/v1/transactions/<txn_id>").put(
                    TransactionHandler {
                        appservice: self.clone(),
                    },
                ),
            )
            .push(
                Router::with_path("/_matrix/app/v1/users/<user_id>").get(UserQueryHandler {
                    appservice: self.clone(),
                }),
            )
            .push(
                Router::with_path("/_matrix/app/v1/rooms/<room_alias>")
                    .get(RoomQueryHandler { appservice: self }),
            )
    }

    fn verify_auth(&self, req: &Request) -> bool {
        if let Some(token) = req.query::<String>("access_token") {
            if token == self.hs_token {
                return true;
            }
        }

        match req.header::<String>("Authorization") {
            Some(header) if header.starts_with("Bearer ") => header[7..] == self.hs_token,
            _ => false,
        }
    }
}

struct TransactionHandler {
    appservice: Arc<AppService>,
}

#[handler]
impl TransactionHandler {
    async fn handle(&self, req: &mut Request, depot: &mut Depot, res: &mut Response) {
        if !self.appservice.verify_auth(req) {
            res.status_code(StatusCode::UNAUTHORIZED);
            res.render(Json(serde_json::json!({ "error": "unauthorized" })));
            return;
        }

        let txn_id = depot
            .get::<String>("txn_id")
            .map(std::string::String::as_str)
            .unwrap_or_default();

        let body = req.payload().await;
        let payload = match body {
            Ok(bytes) => bytes,
            Err(err) => {
                warn!("failed to read transaction body: {err}");
                res.render(Json(serde_json::json!({})));
                return;
            }
        };

        let txn: Transaction = match serde_json::from_slice(payload) {
            Ok(txn) => txn,
            Err(err) => {
                warn!("failed to parse transaction {txn_id}: {err}");
                res.render(Json(serde_json::json!({})));
                return;
            }
        };

        debug!(
            "received matrix txn {txn_id} with {} events",
            txn.events.len()
        );
        if let Err(err) = self
            .appservice
            .bridge
            .handle_transaction(txn_id, txn.events)
            .await
        {
            error!("failed handling matrix txn {txn_id}: {err}");
        }

        res.render(Json(serde_json::json!({})));
    }
}

struct UserQueryHandler {
    appservice: Arc<AppService>,
}

#[handler]
impl UserQueryHandler {
    async fn handle(&self, req: &mut Request, depot: &mut Depot, res: &mut Response) {
        if !self.appservice.verify_auth(req) {
            res.status_code(StatusCode::UNAUTHORIZED);
            res.render(Json(serde_json::json!({ "error": "unauthorized" })));
            return;
        }

        let user_id = depot
            .get::<String>("user_id")
            .map(std::string::String::as_str)
            .unwrap_or_default();

        if self.appservice.bridge.is_user_in_namespace(user_id) {
            res.render(Json(serde_json::json!({})));
        } else {
            res.status_code(StatusCode::NOT_FOUND);
            res.render(Json(serde_json::json!({ "error": "not found" })));
        }
    }
}

struct RoomQueryHandler {
    appservice: Arc<AppService>,
}

#[handler]
impl RoomQueryHandler {
    async fn handle(&self, req: &mut Request, _depot: &mut Depot, res: &mut Response) {
        if !self.appservice.verify_auth(req) {
            res.status_code(StatusCode::UNAUTHORIZED);
            res.render(Json(serde_json::json!({ "error": "unauthorized" })));
            return;
        }

        res.status_code(StatusCode::NOT_FOUND);
        res.render(Json(serde_json::json!({ "error": "not found" })));
    }
}
