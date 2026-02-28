mod common;
mod matrix_parser;
mod qq_parser;

pub use common::{BridgeMessage, MessageType, ParsedMessage};
pub use matrix_parser::{MatrixMessageParser, MatrixToQQConverter};
pub use qq_parser::{QQMessageParser, QQToMatrixConverter};

pub struct MessageUtils;

impl MessageUtils {
    pub fn extract_plain_text(content: &serde_json::Value) -> String {
        if let Some(obj) = content.as_object() {
            if let Some(body) = obj.get("body").and_then(|b| b.as_str()) {
                return body.to_string();
            }
        }
        if let Some(text) = content.as_str() {
            return text.to_string();
        }
        String::new()
    }

    pub fn convert_html_to_qq_markdown(html: &str) -> String {
        let mut result = html.to_string();

        result = Self::convert_tag(&result, "strong", "**");
        result = Self::convert_tag(&result, "b", "**");
        result = Self::convert_tag(&result, "em", "*");
        result = Self::convert_tag(&result, "i", "*");
        result = Self::convert_tag(&result, "code", "`");
        result = Self::convert_tag(&result, "del", "~~");
        result = Self::convert_tag(&result, "s", "~~");

        result = Self::convert_links(&result);

        result = Self::strip_remaining_tags(&result);

        result
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&")
            .replace("&quot;", "\"")
    }

    fn convert_tag(html: &str, tag: &str, markdown: &str) -> String {
        let open_tag = format!("<{}>", tag);
        let close_tag = format!("</{}>", tag);
        let mut result = html.to_string();

        while let Some(start) = result.find(&open_tag) {
            if let Some(end) = result.find(&close_tag) {
                if end > start {
                    let content = &result[start + open_tag.len()..end];
                    let replacement = format!("{}{}{}", markdown, content, markdown);
                    result = format!(
                        "{}{}{}",
                        &result[..start],
                        replacement,
                        &result[end + close_tag.len()..]
                    );
                    continue;
                }
            }
            break;
        }

        result
    }

    fn convert_links(html: &str) -> String {
        use regex::Regex;
        let link_regex = Regex::new(r#"<a[^>]*href="([^"]*)"[^>]*>([^<]*)</a>"#).unwrap();
        link_regex
            .replace_all(html, |caps: &regex::Captures| {
                let url = &caps[1];
                let text = &caps[2];
                format!("[{}]({})", text, url)
            })
            .to_string()
    }

    fn strip_remaining_tags(html: &str) -> String {
        use regex::Regex;
        let tag_regex = Regex::new(r"<[^>]*>").unwrap();
        tag_regex.replace_all(html, "").to_string()
    }

    pub fn get_msgtype(content: &serde_json::Value) -> Option<String> {
        content
            .as_object()
            .and_then(|obj| obj.get("msgtype"))
            .and_then(|t| t.as_str())
            .map(ToString::to_string)
    }

    pub fn is_emote(content: &serde_json::Value) -> bool {
        Self::get_msgtype(content).as_deref() == Some("m.emote")
    }

    pub fn extract_reply_info(content: &serde_json::Value) -> Option<String> {
        let obj = content.as_object()?;
        let relates_to = obj.get("m.relates_to")?.as_object()?;
        let in_reply_to = relates_to.get("m.in_reply_to")?.as_object()?;
        in_reply_to.get("event_id")?.as_str().map(ToOwned::to_owned)
    }

    pub fn extract_edit_info(content: &serde_json::Value) -> Option<String> {
        let obj = content.as_object()?;
        let relates_to = obj.get("m.relates_to")?.as_object()?;

        if relates_to.get("rel_type")?.as_str()? != "m.replace" {
            return None;
        }

        relates_to.get("event_id")?.as_str().map(ToOwned::to_owned)
    }

    pub fn get_new_content(content: &serde_json::Value) -> Option<String> {
        let obj = content.as_object()?;
        let new_content = obj.get("m.new_content")?.as_object()?;
        new_content.get("body")?.as_str().map(ToOwned::to_owned)
    }
}
