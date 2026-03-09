//! Executor module - the single point of SDK interaction.
//!
//! Both CLI and TUI emit Action values that are executed here.

use crate::actions::{Action, Target};
use crate::config::Config;
use crate::errors::CliError;
use sonos_sdk::SonosSystem;

/// Resolved target after looking up speaker/group handles.
#[derive(Debug)]
pub enum ResolvedTarget {
    Speaker(String),
    Group(String),
}

/// Resolve a Target to a concrete Speaker or Group handle.
///
/// Uses `system.get_speaker_by_name()` for speaker validation — the SDK
/// handles auto-rediscovery transparently on miss.
pub fn resolve_target(
    target: Target,
    system: &SonosSystem,
    config: &Config,
) -> Result<ResolvedTarget, CliError> {
    match target {
        Target::Speaker(name) => {
            system
                .get_speaker_by_name(&name)
                .ok_or_else(|| CliError::SpeakerNotFound(name.clone()))?;
            Ok(ResolvedTarget::Speaker(name))
        }
        Target::Group(name) => {
            // Group resolution will use system.groups() when implemented
            Ok(ResolvedTarget::Group(name))
        }
        Target::Default => {
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
pub fn execute(action: Action, system: &SonosSystem, config: &Config) -> Result<String, CliError> {
    match action {
        Action::ListSpeakers => Ok("Speakers listed".to_string()),
        Action::ListGroups => Ok("Groups listed".to_string()),
        Action::Status { target } => {
            let _resolved = resolve_target(target, system, config)?;
            Ok("Status displayed".to_string())
        }
        Action::Play { target } => {
            let _resolved = resolve_target(target, system, config)?;
            Ok("Playback started".to_string())
        }
        Action::Pause { target } => {
            let _resolved = resolve_target(target, system, config)?;
            Ok("Playback paused".to_string())
        }
        Action::Stop { target } => {
            let _resolved = resolve_target(target, system, config)?;
            Ok("Playback stopped".to_string())
        }
        Action::Next { target } => {
            let _resolved = resolve_target(target, system, config)?;
            Ok("Skipped to next track".to_string())
        }
        Action::Previous { target } => {
            let _resolved = resolve_target(target, system, config)?;
            Ok("Skipped to previous track".to_string())
        }
        Action::Seek { position, target } => {
            let _resolved = resolve_target(target, system, config)?;
            Ok(format!("Seeked to {}", position))
        }
        Action::SetPlayMode { mode, target } => {
            let _resolved = resolve_target(target, system, config)?;
            Ok(format!("Play mode set to {:?}", mode))
        }
        Action::SetVolume { level, target } => {
            let _resolved = resolve_target(target, system, config)?;
            Ok(format!("Volume set to {}", level))
        }
        Action::Mute { target } => {
            let _resolved = resolve_target(target, system, config)?;
            Ok("Muted".to_string())
        }
        Action::Unmute { target } => {
            let _resolved = resolve_target(target, system, config)?;
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
            let _resolved = resolve_target(target, system, config)?;
            Ok("Queue displayed".to_string())
        }
        Action::AddToQueue { uri, target } => {
            let _resolved = resolve_target(target, system, config)?;
            Ok(format!("Added {} to queue", uri))
        }
        Action::ClearQueue { target } => {
            let _resolved = resolve_target(target, system, config)?;
            Ok("Queue cleared".to_string())
        }
        Action::JoinGroup { speaker, group } => {
            Ok(format!("{} joined group {}", speaker, group))
        }
        Action::LeaveGroup { speaker } => {
            Ok(format!("{} left group", speaker))
        }
        Action::SetSleepTimer { duration, target } => {
            let _resolved = resolve_target(target, system, config)?;
            Ok(format!("Sleep timer set for {}", duration))
        }
        Action::CancelSleepTimer { target } => {
            let _resolved = resolve_target(target, system, config)?;
            Ok("Sleep timer cancelled".to_string())
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(group: Option<&str>) -> Config {
        Config {
            default_group: group.map(String::from),
            theme: "dark".to_string(),
        }
    }

    /// Helper to create a SonosSystem with test devices.
    /// Requires network for event manager — integration tests only.
    fn test_system(names: &[&str]) -> SonosSystem {
        let devices: Vec<sonos_sdk::sonos_discovery::Device> = names
            .iter()
            .enumerate()
            .map(|(i, name)| sonos_sdk::sonos_discovery::Device {
                id: format!("RINCON_{:03}", i),
                name: name.to_string(),
                room_name: name.to_string(),
                ip_address: format!("192.168.1.{}", 100 + i),
                port: 1400,
                model_name: "Sonos One".to_string(),
            })
            .collect();
        SonosSystem::from_discovered_devices(devices).unwrap()
    }

    #[test]
    fn resolve_explicit_speaker() {
        let system = test_system(&["Kitchen"]);
        let config = test_config(None);
        let target = Target::Speaker("Kitchen".to_string());
        let resolved = resolve_target(target, &system, &config).unwrap();
        assert!(matches!(resolved, ResolvedTarget::Speaker(name) if name == "Kitchen"));
    }

    #[test]
    fn resolve_speaker_not_found() {
        let system = test_system(&["Kitchen"]);
        let config = test_config(None);
        let target = Target::Speaker("Nonexistent".to_string());
        let result = resolve_target(target, &system, &config);
        assert!(matches!(result, Err(CliError::SpeakerNotFound(_))));
    }

    #[test]
    fn resolve_explicit_group() {
        let system = test_system(&[]);
        let config = test_config(None);
        let target = Target::Group("Living Room".to_string());
        let resolved = resolve_target(target, &system, &config).unwrap();
        assert!(matches!(resolved, ResolvedTarget::Group(name) if name == "Living Room"));
    }

    #[test]
    fn resolve_default_uses_config_group() {
        let system = test_system(&[]);
        let config = test_config(Some("Bedroom"));
        let resolved = resolve_target(Target::Default, &system, &config).unwrap();
        assert!(matches!(resolved, ResolvedTarget::Group(name) if name == "Bedroom"));
    }

    #[test]
    fn resolve_default_without_config_fails() {
        let system = test_system(&[]);
        let config = test_config(None);
        let result = resolve_target(Target::Default, &system, &config);
        assert!(result.is_err());
    }

    #[test]
    fn execute_play_with_explicit_target_succeeds() {
        let system = test_system(&["Kitchen"]);
        let config = test_config(None);
        let action = Action::Play {
            target: Target::Speaker("Kitchen".to_string()),
        };
        assert!(execute(action, &system, &config).is_ok());
    }

    #[test]
    fn execute_volume_includes_level_in_message() {
        let system = test_system(&[]);
        let config = test_config(Some("Room"));
        let action = Action::SetVolume {
            level: 50,
            target: Target::Default,
        };
        let msg = execute(action, &system, &config).unwrap();
        assert!(msg.contains("50"));
    }
}
