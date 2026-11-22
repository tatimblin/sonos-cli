use sonos_sdk::{Group, SonosSystem, Speaker};

use crate::cli::GlobalFlags;
use crate::config::Config;
use crate::errors::CliError;

/// What a speaker-or-group command (`volume`, `mute`) acts on.
pub enum Target {
    Speaker(Speaker),
    Group(Group),
}

impl Target {
    /// The name shown in command output: the speaker, or the group's coordinator.
    pub fn display_name(&self) -> String {
        match self {
            Self::Speaker(s) => s.name.clone(),
            Self::Group(g) => coordinator_name(g).unwrap_or_else(|| "unknown".to_string()),
        }
    }
}

/// Resolve --speaker / --group flags for commands that can act on either.
///
/// `--group` wins over `--speaker`: only an explicit `--speaker` on its own
/// targets a single speaker. Everything else — `--group`, both flags, or
/// neither — targets a group, falling back as `resolve_group` does.
pub fn resolve_target(
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
) -> Result<Target, CliError> {
    if global.speaker.is_some() && global.group.is_none() {
        resolve_speaker(system, config, global).map(Target::Speaker)
    } else {
        resolve_group(system, config, global).map(Target::Group)
    }
}

/// The friendly name of a group's coordinator, if it has one.
pub fn coordinator_name(group: &Group) -> Option<String> {
    group.coordinator().map(|c| c.name)
}

/// Look up a speaker by its (already alias-resolved) name.
pub fn find_speaker(system: &SonosSystem, name: &str) -> Result<Speaker, CliError> {
    system
        .speaker(name)
        .ok_or_else(|| CliError::SpeakerNotFound(name.to_string()))
}

/// Look up a group by its (already alias-resolved) coordinator name.
pub fn find_group(system: &SonosSystem, name: &str) -> Result<Group, CliError> {
    system
        .group(name)
        .ok_or_else(|| CliError::GroupNotFound(name.to_string()))
}

/// The value of a flag a command cannot run without, e.g. `--speaker` for `join`.
pub fn required_flag<'a>(
    value: Option<&'a str>,
    flag: &str,
    command_name: &str,
) -> Result<&'a str, CliError> {
    value.ok_or_else(|| CliError::Validation(format!("{flag} is required for {command_name}")))
}

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
        return find_group(system, resolved)?
            .coordinator()
            .ok_or_else(|| CliError::GroupNotFound(resolved.to_string()));
    }

    if let Some(speaker_name) = &global.speaker {
        return find_speaker(system, config.resolve_alias(speaker_name));
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
    // Pick the *largest* group: `groups()` returns largest first, in a stable
    // order. Member count comes from topology and is free to read. Preferring a
    // *playing* group would be more precise but costs a SOAP fetch per group —
    // in a fresh process the cache is cold, so it would turn picking a default
    // into N round-trips. Speakers are grouped in order to play together, so the
    // biggest group is the one the user means.
    if let Some(coordinator) = system.groups().into_iter().find_map(|g| g.coordinator()) {
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
        return find_group(system, config.resolve_alias(group_name));
    }

    // Default: config group → largest group
    if let Some(default_group) = &config.default_group {
        if let Some(g) = system.group(default_group) {
            return Ok(g);
        }
    }

    // Same choice as `resolve_speaker`, so a bare command and its group-scoped
    // equivalent agree. It matters more here: `resolve_group` backs the `volume`
    // and `mute` *write* commands.
    system
        .groups()
        .into_iter()
        .next()
        .ok_or_else(|| CliError::GroupNotFound("no groups available".to_string()))
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
            "{command_name} applies to a single speaker; use --speaker instead of --group"
        )));
    }
    let name = required_flag(global.speaker.as_deref(), "--speaker", command_name)?;
    find_speaker(system, config.resolve_alias(name))
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
            ..Default::default()
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
            ..Default::default()
        };
        let result = resolve_speaker(&system, &config, &global);
        assert!(matches!(result, Err(CliError::SpeakerNotFound(_))));
    }

    #[test]
    fn resolve_speaker_falls_back_to_first() {
        let system = SonosSystem::with_speakers(&["Kitchen"]);
        let config = Config::default();
        let global = GlobalFlags::default();
        let spk = resolve_speaker(&system, &config, &global).unwrap();
        assert_eq!(spk.name, "Kitchen");
    }

    #[test]
    fn resolve_speaker_prefers_group_coordinator() {
        let system = SonosSystem::with_groups(&["Kitchen", "Bedroom"]);
        let config = Config::default();
        let global = GlobalFlags::default();
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
        let global = GlobalFlags::default();
        let result = resolve_speaker(&system, &config, &global);
        assert!(result.is_err());
    }

    #[test]
    fn require_speaker_only_rejects_group() {
        let system = SonosSystem::with_speakers(&["Kitchen"]);
        let global = GlobalFlags {
            speaker: None,
            group: Some("Living Room".into()),
            ..Default::default()
        };
        let result = require_speaker_only(&system, &Config::default(), &global, "bass");
        // The message must name --group: telling the user "--speaker is
        // required" when they passed --group reads as if their flag was lost.
        assert!(
            matches!(result, Err(CliError::Validation(ref msg)) if msg.contains("--group")),
            "expected a validation error naming --group"
        );
    }

    #[test]
    fn require_speaker_only_requires_speaker_flag() {
        let system = SonosSystem::with_speakers(&["Kitchen"]);
        let global = GlobalFlags::default();
        let result = require_speaker_only(&system, &Config::default(), &global, "bass");
        assert!(matches!(result, Err(CliError::Validation(_))));
    }

    #[test]
    fn require_speaker_only_finds_speaker() {
        let system = SonosSystem::with_speakers(&["Kitchen"]);
        let global = GlobalFlags {
            speaker: Some("Kitchen".into()),
            group: None,
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
        };
        let result = resolve_group(&system, &config, &global);
        assert!(matches!(result, Err(CliError::GroupNotFound(_))));
    }

    #[test]
    fn resolve_group_falls_back_to_first() {
        let system = SonosSystem::with_groups(&["Kitchen"]);
        let config = Config::default();
        let global = GlobalFlags::default();
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
        let global = GlobalFlags::default();
        let grp = resolve_group(&system, &config, &global).unwrap();
        let coord = grp.coordinator().unwrap();
        assert_eq!(coord.name, "Bedroom");
    }

    #[test]
    fn resolve_group_empty_system_fails() {
        let system = SonosSystem::with_groups(&[]);
        let config = Config::default();
        let global = GlobalFlags::default();
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
        };
        let result = resolve_speaker(&system, &config, &global);
        assert!(
            matches!(result, Err(CliError::SpeakerNotFound(ref name)) if name == "Master Bedroom")
        );
    }

    #[test]
    fn resolve_group_with_alias() {
        let system = SonosSystem::with_groups(&["Living Room"]);
        let mut config = Config::default();
        config.set_alias("Living Room", "lr");
        let global = GlobalFlags {
            speaker: None,
            group: Some("lr".into()),
            ..Default::default()
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
            ..Default::default()
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
        GlobalFlags::default()
    }

    fn flags(speaker: Option<&str>, group: Option<&str>) -> GlobalFlags {
        GlobalFlags {
            speaker: speaker.map(Into::into),
            group: group.map(Into::into),
            ..Default::default()
        }
    }

    #[test]
    fn resolve_target_speaker_alone_targets_speaker() {
        let system = SonosSystem::with_groups(&["Kitchen", "Bedroom"]);
        let target =
            resolve_target(&system, &Config::default(), &flags(Some("Bedroom"), None)).unwrap();
        assert!(matches!(target, Target::Speaker(ref s) if s.name == "Bedroom"));
        assert_eq!(target.display_name(), "Bedroom");
    }

    #[test]
    fn resolve_target_group_alone_targets_group() {
        let system = SonosSystem::with_groups(&["Kitchen", "Bedroom"]);
        let target =
            resolve_target(&system, &Config::default(), &flags(None, Some("Kitchen"))).unwrap();
        assert!(matches!(target, Target::Group(_)));
        assert_eq!(target.display_name(), "Kitchen");
    }

    /// CLAUDE.md: "--group wins over --speaker".
    #[test]
    fn resolve_target_group_wins_over_speaker() {
        let system = SonosSystem::with_groups(&["Kitchen", "Bedroom"]);
        let target = resolve_target(
            &system,
            &Config::default(),
            &flags(Some("Bedroom"), Some("Kitchen")),
        )
        .unwrap();
        assert!(matches!(target, Target::Group(_)));
        assert_eq!(target.display_name(), "Kitchen");
    }

    #[test]
    fn resolve_target_no_flags_targets_largest_group() {
        let target = resolve_target(
            &system_with_one_big_group(),
            &Config::default(),
            &no_flags(),
        )
        .unwrap();
        assert!(matches!(target, Target::Group(ref g) if g.member_ids.len() == 3));
        assert_eq!(target.display_name(), "Bedroom");
    }

    #[test]
    fn resolve_target_resolves_aliases() {
        let system = SonosSystem::with_speakers(&["Master Bedroom"]);
        let mut config = Config::default();
        config.set_alias("Master Bedroom", "bed");
        let target = resolve_target(&system, &config, &flags(Some("bed"), None)).unwrap();
        assert_eq!(target.display_name(), "Master Bedroom");
    }

    #[test]
    fn resolve_target_reports_missing_names() {
        let system = SonosSystem::with_groups(&["Kitchen"]);
        let config = Config::default();
        assert!(matches!(
            resolve_target(&system, &config, &flags(Some("Nope"), None)),
            Err(CliError::SpeakerNotFound(ref n)) if n == "Nope"
        ));
        assert!(matches!(
            resolve_target(&system, &config, &flags(None, Some("Nope"))),
            Err(CliError::GroupNotFound(ref n)) if n == "Nope"
        ));
    }

    #[test]
    fn required_flag_names_flag_and_command() {
        assert_eq!(required_flag(Some("x"), "--speaker", "join").unwrap(), "x");
        assert!(matches!(
            required_flag(None, "--group", "join"),
            Err(CliError::Validation(ref m)) if m == "--group is required for join"
        ));
    }

    /// Bare `sonos status` must report the group the user is listening to.
    ///
    /// Hardware regression: with a four-speaker group playing and one idle
    /// standalone speaker, `sonos status` reported the idle speaker, because the
    /// old code took `groups().into_iter().next()` and `groups` is a `HashMap`.
    #[test]
    fn resolve_speaker_defaults_to_largest_group() {
        let spk = resolve_speaker(
            &system_with_one_big_group(),
            &Config::default(),
            &no_flags(),
        )
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
        let grp = resolve_group(
            &system_with_one_big_group(),
            &Config::default(),
            &no_flags(),
        )
        .expect("a group must resolve");
        assert_eq!(grp.member_ids.len(), 3, "expected the three-member group");
        assert_eq!(
            grp.coordinator().map(|c| c.name),
            Some("Bedroom".to_string())
        );
    }
}
