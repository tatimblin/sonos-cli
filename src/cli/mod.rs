//! CLI command parsing and helpers.
//!
//! The `Commands` enum handles clap parsing only. Command execution
//! happens in `main.rs` via direct SDK calls — no centralized dispatch.

use clap::{Args, Parser, Subcommand};

use crate::config::Config;
use crate::errors::CliError;
use sonos_sdk::{Speaker, SonosSystem};

/// Top-level CLI parser.
#[derive(Debug, Parser)]
#[command(name = "sonos", about = "Control Sonos speakers", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
    #[command(flatten)]
    pub global: GlobalFlags,
}

/// Global flags accepted by every command.
#[derive(Debug, Args)]
pub struct GlobalFlags {
    /// Target a specific speaker by friendly name
    #[arg(long, global = true)]
    pub speaker: Option<String>,
    /// Target a group by name
    #[arg(long, global = true)]
    pub group: Option<String>,
    /// Suppress all non-error stdout output
    #[arg(long, short, global = true)]
    pub quiet: bool,
    /// Show raw SDK errors and debug output on stderr
    #[arg(long, global = true)]
    pub verbose: bool,
    /// Disable all interactive prompts
    #[arg(long, global = true)]
    pub no_input: bool,
}

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
        /// Play mode: normal, repeat, repeat-one, shuffle, shuffle-no-repeat
        mode: String,
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
        state: String,
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

// ---------------------------------------------------------------------------
// Target resolution helpers
// ---------------------------------------------------------------------------

/// Resolve --speaker / --group flags to a Speaker handle.
///
/// Priority: --group wins over --speaker. If neither is given, uses config default
/// or falls back to the first available speaker.
pub fn resolve_speaker(
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
) -> Result<Speaker, CliError> {
    // --group wins over --speaker
    if let Some(group_name) = &global.group {
        let g = system
            .group(group_name)
            .ok_or_else(|| CliError::GroupNotFound(group_name.to_string()))?;
        return g
            .coordinator()
            .ok_or_else(|| CliError::GroupNotFound(group_name.to_string()));
    }

    if let Some(speaker_name) = &global.speaker {
        return system
            .speaker(speaker_name)
            .ok_or_else(|| CliError::SpeakerNotFound(speaker_name.to_string()));
    }

    // Default: config group → first speaker
    if let Some(default_group) = &config.default_group {
        if let Some(g) = system.group(default_group) {
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

/// Resolve --speaker flag for speaker-only commands (bass, treble, loudness).
/// Rejects --group with a validation error.
pub fn require_speaker_only(
    system: &SonosSystem,
    global: &GlobalFlags,
    command_name: &str,
) -> Result<Speaker, CliError> {
    if global.group.is_some() {
        return Err(CliError::Validation(format!(
            "--speaker is required for {}",
            command_name
        )));
    }
    let name = global.speaker.as_deref().ok_or_else(|| {
        CliError::Validation(format!("--speaker is required for {}", command_name))
    })?;
    system
        .speaker(name)
        .ok_or_else(|| CliError::SpeakerNotFound(name.to_string()))
}

// ---------------------------------------------------------------------------
// Validation / parsing helpers
// ---------------------------------------------------------------------------

/// Validate a seek time string (H:MM:SS or HH:MM:SS format).
pub fn validate_seek_time(input: &str) -> Result<(), CliError> {
    let parts: Vec<&str> = input.split(':').collect();
    if parts.len() != 3 {
        return Err(CliError::Validation(format!(
            "invalid position \"{}\" — expected H:MM:SS format",
            input
        )));
    }
    let _hours: u32 = parts[0].parse().map_err(|_| {
        CliError::Validation(format!("invalid position \"{}\" — hours must be a number", input))
    })?;
    let minutes: u32 = parts[1].parse().map_err(|_| {
        CliError::Validation(format!(
            "invalid position \"{}\" — minutes must be a number",
            input
        ))
    })?;
    let seconds: u32 = parts[2].parse().map_err(|_| {
        CliError::Validation(format!(
            "invalid position \"{}\" — seconds must be a number",
            input
        ))
    })?;
    if minutes > 59 {
        return Err(CliError::Validation(format!(
            "invalid position \"{}\" — minutes must be 0–59",
            input
        )));
    }
    if seconds > 59 {
        return Err(CliError::Validation(format!(
            "invalid position \"{}\" — seconds must be 0–59",
            input
        )));
    }
    Ok(())
}

/// Parse a CLI mode string into an SDK PlayMode.
pub fn parse_play_mode(input: &str) -> Result<sonos_sdk::PlayMode, CliError> {
    match input {
        "normal" => Ok(sonos_sdk::PlayMode::Normal),
        "repeat" => Ok(sonos_sdk::PlayMode::RepeatAll),
        "repeat-one" => Ok(sonos_sdk::PlayMode::RepeatOne),
        "shuffle" => Ok(sonos_sdk::PlayMode::Shuffle),
        "shuffle-no-repeat" => Ok(sonos_sdk::PlayMode::ShuffleNoRepeat),
        _ => Err(CliError::Validation(format!(
            "unknown mode \"{}\" — valid modes: normal, repeat, repeat-one, shuffle, shuffle-no-repeat",
            input
        ))),
    }
}

/// Parse a duration string (e.g. "30m", "1h", "90m") into HH:MM:SS format.
pub fn parse_duration(input: &str) -> Result<String, CliError> {
    let (num_str, unit) = if input.ends_with('m') {
        (&input[..input.len() - 1], 'm')
    } else if input.ends_with('h') {
        (&input[..input.len() - 1], 'h')
    } else {
        return Err(CliError::Validation(format!(
            "invalid duration \"{}\" — use a unit suffix: 30m or 1h",
            input
        )));
    };

    let value: u32 = num_str.parse().map_err(|_| {
        CliError::Validation(format!(
            "invalid duration \"{}\" — use a unit suffix: 30m or 1h",
            input
        ))
    })?;

    let total_minutes = match unit {
        'h' => value * 60,
        'm' => value,
        _ => unreachable!(),
    };

    let hours = total_minutes / 60;
    let minutes = total_minutes % 60;
    Ok(format!("{:02}:{:02}:00", hours, minutes))
}

/// Format a duration string for human-readable output.
pub fn format_duration_human(input: &str) -> String {
    if input.ends_with('m') {
        let num = &input[..input.len() - 1];
        if num == "1" {
            "1 minute".to_string()
        } else {
            format!("{} minutes", num)
        }
    } else if input.ends_with('h') {
        let num = &input[..input.len() - 1];
        if num == "1" {
            "1 hour".to_string()
        } else {
            format!("{} hours", num)
        }
    } else {
        input.to_string()
    }
}

/// Format a playback state as an icon string.
pub fn playback_icon(state: &sonos_sdk::PlaybackState) -> &'static str {
    match state {
        sonos_sdk::PlaybackState::Playing => "\u{25b6}",
        sonos_sdk::PlaybackState::Paused => "\u{23f8}",
        _ => "\u{25a0}",
    }
}

/// Format a playback state as a word.
pub fn playback_label(state: &sonos_sdk::PlaybackState) -> &'static str {
    match state {
        sonos_sdk::PlaybackState::Playing => "Playing",
        sonos_sdk::PlaybackState::Paused => "Paused",
        sonos_sdk::PlaybackState::Stopped => "Stopped",
        sonos_sdk::PlaybackState::Transitioning => "Transitioning",
    }
}

/// Format milliseconds as M:SS or H:MM:SS.
pub fn format_time_ms(ms: u64) -> String {
    let total_secs = ms / 1000;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{}:{:02}", minutes, seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // resolve_speaker tests
    // -----------------------------------------------------------------------

    #[test]
    fn resolve_speaker_by_name() {
        let system = SonosSystem::with_speakers(&["Kitchen"]);
        let config = Config::default();
        let global = GlobalFlags {
            speaker: Some("Kitchen".into()),
            group: None,
            quiet: false,
            verbose: false,
            no_input: false,
        };
        let spk = resolve_speaker(&system, &config, &global).unwrap();
        assert_eq!(spk.name, "Kitchen");
    }

    #[test]
    fn resolve_speaker_not_found() {
        let system = SonosSystem::with_speakers(&["Kitchen"]);
        let config = Config::default();
        let global = GlobalFlags {
            speaker: Some("Nonexistent".into()),
            group: None,
            quiet: false,
            verbose: false,
            no_input: false,
        };
        let result = resolve_speaker(&system, &config, &global);
        assert!(matches!(result, Err(CliError::SpeakerNotFound(_))));
    }

    #[test]
    fn resolve_speaker_falls_back_to_first() {
        let system = SonosSystem::with_speakers(&["Kitchen"]);
        let config = Config::default();
        let global = GlobalFlags {
            speaker: None,
            group: None,
            quiet: false,
            verbose: false,
            no_input: false,
        };
        let spk = resolve_speaker(&system, &config, &global).unwrap();
        assert_eq!(spk.name, "Kitchen");
    }

    #[test]
    fn resolve_speaker_empty_system_fails() {
        let system = SonosSystem::with_speakers(&[]);
        let config = Config::default();
        let global = GlobalFlags {
            speaker: None,
            group: None,
            quiet: false,
            verbose: false,
            no_input: false,
        };
        let result = resolve_speaker(&system, &config, &global);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // require_speaker_only tests
    // -----------------------------------------------------------------------

    #[test]
    fn require_speaker_only_rejects_group() {
        let system = SonosSystem::with_speakers(&["Kitchen"]);
        let global = GlobalFlags {
            speaker: None,
            group: Some("Living Room".into()),
            quiet: false,
            verbose: false,
            no_input: false,
        };
        let result = require_speaker_only(&system, &global, "bass");
        assert!(matches!(result, Err(CliError::Validation(_))));
    }

    #[test]
    fn require_speaker_only_requires_speaker_flag() {
        let system = SonosSystem::with_speakers(&["Kitchen"]);
        let global = GlobalFlags {
            speaker: None,
            group: None,
            quiet: false,
            verbose: false,
            no_input: false,
        };
        let result = require_speaker_only(&system, &global, "bass");
        assert!(matches!(result, Err(CliError::Validation(_))));
    }

    #[test]
    fn require_speaker_only_finds_speaker() {
        let system = SonosSystem::with_speakers(&["Kitchen"]);
        let global = GlobalFlags {
            speaker: Some("Kitchen".into()),
            group: None,
            quiet: false,
            verbose: false,
            no_input: false,
        };
        let spk = require_speaker_only(&system, &global, "bass").unwrap();
        assert_eq!(spk.name, "Kitchen");
    }

    // -----------------------------------------------------------------------
    // validate_seek_time tests
    // -----------------------------------------------------------------------

    #[test]
    fn valid_seek_times() {
        assert!(validate_seek_time("0:00:00").is_ok());
        assert!(validate_seek_time("0:02:30").is_ok());
        assert!(validate_seek_time("1:30:00").is_ok());
        assert!(validate_seek_time("12:59:59").is_ok());
    }

    #[test]
    fn invalid_seek_time_bad_seconds() {
        let result = validate_seek_time("0:02:70");
        assert!(matches!(result, Err(CliError::Validation(ref s)) if s.contains("seconds")));
    }

    #[test]
    fn invalid_seek_time_bad_minutes() {
        let result = validate_seek_time("0:70:00");
        assert!(matches!(result, Err(CliError::Validation(ref s)) if s.contains("minutes")));
    }

    #[test]
    fn invalid_seek_time_bad_format() {
        assert!(validate_seek_time("2:30").is_err());
        assert!(validate_seek_time("abc").is_err());
    }

    // -----------------------------------------------------------------------
    // parse_play_mode tests
    // -----------------------------------------------------------------------

    #[test]
    fn parse_all_play_modes() {
        assert!(matches!(parse_play_mode("normal"), Ok(sonos_sdk::PlayMode::Normal)));
        assert!(matches!(parse_play_mode("repeat"), Ok(sonos_sdk::PlayMode::RepeatAll)));
        assert!(matches!(parse_play_mode("repeat-one"), Ok(sonos_sdk::PlayMode::RepeatOne)));
        assert!(matches!(parse_play_mode("shuffle"), Ok(sonos_sdk::PlayMode::Shuffle)));
        assert!(matches!(
            parse_play_mode("shuffle-no-repeat"),
            Ok(sonos_sdk::PlayMode::ShuffleNoRepeat)
        ));
    }

    #[test]
    fn parse_play_mode_unknown() {
        let result = parse_play_mode("loop");
        assert!(matches!(result, Err(CliError::Validation(ref s)) if s.contains("unknown mode")));
    }

    // -----------------------------------------------------------------------
    // parse_duration tests
    // -----------------------------------------------------------------------

    #[test]
    fn parse_duration_minutes() {
        assert_eq!(parse_duration("30m").unwrap(), "00:30:00");
        assert_eq!(parse_duration("90m").unwrap(), "01:30:00");
        assert_eq!(parse_duration("1m").unwrap(), "00:01:00");
    }

    #[test]
    fn parse_duration_hours() {
        assert_eq!(parse_duration("1h").unwrap(), "01:00:00");
        assert_eq!(parse_duration("2h").unwrap(), "02:00:00");
    }

    #[test]
    fn parse_duration_invalid() {
        assert!(parse_duration("30").is_err());
        assert!(parse_duration("abc").is_err());
        assert!(parse_duration("").is_err());
    }

    // -----------------------------------------------------------------------
    // format_duration_human tests
    // -----------------------------------------------------------------------

    #[test]
    fn format_duration_human_values() {
        assert_eq!(format_duration_human("30m"), "30 minutes");
        assert_eq!(format_duration_human("1m"), "1 minute");
        assert_eq!(format_duration_human("1h"), "1 hour");
        assert_eq!(format_duration_human("2h"), "2 hours");
    }

    // -----------------------------------------------------------------------
    // format_time_ms tests
    // -----------------------------------------------------------------------

    #[test]
    fn format_time_ms_values() {
        assert_eq!(format_time_ms(0), "0:00");
        assert_eq!(format_time_ms(151_000), "2:31");
        assert_eq!(format_time_ms(355_000), "5:55");
        assert_eq!(format_time_ms(3_661_000), "1:01:01");
    }
}
