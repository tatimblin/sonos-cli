//! sonos-cli: CLI/TUI application for controlling Sonos speakers.
//!
//! When run without arguments and stdout is a terminal, launches the TUI.
//! When given a subcommand, executes the command and exits.

use clap::Parser;
use std::io::IsTerminal;
use std::process::ExitCode;

mod actions;
mod cli;
mod config;
mod errors;
mod executor;

/// Control Sonos speakers from the command line.
#[derive(Parser)]
#[command(name = "sonos", about = "Control Sonos speakers")]
struct Cli {
    #[command(subcommand)]
    command: Option<cli::Commands>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let config = config::Config::load();

    match cli.command {
        None => {
            if std::io::stdout().is_terminal() {
                // Launch TUI (future milestone)
                eprintln!("TUI not yet implemented");
                ExitCode::from(1)
            } else {
                eprintln!("error: no command specified and stdout is not a terminal");
                ExitCode::from(1)
            }
        }
        Some(cmd) => {
            let action = cmd.into_action();

            let system = match sonos_sdk::SonosSystem::new() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: {}", e);
                    eprintln!("Check that your speakers are on the same network, then retry.");
                    return ExitCode::from(1);
                }
            };

            match executor::execute(action, &system, &config) {
                Ok(msg) => {
                    println!("{}", msg);
                    ExitCode::SUCCESS
                }
                Err(e) => {
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
