//! Configuration system for the sonos-cli application.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

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
    /// Set when the config file existed but could not be loaded, so these are
    /// fallback defaults. [`Config::save`] refuses to run while this is set:
    /// writing the defaults back would destroy the user's file.
    #[serde(skip)]
    pub(crate) load_error: Option<ConfigError>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_group: None,
            theme: "default".to_string(),
            album_art_mode: AlbumArtMode::default(),
            aliases: HashMap::new(),
            load_error: None,
        }
    }
}

/// The config file exists but could not be used.
///
/// A *missing* file is not an error — it just means defaults.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ConfigError {
    #[error("could not read {}: {message}", path.display())]
    Read { path: PathBuf, message: String },
    #[error("could not parse {}: {message}", path.display())]
    Parse { path: PathBuf, message: String },
}

impl ConfigError {
    /// The config file this error refers to.
    pub fn path(&self) -> &Path {
        match self {
            Self::Read { path, .. } | Self::Parse { path, .. } => path,
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
    /// Load from the config file with environment variable overrides.
    ///
    /// Lenient: a file that cannot be read or parsed prints one warning to
    /// stderr and falls back to defaults, so read-only commands keep working.
    /// The returned config remembers the failure and [`Config::save`] refuses
    /// to overwrite the broken file. Use [`Config::try_load`] to handle the
    /// error yourself.
    pub fn load() -> Self {
        Self::try_load().unwrap_or_else(|e| {
            eprintln!("warning: {e}");
            eprintln!("Using default settings until the config file is fixed.");
            Self::fallback(e)
        })
    }

    /// Load from the config file with environment variable overrides,
    /// failing if the file exists but cannot be read or parsed.
    pub fn try_load() -> Result<Self, ConfigError> {
        let mut config = match config_path() {
            Some(path) => Self::load_from(&path)?,
            None => Config::default(),
        };
        config.apply_env_overrides();
        Ok(config)
    }

    /// Defaults (plus environment overrides) standing in for a config file
    /// that failed to load. Saving it is refused.
    pub fn fallback(error: ConfigError) -> Self {
        let mut config = Config {
            load_error: Some(error),
            ..Config::default()
        };
        config.apply_env_overrides();
        config
    }

    /// Load from `path` without environment overrides. A missing file yields
    /// defaults.
    pub fn load_from(path: &Path) -> Result<Self, ConfigError> {
        let content = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Config::default()),
            Err(e) => {
                return Err(ConfigError::Read {
                    path: path.to_path_buf(),
                    message: e.to_string(),
                })
            }
        };
        toml::from_str(&content).map_err(|e| ConfigError::Parse {
            path: path.to_path_buf(),
            message: e.to_string().trim_end().to_string(),
        })
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(group) = std::env::var("SONOS_DEFAULT_GROUP") {
            self.default_group = Some(group);
        }
    }

    /// Persist the current config to disk.
    ///
    /// Fails without writing if this config stands in for a file that could
    /// not be loaded.
    pub fn save(&self) -> anyhow::Result<()> {
        let path =
            config_path().ok_or_else(|| anyhow::anyhow!("cannot determine config directory"))?;
        self.save_to(&path)
    }

    /// Persist the current config to `path`. Same guard as [`Config::save`].
    pub fn save_to(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(e) = &self.load_error {
            anyhow::bail!(
                "refusing to overwrite {}, which could not be loaded; fix or remove it, then retry",
                e.path().display()
            );
        }
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
            if alias.eq_ignore_ascii_case(input) {
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

    const MALFORMED: &str = "theme = \"bw\n[aliases\n";

    #[test]
    fn load_from_missing_file_is_default() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::load_from(&dir.path().join("config.toml")).unwrap();
        assert_eq!(config.theme, "default");
        assert!(config.load_error.is_none());
    }

    #[test]
    fn load_from_valid_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "theme = \"bw\"\n[aliases]\nKitchen = \"kit\"\n").unwrap();
        let config = Config::load_from(&path).unwrap();
        assert_eq!(config.theme, "bw");
        assert_eq!(config.resolve_alias("kit"), "Kitchen");
    }

    #[test]
    fn load_from_malformed_file_is_parse_error_naming_the_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, MALFORMED).unwrap();
        let err = Config::load_from(&path).unwrap_err();
        assert!(matches!(err, ConfigError::Parse { .. }));
        assert_eq!(err.path(), path);
        assert!(err.to_string().contains(&path.display().to_string()));
    }

    #[test]
    fn load_from_unreadable_path_is_read_error() {
        // A directory where the file should be: it exists but cannot be read.
        let dir = tempfile::tempdir().unwrap();
        let err = Config::load_from(dir.path()).unwrap_err();
        assert!(matches!(err, ConfigError::Read { .. }));
    }

    #[test]
    fn fallback_refuses_to_overwrite_the_broken_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, MALFORMED).unwrap();
        let mut config = Config::fallback(Config::load_from(&path).unwrap_err());
        assert_eq!(config.theme, "default");
        config.set_alias("Kitchen", "kit");

        let err = config.save_to(&path).unwrap_err();
        assert!(err.to_string().contains("refusing to overwrite"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), MALFORMED);
    }

    #[test]
    fn save_to_creates_parent_dirs_and_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sonos").join("config.toml");
        let mut config = Config::default();
        config.set_alias("Kitchen", "kit");
        config.save_to(&path).unwrap();
        let loaded = Config::load_from(&path).unwrap();
        assert_eq!(loaded.resolve_alias("kit"), "Kitchen");
    }

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

    #[test]
    fn resolve_alias_case_insensitive() {
        let mut config = Config::default();
        config.set_alias("Master Bedroom", "bed");
        assert_eq!(config.resolve_alias("Bed"), "Master Bedroom");
        assert_eq!(config.resolve_alias("BED"), "Master Bedroom");
        assert_eq!(config.resolve_alias("bed"), "Master Bedroom");
    }
}
