use anyhow::{anyhow, Result};
use reqwest::Client;

use super::types::{OneBotApiResponse, SendMessageData};

#[derive(Clone)]
pub struct QQClient {
    api_base: String,
    access_token: Option<String>,
    http: Client,
}

impl QQClient {
    pub fn new(api_base: impl Into<String>, access_token: Option<String>) -> Self {
        Self {
            api_base: api_base.into(),
            access_token,
            http: Client::new(),
        }
    }

    pub async fn send_private_msg(&self, user_id: &str, message: &str) -> Result<String> {
        let payload = serde_json::json!({
            "user_id": parse_num_or_string(user_id),
            "message": message,
        });
        let resp: OneBotApiResponse<SendMessageData> =
            self.call_api("send_private_msg", &payload).await?;
        let message_id = resp
            .data
            .as_ref()
            .map(SendMessageData::message_id_string)
            .unwrap_or_default();
        Ok(message_id)
    }

    pub async fn send_group_msg(&self, group_id: &str, message: &str) -> Result<String> {
        let payload = serde_json::json!({
            "group_id": parse_num_or_string(group_id),
            "message": message,
        });
        let resp: OneBotApiResponse<SendMessageData> =
            self.call_api("send_group_msg", &payload).await?;
        let message_id = resp
            .data
            .as_ref()
            .map(SendMessageData::message_id_string)
            .unwrap_or_default();
        Ok(message_id)
    }

    async fn call_api<T: serde::de::DeserializeOwned>(
        &self,
        action: &str,
        payload: &serde_json::Value,
    ) -> Result<T> {
        let url = format!(
            "{}/{}",
            self.api_base.trim_end_matches('/'),
            action.trim_start_matches('/'),
        );

        let mut req = self.http.post(url).json(payload);
        if let Some(token) = &self.access_token {
            req = req.bearer_auth(token);
        }

        let resp = req.send().await?;
        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            return Err(anyhow!("qq api request failed {status}: {text}"));
        }

        let parsed: OneBotApiResponse<serde_json::Value> = serde_json::from_str(&text)
            .map_err(|e| anyhow!("invalid qq api response: {e}; body={text}"))?;

        if parsed.status.as_deref() != Some("ok") || parsed.retcode.unwrap_or(-1) != 0 {
            return Err(anyhow!(
                "qq api returned error: status={:?} retcode={:?} wording={:?}",
                parsed.status,
                parsed.retcode,
                parsed.wording
            ));
        }

        serde_json::from_str(&text)
            .map_err(|e| anyhow!("failed to decode qq api response: {e}; body={text}"))
    }
}

fn parse_num_or_string(input: &str) -> serde_json::Value {
    if let Ok(v) = input.parse::<i64>() {
        serde_json::json!(v)
    } else {
        serde_json::json!(input)
    }
}
