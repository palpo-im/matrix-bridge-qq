mod bridge;
mod kdl_support;

pub use bridge::*;

use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct HomeserverConfig {
    pub address: String,
    pub domain: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_db_type")]
    pub r#type: String,
    pub uri: String,
    #[serde(default = "default_max_open_conns")]
    pub max_open_conns: u32,
    #[serde(default = "default_max_idle_conns")]
    pub max_idle_conns: u32,
}

fn default_db_type() -> String {
    "sqlite".to_string()
}

fn default_max_open_conns() -> u32 {
    20
}

fn default_max_idle_conns() -> u32 {
    2
}

#[derive(Debug, Clone, Deserialize)]
pub struct BotConfig {
    pub username: String,
    pub displayname: Option<String>,
    pub avatar: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppServiceConfig {
    pub address: String,
    pub hostname: String,
    pub port: u16,
    pub database: DatabaseConfig,
    pub id: String,
    pub bot: BotConfig,
    pub as_token: String,
    pub hs_token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub min_level: String,
}

fn default_log_level() -> String {
    "info".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub homeserver: HomeserverConfig,
    pub appservice: AppServiceConfig,
    pub bridge: BridgeConfig,
    #[serde(default)]
    pub logging: Option<LoggingConfig>,
}

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = if kdl_support::is_kdl_file(std::path::Path::new(path)) {
            kdl_support::parse_kdl_config(&content).map_err(|e| anyhow::anyhow!(e))?
        } else {
            serde_yaml::from_str(&content)?
        };
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        if self.homeserver.address.trim().is_empty() {
            anyhow::bail!("homeserver.address must not be empty");
        }
        if self.homeserver.domain.trim().is_empty() {
            anyhow::bail!("homeserver.domain must not be empty");
        }
        if self.appservice.as_token.trim().is_empty() || self.appservice.hs_token.trim().is_empty()
        {
            anyhow::bail!("appservice as_token/hs_token must not be empty");
        }
        if !self.bridge.username_template.contains("{{.}}") {
            anyhow::bail!("bridge.username_template must contain {{.}} placeholder");
        }
        if self.bridge.permissions.is_empty() {
            anyhow::bail!("bridge.permissions must contain at least one entry");
        }
        Ok(())
    }

    pub fn bot_mxid(&self) -> String {
        format!(
            "@{}:{}",
            self.appservice.bot.username, self.homeserver.domain
        )
    }

    pub fn format_qq_localpart(&self, qq_user_id: &str) -> String {
        self.bridge.username_template.replace("{{.}}", qq_user_id)
    }

    pub fn format_qq_mxid(&self, qq_user_id: &str) -> String {
        format!(
            "@{}:{}",
            self.format_qq_localpart(qq_user_id),
            self.homeserver.domain
        )
    }

    pub fn template_parts(&self) -> (&str, &str) {
        if let Some((prefix, suffix)) = self.bridge.username_template.split_once("{{.}}") {
            (prefix, suffix)
        } else {
            ("", "")
        }
    }

    pub fn is_qq_namespace_user(&self, mxid: &str) -> bool {
        let Some(rest) = mxid.strip_prefix('@') else {
            return false;
        };
        let Some((localpart, domain)) = rest.split_once(':') else {
            return false;
        };
        if domain != self.homeserver.domain {
            return false;
        }

        let (prefix, suffix) = self.template_parts();
        localpart.starts_with(prefix)
            && localpart.ends_with(suffix)
            && localpart.len() >= prefix.len() + suffix.len()
    }
}
