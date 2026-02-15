use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct OneBotConfig {
    pub api_base: String,
    pub event_path: String,
    pub listen_secret: Option<String>,
    pub access_token: Option<String>,
    pub self_id: Option<String>,
    #[serde(default = "default_ignore_own_messages")]
    pub ignore_own_messages: bool,
}

fn default_ignore_own_messages() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
pub struct BridgeConfig {
    pub username_template: String,
    #[serde(default = "default_command_prefix")]
    pub command_prefix: String,
    #[serde(default = "default_room_name_private")]
    pub private_room_name_template: String,
    #[serde(default = "default_room_name_group")]
    pub group_room_name_template: String,
    pub onebot: OneBotConfig,
    pub permissions: HashMap<String, String>,
}

fn default_command_prefix() -> String {
    "!qq".to_string()
}

fn default_room_name_private() -> String {
    "QQ Private {{chat_id}}".to_string()
}

fn default_room_name_group() -> String {
    "QQ Group {{chat_id}}".to_string()
}

impl BridgeConfig {
    pub fn render_private_room_name(&self, chat_id: &str) -> String {
        self.private_room_name_template
            .replace("{{chat_id}}", chat_id)
    }

    pub fn render_group_room_name(&self, chat_id: &str) -> String {
        self.group_room_name_template
            .replace("{{chat_id}}", chat_id)
    }
}
