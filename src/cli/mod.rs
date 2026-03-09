//! CLI command parsing using clap.
//!
//! Maps command-line arguments to Action values.

use clap::Subcommand;

use crate::actions::{Action, Target};

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
    /// Convert CLI command to Action.
    pub fn into_action(self) -> Action {
        match self {
            Self::Speakers => Action::ListSpeakers,
            Self::Groups => Action::ListGroups,
            Self::Status { speaker, group } => Action::Status {
                target: resolve_target_args(speaker, group),
            },
            Self::Play { speaker, group } => Action::Play {
                target: resolve_target_args(speaker, group),
            },
            Self::Pause { speaker, group } => Action::Pause {
                target: resolve_target_args(speaker, group),
            },
            Self::Stop { speaker, group } => Action::Stop {
                target: resolve_target_args(speaker, group),
            },
            Self::Next { speaker, group } => Action::Next {
                target: resolve_target_args(speaker, group),
            },
            Self::Previous { speaker, group } => Action::Previous {
                target: resolve_target_args(speaker, group),
            },
            Self::Volume {
                level,
                speaker,
                group,
            } => Action::SetVolume {
                level,
                target: resolve_target_args(speaker, group),
            },
            Self::Mute { speaker, group } => Action::Mute {
                target: resolve_target_args(speaker, group),
            },
            Self::Unmute { speaker, group } => Action::Unmute {
                target: resolve_target_args(speaker, group),
            },
        }
    }
}

/// Convert CLI args to Target. --group wins over --speaker.
fn resolve_target_args(speaker: Option<String>, group: Option<String>) -> Target {
    match (group, speaker) {
        (Some(g), _) => Target::Group(g),
        (None, Some(s)) => Target::Speaker(s),
        (None, None) => Target::Default,
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_wins_over_speaker() {
        let target = resolve_target_args(
            Some("Kitchen".to_string()),
            Some("Living Room".to_string()),
        );
        assert!(matches!(target, Target::Group(g) if g == "Living Room"));
    }

    #[test]
    fn speaker_used_when_no_group() {
        let target = resolve_target_args(Some("Kitchen".to_string()), None);
        assert!(matches!(target, Target::Speaker(s) if s == "Kitchen"));
    }

    #[test]
    fn default_when_neither_specified() {
        let target = resolve_target_args(None, None);
        assert!(matches!(target, Target::Default));
    }

    #[test]
    fn volume_command_preserves_level() {
        let cmd = Commands::Volume {
            level: 75,
            speaker: None,
            group: Some("Room".to_string()),
        };
        match cmd.into_action() {
            Action::SetVolume { level, target } => {
                assert_eq!(level, 75);
                assert!(matches!(target, Target::Group(g) if g == "Room"));
            }
            _ => panic!("Expected SetVolume action"),
        }
    }

    #[test]
    fn play_with_speaker_flag() {
        let cmd = Commands::Play {
            speaker: Some("Bedroom".to_string()),
            group: None,
        };
        match cmd.into_action() {
            Action::Play { target } => {
                assert!(matches!(target, Target::Speaker(s) if s == "Bedroom"));
            }
            _ => panic!("Expected Play action"),
        }
    }
}
