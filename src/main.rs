//! sonos-cli: CLI/TUI application for controlling Sonos speakers.
//!
//! When run without arguments and stdout is a terminal, launches the TUI.
//! When given a subcommand, executes the command and exits.

use clap::Parser;
use std::io::IsTerminal;
use std::process::ExitCode;

use sonos_sdk::SonosSystem;

mod cli;
mod config;
mod errors;

use cli::{run_command, Cli};
use config::Config;

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
                        eprintln!("debug: {e:?}");
                    }
                    eprintln!("error: {e}");
                    eprintln!("Check that your speakers are on the same network, then retry.");
                    return ExitCode::from(1);
                }
            };

            match run_command(cmd, &system, &config, &cli.global) {
                Ok(msg) => {
                    if !cli.global.quiet {
                        println!("{msg}");
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    if cli.global.verbose {
                        eprintln!("debug: {e:?}");
                    }
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
