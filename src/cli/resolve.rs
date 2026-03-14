use sonos_sdk::{Speaker, SonosSystem};

use crate::cli::GlobalFlags;
use crate::config::Config;
use crate::errors::CliError;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
