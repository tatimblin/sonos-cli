use sonos_sdk::{Group, SonosSystem, Speaker};

use crate::cli::GlobalFlags;
use crate::config::Config;
use crate::errors::CliError;

/// Resolve --speaker / --group flags to a Speaker handle.
///
/// Priority: `--group` wins over `--speaker`. With neither flag, falls back to the
/// configured default group, then the coordinator of the **largest** group, then any
/// speaker.
pub fn resolve_speaker(
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
) -> Result<Speaker, CliError> {
    // --group wins over --speaker
    if let Some(group_name) = &global.group {
        let resolved = config.resolve_alias(group_name);
        let g = system
            .group(resolved)
            .ok_or_else(|| CliError::GroupNotFound(resolved.to_string()))?;
        return g
            .coordinator()
            .ok_or_else(|| CliError::GroupNotFound(resolved.to_string()));
    }

    if let Some(speaker_name) = &global.speaker {
        let resolved = config.resolve_alias(speaker_name);
        return system
            .speaker(resolved)
            .ok_or_else(|| CliError::SpeakerNotFound(resolved.to_string()));
    }

    // Default: config group → first group coordinator → first speaker
    if let Some(default_group) = &config.default_group {
        if let Some(g) = system.group(default_group) {
            if let Some(coordinator) = g.coordinator() {
                return Ok(coordinator);
            }
        }
    }

    // Prefer a group coordinator so we get track/position data
    // (non-coordinator speakers return NOT_IMPLEMENTED for these).
    //
    // Pick the *largest* group rather than whichever one happens to come first.
    // `groups()` has no meaningful order, so `.next()` chose arbitrarily: on a
    // household with a 4-speaker group playing and one idle standalone speaker,
    // bare `sonos status` reported the idle speaker as often as not.
    //
    // Member count is the right tiebreak because it comes from topology and is
    // free to read. Preferring a *playing* group would be more precise but costs
    // a SOAP fetch per group — in a fresh process the cache is cold, so it would
    // turn picking a default into N round-trips. Speakers are grouped in order to
    // play together, so the biggest group is the one the user means.
    //
    // Ties break on group id so repeated invocations agree.
    if let Some(coordinator) = largest_group_first(system)
        .into_iter()
        .find_map(|g| g.coordinator())
    {
        return Ok(coordinator);
    }

    // Last resort: first speaker (standalone, no groups)
    system
        .speakers()
        .into_iter()
        .next()
        .ok_or_else(|| CliError::SpeakerNotFound("no speakers available".to_string()))
}

/// Resolve --group / --speaker flags to a Group handle.
///
/// Priority: `--group` wins. With no flag, falls back to the configured default
/// group, then the **largest** group.
pub fn resolve_group(
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
) -> Result<Group, CliError> {
    if let Some(group_name) = &global.group {
        let resolved = config.resolve_alias(group_name);
        return system
            .group(resolved)
            .ok_or_else(|| CliError::GroupNotFound(resolved.to_string()));
    }

    // Default: config group → largest group
    if let Some(default_group) = &config.default_group {
        if let Some(g) = system.group(default_group) {
            return Ok(g);
        }
    }

    // Same reasoning as `resolve_speaker`: `groups()` is backed by a HashMap, so
    // `.next()` picked arbitrarily. This one matters more — `resolve_group` backs
    // the `volume` and `mute` *write* commands, so a bare `sonos volume 30` could
    // change the volume of whichever group happened to come out first.
    largest_group_first(system)
        .into_iter()
        .next()
        .ok_or_else(|| CliError::GroupNotFound("no groups available".to_string()))
}

/// Groups ordered by member count descending, ties broken on group id.
///
/// Shared by `resolve_speaker` and `resolve_group` so a bare command and its
/// group-scoped equivalent cannot disagree about which group they mean.
fn largest_group_first(system: &SonosSystem) -> Vec<Group> {
    let mut groups = system.groups();
    groups.sort_by(|a, b| {
        b.member_ids
            .len()
            .cmp(&a.member_ids.len())
            .then_with(|| a.id.as_str().cmp(b.id.as_str()))
    });
    groups
}

/// Resolve --speaker flag for speaker-only commands (bass, treble, loudness).
/// Rejects --group with a validation error.
pub fn require_speaker_only(
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
    command_name: &str,
) -> Result<Speaker, CliError> {
    if global.group.is_some() {
        return Err(CliError::Validation(format!(
            "--speaker is required for {command_name}"
        )));
    }
    let name = global
        .speaker
        .as_deref()
        .ok_or_else(|| CliError::Validation(format!("--speaker is required for {command_name}")))?;
    let resolved = config.resolve_alias(name);
    system
        .speaker(resolved)
        .ok_or_else(|| CliError::SpeakerNotFound(resolved.to_string()))
}

#[cfg(all(test, feature = "test-helpers"))]
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
            verbose: 0,
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
            verbose: 0,
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
            verbose: 0,
            no_input: false,
        };
        let spk = resolve_speaker(&system, &config, &global).unwrap();
        assert_eq!(spk.name, "Kitchen");
    }

    #[test]
    fn resolve_speaker_prefers_group_coordinator() {
        let system = SonosSystem::with_groups(&["Kitchen", "Bedroom"]);
        let config = Config::default();
        let global = GlobalFlags {
            speaker: None,
            group: None,
            quiet: false,
            verbose: 0,
            no_input: false,
        };
        let spk = resolve_speaker(&system, &config, &global).unwrap();

        // Must be *some* group's coordinator, not an arbitrary member. This
        // deliberately does not assert which one: the previous version compared
        // against `groups().into_iter().next()`, which baked the unordered
        // iteration order into the expectation — the very behaviour that made
        // bare `sonos status` pick a random speaker on real hardware.
        let is_a_coordinator = system
            .groups()
            .into_iter()
            .filter_map(|g| g.coordinator())
            .any(|c| c.name == spk.name);
        assert!(
            is_a_coordinator,
            "{} is not any group's coordinator",
            spk.name
        );
    }

    #[test]
    fn resolve_speaker_empty_system_fails() {
        let system = SonosSystem::with_speakers(&[]);
        let config = Config::default();
        let global = GlobalFlags {
            speaker: None,
            group: None,
            quiet: false,
            verbose: 0,
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
            verbose: 0,
            no_input: false,
        };
        let result = require_speaker_only(&system, &Config::default(), &global, "bass");
        assert!(matches!(result, Err(CliError::Validation(_))));
    }

    #[test]
    fn require_speaker_only_requires_speaker_flag() {
        let system = SonosSystem::with_speakers(&["Kitchen"]);
        let global = GlobalFlags {
            speaker: None,
            group: None,
            quiet: false,
            verbose: 0,
            no_input: false,
        };
        let result = require_speaker_only(&system, &Config::default(), &global, "bass");
        assert!(matches!(result, Err(CliError::Validation(_))));
    }

    #[test]
    fn require_speaker_only_finds_speaker() {
        let system = SonosSystem::with_speakers(&["Kitchen"]);
        let global = GlobalFlags {
            speaker: Some("Kitchen".into()),
            group: None,
            quiet: false,
            verbose: 0,
            no_input: false,
        };
        let spk = require_speaker_only(&system, &Config::default(), &global, "bass").unwrap();
        assert_eq!(spk.name, "Kitchen");
    }

    #[test]
    fn resolve_group_by_name() {
        let system = SonosSystem::with_groups(&["Kitchen", "Bedroom"]);
        let config = Config::default();
        let global = GlobalFlags {
            speaker: None,
            group: Some("Kitchen".into()),
            quiet: false,
            verbose: 0,
            no_input: false,
        };
        let grp = resolve_group(&system, &config, &global).unwrap();
        let coord = grp.coordinator().unwrap();
        assert_eq!(coord.name, "Kitchen");
    }

    #[test]
    fn resolve_group_not_found() {
        let system = SonosSystem::with_groups(&["Kitchen"]);
        let config = Config::default();
        let global = GlobalFlags {
            speaker: None,
            group: Some("Nonexistent".into()),
            quiet: false,
            verbose: 0,
            no_input: false,
        };
        let result = resolve_group(&system, &config, &global);
        assert!(matches!(result, Err(CliError::GroupNotFound(_))));
    }

    #[test]
    fn resolve_group_falls_back_to_first() {
        let system = SonosSystem::with_groups(&["Kitchen"]);
        let config = Config::default();
        let global = GlobalFlags {
            speaker: None,
            group: None,
            quiet: false,
            verbose: 0,
            no_input: false,
        };
        let grp = resolve_group(&system, &config, &global).unwrap();
        let coord = grp.coordinator().unwrap();
        assert_eq!(coord.name, "Kitchen");
    }

    #[test]
    fn resolve_group_uses_config_default() {
        let system = SonosSystem::with_groups(&["Kitchen", "Bedroom"]);
        let config = Config {
            default_group: Some("Bedroom".into()),
            ..Config::default()
        };
        let global = GlobalFlags {
            speaker: None,
            group: None,
            quiet: false,
            verbose: 0,
            no_input: false,
        };
        let grp = resolve_group(&system, &config, &global).unwrap();
        let coord = grp.coordinator().unwrap();
        assert_eq!(coord.name, "Bedroom");
    }

    #[test]
    fn resolve_group_empty_system_fails() {
        let system = SonosSystem::with_groups(&[]);
        let config = Config::default();
        let global = GlobalFlags {
            speaker: None,
            group: None,
            quiet: false,
            verbose: 0,
            no_input: false,
        };
        let result = resolve_group(&system, &config, &global);
        assert!(matches!(result, Err(CliError::GroupNotFound(_))));
    }

    #[test]
    fn resolve_group_flag_wins_over_speaker() {
        let system = SonosSystem::with_groups(&["Kitchen", "Bedroom"]);
        let config = Config::default();
        let global = GlobalFlags {
            speaker: Some("Bedroom".into()),
            group: Some("Kitchen".into()),
            quiet: false,
            verbose: 0,
            no_input: false,
        };
        let grp = resolve_group(&system, &config, &global).unwrap();
        let coord = grp.coordinator().unwrap();
        assert_eq!(coord.name, "Kitchen");
    }

    #[test]
    fn resolve_speaker_with_alias() {
        let system = SonosSystem::with_speakers(&["Master Bedroom"]);
        let mut config = Config::default();
        config.set_alias("Master Bedroom", "bed");
        let global = GlobalFlags {
            speaker: Some("bed".into()),
            group: None,
            quiet: false,
            verbose: 0,
            no_input: false,
        };
        let spk = resolve_speaker(&system, &config, &global).unwrap();
        assert_eq!(spk.name, "Master Bedroom");
    }

    #[test]
    fn resolve_speaker_alias_not_found_shows_resolved_name() {
        let system = SonosSystem::with_speakers(&["Kitchen"]);
        let mut config = Config::default();
        config.set_alias("Master Bedroom", "bed");
        let global = GlobalFlags {
            speaker: Some("bed".into()),
            group: None,
            quiet: false,
            verbose: 0,
            no_input: false,
        };
        let result = resolve_speaker(&system, &config, &global);
        assert!(matches!(result, Err(CliError::SpeakerNotFound(ref name)) if name == "Master Bedroom"));
    }

    #[test]
    fn resolve_group_with_alias() {
        let system = SonosSystem::with_groups(&["Living Room"]);
        let mut config = Config::default();
        config.set_alias("Living Room", "lr");
        let global = GlobalFlags {
            speaker: None,
            group: Some("lr".into()),
            quiet: false,
            verbose: 0,
            no_input: false,
        };
        let grp = resolve_group(&system, &config, &global).unwrap();
        let coord = grp.coordinator().unwrap();
        assert_eq!(coord.name, "Living Room");
    }

    #[test]
    fn require_speaker_only_resolves_alias() {
        let system = SonosSystem::with_speakers(&["Master Bedroom"]);
        let mut config = Config::default();
        config.set_alias("Master Bedroom", "bed");
        let global = GlobalFlags {
            speaker: Some("bed".into()),
            group: None,
            quiet: false,
            verbose: 0,
            no_input: false,
        };
        let spk = require_speaker_only(&system, &config, &global, "bass").unwrap();
        assert_eq!(spk.name, "Master Bedroom");
    }

    /// Build a system with one multi-member group beside a standalone speaker.
    ///
    /// The standalone group is listed *first* so an implementation that takes
    /// whatever `groups()` yields first can pick the wrong one.
    fn system_with_one_big_group() -> SonosSystem {
        use sonos_sdk::sonos_discovery::Device;
        use sonos_sdk::{GroupId, SpeakerId};

        let devices: Vec<Device> = ["Bedroom", "Living Room", "Bathroom", "Kitchen"]
            .iter()
            .enumerate()
            .map(|(i, name)| Device {
                id: format!("RINCON_{i:03}"),
                name: (*name).to_string(),
                room_name: (*name).to_string(),
                ip_address: format!("192.0.2.{}", 10 + i),
                port: 1400,
                model_name: "Sonos One".to_string(),
            })
            .collect();

        SonosSystem::from_devices_offline_with_groups(
            devices,
            vec![
                // Kitchen, standalone — deliberately first.
                (
                    GroupId::new("RINCON_003:1"),
                    SpeakerId::new("RINCON_003"),
                    vec![SpeakerId::new("RINCON_003")],
                ),
                // Bedroom coordinating a three-speaker group.
                (
                    GroupId::new("RINCON_000:1"),
                    SpeakerId::new("RINCON_000"),
                    vec![
                        SpeakerId::new("RINCON_000"),
                        SpeakerId::new("RINCON_001"),
                        SpeakerId::new("RINCON_002"),
                    ],
                ),
            ],
        )
        .expect("offline construction cannot fail")
    }

    fn no_flags() -> GlobalFlags {
        GlobalFlags {
            speaker: None,
            group: None,
            quiet: false,
            verbose: 0,
            no_input: false,
        }
    }

    /// Bare `sonos status` must report the group the user is listening to.
    ///
    /// Hardware regression: with a four-speaker group playing and one idle
    /// standalone speaker, `sonos status` reported the idle speaker, because the
    /// old code took `groups().into_iter().next()` and `groups` is a `HashMap`.
    #[test]
    fn resolve_speaker_defaults_to_largest_group() {
        let spk = resolve_speaker(&system_with_one_big_group(), &Config::default(), &no_flags())
            .expect("a speaker must resolve");
        assert_eq!(
            spk.name, "Bedroom",
            "expected the three-member group's coordinator, got {}",
            spk.name
        );
    }

    /// Same for `resolve_group`, which backs the `volume` and `mute` *writes* —
    /// so picking arbitrarily here changes the wrong speakers' volume.
    #[test]
    fn resolve_group_defaults_to_largest_group() {
        let grp = resolve_group(&system_with_one_big_group(), &Config::default(), &no_flags())
            .expect("a group must resolve");
        assert_eq!(grp.member_ids.len(), 3, "expected the three-member group");
        assert_eq!(
            grp.coordinator().map(|c| c.name),
            Some("Bedroom".to_string())
        );
    }
}
