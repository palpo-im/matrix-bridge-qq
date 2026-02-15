use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    #[serde(default)]
    pub events: Vec<RoomEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default)]
    pub content: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_server_ts: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRoomRequest {
    pub visibility: String,
    pub name: String,
    #[serde(default)]
    pub preset: String,
    #[serde(default)]
    pub is_direct: bool,
}

impl CreateRoomRequest {
    pub fn private(name: impl Into<String>, is_direct: bool) -> Self {
        Self {
            visibility: "private".to_string(),
            name: name.into(),
            preset: if is_direct {
                "trusted_private_chat".to_string()
            } else {
                "private_chat".to_string()
            },
            is_direct,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRoomResponse {
    pub room_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixErrorResponse {
    pub errcode: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEventContent {
    pub msgtype: String,
    pub body: String,
}

impl MessageEventContent {
    pub fn text(body: impl Into<String>) -> Self {
        Self {
            msgtype: "m.text".to_string(),
            body: body.into(),
        }
    }
}

pub fn extract_text_content(content: &serde_json::Value) -> Option<String> {
    let msgtype = content.get("msgtype")?.as_str()?;
    if msgtype != "m.text" && msgtype != "m.notice" {
        return None;
    }
    content.get("body")?.as_str().map(ToString::to_string)
}
