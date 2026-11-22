//! sonos-cli: CLI/TUI application for controlling Sonos speakers.
//!
//! When run without arguments and stdout is a terminal, launches the TUI.
//! When given a subcommand, executes the command and exits.

use clap::{CommandFactory, Parser};
use std::io::IsTerminal;
use std::process::ExitCode;

use sonos_sdk::{SdkError, SonosSystem};

use sonos_cli::cli::{run_command, run_config_command, Cli, Commands};
use sonos_cli::config::Config;
use sonos_cli::{diagnostics, liveness, logging, tui};

fn main() -> ExitCode {
    clap_complete::env::CompleteEnv::with_factory(Cli::command).complete();

    let cli = Cli::parse();
    let mut config = Config::load();

    let is_tui = cli.command.is_none() && std::io::stdout().is_terminal();
    logging::init_logging(cli.global.verbose, is_tui);

    match cli.command {
        None => {
            if std::io::stdout().is_terminal() {
                match tui::run(config) {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(e) => {
                        eprintln!("error: {e}");
                        if format!("{e:?}").contains("DiscoveryFailed") {
                            eprintln!("{}", diagnostics::discovery_hint());
                        }
                        ExitCode::from(1)
                    }
                }
            } else {
                eprintln!("error: no command specified and stdout is not a terminal");
                ExitCode::from(1)
            }
        }
        // Config commands don't need speaker discovery.
        Some(Commands::Config { action }) => {
            run_config_command(action.as_ref(), &mut config, &cli.global)
        }
        Some(cmd) => {
            let system = match SonosSystem::new() {
                Ok(s) => s,
                Err(e) => {
                    tracing::debug!("{e:?}");
                    eprintln!("error: {e}");
                    if matches!(&e, SdkError::DiscoveryFailed(_)) {
                        eprintln!("{}", diagnostics::discovery_hint());
                        diagnostics::offer_open_settings(&cli.global);
                    } else {
                        eprintln!("Check that your speakers are on the same network, then retry.");
                    }
                    return ExitCode::from(1);
                }
            };

            match run_command(cmd, &system, &config, &cli.global) {
                Ok(out) => {
                    // A speaker list assembled entirely from the discovery
                    // cache used to be indistinguishable from a verified one.
                    // The verdict decides that here, not the handler.
                    let disposition = liveness::disposition(
                        out.liveness,
                        cli.global.offline,
                        cli.global.require_live,
                    );
                    if disposition.prints_stdout() && !cli.global.quiet {
                        println!("{}", out.stdout);
                    }
                    if disposition.prints_banner() {
                        eprintln!("{}", diagnostics::unvalidated_output_banner());
                    }
                    if disposition.prints_note() {
                        eprintln!("{}", diagnostics::unvalidated_output_note());
                    }
                    ExitCode::from(disposition.exit_code())
                }
                Err(e) => {
                    tracing::debug!("{e:?}");
                    eprintln!("error: {e}");
                    if let Some(hint) = e.recovery_hint() {
                        eprintln!("{hint}");
                    }
                    e.exit_code()
                }
            }
        }
    }
}
