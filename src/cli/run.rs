use std::io::IsTerminal;

use sonos_sdk::{
    MusicSurroundLevel, NightMode, SatelliteRole, SeekTarget, SonosSystem, Speaker,
    SpeechEnhancement, SubEnabled, SubLevel, SurroundEnabled, SurroundLevel,
};

use super::config_cmd::run_config_action;
use super::{
    coordinator_name, find_group, find_speaker, format_duration_human, format_time_ms,
    parse_duration, require_speaker_only, required_flag, resolve_speaker, resolve_target,
    state_display, track_display, validate_seek_time, Commands, GlobalFlags, LevelOrSwitch, OnOff,
    QueueAction, Target,
};
use crate::config::Config;
use crate::diagnostics;
use crate::errors::CliError;
use crate::liveness::{CommandOutput, Probes};

/// Run a command and report both what it printed and whether a speaker
/// actually answered while producing it.
///
/// Most arms return [`CommandOutput::live`]: they end in a SOAP call whose
/// error is propagated with `?`, so reaching `Ok` is itself proof of contact.
/// The read-only listings (`speakers`, `groups`, `status`) are the exceptions —
/// they tolerate per-field failure by design, which is exactly how a total
/// discovery failure used to render as success.
pub fn run_command(
    cmd: Commands,
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
) -> Result<CommandOutput, CliError> {
    let spk = || resolve_speaker(system, config, global);

    match cmd {
        Commands::Speakers => cmd_speakers(system),
        Commands::Groups => cmd_groups(system),
        Commands::Status => cmd_status(system, config, global),
        Commands::Join => cmd_join(system, config, global),
        Commands::Leave => cmd_leave(system, config, global),
        Commands::Bass { level } => cmd_bass(system, config, global, level),
        Commands::Treble { level } => cmd_treble(system, config, global, level),
        Commands::Loudness { state } => cmd_loudness(system, config, global, state),
        Commands::Sub { value } => cmd_sub(system, config, global, value),
        Commands::Surround { value, tv } => cmd_surround(system, config, global, value, tv),
        Commands::Night { state } => cmd_night(system, config, global, state),
        Commands::Speech { state } => cmd_speech(system, config, global, state),
        Commands::Sleep { duration } => cmd_sleep(system, config, global, &duration),
        Commands::Queue { action } => cmd_queue(system, config, global, action),
        // `main` runs these before discovery via `run_config_command`; this arm
        // keeps `run_command` total for library callers. No speaker is probed,
        // so there is nothing unvalidated to flag.
        Commands::Config {
            action: Some(action),
        } => run_config_action(&action, &mut config.clone()).map(CommandOutput::live),
        Commands::Config { action: None } => Err(CliError::Validation(
            "missing config action (see `sonos config --help`)".to_string(),
        )),

        Commands::Play => {
            let s = spk()?;
            s.play()?;
            Ok(CommandOutput::live(format!("Playing ({})", s.name)))
        }
        Commands::Pause => {
            let s = spk()?;
            s.pause()?;
            Ok(CommandOutput::live(format!("Paused ({})", s.name)))
        }
        Commands::Stop => {
            let s = spk()?;
            s.stop()?;
            Ok(CommandOutput::live(format!("Stopped ({})", s.name)))
        }
        Commands::Next => {
            let s = spk()?;
            s.next()?;
            Ok(CommandOutput::live(format!("Next track ({})", s.name)))
        }
        Commands::Previous => {
            let s = spk()?;
            s.previous()?;
            Ok(CommandOutput::live(format!("Previous track ({})", s.name)))
        }
        Commands::Seek { position } => {
            validate_seek_time(&position)?;
            let s = spk()?;
            s.seek(SeekTarget::Time(position.clone()))?;
            Ok(CommandOutput::live(format!(
                "Seeked to {} ({})",
                position, s.name
            )))
        }
        Commands::Mode { mode } => {
            let s = spk()?;
            s.set_play_mode(mode.to_sdk())?;
            Ok(CommandOutput::live(format!(
                "Mode set to {:?} ({})",
                mode, s.name
            )))
        }
        Commands::Volume { level } => cmd_volume(system, config, global, level),
        Commands::Mute => cmd_mute(system, config, global, true),
        Commands::Unmute => cmd_mute(system, config, global, false),
    }
}

// -- Command handlers ---------------------------------------------------------

fn cmd_speakers(system: &SonosSystem) -> Result<CommandOutput, CliError> {
    let speakers = system.speakers();
    if speakers.is_empty() {
        eprintln!("{}", diagnostics::discovery_hint());
        // No speaker was claimed, so there is nothing unvalidated to flag; the
        // hint above already says what went wrong. See `liveness::verdict`.
        return Ok(CommandOutput::live("No speakers found"));
    }
    // Two probes per speaker. The visible symptom of a dead network is these
    // two failing and each row collapsing to a bare cached name — so counting
    // them is exactly the evidence that the row is unverified.
    let mut probes = Probes::default();
    let lines: Vec<String> = speakers
        .iter()
        .map(|s| {
            let state = probes.record(s.playback_state.fetch());
            let vol = probes.record(s.volume.fetch());
            // `group()` reads the in-memory topology, not the network — not a probe.
            let group_name = s
                .group()
                .and_then(|g| coordinator_name(&g))
                .unwrap_or_default();

            let state_str = state.as_ref().map(state_display).unwrap_or_default();
            let vol_str = vol.map(|v| format!("vol:{}", v.0)).unwrap_or_default();

            let mut parts = vec![s.name.clone()];
            if !state_str.is_empty() {
                parts.push(state_str);
            }
            if !vol_str.is_empty() {
                parts.push(vol_str);
            }
            if !group_name.is_empty() {
                parts.push(format!("({group_name})"));
            }
            parts.join("   ")
        })
        .collect();
    Ok(CommandOutput::new(lines.join("\n"), probes.verdict()))
}

fn cmd_groups(system: &SonosSystem) -> Result<CommandOutput, CliError> {
    let groups = system.groups();
    if groups.is_empty() {
        eprintln!("{}", diagnostics::discovery_hint());
        return Ok(CommandOutput::live("No groups found"));
    }
    let mut probes = Probes::default();
    let lines: Vec<String> = groups
        .iter()
        .map(|g| {
            let coord = g.coordinator();
            let coord_name = coord
                .as_ref()
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "unknown".to_string());

            // A group with no coordinator attempts nothing on the speaker, so
            // it must not be counted as two failures.
            let state = coord
                .as_ref()
                .and_then(|c| probes.record(c.playback_state.fetch()));
            let track = coord
                .as_ref()
                .and_then(|c| probes.record(c.current_track.fetch()));
            let vol = probes.record(g.volume.fetch());

            let state_str = state.as_ref().map(state_display).unwrap_or_default();
            let track_str = track.as_ref().map(track_display).unwrap_or_default();
            let vol_str = vol.map(|v| format!("vol:{}", v.0)).unwrap_or_default();

            let mut parts = vec![coord_name];
            if !state_str.is_empty() {
                parts.push(state_str);
            }
            if !track_str.is_empty() {
                parts.push(track_str);
            }
            if !vol_str.is_empty() {
                parts.push(vol_str);
            }
            parts.join("   ")
        })
        .collect();
    Ok(CommandOutput::new(lines.join("\n"), probes.verdict()))
}

fn cmd_volume(
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
    level: u8,
) -> Result<CommandOutput, CliError> {
    let target = resolve_target(system, config, global)?;
    match &target {
        Target::Speaker(s) => s.set_volume(level)?,
        // Group volume goes through GroupRenderingControl, which takes a u16.
        Target::Group(g) => g.set_volume(u16::from(level))?,
    }
    Ok(CommandOutput::live(format!(
        "Volume set to {level} ({})",
        target.display_name()
    )))
}

fn cmd_mute(
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
    muted: bool,
) -> Result<CommandOutput, CliError> {
    let label = if muted { "Muted" } else { "Unmuted" };
    let target = resolve_target(system, config, global)?;
    match &target {
        Target::Speaker(s) => s.set_mute(muted)?,
        Target::Group(g) => g.set_mute(muted)?,
    }
    Ok(CommandOutput::live(format!(
        "{label} ({})",
        target.display_name()
    )))
}

fn cmd_status(
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
) -> Result<CommandOutput, CliError> {
    let spk = resolve_speaker(system, config, global)?;
    // Same bug as `speakers`, one row wide: with every fetch failing this
    // printed "<name> unknown" and exited 0.
    //
    // A non-coordinator speaker answers NOT_IMPLEMENTED for track and
    // position — a real reply from a real speaker that arrives here as an
    // `Err`. That is fine: `volume` still succeeds, and one success is enough
    // for `Live`, so a partially-answering speaker is never reported as dead.
    let mut probes = Probes::default();
    let state = probes.record(spk.playback_state.fetch());
    let track = probes.record(spk.current_track.fetch());
    let pos = probes.record(spk.position.fetch());
    let vol = probes.record(spk.volume.fetch());

    let state_str = state
        .as_ref()
        .map(state_display)
        .unwrap_or_else(|| "unknown".to_string());
    let track_str = track.as_ref().map(track_display).unwrap_or_default();
    let pos_str = pos
        .as_ref()
        .map(|p| {
            format!(
                "{}/{}",
                format_time_ms(p.position_ms),
                format_time_ms(p.duration_ms)
            )
        })
        .unwrap_or_default();
    let vol_str = vol.map(|v| format!("vol:{}", v.0)).unwrap_or_default();

    let mut parts = vec![spk.name.clone(), state_str];
    if !track_str.is_empty() {
        parts.push(track_str);
    }
    if !pos_str.is_empty() {
        parts.push(pos_str);
    }
    if !vol_str.is_empty() {
        parts.push(vol_str);
    }
    let mut output = parts.join("  ");
    output.push_str(&satellite_lines(&spk));
    Ok(CommandOutput::new(output, probes.verdict()))
}

/// One indented line per satellite (`  Left surround • Sonos One`), each
/// preceded by a newline; empty for a speaker without satellites.
fn satellite_lines(spk: &Speaker) -> String {
    spk.satellites()
        .iter()
        .map(|sat| match &sat.model_name {
            Some(model) => format!("\n  {} \u{2022} {model}", sat.role.label()),
            None => format!("\n  {}", sat.role.label()),
        })
        .collect()
}

fn cmd_join(
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
) -> Result<CommandOutput, CliError> {
    // Both flags are checked before either lookup, so a missing flag is
    // reported ahead of a name that doesn't resolve.
    let raw_speaker = required_flag(global.speaker.as_deref(), "--speaker", "join")?;
    let raw_group = required_flag(global.group.as_deref(), "--group", "join")?;
    let speaker_name = config.resolve_alias(raw_speaker);
    let group_name = config.resolve_alias(raw_group);
    let spk = find_speaker(system, speaker_name)?;
    let grp = find_group(system, group_name)?;
    grp.add_speaker(&spk)?;
    Ok(CommandOutput::live(format!(
        "{speaker_name} joined {group_name}"
    )))
}

fn cmd_leave(
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
) -> Result<CommandOutput, CliError> {
    let raw_speaker = required_flag(global.speaker.as_deref(), "--speaker", "leave")?;
    let speaker_name = config.resolve_alias(raw_speaker);
    let spk = find_speaker(system, speaker_name)?;
    let group_name = spk
        .group()
        .and_then(|g| coordinator_name(&g))
        .unwrap_or_else(|| "its group".into());
    spk.leave_group()?;
    Ok(CommandOutput::live(format!(
        "{speaker_name} left {group_name}"
    )))
}

fn cmd_bass(
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
    level: i8,
) -> Result<CommandOutput, CliError> {
    let spk = require_speaker_only(system, config, global, "bass")?;
    spk.set_bass(level)?;
    Ok(CommandOutput::live(format!(
        "Bass set to {} ({})",
        level, spk.name
    )))
}

fn cmd_treble(
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
    level: i8,
) -> Result<CommandOutput, CliError> {
    let spk = require_speaker_only(system, config, global, "treble")?;
    spk.set_treble(level)?;
    Ok(CommandOutput::live(format!(
        "Treble set to {} ({})",
        level, spk.name
    )))
}

fn cmd_loudness(
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
    state: OnOff,
) -> Result<CommandOutput, CliError> {
    let spk = require_speaker_only(system, config, global, "loudness")?;
    let enabled = matches!(state, OnOff::On);
    spk.set_loudness(enabled)?;
    if enabled {
        Ok(CommandOutput::live(format!(
            "Loudness enabled ({})",
            spk.name
        )))
    } else {
        Ok(CommandOutput::live(format!(
            "Loudness disabled ({})",
            spk.name
        )))
    }
}

/// The home theater speaker for a satellite setting, or an error naming the
/// missing hardware: a soundbar accepts sub and surround settings even with
/// none bonded, so without this check the command would silently do nothing.
fn require_satellite(
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
    command_name: &str,
    has: fn(&SatelliteRole) -> bool,
    missing: &str,
) -> Result<Speaker, CliError> {
    let spk = require_speaker_only(system, config, global, command_name)?;
    if !spk.satellites().iter().any(|s| has(&s.role)) {
        return Err(CliError::Unsupported(format!(
            "{} has no {missing}",
            spk.name
        )));
    }
    Ok(spk)
}

fn on_off(enabled: bool) -> &'static str {
    if enabled {
        "on"
    } else {
        "off"
    }
}

fn cmd_sub(
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
    value: LevelOrSwitch,
) -> Result<CommandOutput, CliError> {
    let spk = require_satellite(
        system,
        config,
        global,
        "sub",
        |r| *r == SatelliteRole::Sub,
        "sub",
    )?;
    let message = match value {
        LevelOrSwitch::Level(level) => {
            spk.sub_level.set(SubLevel::new(level))?;
            format!("Sub level set to {level:+} ({})", spk.name)
        }
        LevelOrSwitch::Switch(enabled) => {
            spk.sub_enabled.set(SubEnabled(enabled))?;
            format!("Sub {} ({})", on_off(enabled), spk.name)
        }
    };
    Ok(CommandOutput::live(message))
}

fn cmd_surround(
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
    value: LevelOrSwitch,
    tv: bool,
) -> Result<CommandOutput, CliError> {
    let spk = require_satellite(
        system,
        config,
        global,
        "surround",
        SatelliteRole::is_surround,
        "surrounds",
    )?;
    let message =
        match value {
            LevelOrSwitch::Level(level) if tv => {
                spk.surround_level.set(SurroundLevel::new(level))?;
                format!("TV surround level set to {level:+} ({})", spk.name)
            }
            LevelOrSwitch::Level(level) => {
                spk.music_surround_level
                    .set(MusicSurroundLevel::new(level))?;
                format!("Music surround level set to {level:+} ({})", spk.name)
            }
            LevelOrSwitch::Switch(_) if tv => return Err(CliError::Validation(
                "--tv applies to a level; surrounds switch on and off for TV and music together"
                    .to_string(),
            )),
            LevelOrSwitch::Switch(enabled) => {
                spk.surround_enabled.set(SurroundEnabled(enabled))?;
                format!("Surrounds {} ({})", on_off(enabled), spk.name)
            }
        };
    Ok(CommandOutput::live(message))
}

fn cmd_night(
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
    state: OnOff,
) -> Result<CommandOutput, CliError> {
    let spk = require_speaker_only(system, config, global, "night")?;
    let enabled = matches!(state, OnOff::On);
    spk.night_mode.set(NightMode(enabled))?;
    Ok(CommandOutput::live(format!(
        "Night mode {} ({})",
        on_off(enabled),
        spk.name
    )))
}

fn cmd_speech(
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
    state: OnOff,
) -> Result<CommandOutput, CliError> {
    let spk = require_speaker_only(system, config, global, "speech")?;
    let enabled = matches!(state, OnOff::On);
    spk.speech_enhancement.set(SpeechEnhancement(enabled))?;
    Ok(CommandOutput::live(format!(
        "Speech enhancement {} ({})",
        on_off(enabled),
        spk.name
    )))
}

fn cmd_sleep(
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
    duration: &str,
) -> Result<CommandOutput, CliError> {
    let spk = resolve_speaker(system, config, global)?;
    if duration == "cancel" {
        spk.cancel_sleep_timer()?;
        Ok(CommandOutput::live(format!(
            "Sleep timer cancelled ({})",
            spk.name
        )))
    } else {
        let hh_mm_ss = parse_duration(duration)?;
        let human = format_duration_human(duration);
        spk.configure_sleep_timer(&hh_mm_ss)?;
        Ok(CommandOutput::live(format!(
            "Sleep timer set for {} ({})",
            human, spk.name
        )))
    }
}

fn cmd_queue(
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
    action: Option<QueueAction>,
) -> Result<CommandOutput, CliError> {
    let spk = resolve_speaker(system, config, global)?;
    match action {
        None => {
            let info = spk.get_media_info()?;
            if info.nr_tracks == 0 {
                return Ok(CommandOutput::live(format!(
                    "queue is empty ({})",
                    spk.name
                )));
            }
            Ok(CommandOutput::live(format!(
                "{} — {} tracks",
                spk.name, info.nr_tracks
            )))
        }
        Some(QueueAction::Add { uri }) => {
            spk.add_uri_to_queue(&uri, "", 0, false)?;
            Ok(CommandOutput::live(format!(
                "Added to queue ({})",
                spk.name
            )))
        }
        Some(QueueAction::Clear) => {
            if std::io::stdin().is_terminal() && !global.no_input {
                eprint!("Clear queue for {}? [y/N] ", spk.name);
                let mut input = String::new();
                std::io::stdin()
                    .read_line(&mut input)
                    .map_err(|e| CliError::Validation(e.to_string()))?;
                if !input.trim().eq_ignore_ascii_case("y") {
                    return Ok(CommandOutput::live("Cancelled"));
                }
            }
            spk.remove_all_tracks_from_queue()?;
            Ok(CommandOutput::live(format!("Queue cleared ({})", spk.name)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sonos_sdk::sonos_discovery::Device;
    use sonos_sdk::{GroupId, SpeakerId};

    /// Basement: a Playbar bonded to two Sonos One surrounds, no sub.
    fn basement() -> SonosSystem {
        let device = |id: &str, ip: &str, model: &str| Device {
            id: id.to_string(),
            name: "Basement".to_string(),
            room_name: "Basement".to_string(),
            ip_address: ip.to_string(),
            port: 1400,
            model_name: model.to_string(),
        };
        let playbar = SpeakerId::new("RINCON_PLAYBAR");
        SonosSystem::from_devices_offline_with_home_theaters(
            vec![
                device("RINCON_PLAYBAR", "192.0.2.10", "Sonos Playbar"),
                device("RINCON_LEFT", "192.0.2.11", "Sonos One"),
                device("RINCON_RIGHT", "192.0.2.12", "Sonos One"),
            ],
            vec![(
                GroupId::new("RINCON_PLAYBAR:1"),
                playbar.clone(),
                vec![playbar.clone()],
            )],
            vec![(
                playbar,
                vec![
                    (SpeakerId::new("RINCON_LEFT"), SatelliteRole::LeftSurround),
                    (SpeakerId::new("RINCON_RIGHT"), SatelliteRole::RightSurround),
                ],
            )],
        )
        .unwrap()
    }

    fn basement_flags() -> GlobalFlags {
        GlobalFlags {
            speaker: Some("Basement".into()),
            ..Default::default()
        }
    }

    #[test]
    fn sub_settings_need_a_bonded_sub() {
        let system = basement();
        let result = require_satellite(
            &system,
            &Config::default(),
            &basement_flags(),
            "sub",
            |r| *r == SatelliteRole::Sub,
            "sub",
        );
        match result {
            Err(e @ CliError::Unsupported(_)) => {
                assert_eq!(e.to_string(), "Basement has no sub");
                assert_eq!(e.exit_code(), std::process::ExitCode::from(1));
            }
            other => panic!("expected Unsupported, got {:?}", other.map(|s| s.name)),
        }
    }

    #[test]
    fn surround_settings_find_the_surrounds() {
        let system = basement();
        let spk = require_satellite(
            &system,
            &Config::default(),
            &basement_flags(),
            "surround",
            SatelliteRole::is_surround,
            "surrounds",
        )
        .unwrap();
        assert_eq!(spk.name, "Basement");
    }

    #[test]
    fn surround_on_off_rejects_tv() {
        let result = cmd_surround(
            &basement(),
            &Config::default(),
            &basement_flags(),
            LevelOrSwitch::Switch(false),
            true,
        );
        assert!(matches!(result, Err(CliError::Validation(ref m)) if m.contains("--tv")));
    }

    #[test]
    fn status_lists_each_satellite() {
        let system = basement();
        let spk = system.speaker("Basement").unwrap();
        assert_eq!(
            satellite_lines(&spk),
            "\n  Left surround \u{2022} Sonos One\n  Right surround \u{2022} Sonos One"
        );
        let kitchen = SonosSystem::with_speakers(&["Kitchen"]);
        assert_eq!(satellite_lines(&kitchen.speaker("Kitchen").unwrap()), "");
    }
}
