use std::io::IsTerminal;

use sonos_sdk::{SeekTarget, SonosSystem};

use super::{
    format_duration_human, format_time_ms, parse_duration, playback_icon, playback_label,
    require_speaker_only, resolve_speaker, validate_seek_time, Commands, GlobalFlags, OnOff,
    QueueAction,
};
use crate::config::Config;
use crate::errors::CliError;

pub fn run_command(
    cmd: Commands,
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
) -> Result<String, CliError> {
    let spk = || resolve_speaker(system, config, global);

    match cmd {
        Commands::Speakers => cmd_speakers(system),
        Commands::Groups => cmd_groups(system),
        Commands::Status => cmd_status(system, config, global),
        Commands::Join => cmd_join(system, global),
        Commands::Leave => cmd_leave(system, global),
        Commands::Bass { level } => cmd_bass(system, global, level),
        Commands::Treble { level } => cmd_treble(system, global, level),
        Commands::Loudness { state } => cmd_loudness(system, global, state),
        Commands::Sleep { duration } => cmd_sleep(system, config, global, &duration),
        Commands::Queue { action } => cmd_queue(system, config, global, action),

        Commands::Play => {
            let s = spk()?;
            s.play()?;
            Ok(format!("Playing ({})", s.name))
        }
        Commands::Pause => {
            let s = spk()?;
            s.pause()?;
            Ok(format!("Paused ({})", s.name))
        }
        Commands::Stop => {
            let s = spk()?;
            s.stop()?;
            Ok(format!("Stopped ({})", s.name))
        }
        Commands::Next => {
            let s = spk()?;
            s.next()?;
            Ok(format!("Next track ({})", s.name))
        }
        Commands::Previous => {
            let s = spk()?;
            s.previous()?;
            Ok(format!("Previous track ({})", s.name))
        }
        Commands::Seek { position } => {
            validate_seek_time(&position)?;
            let s = spk()?;
            s.seek(SeekTarget::Time(position.clone()))?;
            Ok(format!("Seeked to {} ({})", position, s.name))
        }
        Commands::Mode { mode } => {
            let s = spk()?;
            s.set_play_mode(mode.to_sdk())?;
            Ok(format!("Mode set to {:?} ({})", mode, s.name))
        }
        Commands::Volume { level } => {
            let s = spk()?;
            s.set_volume(level)?;
            Ok(format!("Volume set to {} ({})", level, s.name))
        }
        Commands::Mute => {
            let s = spk()?;
            s.set_mute(true)?;
            Ok(format!("Muted ({})", s.name))
        }
        Commands::Unmute => {
            let s = spk()?;
            s.set_mute(false)?;
            Ok(format!("Unmuted ({})", s.name))
        }
    }
}

// -- Command handlers ---------------------------------------------------------

fn cmd_speakers(system: &SonosSystem) -> Result<String, CliError> {
    let speakers = system.speakers();
    if speakers.is_empty() {
        return Ok("No speakers found".to_string());
    }
    let lines: Vec<String> = speakers
        .iter()
        .map(|s| {
            let state = s.playback_state.fetch().ok();
            let vol = s.volume.fetch().ok();
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
                parts.push(format!("({})", group_name));
            }
            parts.join("   ")
        })
        .collect();
    Ok(lines.join("\n"))
}

fn cmd_groups(system: &SonosSystem) -> Result<String, CliError> {
    let groups = system.groups();
    if groups.is_empty() {
        return Ok("No groups found".to_string());
    }
    let lines: Vec<String> = groups
        .iter()
        .map(|g| {
            let coord = g.coordinator();
            let coord_name = coord
                .as_ref()
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "unknown".to_string());

            let state = coord.as_ref().and_then(|c| c.playback_state.fetch().ok());
            let track = coord.as_ref().and_then(|c| c.current_track.fetch().ok());
            let vol = g.volume.fetch().ok();

            let state_str = state
                .as_ref()
                .map(|st| format!("{} {}", playback_icon(st), playback_label(st)))
                .unwrap_or_default();
            let track_str = track.as_ref().map(|t| t.display()).unwrap_or_default();
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
    Ok(lines.join("\n"))
}

fn cmd_status(
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
) -> Result<String, CliError> {
    let spk = resolve_speaker(system, config, global)?;
    let state = spk.playback_state.fetch().ok();
    let track = spk.current_track.fetch().ok();
    let pos = spk.position.fetch().ok();
    let vol = spk.volume.fetch().ok();

    let state_str = state
        .as_ref()
        .map(|st| format!("{} {}", playback_icon(st), playback_label(st)))
        .unwrap_or_else(|| "unknown".to_string());
    let track_str = track.as_ref().map(|t| t.display()).unwrap_or_default();
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
    Ok(parts.join("  "))
}

fn cmd_join(system: &SonosSystem, global: &GlobalFlags) -> Result<String, CliError> {
    let speaker_name = global
        .speaker
        .as_deref()
        .ok_or_else(|| CliError::Validation("--speaker is required for join".into()))?;
    let group_name = global
        .group
        .as_deref()
        .ok_or_else(|| CliError::Validation("--group is required for join".into()))?;
    let spk = system
        .speaker(speaker_name)
        .ok_or_else(|| CliError::SpeakerNotFound(speaker_name.into()))?;
    let grp = system
        .group(group_name)
        .ok_or_else(|| CliError::GroupNotFound(group_name.into()))?;
    grp.add_speaker(&spk)?;
    Ok(format!("{} joined {}", speaker_name, group_name))
}

fn cmd_leave(system: &SonosSystem, global: &GlobalFlags) -> Result<String, CliError> {
    let speaker_name = global
        .speaker
        .as_deref()
        .ok_or_else(|| CliError::Validation("--speaker is required for leave".into()))?;
    let spk = system
        .speaker(speaker_name)
        .ok_or_else(|| CliError::SpeakerNotFound(speaker_name.into()))?;
    let group_name = spk
        .group()
        .and_then(|g| g.coordinator().map(|c| c.name))
        .unwrap_or_else(|| "its group".into());
    spk.leave_group()?;
    Ok(format!("{} left {}", speaker_name, group_name))
}

fn cmd_bass(system: &SonosSystem, global: &GlobalFlags, level: i8) -> Result<String, CliError> {
    let spk = require_speaker_only(system, global, "bass")?;
    spk.set_bass(level)?;
    Ok(format!("Bass set to {} ({})", level, spk.name))
}

fn cmd_treble(system: &SonosSystem, global: &GlobalFlags, level: i8) -> Result<String, CliError> {
    let spk = require_speaker_only(system, global, "treble")?;
    spk.set_treble(level)?;
    Ok(format!("Treble set to {} ({})", level, spk.name))
}

fn cmd_loudness(
    system: &SonosSystem,
    global: &GlobalFlags,
    state: OnOff,
) -> Result<String, CliError> {
    let spk = require_speaker_only(system, global, "loudness")?;
    let enabled = matches!(state, OnOff::On);
    spk.set_loudness(enabled)?;
    if enabled {
        Ok(format!("Loudness enabled ({})", spk.name))
    } else {
        Ok(format!("Loudness disabled ({})", spk.name))
    }
}

fn cmd_sleep(
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
    duration: &str,
) -> Result<String, CliError> {
    let spk = resolve_speaker(system, config, global)?;
    if duration == "cancel" {
        spk.cancel_sleep_timer()?;
        Ok(format!("Sleep timer cancelled ({})", spk.name))
    } else {
        let hh_mm_ss = parse_duration(duration)?;
        let human = format_duration_human(duration);
        spk.configure_sleep_timer(&hh_mm_ss)?;
        Ok(format!("Sleep timer set for {} ({})", human, spk.name))
    }
}

fn cmd_queue(
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
    action: Option<QueueAction>,
) -> Result<String, CliError> {
    let spk = resolve_speaker(system, config, global)?;
    match action {
        None => {
            let info = spk.get_media_info()?;
            if info.nr_tracks == 0 {
                return Ok(format!("queue is empty ({})", spk.name));
            }
            Ok(format!("{} — {} tracks", spk.name, info.nr_tracks))
        }
        Some(QueueAction::Add { uri }) => {
            spk.add_uri_to_queue(&uri, "", 0, false)?;
            Ok(format!("Added to queue ({})", spk.name))
        }
        Some(QueueAction::Clear) => {
            if std::io::stdin().is_terminal() && !global.no_input {
                eprint!("Clear queue for {}? [y/N] ", spk.name);
                let mut input = String::new();
                std::io::stdin()
                    .read_line(&mut input)
                    .map_err(|e| CliError::Validation(e.to_string()))?;
                if !input.trim().eq_ignore_ascii_case("y") {
                    return Ok("Cancelled".into());
                }
            }
            spk.remove_all_tracks_from_queue()?;
            Ok(format!("Queue cleared ({})", spk.name))
        }
    }
}
