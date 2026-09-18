//! CLI command parsing, helpers, and command execution.

mod commands;
mod format;
mod parse;
mod resolve;
mod run;

pub use commands::*;
pub use format::*;
pub use parse::*;
pub use resolve::*;
pub use run::run_command;

use clap::{Args, Parser};

/// Top-level CLI parser.
#[derive(Debug, Parser)]
#[command(name = "sonos", about = "Control Sonos speakers", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
    #[command(flatten)]
    pub global: GlobalFlags,
}

/// Global flags accepted by every command.
///
/// Implements `Default` so tests can name only the flags they care about; the
/// clap-facing defaults and the `Default` impl are the same values.
#[derive(Debug, Default, Args)]
pub struct GlobalFlags {
    /// Target a specific speaker by friendly name
    #[arg(long, short = 's', global = true)]
    pub speaker: Option<String>,
    /// Target a group by name
    #[arg(long, short = 'g', global = true)]
    pub group: Option<String>,
    /// Suppress all non-error stdout output
    #[arg(long, short, global = true)]
    pub quiet: bool,
    /// Increase log verbosity (-v info, -vv debug, -vvv trace)
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,
    /// Disable all interactive prompts
    #[arg(long, global = true)]
    pub no_input: bool,
    /// Accept cached data when no speaker responds (exit 0, no warning banner)
    #[arg(long, global = true, conflicts_with = "require_live")]
    pub offline: bool,
    /// Fail with no output unless a speaker was actually reached
    #[arg(long, global = true)]
    pub require_live: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn parse(args: &[&str]) -> Cli {
        Cli::try_parse_from(args).expect("should parse")
    }

    #[test]
    fn liveness_flags_default_to_off() {
        let cli = parse(&["sonos", "speakers"]);
        assert!(!cli.global.offline);
        assert!(!cli.global.require_live);
    }

    #[test]
    fn liveness_flags_are_global() {
        // `global = true` means they may follow the subcommand, which is where
        // people actually type them.
        assert!(parse(&["sonos", "speakers", "--offline"]).global.offline);
        assert!(parse(&["sonos", "--offline", "speakers"]).global.offline);
        assert!(
            parse(&["sonos", "status", "--require-live"])
                .global
                .require_live
        );
    }

    /// Asking to accept unverified data and to refuse it is not a coherent
    /// request; clap rejects it as a usage error (exit 2) rather than letting
    /// one silently win.
    #[test]
    fn offline_and_require_live_conflict() {
        let err = Cli::try_parse_from(["sonos", "speakers", "--offline", "--require-live"])
            .expect_err("conflicting flags must be rejected");
        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn global_flags_default_matches_clap_default() {
        let parsed = parse(&["sonos", "speakers"]).global;
        let defaulted = GlobalFlags::default();
        assert_eq!(parsed.speaker, defaulted.speaker);
        assert_eq!(parsed.group, defaulted.group);
        assert_eq!(parsed.quiet, defaulted.quiet);
        assert_eq!(parsed.verbose, defaulted.verbose);
        assert_eq!(parsed.no_input, defaulted.no_input);
        assert_eq!(parsed.offline, defaulted.offline);
        assert_eq!(parsed.require_live, defaulted.require_live);
    }
}
