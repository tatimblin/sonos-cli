//! CLI command parsing and execution.
//!
//! Each command resolves its target speaker/group and calls SDK methods directly.

use clap::Subcommand;

use crate::config::Config;
use crate::errors::CliError;
use sonos_sdk::{Speaker, SonosSystem};

/// CLI subcommands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// List all speakers
    Speakers,

    /// List all groups
    Groups,

    /// Show current playback status
    Status {
        #[arg(long)]
        speaker: Option<String>,
        #[arg(long)]
        group: Option<String>,
    },

    /// Start playback
    Play {
        #[arg(long)]
        speaker: Option<String>,
        #[arg(long)]
        group: Option<String>,
    },

    /// Pause playback
    Pause {
        #[arg(long)]
        speaker: Option<String>,
        #[arg(long)]
        group: Option<String>,
    },

    /// Stop playback
    Stop {
        #[arg(long)]
        speaker: Option<String>,
        #[arg(long)]
        group: Option<String>,
    },

    /// Skip to next track
    Next {
        #[arg(long)]
        speaker: Option<String>,
        #[arg(long)]
        group: Option<String>,
    },

    /// Skip to previous track
    Previous {
        #[arg(long)]
        speaker: Option<String>,
        #[arg(long)]
        group: Option<String>,
    },

    /// Set volume level (0-100)
    Volume {
        /// Volume level (0-100)
        level: u8,
        #[arg(long)]
        speaker: Option<String>,
        #[arg(long)]
        group: Option<String>,
    },

    /// Mute playback
    Mute {
        #[arg(long)]
        speaker: Option<String>,
        #[arg(long)]
        group: Option<String>,
    },

    /// Unmute playback
    Unmute {
        #[arg(long)]
        speaker: Option<String>,
        #[arg(long)]
        group: Option<String>,
    },
}

impl Commands {
    /// Execute this command against the Sonos system.
    /// Returns a human-readable success message.
    pub fn run(&self, system: &SonosSystem, config: &Config) -> Result<String, CliError> {
        match self {
            Commands::Speakers => {
                let speakers = system.speakers();
                if speakers.is_empty() {
                    return Ok("No speakers found".to_string());
                }
                let lines: Vec<String> = speakers
                    .iter()
                    .map(|s| format!("{} ({})", s.name, s.model_name))
                    .collect();
                Ok(lines.join("\n"))
            }
            Commands::Groups => {
                let groups = system.groups();
                if groups.is_empty() {
                    return Ok("No groups found".to_string());
                }
                let lines: Vec<String> = groups
                    .iter()
                    .map(|g| {
                        let coordinator_name = g
                            .coordinator()
                            .map(|c| c.name)
                            .unwrap_or_else(|| "unknown".to_string());
                        format!("{} ({} members)", coordinator_name, g.member_count())
                    })
                    .collect();
                Ok(lines.join("\n"))
            }
            Commands::Status { speaker, group } => {
                let spk = resolve_speaker(system, config, speaker.as_deref(), group.as_deref())?;
                let state = spk.playback_state.get();
                let state_str = state
                    .map(|s| format!("{:?}", s))
                    .unwrap_or_else(|| "unknown".to_string());
                Ok(format!("{}: {}", spk.name, state_str))
            }
            Commands::Play { speaker, group } => {
                let spk = resolve_speaker(system, config, speaker.as_deref(), group.as_deref())?;
                spk.play()?;
                Ok(format!("Playing on {}", spk.name))
            }
            Commands::Pause { speaker, group } => {
                let spk = resolve_speaker(system, config, speaker.as_deref(), group.as_deref())?;
                spk.pause()?;
                Ok(format!("Paused on {}", spk.name))
            }
            Commands::Stop { speaker, group } => {
                let spk = resolve_speaker(system, config, speaker.as_deref(), group.as_deref())?;
                spk.stop()?;
                Ok(format!("Stopped on {}", spk.name))
            }
            Commands::Next { speaker, group } => {
                let spk = resolve_speaker(system, config, speaker.as_deref(), group.as_deref())?;
                spk.next()?;
                Ok(format!("Skipped to next track on {}", spk.name))
            }
            Commands::Previous { speaker, group } => {
                let spk = resolve_speaker(system, config, speaker.as_deref(), group.as_deref())?;
                spk.previous()?;
                Ok(format!("Skipped to previous track on {}", spk.name))
            }
            Commands::Volume {
                level,
                speaker,
                group,
            } => {
                let spk = resolve_speaker(system, config, speaker.as_deref(), group.as_deref())?;
                spk.set_volume(*level)?;
                Ok(format!("Volume set to {} on {}", level, spk.name))
            }
            Commands::Mute { speaker, group } => {
                let spk = resolve_speaker(system, config, speaker.as_deref(), group.as_deref())?;
                spk.set_mute(true)?;
                Ok(format!("Muted {}", spk.name))
            }
            Commands::Unmute { speaker, group } => {
                let spk = resolve_speaker(system, config, speaker.as_deref(), group.as_deref())?;
                spk.set_mute(false)?;
                Ok(format!("Unmuted {}", spk.name))
            }
        }
    }
}

/// Resolve --speaker / --group flags to a Speaker handle.
///
/// Priority: --group wins over --speaker. If neither is given, uses config default
/// or falls back to the first available speaker.
fn resolve_speaker(
    system: &SonosSystem,
    config: &Config,
    speaker: Option<&str>,
    group: Option<&str>,
) -> Result<Speaker, CliError> {
    // --group wins over --speaker
    if let Some(group_name) = group {
        let g = system
            .get_group_by_name(group_name)
            .ok_or_else(|| CliError::GroupNotFound(group_name.to_string()))?;
        return g
            .coordinator()
            .ok_or_else(|| CliError::GroupNotFound(group_name.to_string()));
    }

    if let Some(speaker_name) = speaker {
        return system
            .get_speaker_by_name(speaker_name)
            .ok_or_else(|| CliError::SpeakerNotFound(speaker_name.to_string()));
    }

    // Default: config group → first speaker
    if let Some(default_group) = &config.default_group {
        if let Some(g) = system.get_group_by_name(default_group) {
            if let Some(coordinator) = g.coordinator() {
                return Ok(coordinator);
            }
        }
    }

    // Last resort: first speaker
    system
        .speakers()
        .into_iter()
        .next()
        .ok_or_else(|| CliError::SpeakerNotFound("no speakers available".to_string()))
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speakers_command_returns_list() {
        let system = SonosSystem::with_speakers(&["Kitchen", "Bedroom"]);
        let config = Config::default();
        let cmd = Commands::Speakers;
        let result = cmd.run(&system, &config).unwrap();
        assert!(result.contains("Kitchen"));
        assert!(result.contains("Bedroom"));
    }

    #[test]
    fn speakers_command_empty_system() {
        let system = SonosSystem::with_speakers(&[]);
        let config = Config::default();
        let cmd = Commands::Speakers;
        let result = cmd.run(&system, &config).unwrap();
        assert_eq!(result, "No speakers found");
    }

    #[test]
    fn resolve_speaker_by_name() {
        let system = SonosSystem::with_speakers(&["Kitchen"]);
        let config = Config::default();
        let spk = resolve_speaker(&system, &config, Some("Kitchen"), None).unwrap();
        assert_eq!(spk.name, "Kitchen");
    }

    #[test]
    fn resolve_speaker_not_found() {
        let system = SonosSystem::with_speakers(&["Kitchen"]);
        let config = Config::default();
        let result = resolve_speaker(&system, &config, Some("Nonexistent"), None);
        assert!(matches!(result, Err(CliError::SpeakerNotFound(_))));
    }

    #[test]
    fn resolve_speaker_falls_back_to_first() {
        let system = SonosSystem::with_speakers(&["Kitchen"]);
        let config = Config::default();
        let spk = resolve_speaker(&system, &config, None, None).unwrap();
        assert_eq!(spk.name, "Kitchen");
    }

    #[test]
    fn resolve_speaker_empty_system_fails() {
        let system = SonosSystem::with_speakers(&[]);
        let config = Config::default();
        let result = resolve_speaker(&system, &config, None, None);
        assert!(result.is_err());
    }
}
