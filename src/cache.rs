//! Cache system for discovered speakers and groups.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// Cached speaker information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CachedSpeaker {
    pub name: String,
    pub id: String,
    pub ip: String,
    pub model_name: String,
}

/// Cached group information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CachedGroup {
    pub id: String,
    pub coordinator_id: String,
    pub member_ids: Vec<String>,
}

/// Cached system state containing speakers and groups.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CachedSystem {
    pub speakers: Vec<CachedSpeaker>,
    pub groups: Vec<CachedGroup>,
    pub cached_at: SystemTime,
}

impl CachedSystem {
    /// Get the cache directory path.
    fn cache_dir() -> Option<PathBuf> {
        std::env::var("SONOS_CONFIG_DIR")
            .map(PathBuf::from)
            .ok()
            .or_else(|| dirs::config_dir().map(|p| p.join("sonos")))
    }

    /// Get the cache file path.
    fn cache_path() -> Option<PathBuf> {
        Self::cache_dir().map(|d| d.join("cache.json"))
    }

    /// Load from ~/.config/sonos/cache.json.
    /// Returns None if file is missing or unparseable.
    pub fn load() -> Option<Self> {
        let cache_path = Self::cache_path()?;
        let contents = fs::read_to_string(&cache_path).ok()?;
        serde_json::from_str(&contents).ok()
    }

    /// Save to ~/.config/sonos/cache.json atomically.
    /// Writes to temp file first, then renames.
    pub fn save(&self) -> Result<(), io::Error> {
        let cache_dir = Self::cache_dir()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "config dir not found"))?;

        fs::create_dir_all(&cache_dir)?;

        let cache_path = cache_dir.join("cache.json");
        let temp_path = cache_dir.join("cache.json.tmp");

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        fs::write(&temp_path, json)?;
        fs::rename(&temp_path, &cache_path)?;

        Ok(())
    }

    /// Check if cache has exceeded TTL.
    pub fn is_stale(&self, ttl_hours: u64) -> bool {
        let ttl = Duration::from_secs(ttl_hours * 3600);
        match self.cached_at.elapsed() {
            Ok(elapsed) => elapsed >= ttl,
            Err(_) => true, // If system time went backwards, consider stale
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn make_system(cached_at: SystemTime) -> CachedSystem {
        CachedSystem {
            speakers: vec![CachedSpeaker {
                name: "Kitchen".to_string(),
                id: "RINCON_123".to_string(),
                ip: "192.168.1.10".to_string(),
                model_name: "Sonos One".to_string(),
            }],
            groups: vec![CachedGroup {
                id: "group1".to_string(),
                coordinator_id: "RINCON_123".to_string(),
                member_ids: vec!["RINCON_123".to_string()],
            }],
            cached_at,
        }
    }

    #[test]
    fn fresh_cache_is_not_stale() {
        let system = make_system(SystemTime::now());
        assert!(!system.is_stale(24));
    }

    #[test]
    fn expired_cache_is_stale() {
        let old_time = SystemTime::now() - Duration::from_secs(25 * 3600);
        let system = make_system(old_time);
        assert!(system.is_stale(24));
    }

    #[test]
    fn cache_at_exact_ttl_boundary_is_stale() {
        let boundary_time = SystemTime::now() - Duration::from_secs(24 * 3600);
        let system = make_system(boundary_time);
        assert!(system.is_stale(24));
    }

    #[test]
    fn serialization_roundtrip() {
        let system = make_system(SystemTime::now());
        let json = serde_json::to_string(&system).unwrap();
        let restored: CachedSystem = serde_json::from_str(&json).unwrap();
        assert_eq!(system, restored);
    }
}
