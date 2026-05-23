//! Configuration system for the sonos-cli application.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

/// Album art rendering mode.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlbumArtMode {
    #[default]
    Image,
    Halfblock,
    Off,
    /// Catch-all for unrecognized values (e.g. old "auto") — behaves like Image.
    #[serde(other)]
    Other,
}

impl AlbumArtMode {
    /// Returns true when album art should be disabled.
    pub fn is_off(&self) -> bool {
        *self == Self::Off
    }
}

impl fmt::Display for AlbumArtMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Image | Self::Other => write!(f, "image"),
            Self::Halfblock => write!(f, "halfblock"),
            Self::Off => write!(f, "off"),
        }
    }
}

/// Custom Serialize: `Other` normalizes to `"image"` on save.
impl Serialize for AlbumArtMode {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

/// User configuration loaded from config file with environment variable overrides.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    /// Default group to target when --speaker/--group not specified
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_group: Option<String>,
    /// TUI color theme: "default", "bw", "minimal", or "dance_party"
    pub theme: String,
    /// Album art rendering mode: "image", "halfblock", or "off"
    pub album_art_mode: AlbumArtMode,
    /// Speaker/group aliases: key = full name, value = short alias
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub aliases: HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_group: None,
            theme: "default".to_string(),
            album_art_mode: AlbumArtMode::default(),
            aliases: HashMap::new(),
        }
    }
}

/// Return the path to the config file.
fn config_path() -> Option<PathBuf> {
    std::env::var("SONOS_CONFIG_DIR")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| dirs::config_dir().map(|p| p.join("sonos")))
        .map(|d| d.join("config.toml"))
}

impl Config {
    /// Load from config file with environment variable overrides.
    pub fn load() -> Self {
        let mut config: Config = config_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default();

        // Environment variable overrides
        if let Ok(group) = std::env::var("SONOS_DEFAULT_GROUP") {
            config.default_group = Some(group);
        }

        config
    }

    /// Persist the current config to disk.
    pub fn save(&self) -> anyhow::Result<()> {
        let path =
            config_path().ok_or_else(|| anyhow::anyhow!("cannot determine config directory"))?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Resolve an alias to the full speaker/group name.
    /// If `input` matches an alias value, returns the corresponding key (full name).
    /// Otherwise returns `input` unchanged.
    pub fn resolve_alias<'a>(&'a self, input: &'a str) -> &'a str {
        for (name, alias) in &self.aliases {
            if alias == input {
                return name;
            }
        }
        input
    }

    /// Set or replace an alias for a speaker/group name.
    pub fn set_alias(&mut self, name: &str, alias: &str) {
        self.aliases.insert(name.to_string(), alias.to_string());
    }

    /// Clear the alias for a speaker/group name. Returns the old alias if one existed.
    pub fn clear_alias(&mut self, name: &str) -> Option<String> {
        self.aliases.remove(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_alias_returns_full_name() {
        let mut config = Config::default();
        config.set_alias("Master Bedroom", "bed");
        assert_eq!(config.resolve_alias("bed"), "Master Bedroom");
    }

    #[test]
    fn resolve_alias_passthrough_when_no_match() {
        let config = Config::default();
        assert_eq!(config.resolve_alias("Kitchen"), "Kitchen");
    }

    #[test]
    fn set_alias_replaces_existing() {
        let mut config = Config::default();
        config.set_alias("Master Bedroom", "bed");
        config.set_alias("Master Bedroom", "mb");
        assert_eq!(config.resolve_alias("mb"), "Master Bedroom");
        assert_eq!(config.resolve_alias("bed"), "bed");
    }

    #[test]
    fn clear_alias_removes_and_returns_old() {
        let mut config = Config::default();
        config.set_alias("Master Bedroom", "bed");
        let old = config.clear_alias("Master Bedroom");
        assert_eq!(old, Some("bed".to_string()));
        assert_eq!(config.resolve_alias("bed"), "bed");
    }

    #[test]
    fn clear_alias_returns_none_when_absent() {
        let mut config = Config::default();
        assert_eq!(config.clear_alias("Kitchen"), None);
    }

    #[test]
    fn aliases_roundtrip_through_toml() {
        let mut config = Config::default();
        config.set_alias("Master Bedroom", "bed");
        config.set_alias("Kitchen", "kit");
        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.resolve_alias("bed"), "Master Bedroom");
        assert_eq!(deserialized.resolve_alias("kit"), "Kitchen");
    }

    #[test]
    fn empty_aliases_not_serialized() {
        let config = Config::default();
        let serialized = toml::to_string_pretty(&config).unwrap();
        assert!(!serialized.contains("[aliases]"));
    }
}
