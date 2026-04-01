//! Configuration system for the sonos-cli application.

use serde::Deserialize;
use std::path::PathBuf;

/// User configuration loaded from config file with environment variable overrides.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Default group to target when --speaker/--group not specified
    pub default_group: Option<String>,
    /// TUI color theme: "dark" or "light"
    pub theme: String,
    /// Album art rendering mode: "auto", "halfblock", or "off"
    pub album_art_mode: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_group: None,
            theme: "dark".to_string(),
            album_art_mode: "auto".to_string(),
        }
    }
}

impl Config {
    /// Load from config file with environment variable overrides.
    pub fn load() -> Self {
        let config_dir = std::env::var("SONOS_CONFIG_DIR")
            .ok()
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| dirs::config_dir().map(|p| p.join("sonos")));

        let mut config: Config = config_dir
            .map(|d| d.join("config.toml"))
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default();

        // Environment variable overrides
        if let Ok(group) = std::env::var("SONOS_DEFAULT_GROUP") {
            config.default_group = Some(group);
        }

        config
    }
}
