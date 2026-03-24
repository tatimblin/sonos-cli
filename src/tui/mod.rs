//! Interactive TUI for controlling Sonos speakers.
//!
//! Launched when `sonos` is run without arguments in a terminal.

mod app;
mod event;
mod screens;
mod theme;
mod ui;
mod widgets;

pub use app::App;

use crate::config::Config;
use anyhow::Result;

/// Launch the interactive TUI. Blocks until the user quits.
pub fn run(config: Config) -> Result<()> {
    let theme = theme::Theme::from_name(&config.theme);
    let app = App::new(config, theme)?;
    event::run_event_loop(app)
}
