use std::sync::Arc;

use super::common::{AttachmentInfo, BridgeMessage, QQSegment, QQSegmentType};
use crate::config::Config;
use crate::qq::types::{OneBotEvent, OneBotMessage, OneBotSegment};

pub struct QQMessageParser;

impl QQMessageParser {
    pub fn new() -> Self {
        Self
    }

    pub fn parse(event: &OneBotEvent) -> Option<Vec<QQSegment>> {
        let message = event.message.as_ref()?;

        match message {
            OneBotMessage::Text(text) => {
                if text.trim().is_empty() {
                    return None;
                }
                Some(vec![QQSegment::text(text)])
            }
            OneBotMessage::Segments(segments) => {
                let parsed: Vec<QQSegment> = segments
                    .iter()
                    .filter_map(|s| Self::parse_segment(s))
                    .collect();
                if parsed.is_empty() {
                    return None;
                }
                Some(parsed)
            }
        }
    }

    fn parse_segment(segment: &OneBotSegment) -> Option<QQSegment> {
        match segment.segment_type.as_str() {
            "text" => {
                let text = segment.data.get("text").and_then(|v| v.as_str())?;
                if text.trim().is_empty() {
                    return None;
                }
                Some(QQSegment::text(text))
            }
            "at" => {
                let qq = segment.data.get("qq").and_then(|v| v.as_str())?;
                if qq == "all" {
                    Some(QQSegment::at_all())
                } else {
                    Some(QQSegment::at(qq))
                }
            }
            "image" => {
                let url = segment
                    .data
                    .get("url")
                    .or_else(|| segment.data.get("file"))
                    .and_then(|v| v.as_str())?;
                Some(QQSegment::image(url))
            }
            "face" => {
                let id = segment.data.get("id").and_then(|v| v.as_i64())? as i32;
                Some(QQSegment::face(id))
            }
            "reply" => {
                let id = segment.data.get("id").and_then(|v| v.as_str())?;
                Some(QQSegment::reply(id))
            }
            "video" | "record" | "file" => Some(QQSegment {
                segment_type: if segment.segment_type == "video" {
                    QQSegmentType::Video
                } else if segment.segment_type == "record" {
                    QQSegmentType::Voice
                } else {
                    QQSegmentType::File
                },
                data: segment.data.clone(),
            }),
            "xml" => Some(QQSegment {
                segment_type: QQSegmentType::Xml,
                data: segment.data.clone(),
            }),
            "json" => Some(QQSegment {
                segment_type: QQSegmentType::Json,
                data: segment.data.clone(),
            }),
            "location" => Some(QQSegment {
                segment_type: QQSegmentType::Location,
                data: segment.data.clone(),
            }),
            "forward" => Some(QQSegment {
                segment_type: QQSegmentType::Forward,
                data: segment.data.clone(),
            }),
            _ => None,
        }
    }

    pub fn segments_to_text(segments: &[QQSegment]) -> String {
        let mut text = String::new();
        for segment in segments {
            match &segment.segment_type {
                QQSegmentType::Text => {
                    if let Some(t) = segment.data.get("text").and_then(|v| v.as_str()) {
                        text.push_str(t);
                    }
                }
                QQSegmentType::At => {
                    if let Some(qq) = segment.data.get("qq").and_then(|v| v.as_str()) {
                        if !text.is_empty() && !text.ends_with(' ') {
                            text.push(' ');
                        }
                        text.push_str(&format!("@{}", qq));
                    }
                }
                QQSegmentType::Face => {
                    if let Some(id) = segment.data.get("id").and_then(|v| v.as_i64()) {
                        text.push_str(&format!("[Face:{}]", id));
                    }
                }
                QQSegmentType::Image => {
                    text.push_str("[Image]");
                }
                QQSegmentType::Voice => {
                    text.push_str("[Voice]");
                }
                QQSegmentType::Video => {
                    text.push_str("[Video]");
                }
                QQSegmentType::File => {
                    text.push_str("[File]");
                }
                QQSegmentType::Reply => {}
                QQSegmentType::Forward => {
                    text.push_str("[Forward]");
                }
                QQSegmentType::Xml | QQSegmentType::Json => {
                    if let Some(content) = segment.data.get("data").and_then(|v| v.as_str()) {
                        text.push_str(&Self::parse_rich_content(content));
                    }
                }
                QQSegmentType::Location => {
                    if let Some(lat) = segment.data.get("lat").and_then(|v| v.as_str()) {
                        if let Some(lng) = segment.data.get("lng").and_then(|v| v.as_str()) {
                            text.push_str(&format!("[Location: {}, {}]", lat, lng));
                        }
                    }
                }
                QQSegmentType::Unknown => {}
            }
        }
        text
    }

    fn parse_rich_content(content: &str) -> String {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
            if let Some(prompt) = json.get("prompt").and_then(|v| v.as_str()) {
                return prompt.to_string();
            }
            if let Some(desc) = json
                .get("meta")
                .and_then(|m| m.as_object())
                .and_then(|m| m.values().next())
                .and_then(|v| v.get("desc"))
                .and_then(|v| v.as_str())
            {
                return desc.to_string();
            }
        }
        String::new()
    }
}

pub struct QQToMatrixConverter {
    config: Arc<Config>,
}

impl QQToMatrixConverter {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }

    pub fn convert_to_matrix_content(&self, segments: &[QQSegment]) -> (String, Option<String>) {
        let plain_text = QQMessageParser::segments_to_text(segments);
        let formatted = self.build_formatted_body(segments);

        (plain_text, Some(formatted))
    }

    fn build_formatted_body(&self, segments: &[QQSegment]) -> String {
        let mut html = String::new();

        for segment in segments {
            match &segment.segment_type {
                QQSegmentType::Text => {
                    if let Some(t) = segment.data.get("text").and_then(|v| v.as_str()) {
                        html.push_str(&html_escape(t));
                    }
                }
                QQSegmentType::At => {
                    if let Some(qq) = segment.data.get("qq").and_then(|v| v.as_str()) {
                        if qq == "all" {
                            html.push_str("<a href=\"https://matrix.to/#/@room\">@room</a>");
                        } else {
                            let mxid = self.config.format_qq_mxid(qq);
                            html.push_str(&format!(
                                "<a href=\"https://matrix.to/#/{}\">@{}</a>",
                                mxid, qq
                            ));
                        }
                    }
                }
                QQSegmentType::Face => {
                    if let Some(id) = segment.data.get("id").and_then(|v| v.as_i64()) {
                        html.push_str(&format!("[Face:{}]", id));
                    }
                }
                QQSegmentType::Image => {
                    html.push_str("[Image]");
                }
                QQSegmentType::Voice => {
                    html.push_str("[Voice]");
                }
                QQSegmentType::Video => {
                    html.push_str("[Video]");
                }
                QQSegmentType::File => {
                    if let Some(name) = segment.data.get("file").and_then(|v| v.as_str()) {
                        html.push_str(&format!("[File: {}]", html_escape(name)));
                    } else {
                        html.push_str("[File]");
                    }
                }
                QQSegmentType::Reply => {}
                QQSegmentType::Forward => {
                    html.push_str("[Forward]");
                }
                QQSegmentType::Xml | QQSegmentType::Json => {
                    if let Some(content) = segment.data.get("data").and_then(|v| v.as_str()) {
                        html.push_str(&html_escape(&QQMessageParser::parse_rich_content(content)));
                    }
                }
                QQSegmentType::Location => {
                    if let (Some(lat), Some(lng)) = (
                        segment.data.get("lat").and_then(|v| v.as_str()),
                        segment.data.get("lng").and_then(|v| v.as_str()),
                    ) {
                        html.push_str(&format!(
                            "<a href=\"https://maps.google.com/?q={},{}\">Location: {}, {}</a>",
                            lat, lng, lat, lng
                        ));
                    }
                }
                QQSegmentType::Unknown => {}
            }
        }

        html
    }

    pub fn extract_attachments(segments: &[QQSegment]) -> Vec<AttachmentInfo> {
        let mut attachments = Vec::new();

        for segment in segments {
            match &segment.segment_type {
                QQSegmentType::Image => {
                    if let Some(url) = segment
                        .data
                        .get("url")
                        .or_else(|| segment.data.get("file"))
                        .and_then(|v| v.as_str())
                    {
                        attachments.push(AttachmentInfo {
                            url: url.to_string(),
                            file_name: "image".to_string(),
                            mime_type: Some("image/png".to_string()),
                            size: None,
                        });
                    }
                }
                QQSegmentType::Video => {
                    if let Some(url) = segment.data.get("url").and_then(|v| v.as_str()) {
                        attachments.push(AttachmentInfo {
                            url: url.to_string(),
                            file_name: "video.mp4".to_string(),
                            mime_type: Some("video/mp4".to_string()),
                            size: None,
                        });
                    }
                }
                QQSegmentType::Voice => {
                    if let Some(url) = segment.data.get("url").and_then(|v| v.as_str()) {
                        attachments.push(AttachmentInfo {
                            url: url.to_string(),
                            file_name: "voice.mp3".to_string(),
                            mime_type: Some("audio/mpeg".to_string()),
                            size: None,
                        });
                    }
                }
                QQSegmentType::File => {
                    if let Some(url) = segment.data.get("url").and_then(|v| v.as_str()) {
                        let name = segment
                            .data
                            .get("file")
                            .and_then(|v| v.as_str())
                            .unwrap_or("file");
                        attachments.push(AttachmentInfo {
                            url: url.to_string(),
                            file_name: name.to_string(),
                            mime_type: None,
                            size: None,
                        });
                    }
                }
                _ => {}
            }
        }

        attachments
    }

    pub fn extract_reply_to(segments: &[QQSegment]) -> Option<String> {
        for segment in segments {
            if segment.segment_type == QQSegmentType::Reply {
                return segment
                    .data
                    .get("id")
                    .and_then(|v| v.as_str())
                    .map(ToOwned::to_owned);
            }
        }
        None
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

impl Default for QQMessageParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_config() -> Arc<Config> {
        Arc::new(Config {
            homeserver: crate::config::HomeserverConfig {
                address: "http://localhost:8008".to_string(),
                domain: "example.org".to_string(),
            },
            appservice: crate::config::AppServiceConfig {
                address: "http://localhost:9999".to_string(),
                hostname: "0.0.0.0".to_string(),
                port: 9999,
                database: crate::config::DatabaseConfig {
                    r#type: "sqlite".to_string(),
                    uri: ":memory:".to_string(),
                    max_open_conns: 10,
                    max_idle_conns: 2,
                },
                id: "qq".to_string(),
                bot: crate::config::BotConfig {
                    username: "qqbot".to_string(),
                    displayname: None,
                    avatar: None,
                },
                as_token: "token".to_string(),
                hs_token: "token".to_string(),
            },
            bridge: crate::config::BridgeConfig {
                username_template: "_qq_{{.}}".to_string(),
                command_prefix: "!qq".to_string(),
                private_room_name_template: "QQ Private {{chat_id}}".to_string(),
                group_room_name_template: "QQ Group {{chat_id}}".to_string(),
                onebot: crate::config::OneBotConfig {
                    api_base: "http://localhost:5700".to_string(),
                    event_path: "/qq/events".to_string(),
                    listen_secret: None,
                    access_token: None,
                    self_id: None,
                    ignore_own_messages: true,
                },
                permissions: std::collections::HashMap::from([(
                    "*".to_string(),
                    "admin".to_string(),
                )]),
            },
            logging: None,
        })
    }

    #[test]
    fn parses_text_segment() {
        let segment = OneBotSegment {
            segment_type: "text".to_string(),
            data: {
                let mut map = serde_json::Map::new();
                map.insert("text".to_string(), json!("Hello world"));
                map
            },
        };
        let parsed = QQMessageParser::parse_segment(&segment);
        assert!(parsed.is_some());
        let parsed = parsed.unwrap();
        assert!(matches!(parsed.segment_type, QQSegmentType::Text));
    }

    #[test]
    fn parses_at_segment() {
        let segment = OneBotSegment {
            segment_type: "at".to_string(),
            data: {
                let mut map = serde_json::Map::new();
                map.insert("qq".to_string(), json!("12345"));
                map
            },
        };
        let parsed = QQMessageParser::parse_segment(&segment);
        assert!(parsed.is_some());
        let parsed = parsed.unwrap();
        assert!(matches!(parsed.segment_type, QQSegmentType::At));
    }

    #[test]
    fn converts_segments_to_text() {
        let segments = vec![
            QQSegment::text("Hello "),
            QQSegment::at("12345"),
            QQSegment::text("!"),
        ];
        let text = QQMessageParser::segments_to_text(&segments);
        assert_eq!(text, "Hello @12345!");
    }
}
