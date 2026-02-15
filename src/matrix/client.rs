use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::de::DeserializeOwned;
use tracing::debug;

use super::types::{
    CreateRoomRequest, CreateRoomResponse, MatrixErrorResponse, MessageEventContent,
};

#[derive(Clone)]
pub struct MatrixClient {
    homeserver: String,
    as_token: String,
    http: Client,
}

impl MatrixClient {
    pub fn new(homeserver: impl Into<String>, as_token: impl Into<String>) -> Self {
        Self {
            homeserver: homeserver.into(),
            as_token: as_token.into(),
            http: Client::new(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.homeserver.trim_end_matches('/'), path)
    }

    fn with_user_id_query(path: &str, user_id: Option<&str>) -> String {
        if let Some(user_id) = user_id {
            let delimiter = if path.contains('?') { '&' } else { '?' };
            format!("{path}{delimiter}user_id={}", urlencoding::encode(user_id))
        } else {
            path.to_string()
        }
    }

    async fn request<T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<T> {
        let url = self.url(path);
        let mut req = self
            .http
            .request(method.clone(), &url)
            .bearer_auth(&self.as_token);

        if let Some(body) = body {
            req = req.json(body);
        }

        debug!("matrix request: {:?} {}", method, url);
        let resp = req.send().await?;
        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            if let Ok(err) = serde_json::from_str::<MatrixErrorResponse>(&text) {
                return Err(anyhow!("matrix error {}: {}", err.errcode, err.error));
            }
            return Err(anyhow!("matrix request failed {status}: {text}"));
        }

        if text.trim().is_empty() {
            return serde_json::from_str("{}").map_err(Into::into);
        }

        serde_json::from_str(&text)
            .map_err(|e| anyhow!("invalid matrix response: {e}; body={text}"))
    }

    pub async fn create_private_room(&self, name: &str, is_direct: bool) -> Result<String> {
        let path = "/_matrix/client/v3/createRoom";
        let payload = serde_json::to_value(CreateRoomRequest::private(name, is_direct))?;
        let resp: CreateRoomResponse = self
            .request(reqwest::Method::POST, path, Some(&payload))
            .await?;
        Ok(resp.room_id)
    }

    pub async fn invite_user(&self, room_id: &str, user_id: &str) -> Result<()> {
        let path = format!(
            "/_matrix/client/v3/rooms/{}/invite",
            urlencoding::encode(room_id)
        );
        let payload = serde_json::json!({ "user_id": user_id });
        let _: serde_json::Value = self
            .request(reqwest::Method::POST, &path, Some(&payload))
            .await?;
        Ok(())
    }

    pub async fn join_room_as(&self, room_id: &str, user_id: &str) -> Result<()> {
        let base = format!("/_matrix/client/v3/join/{}", urlencoding::encode(room_id));
        let path = Self::with_user_id_query(&base, Some(user_id));
        let payload = serde_json::json!({});
        let _: serde_json::Value = self
            .request(reqwest::Method::POST, &path, Some(&payload))
            .await?;
        Ok(())
    }

    pub async fn ensure_user_profile(
        &self,
        user_id: &str,
        displayname: &str,
        avatar_url: Option<&str>,
    ) -> Result<()> {
        let path = Self::with_user_id_query(
            &format!(
                "/_matrix/client/v3/profile/{}/displayname",
                urlencoding::encode(user_id)
            ),
            Some(user_id),
        );
        let payload = serde_json::json!({ "displayname": displayname });
        let _: serde_json::Value = self
            .request(reqwest::Method::PUT, &path, Some(&payload))
            .await?;

        if let Some(url) = avatar_url {
            let avatar_path = Self::with_user_id_query(
                &format!(
                    "/_matrix/client/v3/profile/{}/avatar_url",
                    urlencoding::encode(user_id)
                ),
                Some(user_id),
            );
            let payload = serde_json::json!({ "avatar_url": url });
            let _: serde_json::Value = self
                .request(reqwest::Method::PUT, &avatar_path, Some(&payload))
                .await?;
        }

        Ok(())
    }

    pub async fn send_text_as(
        &self,
        room_id: &str,
        sender_user_id: &str,
        text: &str,
        txn_id: &str,
    ) -> Result<String> {
        let base = format!(
            "/_matrix/client/v3/rooms/{}/send/m.room.message/{}",
            urlencoding::encode(room_id),
            urlencoding::encode(txn_id)
        );
        let path = Self::with_user_id_query(&base, Some(sender_user_id));
        let content = serde_json::to_value(MessageEventContent::text(text))?;
        let resp: serde_json::Value = self
            .request(reqwest::Method::PUT, &path, Some(&content))
            .await?;
        resp.get("event_id")
            .and_then(|v| v.as_str())
            .map(ToString::to_string)
            .ok_or_else(|| anyhow!("no event_id returned from matrix"))
    }

    pub async fn send_notice(&self, room_id: &str, text: &str, txn_id: &str) -> Result<String> {
        let path = format!(
            "/_matrix/client/v3/rooms/{}/send/m.room.message/{}",
            urlencoding::encode(room_id),
            urlencoding::encode(txn_id)
        );
        let content = serde_json::json!({
            "msgtype": "m.notice",
            "body": text,
        });
        let resp: serde_json::Value = self
            .request(reqwest::Method::PUT, &path, Some(&content))
            .await?;
        resp.get("event_id")
            .and_then(|v| v.as_str())
            .map(ToString::to_string)
            .ok_or_else(|| anyhow!("no event_id returned from matrix"))
    }
}
