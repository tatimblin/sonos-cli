//! sonos-cli: CLI/TUI application for controlling Sonos speakers.
//!
//! When run without arguments and stdout is a terminal, launches the TUI.
//! When given a subcommand, executes the command and exits.

use clap::Parser;
use std::io::IsTerminal;
use std::process::ExitCode;

use sonos_sdk::{SdkError, SonosSystem};

mod cli;
mod config;
mod diagnostics;
mod errors;
mod logging;
mod tui;

use cli::{run_command, Cli, Commands, ConfigAction};
use config::Config;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let mut config = Config::load();

    let is_tui = cli.command.is_none() && std::io::stdout().is_terminal();
    logging::init_logging(cli.global.verbose, is_tui);

    // Config commands don't need speaker discovery — dispatch early.
    if let Some(Commands::Config { action }) = &cli.command {
        return handle_config_command(action.as_ref(), &mut config, &cli.global);
    }

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
                Ok(msg) => {
                    if !cli.global.quiet {
                        println!("{msg}");
                    }
                    ExitCode::SUCCESS
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

fn handle_config_command(
    action: Option<&ConfigAction>,
    config: &mut Config,
    global: &cli::GlobalFlags,
) -> ExitCode {
    let result = match action {
        None => {
            // Bare `sonos config` — show help hint
            eprintln!("Usage: sonos config <COMMAND>\n\nCommands:\n  alias  Manage speaker/group aliases\n\nFor more information, try 'sonos config --help'");
            return ExitCode::from(2);
        }
        Some(ConfigAction::Alias { name, alias }) => match (name, alias) {
            (None, _) => cmd_config_alias_list(config),
            (Some(name), None) => cmd_config_alias_clear(config, name),
            (Some(name), Some(alias)) => cmd_config_alias_set(config, name, alias),
        },
    };

    match result {
        Ok(msg) => {
            if !global.quiet && !msg.is_empty() {
                println!("{msg}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            e.exit_code()
        }
    }
}

fn cmd_config_alias_list(config: &Config) -> Result<String, errors::CliError> {
    if config.aliases.is_empty() {
        return Ok("No aliases configured".to_string());
    }
    let mut lines: Vec<String> = config
        .aliases
        .iter()
        .map(|(name, alias)| format!("{alias} \u{2192} {name}"))
        .collect();
    lines.sort();
    Ok(lines.join("\n"))
}

fn cmd_config_alias_set(
    config: &mut Config,
    name: &str,
    alias: &str,
) -> Result<String, errors::CliError> {
    if alias.is_empty() {
        return Err(errors::CliError::Validation(
            "alias must not be empty".to_string(),
        ));
    }
    if alias.contains(char::is_whitespace) {
        return Err(errors::CliError::Validation(
            "alias must not contain whitespace".to_string(),
        ));
    }
    // Check uniqueness: alias string must not already be used by another name
    for (existing_name, existing_alias) in &config.aliases {
        if existing_alias == alias && existing_name != name {
            return Err(errors::CliError::Validation(format!(
                "alias \"{alias}\" is already used by \"{existing_name}\""
            )));
        }
    }
    config.set_alias(name, alias);
    config.save().map_err(|e| errors::CliError::Config(e.to_string()))?;
    Ok(format!("Alias set: {alias} \u{2192} {name}"))
}

fn cmd_config_alias_clear(
    config: &mut Config,
    name: &str,
) -> Result<String, errors::CliError> {
    match config.clear_alias(name) {
        Some(old_alias) => {
            config.save().map_err(|e| errors::CliError::Config(e.to_string()))?;
            Ok(format!("Alias cleared: {old_alias} \u{2192} {name}"))
        }
        None => Err(errors::CliError::Validation(format!(
            "no alias configured for \"{name}\""
        ))),
    }
}
