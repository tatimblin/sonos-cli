//! Interactive TUI for controlling Sonos speakers.
//!
//! Launched when `sonos` is run without arguments in a terminal.

mod app;
mod event;
mod handlers;
pub mod hooks;
pub mod image_loader;
mod screens;
mod theme;
mod ui;
mod widgets;

pub use app::App;

use crate::config::Config;
use anyhow::Result;
use ratatui_image::picker::Picker;

/// Launch the interactive TUI. Blocks until the user quits.
pub fn run(config: Config) -> Result<()> {
    // Detect terminal image protocol BEFORE entering raw mode.
    // from_query_stdio() sends escape sequences to discover font size and
    // graphics protocol support — must happen before ratatui::init().
    let picker = if config.album_art_mode.is_off() {
        None
    } else {
        match Picker::from_query_stdio() {
            Ok(p) => Some(p),
            Err(e) => {
                tracing::debug!("Terminal image protocol detection failed: {e}");
                None
            }
        }
    };

    let theme = theme::Theme::from_name(&config.theme);
    let app = App::new(config, theme, picker)?;
    event::run_event_loop(app)
}
