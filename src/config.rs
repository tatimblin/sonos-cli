//! Configuration system for the sonos-cli application.

use serde::Deserialize;
use std::path::PathBuf;

/// User configuration loaded from config file with environment variable overrides.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Default group to target when --speaker/--group not specified
    pub default_group: Option<String>,
    /// Hours before cache is considered stale
    pub cache_ttl_hours: u64,
    /// TUI color theme: "dark" or "light"
    pub theme: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_group: None,
            cache_ttl_hours: 24,
            theme: "dark".to_string(),
        }
    }
}

impl Config {
    /// Load from config file with environment variable overrides.
    pub fn load() -> Self {
        let config_dir = std::env::var("SONOS_CONFIG_DIR")
            .map(PathBuf::from)
            .ok()
            .or_else(|| dirs::config_dir().map(|p| p.join("sonos")))
            .unwrap_or_else(|| PathBuf::from("."));

        let config_path = config_dir.join("config.toml");

        let mut config: Config = std::fs::read_to_string(&config_path)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default();

        // Environment variable overrides
        if let Ok(group) = std::env::var("SONOS_DEFAULT_GROUP") {
            config.default_group = Some(group);
        }

        config
    }
}
