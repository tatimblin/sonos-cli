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
#[derive(Debug, Args)]
pub struct GlobalFlags {
    /// Target a specific speaker by friendly name
    #[arg(long, global = true)]
    pub speaker: Option<String>,
    /// Target a group by name
    #[arg(long, global = true)]
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
}
