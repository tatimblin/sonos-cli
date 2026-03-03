//! Executor module - the single point of SDK interaction.
//!
//! Both CLI and TUI emit Action values that are executed here.

use crate::actions::{Action, Target};
use crate::config::Config;
use crate::errors::CliError;

/// Resolved target after looking up speaker/group handles.
#[derive(Debug)]
pub enum ResolvedTarget {
    /// A resolved speaker (placeholder for now)
    Speaker(String),
    /// A resolved group (placeholder for now)
    Group(String),
}

/// Resolve a Target to a concrete Speaker or Group handle.
pub fn resolve_target(target: Target, config: &Config) -> Result<ResolvedTarget, CliError> {
    match target {
        Target::Speaker(name) => {
            // Stub: In real implementation, would look up speaker in system
            Ok(ResolvedTarget::Speaker(name))
        }
        Target::Group(name) => {
            // Stub: In real implementation, would look up group in system
            Ok(ResolvedTarget::Group(name))
        }
        Target::Default => {
            // Use config.default_group if set, otherwise return error
            if let Some(group) = &config.default_group {
                Ok(ResolvedTarget::Group(group.clone()))
            } else {
                Err(CliError::GroupNotFound("no groups discovered".to_string()))
            }
        }
    }
}

/// Execute an action against the Sonos system.
/// Returns a human-readable success message.
pub fn execute(action: Action, config: &Config) -> Result<String, CliError> {
    match action {
        Action::Discover => Ok("Discovery complete".to_string()),
        Action::ListSpeakers => Ok("Speakers listed".to_string()),
        Action::ListGroups => Ok("Groups listed".to_string()),
        Action::Status { target } => {
            let _resolved = resolve_target(target, config)?;
            Ok("Status displayed".to_string())
        }
        Action::Play { target } => {
            let _resolved = resolve_target(target, config)?;
            Ok("Playback started".to_string())
        }
        Action::Pause { target } => {
            let _resolved = resolve_target(target, config)?;
            Ok("Playback paused".to_string())
        }
        Action::Stop { target } => {
            let _resolved = resolve_target(target, config)?;
            Ok("Playback stopped".to_string())
        }
        Action::Next { target } => {
            let _resolved = resolve_target(target, config)?;
            Ok("Skipped to next track".to_string())
        }
        Action::Previous { target } => {
            let _resolved = resolve_target(target, config)?;
            Ok("Skipped to previous track".to_string())
        }
        Action::Seek { position, target } => {
            let _resolved = resolve_target(target, config)?;
            Ok(format!("Seeked to {}", position))
        }
        Action::SetPlayMode { mode, target } => {
            let _resolved = resolve_target(target, config)?;
            Ok(format!("Play mode set to {:?}", mode))
        }
        Action::SetVolume { level, target } => {
            let _resolved = resolve_target(target, config)?;
            Ok(format!("Volume set to {}", level))
        }
        Action::Mute { target } => {
            let _resolved = resolve_target(target, config)?;
            Ok("Muted".to_string())
        }
        Action::Unmute { target } => {
            let _resolved = resolve_target(target, config)?;
            Ok("Unmuted".to_string())
        }
        Action::SetBass { level, speaker } => {
            Ok(format!("Bass set to {} on {}", level, speaker))
        }
        Action::SetTreble { level, speaker } => {
            Ok(format!("Treble set to {} on {}", level, speaker))
        }
        Action::SetLoudness { enabled, speaker } => {
            let state = if enabled { "enabled" } else { "disabled" };
            Ok(format!("Loudness {} on {}", state, speaker))
        }
        Action::ShowQueue { target } => {
            let _resolved = resolve_target(target, config)?;
            Ok("Queue displayed".to_string())
        }
        Action::AddToQueue { uri, target } => {
            let _resolved = resolve_target(target, config)?;
            Ok(format!("Added {} to queue", uri))
        }
        Action::ClearQueue { target } => {
            let _resolved = resolve_target(target, config)?;
            Ok("Queue cleared".to_string())
        }
        Action::JoinGroup { speaker, group } => {
            Ok(format!("{} joined group {}", speaker, group))
        }
        Action::LeaveGroup { speaker } => {
            Ok(format!("{} left group", speaker))
        }
        Action::SetSleepTimer { duration, target } => {
            let _resolved = resolve_target(target, config)?;
            Ok(format!("Sleep timer set for {}", duration))
        }
        Action::CancelSleepTimer { target } => {
            let _resolved = resolve_target(target, config)?;
            Ok("Sleep timer cancelled".to_string())
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn config_with_default_group(group: Option<&str>) -> Config {
        Config {
            default_group: group.map(String::from),
            cache_ttl_hours: 24,
            theme: "dark".to_string(),
        }
    }

    #[test]
    fn resolve_explicit_speaker() {
        let config = config_with_default_group(None);
        let target = Target::Speaker("Kitchen".to_string());
        let resolved = resolve_target(target, &config).unwrap();
        assert!(matches!(resolved, ResolvedTarget::Speaker(name) if name == "Kitchen"));
    }

    #[test]
    fn resolve_explicit_group() {
        let config = config_with_default_group(None);
        let target = Target::Group("Living Room".to_string());
        let resolved = resolve_target(target, &config).unwrap();
        assert!(matches!(resolved, ResolvedTarget::Group(name) if name == "Living Room"));
    }

    #[test]
    fn resolve_default_uses_config_group() {
        let config = config_with_default_group(Some("Bedroom"));
        let resolved = resolve_target(Target::Default, &config).unwrap();
        assert!(matches!(resolved, ResolvedTarget::Group(name) if name == "Bedroom"));
    }

    #[test]
    fn resolve_default_without_config_fails() {
        let config = config_with_default_group(None);
        let result = resolve_target(Target::Default, &config);
        assert!(result.is_err());
    }

    #[test]
    fn execute_discover_succeeds() {
        let config = config_with_default_group(None);
        let result = execute(Action::Discover, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn execute_play_with_explicit_target_succeeds() {
        let config = config_with_default_group(None);
        let action = Action::Play {
            target: Target::Speaker("Kitchen".to_string()),
        };
        assert!(execute(action, &config).is_ok());
    }

    #[test]
    fn execute_volume_includes_level_in_message() {
        let config = config_with_default_group(Some("Room"));
        let action = Action::SetVolume {
            level: 50,
            target: Target::Default,
        };
        let msg = execute(action, &config).unwrap();
        assert!(msg.contains("50"));
    }
}
