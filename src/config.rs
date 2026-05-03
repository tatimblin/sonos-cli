//! Configuration system for the sonos-cli application.

use serde::{Deserialize, Serialize};
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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_group: None,
            theme: "default".to_string(),
            album_art_mode: AlbumArtMode::default(),
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
}
