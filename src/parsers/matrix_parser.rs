use regex::Regex;
use serde_json::Value;
use std::sync::Arc;

use super::MessageUtils;
use super::common::{AttachmentInfo, BridgeMessage, MessageType, ParsedMessage};
use crate::config::Config;

pub struct MatrixMessageParser;

impl MatrixMessageParser {
    pub fn new() -> Self {
        Self
    }

    pub fn parse(content: &Value) -> ParsedMessage {
        let obj = content.as_object();

        let body = MessageUtils::extract_plain_text(content);
        let formatted_body = obj
            .and_then(|o| o.get("formatted_body"))
            .and_then(|v| v.as_str())
            .map(ToString::to_string);

        let msg_type = MessageUtils::get_msgtype(content);
        let parsed_type = Self::determine_message_type(&msg_type, content);

        let url = obj
            .and_then(|o| o.get("url"))
            .and_then(|v| v.as_str())
            .map(ToString::to_string);

        let file_name = obj
            .and_then(|o| o.get("filename"))
            .or_else(|| obj.and_then(|o| o.get("body")))
            .and_then(|v| v.as_str())
            .map(ToString::to_string);

        let info = obj.and_then(|o| o.get("info")).and_then(|v| v.as_object());
        let mime_type = info
            .and_then(|i| i.get("mimetype"))
            .and_then(|v| v.as_str())
            .map(ToString::to_string);
        let file_size = info
            .and_then(|i| i.get("size"))
            .and_then(|v| v.as_u64())
            .map(|s| s as usize);

        let reply_to = MessageUtils::extract_reply_info(content);
        let edit_of = MessageUtils::extract_edit_info(content);

        let geo_uri = obj
            .and_then(|o| o.get("geo_uri"))
            .and_then(|v| v.as_str())
            .map(ToString::to_string);

        let mentions = Self::extract_mentions(content);

        let mut msg = ParsedMessage::new(&body)
            .with_type(parsed_type)
            .with_mentions(mentions);

        if let Some(fb) = formatted_body {
            msg = msg.with_formatted_body(&fb);
        }
        if let Some(u) = url {
            msg = msg.with_url(&u);
        }
        if let Some(fn_) = file_name {
            msg = msg.with_file_name(&fn_);
        }
        if let Some(fs) = file_size {
            msg = msg.with_file_size(fs);
        }
        if let Some(mt) = mime_type {
            msg = msg.with_mime_type(&mt);
        }
        if let Some(rt) = reply_to {
            msg = msg.with_reply_to(&rt);
        }
        if let Some(eo) = edit_of {
            msg = msg.with_edit_of(&eo);
        }
        if let Some(gu) = geo_uri {
            msg = msg.with_geo_uri(&gu);
        }

        msg
    }

    fn determine_message_type(msg_type: &Option<String>, content: &Value) -> MessageType {
        match msg_type.as_deref() {
            Some("m.text") => MessageType::Text,
            Some("m.notice") => MessageType::Notice,
            Some("m.emote") => MessageType::Emote,
            Some("m.image") => MessageType::Image,
            Some("m.video") => MessageType::Video,
            Some("m.audio") => MessageType::Audio,
            Some("m.file") => MessageType::File,
            Some("m.location") => MessageType::Location,
            Some("m.sticker") => MessageType::Sticker,
            _ => {
                if MessageUtils::extract_reply_info(content).is_some() {
                    return MessageType::Reply;
                }
                if MessageUtils::extract_edit_info(content).is_some() {
                    return MessageType::Edit;
                }
                MessageType::Unknown
            }
        }
    }

    fn extract_mentions(content: &Value) -> Vec<String> {
        let mut mentions = Vec::new();

        if let Some(obj) = content.as_object() {
            if let Some(m) = obj.get("m.mentions").and_then(|v| v.as_object()) {
                if let Some(user_ids) = m.get("user_ids").and_then(|v| v.as_array()) {
                    for uid in user_ids {
                        if let Some(id) = uid.as_str() {
                            mentions.push(id.to_string());
                        }
                    }
                }
            }
        }

        mentions
    }
}

pub struct MatrixToQQConverter {
    config: Arc<Config>,
    ghost_user_regex: Regex,
}

impl MatrixToQQConverter {
    pub fn new(config: Arc<Config>) -> Self {
        let pattern = Self::build_ghost_pattern(&config);
        let ghost_user_regex =
            Regex::new(&pattern).unwrap_or_else(|_| Regex::new(r"@_qq_\d+:[^:]+").unwrap());

        Self {
            config,
            ghost_user_regex,
        }
    }

    fn build_ghost_pattern(config: &Config) -> String {
        let (prefix, suffix) = config.template_parts();
        let escaped_prefix = regex::escape(prefix);
        let escaped_suffix = regex::escape(suffix);
        format!(r"@{}(\d+){}:[^:]+", escaped_prefix, escaped_suffix)
    }

    pub fn format_for_qq(&self, message: &str) -> String {
        let mut result = message.to_string();
        result = self.convert_ghost_users_to_qq(&result);
        result
    }

    pub fn format_html_for_qq(&self, html: &str) -> String {
        let mut result = MessageUtils::convert_html_to_qq_markdown(html);
        result = self.format_for_qq(&result);
        result
    }

    fn convert_ghost_users_to_qq(&self, text: &str) -> String {
        self.ghost_user_regex
            .replace_all(text, |caps: &regex::Captures| {
                let user_id = &caps[1];
                format!("[CQ:at,qq={}]", user_id)
            })
            .to_string()
    }

    pub fn convert_parsed_message(&self, parsed: &ParsedMessage) -> Vec<super::common::QQSegment> {
        let mut segments = Vec::new();

        if let Some(ref reply_event_id) = parsed.reply_to {
            segments.push(super::common::QQSegment::reply(reply_event_id));
        }

        match parsed.msg_type {
            MessageType::Text | MessageType::Notice | MessageType::Emote => {
                let content = if let Some(ref formatted) = parsed.formatted_body {
                    self.format_html_for_qq(formatted)
                } else {
                    self.format_for_qq(&parsed.body)
                };

                let text = if parsed.msg_type == MessageType::Emote {
                    format!("* {}", content)
                } else {
                    content
                };
                segments.push(super::common::QQSegment::text(&text));
            }
            MessageType::Image => {
                if let Some(ref url) = parsed.url {
                    segments.push(super::common::QQSegment::image(url));
                }
                if !parsed.body.is_empty() && parsed.body != "image" {
                    segments.push(super::common::QQSegment::text(&parsed.body));
                }
            }
            MessageType::Video | MessageType::Audio | MessageType::File => {
                if let Some(ref url) = parsed.url {
                    let file_name = parsed.file_name.as_deref().unwrap_or("file");
                    segments.push(super::common::QQSegment::text(&format!(
                        "[{}: {}]",
                        match parsed.msg_type {
                            MessageType::Video => "Video",
                            MessageType::Audio => "Audio",
                            _ => "File",
                        },
                        file_name
                    )));
                }
            }
            MessageType::Location => {
                if let Some(ref geo_uri) = parsed.geo_uri {
                    if let Some((lat, lng)) = Self::parse_geo_uri(geo_uri) {
                        segments.push(super::common::QQSegment::text(&format!(
                            "[Location: {}, {}]",
                            lat, lng
                        )));
                    }
                }
            }
            MessageType::Reply => {
                let content = if let Some(ref formatted) = parsed.formatted_body {
                    self.format_html_for_qq(formatted)
                } else {
                    self.format_for_qq(&parsed.body)
                };
                segments.push(super::common::QQSegment::text(&content));
            }
            MessageType::Edit => {
                if let Some(new_body) = MessageUtils::get_new_content(
                    &serde_json::json!({ "m.new_content": { "body": parsed.body } }),
                ) {
                    segments.push(super::common::QQSegment::text(&format!(
                        "(edited) {}",
                        self.format_for_qq(&new_body)
                    )));
                }
            }
            MessageType::Sticker => {
                if let Some(ref url) = parsed.url {
                    segments.push(super::common::QQSegment::image(url));
                }
            }
            MessageType::Unknown => {
                if !parsed.body.is_empty() {
                    segments.push(super::common::QQSegment::text(&parsed.body));
                }
            }
        }

        segments
    }

    fn parse_geo_uri(uri: &str) -> Option<(f64, f64)> {
        if !uri.starts_with("geo:") {
            return None;
        }
        let coords = uri.trim_start_matches("geo:").split(';').next()?;
        let parts: Vec<&str> = coords.split(',').collect();
        if parts.len() != 2 {
            return None;
        }
        let lat = parts[0].parse::<f64>().ok()?;
        let lng = parts[1].parse::<f64>().ok()?;
        Some((lat, lng))
    }
}

impl Default for MatrixMessageParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_text_message() {
        let content = serde_json::json!({
            "msgtype": "m.text",
            "body": "Hello world"
        });
        let parsed = MatrixMessageParser::parse(&content);
        assert_eq!(parsed.body, "Hello world");
        assert!(matches!(parsed.msg_type, MessageType::Text));
    }

    #[test]
    fn parses_image_message() {
        let content = serde_json::json!({
            "msgtype": "m.image",
            "body": "image.png",
            "url": "mxc://example.org/abc123",
            "info": {
                "mimetype": "image/png",
                "size": 12345
            }
        });
        let parsed = MatrixMessageParser::parse(&content);
        assert!(matches!(parsed.msg_type, MessageType::Image));
        assert_eq!(parsed.url, Some("mxc://example.org/abc123".to_string()));
        assert_eq!(parsed.mime_type, Some("image/png".to_string()));
    }

    #[test]
    fn parses_reply_message() {
        let content = serde_json::json!({
            "msgtype": "m.text",
            "body": "Reply text",
            "m.relates_to": {
                "m.in_reply_to": {
                    "event_id": "$original_event"
                }
            }
        });
        let parsed = MatrixMessageParser::parse(&content);
        assert_eq!(parsed.reply_to, Some("$original_event".to_string()));
    }

    #[test]
    fn parses_edit_message() {
        let content = serde_json::json!({
            "msgtype": "m.text",
            "body": "Edited text",
            "m.relates_to": {
                "rel_type": "m.replace",
                "event_id": "$original_event"
            },
            "m.new_content": {
                "msgtype": "m.text",
                "body": "New content"
            }
        });
        let parsed = MatrixMessageParser::parse(&content);
        assert_eq!(parsed.edit_of, Some("$original_event".to_string()));
    }
}
