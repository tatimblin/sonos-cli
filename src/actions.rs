//! Action and Target enums for the sonos-cli application.
//!
//! Both CLI and TUI emit Action values, which are dispatched through the executor.

/// Target specifies which speaker or group an action applies to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A specific speaker by friendly name
    Speaker(String),
    /// A specific group by coordinator name
    Group(String),
    /// Use config.default_group or first discovered group
    Default,
}

/// Play mode for playback control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayMode {
    Normal,
    RepeatAll,
    RepeatOne,
    ShuffleNoRepeat,
    Shuffle,
    ShuffleRepeatOne,
}

/// All operations the CLI/TUI can perform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    // Queries
    ListSpeakers,
    ListGroups,
    Status { target: Target },

    // Playback
    Play { target: Target },
    Pause { target: Target },
    Stop { target: Target },
    Next { target: Target },
    Previous { target: Target },
    Seek { position: String, target: Target },
    SetPlayMode { mode: PlayMode, target: Target },

    // Volume
    SetVolume { level: u8, target: Target },
    Mute { target: Target },
    Unmute { target: Target },

    // EQ (speaker-only)
    SetBass { level: i8, speaker: String },
    SetTreble { level: i8, speaker: String },
    SetLoudness { enabled: bool, speaker: String },

    // Queue
    ShowQueue { target: Target },
    AddToQueue { uri: String, target: Target },
    ClearQueue { target: Target },

    // Grouping
    JoinGroup { speaker: String, group: String },
    LeaveGroup { speaker: String },

    // Sleep timer
    SetSleepTimer { duration: String, target: Target },
    CancelSleepTimer { target: Target },
}
