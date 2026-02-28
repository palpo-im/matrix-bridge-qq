use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub enum MessageType {
    Text,
    Image,
    Video,
    Audio,
    File,
    Location,
    Sticker,
    Notice,
    Emote,
    Reply,
    Edit,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ParsedMessage {
    pub msg_type: MessageType,
    pub body: String,
    pub formatted_body: Option<String>,
    pub url: Option<String>,
    pub file_name: Option<String>,
    pub file_size: Option<usize>,
    pub mime_type: Option<String>,
    pub reply_to: Option<String>,
    pub edit_of: Option<String>,
    pub mentions: Vec<String>,
    pub geo_uri: Option<String>,
}

impl ParsedMessage {
    pub fn new(body: &str) -> Self {
        Self {
            msg_type: MessageType::Text,
            body: body.to_string(),
            formatted_body: None,
            url: None,
            file_name: None,
            file_size: None,
            mime_type: None,
            reply_to: None,
            edit_of: None,
            mentions: Vec::new(),
            geo_uri: None,
        }
    }

    pub fn with_type(mut self, msg_type: MessageType) -> Self {
        self.msg_type = msg_type;
        self
    }

    pub fn with_formatted_body(mut self, formatted: &str) -> Self {
        self.formatted_body = Some(formatted.to_string());
        self
    }

    pub fn with_url(mut self, url: &str) -> Self {
        self.url = Some(url.to_string());
        self
    }

    pub fn with_file_name(mut self, name: &str) -> Self {
        self.file_name = Some(name.to_string());
        self
    }

    pub fn with_file_size(mut self, size: usize) -> Self {
        self.file_size = Some(size);
        self
    }

    pub fn with_mime_type(mut self, mime: &str) -> Self {
        self.mime_type = Some(mime.to_string());
        self
    }

    pub fn with_reply_to(mut self, event_id: &str) -> Self {
        self.reply_to = Some(event_id.to_string());
        self
    }

    pub fn with_edit_of(mut self, event_id: &str) -> Self {
        self.edit_of = Some(event_id.to_string());
        self
    }

    pub fn with_mentions(mut self, mentions: Vec<String>) -> Self {
        self.mentions = mentions;
        self
    }

    pub fn with_geo_uri(mut self, uri: &str) -> Self {
        self.geo_uri = Some(uri.to_string());
        self
    }
}

#[derive(Debug, Clone)]
pub struct BridgeMessage {
    pub source_platform: String,
    pub target_platform: String,
    pub source_id: String,
    pub target_id: String,
    pub content: String,
    pub formatted_content: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub attachments: Vec<AttachmentInfo>,
    pub reply_to: Option<String>,
    pub edit_of: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AttachmentInfo {
    pub url: String,
    pub file_name: String,
    pub mime_type: Option<String>,
    pub size: Option<usize>,
}

impl BridgeMessage {
    pub fn new(
        source_platform: &str,
        target_platform: &str,
        source_id: &str,
        target_id: &str,
        content: &str,
    ) -> Self {
        Self {
            source_platform: source_platform.to_string(),
            target_platform: target_platform.to_string(),
            source_id: source_id.to_string(),
            target_id: target_id.to_string(),
            content: content.to_string(),
            formatted_content: None,
            timestamp: Utc::now(),
            attachments: Vec::new(),
            reply_to: None,
            edit_of: None,
        }
    }

    pub fn with_attachment(mut self, attachment: AttachmentInfo) -> Self {
        self.attachments.push(attachment);
        self
    }

    pub fn with_reply_to(mut self, event_id: &str) -> Self {
        self.reply_to = Some(event_id.to_string());
        self
    }

    pub fn with_edit_of(mut self, event_id: &str) -> Self {
        self.edit_of = Some(event_id.to_string());
        self
    }
}

#[derive(Debug, Clone)]
pub struct QQSegment {
    pub segment_type: QQSegmentType,
    pub data: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub enum QQSegmentType {
    Text,
    At,
    Image,
    Voice,
    Video,
    File,
    Face,
    Reply,
    Forward,
    Xml,
    Json,
    Location,
    Unknown,
}

impl QQSegment {
    pub fn text(content: &str) -> Self {
        let mut data = serde_json::Map::new();
        data.insert("text".to_string(), serde_json::json!(content));
        Self {
            segment_type: QQSegmentType::Text,
            data,
        }
    }

    pub fn at(user_id: &str) -> Self {
        let mut data = serde_json::Map::new();
        data.insert("qq".to_string(), serde_json::json!(user_id));
        Self {
            segment_type: QQSegmentType::At,
            data,
        }
    }

    pub fn at_all() -> Self {
        let mut data = serde_json::Map::new();
        data.insert("qq".to_string(), serde_json::json!("all"));
        Self {
            segment_type: QQSegmentType::At,
            data,
        }
    }

    pub fn image(url: &str) -> Self {
        let mut data = serde_json::Map::new();
        data.insert("url".to_string(), serde_json::json!(url));
        Self {
            segment_type: QQSegmentType::Image,
            data,
        }
    }

    pub fn face(id: i32) -> Self {
        let mut data = serde_json::Map::new();
        data.insert("id".to_string(), serde_json::json!(id));
        Self {
            segment_type: QQSegmentType::Face,
            data,
        }
    }

    pub fn reply(message_id: &str) -> Self {
        let mut data = serde_json::Map::new();
        data.insert("id".to_string(), serde_json::json!(message_id));
        Self {
            segment_type: QQSegmentType::Reply,
            data,
        }
    }
}
