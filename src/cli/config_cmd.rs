//! `sonos config` — local configuration commands.
//!
//! These never touch a speaker, so `main` runs them before discovery. The
//! alias handlers only mutate the in-memory [`Config`]; persisting it is left
//! to [`run_config_action`] so the handlers are testable without writing the
//! user's real config file.

use std::process::ExitCode;

use clap::CommandFactory;

use super::{Cli, ConfigAction, GlobalFlags};
use crate::config::Config;
use crate::errors::CliError;

/// Run `sonos config [action]`, print its output, and return the exit code.
///
/// With no action, prints the `config` subcommand help and exits 2 (usage).
pub fn run_config_command(
    action: Option<&ConfigAction>,
    config: &mut Config,
    global: &GlobalFlags,
) -> ExitCode {
    let Some(action) = action else {
        let mut cmd = Cli::command();
        let config_cmd = cmd.find_subcommand_mut("config").unwrap();
        config_cmd.print_help().ok();
        return ExitCode::from(2);
    };

    match run_config_action(action, config) {
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

/// Apply a config action and save the config if it changed.
pub fn run_config_action(action: &ConfigAction, config: &mut Config) -> Result<String, CliError> {
    match action {
        ConfigAction::Alias { name, alias } => match (name, alias) {
            (None, _) => Ok(alias_list(config)),
            (Some(name), None) => {
                let msg = alias_clear(config, name)?;
                save(config)?;
                Ok(msg)
            }
            (Some(name), Some(alias)) => {
                let msg = alias_set(config, name, alias)?;
                save(config)?;
                Ok(msg)
            }
        },
    }
}

fn save(config: &Config) -> Result<(), CliError> {
    config.save().map_err(|e| CliError::Config(e.to_string()))
}

fn alias_list(config: &Config) -> String {
    if config.aliases.is_empty() {
        return "No aliases configured".to_string();
    }
    let mut lines: Vec<String> = config
        .aliases
        .iter()
        .map(|(name, alias)| format!("{alias} \u{2192} {name}"))
        .collect();
    lines.sort();
    lines.join("\n")
}

fn alias_set(config: &mut Config, name: &str, alias: &str) -> Result<String, CliError> {
    if alias.is_empty() {
        return Err(CliError::Validation("alias must not be empty".to_string()));
    }
    if alias.contains(char::is_whitespace) {
        return Err(CliError::Validation(
            "alias must not contain whitespace".to_string(),
        ));
    }
    // Check uniqueness: alias string must not already be used by another name
    for (existing_name, existing_alias) in &config.aliases {
        if existing_alias == alias && existing_name != name {
            return Err(CliError::Validation(format!(
                "alias \"{alias}\" is already used by \"{existing_name}\""
            )));
        }
    }
    config.set_alias(name, alias);
    Ok(format!("Alias set: {alias} \u{2192} {name}"))
}

fn alias_clear(config: &mut Config, name: &str) -> Result<String, CliError> {
    match config.clear_alias(name) {
        Some(old_alias) => Ok(format!("Alias cleared: {old_alias} \u{2192} {name}")),
        None => Err(CliError::Validation(format!(
            "no alias configured for \"{name}\""
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigError;

    #[test]
    fn alias_list_empty() {
        assert_eq!(alias_list(&Config::default()), "No aliases configured");
    }

    #[test]
    fn alias_list_is_sorted_by_alias() {
        let mut config = Config::default();
        config.set_alias("Master Bedroom", "bed");
        config.set_alias("Kitchen", "kit");
        config.set_alias("Living Room", "den");
        assert_eq!(
            alias_list(&config),
            "bed \u{2192} Master Bedroom\nden \u{2192} Living Room\nkit \u{2192} Kitchen"
        );
    }

    #[test]
    fn alias_set_assigns_and_reports() {
        let mut config = Config::default();
        let msg = alias_set(&mut config, "Kitchen", "kit").unwrap();
        assert_eq!(msg, "Alias set: kit \u{2192} Kitchen");
        assert_eq!(config.resolve_alias("kit"), "Kitchen");
    }

    #[test]
    fn alias_set_rejects_empty_and_whitespace() {
        let mut config = Config::default();
        assert!(matches!(
            alias_set(&mut config, "Kitchen", ""),
            Err(CliError::Validation(_))
        ));
        assert!(matches!(
            alias_set(&mut config, "Kitchen", "k it"),
            Err(CliError::Validation(_))
        ));
        assert!(config.aliases.is_empty());
    }

    #[test]
    fn alias_set_rejects_alias_owned_by_another_name() {
        let mut config = Config::default();
        config.set_alias("Kitchen", "kit");
        let err = alias_set(&mut config, "Bedroom", "kit").unwrap_err();
        assert!(err.to_string().contains("already used by \"Kitchen\""));
        assert_eq!(config.resolve_alias("kit"), "Kitchen");
    }

    #[test]
    fn alias_set_allows_reassigning_same_name() {
        let mut config = Config::default();
        config.set_alias("Kitchen", "kit");
        assert!(alias_set(&mut config, "Kitchen", "kit").is_ok());
    }

    #[test]
    fn alias_clear_reports_old_alias() {
        let mut config = Config::default();
        config.set_alias("Kitchen", "kit");
        let msg = alias_clear(&mut config, "Kitchen").unwrap();
        assert_eq!(msg, "Alias cleared: kit \u{2192} Kitchen");
        assert!(config.aliases.is_empty());
    }

    #[test]
    fn alias_clear_missing_is_validation_error() {
        let mut config = Config::default();
        let err = alias_clear(&mut config, "Kitchen").unwrap_err();
        assert!(matches!(err, CliError::Validation(_)));
    }

    /// A config standing in for an unparseable file must make alias writes
    /// fail (exit 1) rather than overwrite it. `save()` refuses before it
    /// touches the filesystem, so this never reaches the real config file.
    #[test]
    fn run_config_action_refuses_to_save_over_broken_config() {
        let mut config = Config::fallback(ConfigError::Parse {
            path: "/nonexistent/sonos/config.toml".into(),
            message: "expected `=`".to_string(),
        });
        let action = ConfigAction::Alias {
            name: Some("Kitchen".to_string()),
            alias: Some("kit".to_string()),
        };
        let err = run_config_action(&action, &mut config).unwrap_err();
        assert!(matches!(err, CliError::Config(_)));
        assert!(err.to_string().contains("refusing to overwrite"));
        assert_eq!(err.exit_code(), std::process::ExitCode::from(1));
    }

    /// Listing reads only, so it is safe to drive through the dispatcher —
    /// it must never reach `save()` and the user's real config file.
    #[test]
    fn run_config_action_lists_without_saving() {
        let mut config = Config::default();
        config.set_alias("Kitchen", "kit");
        let action = ConfigAction::Alias {
            name: None,
            alias: None,
        };
        assert_eq!(
            run_config_action(&action, &mut config).unwrap(),
            "kit \u{2192} Kitchen"
        );
    }
}
