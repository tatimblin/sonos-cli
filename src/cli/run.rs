use std::io::IsTerminal;

use sonos_sdk::{SeekTarget, SonosSystem};

use super::{
    format_duration_human, format_time_ms, parse_duration, playback_icon, playback_label,
    require_speaker_only, resolve_group, resolve_speaker, validate_seek_time, Commands,
    GlobalFlags, OnOff, QueueAction,
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
        Commands::Sleep { duration } => cmd_sleep(system, config, global, &duration),
        Commands::Queue { action } => cmd_queue(system, config, global, action),
        Commands::Config { .. } => unreachable!("config commands dispatched before discovery"),

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
                .and_then(|g| g.coordinator().map(|c| c.name))
                .unwrap_or_default();

            let state_str = state
                .as_ref()
                .map(|st| format!("{} {}", playback_icon(st), playback_label(st)))
                .unwrap_or_default();
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

            let state_str = state
                .as_ref()
                .map(|st| format!("{} {}", playback_icon(st), playback_label(st)))
                .unwrap_or_default();
            let track_str = track
                .as_ref()
                .map(|t| {
                    let d = t.display();
                    if d == "Unknown" {
                        t.uri
                            .as_deref()
                            .filter(|u| !u.is_empty() && !u.starts_with("x-rincon:"))
                            .map(|_| "Playing (no metadata)".to_string())
                            .unwrap_or(d)
                    } else {
                        d
                    }
                })
                .unwrap_or_default();
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
    // Explicit --speaker (without --group) → Speaker.set_volume(u8)
    if global.speaker.is_some() && global.group.is_none() {
        let s = resolve_speaker(system, config, global)?;
        s.set_volume(level)?;
        return Ok(CommandOutput::live(format!(
            "Volume set to {} ({})",
            level, s.name
        )));
    }
    // Otherwise → Group.set_volume(u16) via GroupRenderingControl
    let g = resolve_group(system, config, global)?;
    let name = g
        .coordinator()
        .map(|c| c.name)
        .unwrap_or_else(|| "unknown".to_string());
    g.set_volume(level as u16)?;
    Ok(CommandOutput::live(format!(
        "Volume set to {level} ({name})"
    )))
}

fn cmd_mute(
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
    muted: bool,
) -> Result<CommandOutput, CliError> {
    let label = if muted { "Muted" } else { "Unmuted" };
    // Explicit --speaker (without --group) → Speaker.set_mute(bool)
    if global.speaker.is_some() && global.group.is_none() {
        let s = resolve_speaker(system, config, global)?;
        s.set_mute(muted)?;
        return Ok(CommandOutput::live(format!("{} ({})", label, s.name)));
    }
    // Otherwise → Group.set_mute(bool) via GroupRenderingControl
    let g = resolve_group(system, config, global)?;
    let name = g
        .coordinator()
        .map(|c| c.name)
        .unwrap_or_else(|| "unknown".to_string());
    g.set_mute(muted)?;
    Ok(CommandOutput::live(format!("{label} ({name})")))
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
        .map(|st| format!("{} {}", playback_icon(st), playback_label(st)))
        .unwrap_or_else(|| "unknown".to_string());
    let track_str = track
        .as_ref()
        .map(|t| {
            let d = t.display();
            if d == "Unknown" {
                t.uri
                    .as_deref()
                    .filter(|u| !u.is_empty() && !u.starts_with("x-rincon:"))
                    .map(|_| "Playing (no metadata)".to_string())
                    .unwrap_or(d)
            } else {
                d
            }
        })
        .unwrap_or_default();
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
    Ok(CommandOutput::new(parts.join("  "), probes.verdict()))
}

fn cmd_join(
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
) -> Result<CommandOutput, CliError> {
    let raw_speaker = global
        .speaker
        .as_deref()
        .ok_or_else(|| CliError::Validation("--speaker is required for join".into()))?;
    let raw_group = global
        .group
        .as_deref()
        .ok_or_else(|| CliError::Validation("--group is required for join".into()))?;
    let speaker_name = config.resolve_alias(raw_speaker);
    let group_name = config.resolve_alias(raw_group);
    let spk = system
        .speaker(speaker_name)
        .ok_or_else(|| CliError::SpeakerNotFound(speaker_name.into()))?;
    let grp = system
        .group(group_name)
        .ok_or_else(|| CliError::GroupNotFound(group_name.into()))?;
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
    let raw_speaker = global
        .speaker
        .as_deref()
        .ok_or_else(|| CliError::Validation("--speaker is required for leave".into()))?;
    let speaker_name = config.resolve_alias(raw_speaker);
    let spk = system
        .speaker(speaker_name)
        .ok_or_else(|| CliError::SpeakerNotFound(speaker_name.into()))?;
    let group_name = spk
        .group()
        .and_then(|g| g.coordinator().map(|c| c.name))
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
