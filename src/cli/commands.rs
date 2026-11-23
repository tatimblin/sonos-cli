use clap::{Subcommand, ValueEnum};

/// CLI subcommands — parsing only, no execution logic.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// List all speakers
    Speakers,
    /// List all groups
    Groups,
    /// Show current playback status
    Status,
    /// Start playback
    Play,
    /// Pause playback
    Pause,
    /// Stop playback
    Stop,
    /// Skip to next track
    Next,
    /// Skip to previous track
    #[command(name = "prev")]
    Previous,
    /// Set volume level (0-100)
    Volume {
        /// Volume level (0-100)
        // Ranges here mirror sonos-api's own validation, so an out-of-range
        // value is a clap usage error (exit 2) rather than an SDK error that
        // exits 1 with a network hint.
        #[arg(value_parser = clap::value_parser!(u8).range(0..=100))]
        level: u8,
    },
    /// Mute playback
    Mute,
    /// Unmute playback
    Unmute,
    /// Seek to a position in the current track
    Seek {
        /// Target position in H:MM:SS format
        position: String,
    },
    /// Set play mode
    Mode {
        /// Play mode
        #[arg(value_enum)]
        mode: PlayModeArg,
    },
    /// Set bass level (-10 to 10), speaker-only
    Bass {
        /// Bass level (-10 to 10)
        #[arg(value_parser = clap::value_parser!(i8).range(-10..=10), allow_negative_numbers = true)]
        level: i8,
    },
    /// Set treble level (-10 to 10), speaker-only
    Treble {
        /// Treble level (-10 to 10)
        #[arg(value_parser = clap::value_parser!(i8).range(-10..=10), allow_negative_numbers = true)]
        level: i8,
    },
    /// Set loudness compensation (on/off), speaker-only
    Loudness {
        /// on or off
        #[arg(value_enum)]
        state: OnOff,
    },
    /// Set the sub level (-15 to 15) or turn the sub on/off, speaker-only
    Sub {
        /// Level (-15 to 15), on, or off
        #[arg(value_parser = parse_level_or_switch, allow_negative_numbers = true)]
        value: LevelOrSwitch,
    },
    /// Set the surround level (-15 to 15) or turn the surrounds on/off, speaker-only
    Surround {
        /// Level (-15 to 15), on, or off
        #[arg(value_parser = parse_level_or_switch, allow_negative_numbers = true)]
        value: LevelOrSwitch,
        /// Set the level used while playing TV instead of music
        #[arg(long)]
        tv: bool,
    },
    /// Set night mode (on/off) on a home theater speaker
    Night {
        /// on or off
        #[arg(value_enum)]
        state: OnOff,
    },
    /// Set speech enhancement (on/off) on a home theater speaker
    Speech {
        /// on or off
        #[arg(value_enum)]
        state: OnOff,
    },
    /// Add a speaker to a group
    Join,
    /// Remove a speaker from its group
    Leave,
    /// Set or cancel a sleep timer
    Sleep {
        /// Duration (30m, 1h, 90m) or "cancel"
        duration: String,
    },
    /// Manage the playback queue
    Queue {
        #[command(subcommand)]
        action: Option<QueueAction>,
    },
    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
}

/// Config sub-subcommands.
#[derive(Debug, Subcommand)]
pub enum ConfigAction {
    /// Manage speaker/group aliases
    Alias {
        /// Speaker or group name
        name: Option<String>,
        /// Short alias to assign
        alias: Option<String>,
    },
}

/// Queue sub-subcommands.
#[derive(Debug, Subcommand)]
pub enum QueueAction {
    /// Add a URI to the queue
    Add {
        /// Sonos URI to add
        uri: String,
    },
    /// Clear the entire queue
    Clear,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum OnOff {
    On,
    Off,
}

/// A home theater level (-15 to 15), or switching the hardware on or off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelOrSwitch {
    Level(i8),
    Switch(bool),
}

/// Parse `on`, `off`, or a level from -15 to 15. Out of range is a usage
/// error, matching the SDK's own validation.
fn parse_level_or_switch(value: &str) -> Result<LevelOrSwitch, String> {
    match value.to_ascii_lowercase().as_str() {
        "on" => return Ok(LevelOrSwitch::Switch(true)),
        "off" => return Ok(LevelOrSwitch::Switch(false)),
        _ => {}
    }
    match value.parse::<i8>() {
        Ok(level) if (-15..=15).contains(&level) => Ok(LevelOrSwitch::Level(level)),
        _ => Err("expected a level from -15 to 15, on, or off".to_string()),
    }
}

#[derive(Debug, Clone, ValueEnum)]
pub enum PlayModeArg {
    Normal,
    Repeat,
    #[value(name = "repeat-one")]
    RepeatOne,
    Shuffle,
    #[value(name = "shuffle-no-repeat")]
    ShuffleNoRepeat,
}

impl PlayModeArg {
    pub fn to_sdk(&self) -> sonos_sdk::PlayMode {
        match self {
            Self::Normal => sonos_sdk::PlayMode::Normal,
            Self::Repeat => sonos_sdk::PlayMode::RepeatAll,
            Self::RepeatOne => sonos_sdk::PlayMode::RepeatOne,
            Self::Shuffle => sonos_sdk::PlayMode::Shuffle,
            Self::ShuffleNoRepeat => sonos_sdk::PlayMode::ShuffleNoRepeat,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Commands;
    use crate::cli::Cli;
    use clap::error::ErrorKind;
    use clap::Parser;

    fn command(args: &[&str]) -> Commands {
        Cli::try_parse_from(args)
            .expect("should parse")
            .command
            .expect("a subcommand")
    }

    fn parse_err(args: &[&str]) -> clap::Error {
        Cli::try_parse_from(args).expect_err("should be rejected")
    }

    #[test]
    fn volume_accepts_bounds() {
        assert!(matches!(
            command(&["sonos", "volume", "0"]),
            Commands::Volume { level: 0 }
        ));
        assert!(matches!(
            command(&["sonos", "volume", "100"]),
            Commands::Volume { level: 100 }
        ));
    }

    #[test]
    fn volume_out_of_range_is_usage_error() {
        for level in ["101", "150"] {
            let err = parse_err(&["sonos", "volume", level]);
            assert_eq!(err.kind(), ErrorKind::ValueValidation);
            // clap usage errors exit 2, the same as `CliError::Validation`.
            assert_eq!(err.exit_code(), 2);
        }
    }

    #[test]
    fn sub_and_surround_take_a_level_or_a_switch() {
        use super::LevelOrSwitch;
        assert!(matches!(
            command(&["sonos", "sub", "-15", "--speaker", "Basement"]),
            Commands::Sub {
                value: LevelOrSwitch::Level(-15)
            }
        ));
        assert!(matches!(
            command(&["sonos", "sub", "off"]),
            Commands::Sub {
                value: LevelOrSwitch::Switch(false)
            }
        ));
        assert!(matches!(
            command(&["sonos", "surround", "8", "--tv"]),
            Commands::Surround {
                value: LevelOrSwitch::Level(8),
                tv: true
            }
        ));
        assert!(matches!(
            command(&["sonos", "surround", "ON"]),
            Commands::Surround {
                value: LevelOrSwitch::Switch(true),
                tv: false
            }
        ));
    }

    #[test]
    fn sub_and_surround_reject_out_of_range_levels() {
        for args in [
            &["sonos", "sub", "16"][..],
            &["sonos", "surround", "-16"],
            &["sonos", "surround", "loud"],
        ] {
            let err = parse_err(args);
            assert_eq!(err.kind(), ErrorKind::ValueValidation, "{args:?}");
            assert_eq!(err.exit_code(), 2);
        }
    }

    #[test]
    fn bass_and_treble_accept_bounds() {
        // Negative levels must parse as values, not as unknown short flags.
        assert!(matches!(
            command(&["sonos", "bass", "-10", "--speaker", "Kitchen"]),
            Commands::Bass { level: -10 }
        ));
        assert!(matches!(
            command(&["sonos", "treble", "10"]),
            Commands::Treble { level: 10 }
        ));
    }

    #[test]
    fn bass_and_treble_out_of_range_are_usage_errors() {
        for cmd in ["bass", "treble"] {
            for level in ["11", "-11"] {
                let err = parse_err(&["sonos", cmd, level]);
                assert_eq!(err.kind(), ErrorKind::ValueValidation, "{cmd} {level}");
                assert_eq!(err.exit_code(), 2);
            }
        }
    }
}
