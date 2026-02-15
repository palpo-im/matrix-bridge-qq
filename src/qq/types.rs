use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OneBotEvent {
    pub post_type: Option<String>,
    pub message_type: Option<String>,
    pub message: Option<OneBotMessage>,
    pub raw_message: Option<String>,
    pub message_id: Option<serde_json::Value>,
    pub user_id: Option<serde_json::Value>,
    pub group_id: Option<serde_json::Value>,
    pub self_id: Option<serde_json::Value>,
    pub sender: Option<OneBotSender>,
    pub time: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OneBotMessage {
    Text(String),
    Segments(Vec<OneBotSegment>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OneBotSegment {
    #[serde(rename = "type")]
    pub segment_type: String,
    #[serde(default)]
    pub data: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OneBotSender {
    pub nickname: Option<String>,
    pub card: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ChatType {
    Private,
    Group,
}

impl ChatType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChatType::Private => "private",
            ChatType::Group => "group",
        }
    }
}

#[derive(Debug, Clone)]
pub struct NormalizedQQMessageEvent {
    pub chat_type: ChatType,
    pub chat_id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub message_id: String,
    pub text: String,
    pub self_id: Option<String>,
}

impl OneBotEvent {
    pub fn to_normalized_message_event(&self) -> Option<NormalizedQQMessageEvent> {
        if self.post_type.as_deref()? != "message" {
            return None;
        }

        let message_type = self.message_type.as_deref()?;
        let sender_id = value_to_string(self.user_id.as_ref()?)?;
        let message_id = value_to_string(self.message_id.as_ref()?)?;
        let self_id = self.self_id.as_ref().and_then(value_to_string);

        let chat_type = match message_type {
            "private" => ChatType::Private,
            "group" => ChatType::Group,
            _ => return None,
        };

        let chat_id = match chat_type {
            ChatType::Private => sender_id.clone(),
            ChatType::Group => value_to_string(self.group_id.as_ref()?)?,
        };

        let sender_name = self
            .sender
            .as_ref()
            .and_then(|s| {
                s.card
                    .as_deref()
                    .filter(|s| !s.trim().is_empty())
                    .map(ToString::to_string)
                    .or_else(|| s.nickname.clone())
            })
            .unwrap_or_else(|| sender_id.clone());

        let text = extract_text(self);
        if text.trim().is_empty() {
            return None;
        }

        Some(NormalizedQQMessageEvent {
            chat_type,
            chat_id,
            sender_id,
            sender_name,
            message_id,
            text,
            self_id,
        })
    }
}

fn value_to_string(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(s) => Some(s.to_string()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn extract_text(event: &OneBotEvent) -> String {
    if let Some(raw) = &event.raw_message {
        if !raw.trim().is_empty() {
            return raw.to_string();
        }
    }

    match event.message.as_ref() {
        Some(OneBotMessage::Text(s)) => s.clone(),
        Some(OneBotMessage::Segments(segs)) => {
            let mut out = String::new();
            for seg in segs {
                match seg.segment_type.as_str() {
                    "text" => {
                        if let Some(serde_json::Value::String(text)) = seg.data.get("text") {
                            out.push_str(text);
                        }
                    }
                    "at" => {
                        if let Some(target) = seg.data.get("qq").and_then(value_to_string) {
                            if !out.is_empty() {
                                out.push(' ');
                            }
                            out.push('@');
                            out.push_str(&target);
                        }
                    }
                    _ => {}
                }
            }
            out
        }
        None => String::new(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OneBotApiResponse<T> {
    pub status: Option<String>,
    pub retcode: Option<i64>,
    pub data: Option<T>,
    pub wording: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageData {
    pub message_id: serde_json::Value,
}

impl SendMessageData {
    pub fn message_id_string(&self) -> String {
        match &self.message_id {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            _ => String::new(),
        }
    }
}
