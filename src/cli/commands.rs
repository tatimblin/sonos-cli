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
        level: i8,
    },
    /// Set treble level (-10 to 10), speaker-only
    Treble {
        /// Treble level (-10 to 10)
        level: i8,
    },
    /// Set loudness compensation (on/off), speaker-only
    Loudness {
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
