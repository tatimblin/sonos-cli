//! sonos-cli: CLI/TUI application for controlling Sonos speakers.
//!
//! When run without arguments and stdout is a terminal, launches the TUI.
//! When given a subcommand, executes the command and exits.

use clap::Parser;
use std::io::IsTerminal;
use std::process::ExitCode;

use sonos_sdk::{SeekTarget, SonosSystem};

mod cli;
mod config;
mod errors;

use cli::{
    format_duration_human, format_time_ms, parse_duration, parse_play_mode, playback_icon,
    playback_label, require_speaker_only, resolve_speaker, validate_seek_time, Cli, Commands,
    GlobalFlags, QueueAction,
};
use config::Config;
use errors::CliError;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let config = Config::load();

    match cli.command {
        None => {
            if std::io::stdout().is_terminal() {
                eprintln!("TUI not yet implemented");
                ExitCode::from(1)
            } else {
                eprintln!("error: no command specified and stdout is not a terminal");
                ExitCode::from(1)
            }
        }
        Some(cmd) => {
            let system = match SonosSystem::new() {
                Ok(s) => s,
                Err(e) => {
                    if cli.global.verbose {
                        eprintln!("debug: {:?}", e);
                    }
                    eprintln!("error: {}", e);
                    eprintln!("Check that your speakers are on the same network, then retry.");
                    return ExitCode::from(1);
                }
            };

            match run_command(cmd, &system, &config, &cli.global) {
                Ok(msg) => {
                    if !cli.global.quiet {
                        println!("{}", msg);
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    if cli.global.verbose {
                        eprintln!("debug: {:?}", e);
                    }
                    eprintln!("error: {}", e);
                    if let Some(hint) = e.recovery_hint() {
                        eprintln!("{}", hint);
                    }
                    e.exit_code()
                }
            }
        }
    }
}

fn run_command(
    cmd: Commands,
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
) -> Result<String, CliError> {
    match cmd {
        // -- Discovery & System ------------------------------------------------

        Commands::Speakers => {
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
                    let vol_str = vol
                        .map(|v| format!("vol:{}", v.0))
                        .unwrap_or_default();

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

        Commands::Groups => {
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
                    let track_str = track
                        .as_ref()
                        .map(|t| t.display())
                        .unwrap_or_default();
                    let vol_str = vol
                        .map(|v| format!("vol:{}", v.0))
                        .unwrap_or_default();

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

        Commands::Status => {
            let spk = resolve_speaker(system, config, global)?;
            let state = spk.playback_state.fetch().ok();
            let track = spk.current_track.fetch().ok();
            let pos = spk.position.fetch().ok();
            let vol = spk.volume.fetch().ok();

            let state_str = state
                .as_ref()
                .map(|st| format!("{} {}", playback_icon(st), playback_label(st)))
                .unwrap_or_else(|| "unknown".to_string());
            let track_str = track
                .as_ref()
                .map(|t| t.display())
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
            let vol_str = vol
                .map(|v| format!("vol:{}", v.0))
                .unwrap_or_default();

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

        // -- Playback ----------------------------------------------------------

        Commands::Play => {
            let spk = resolve_speaker(system, config, global)?;
            spk.play()?;
            Ok(format!("Playing ({})", spk.name))
        }

        Commands::Pause => {
            let spk = resolve_speaker(system, config, global)?;
            spk.pause()?;
            Ok(format!("Paused ({})", spk.name))
        }

        Commands::Stop => {
            let spk = resolve_speaker(system, config, global)?;
            spk.stop()?;
            Ok(format!("Stopped ({})", spk.name))
        }

        Commands::Next => {
            let spk = resolve_speaker(system, config, global)?;
            spk.next()?;
            Ok(format!("Next track ({})", spk.name))
        }

        Commands::Previous => {
            let spk = resolve_speaker(system, config, global)?;
            spk.previous()?;
            Ok(format!("Previous track ({})", spk.name))
        }

        Commands::Seek { position } => {
            validate_seek_time(&position)?;
            let spk = resolve_speaker(system, config, global)?;
            spk.seek(SeekTarget::Time(position.clone()))?;
            Ok(format!("Seeked to {} ({})", position, spk.name))
        }

        Commands::Mode { mode } => {
            let play_mode = parse_play_mode(&mode)?;
            let spk = resolve_speaker(system, config, global)?;
            spk.set_play_mode(play_mode)?;
            Ok(format!("Mode set to {} ({})", mode, spk.name))
        }

        // -- Volume & EQ -------------------------------------------------------

        Commands::Volume { level } => {
            let spk = resolve_speaker(system, config, global)?;
            spk.set_volume(level)?;
            Ok(format!("Volume set to {} ({})", level, spk.name))
        }

        Commands::Mute => {
            let spk = resolve_speaker(system, config, global)?;
            spk.set_mute(true)?;
            Ok(format!("Muted ({})", spk.name))
        }

        Commands::Unmute => {
            let spk = resolve_speaker(system, config, global)?;
            spk.set_mute(false)?;
            Ok(format!("Unmuted ({})", spk.name))
        }

        Commands::Bass { level } => {
            let spk = require_speaker_only(system, global, "bass")?;
            spk.set_bass(level)?;
            Ok(format!("Bass set to {} ({})", level, spk.name))
        }

        Commands::Treble { level } => {
            let spk = require_speaker_only(system, global, "treble")?;
            spk.set_treble(level)?;
            Ok(format!("Treble set to {} ({})", level, spk.name))
        }

        Commands::Loudness { state } => {
            let enabled = match state.as_str() {
                "on" => true,
                "off" => false,
                _ => {
                    return Err(CliError::Validation(format!(
                        "invalid value \"{}\" — use on or off",
                        state
                    )));
                }
            };
            let spk = require_speaker_only(system, global, "loudness")?;
            spk.set_loudness(enabled)?;
            if enabled {
                Ok(format!("Loudness enabled ({})", spk.name))
            } else {
                Ok(format!("Loudness disabled ({})", spk.name))
            }
        }

        // -- Grouping ----------------------------------------------------------

        Commands::Join => {
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

        Commands::Leave => {
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

        // -- Sleep Timer -------------------------------------------------------

        Commands::Sleep { duration } => {
            let spk = resolve_speaker(system, config, global)?;
            if duration == "cancel" {
                spk.cancel_sleep_timer()?;
                Ok(format!("Sleep timer cancelled ({})", spk.name))
            } else {
                let hh_mm_ss = parse_duration(&duration)?;
                let human = format_duration_human(&duration);
                spk.configure_sleep_timer(&hh_mm_ss)?;
                Ok(format!("Sleep timer set for {} ({})", human, spk.name))
            }
        }

        // -- Queue -------------------------------------------------------------

        Commands::Queue { action } => {
            let spk = resolve_speaker(system, config, global)?;
            match action {
                None => {
                    let info = spk.get_media_info()?;
                    if info.nr_tracks == 0 {
                        return Ok(format!("queue is empty ({})", spk.name));
                    }
                    Ok(format!(
                        "{} — {} tracks",
                        spk.name, info.nr_tracks
                    ))
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
    }
}
