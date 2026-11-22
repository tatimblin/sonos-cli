//! Settings screen key handling — row navigation, dropdown open/close, config persistence.

use crossterm::event::KeyEvent;

use crate::config::AlbumArtMode;
use crate::tui::app::{App, Toast};
use crate::tui::keymap::{lookup, DropdownCommand, SettingsCommand, DROPDOWN_KEYS, SETTINGS_KEYS};
use crate::tui::theme::Theme;
use crate::tui::types::SettingsAction;

/// Available theme names.
pub(crate) const THEME_OPTIONS: &[&str] = &["default", "bw", "minimal", "dance_party"];

/// Available album art mode options.
pub(crate) const ALBUM_ART_OPTIONS: &[&str] = &["image", "halfblock", "off"];

/// Display label for "no default group" (auto-detect).
pub(crate) const AUTO_GROUP_LABEL: &str = "(auto)";

/// Total number of settings rows.
const ROW_COUNT: usize = 3;

/// Handle a key event for the settings form. Returns an action for the caller.
pub fn handle_key(app: &mut App, key: KeyEvent) -> SettingsAction {
    if app.navigation.settings_state.dropdown_open {
        handle_dropdown_key(app, key)
    } else {
        handle_row_key(app, key)
    }
}

// ---------------------------------------------------------------------------
// Row navigation (dropdown closed)
// ---------------------------------------------------------------------------

fn handle_row_key(app: &mut App, key: KeyEvent) -> SettingsAction {
    let Some(command) = lookup(SETTINGS_KEYS, key.code) else {
        return SettingsAction::Handled;
    };
    let row = app.navigation.settings_state.selected_row;

    match command {
        SettingsCommand::Up => {
            if row == 0 {
                return SettingsAction::FocusTabBar;
            }
            app.navigation.settings_state.selected_row = row.saturating_sub(1);
            SettingsAction::Handled
        }
        SettingsCommand::Down => {
            if row + 1 < ROW_COUNT {
                app.navigation.settings_state.selected_row = row + 1;
            }
            SettingsAction::Handled
        }
        SettingsCommand::Open => {
            open_dropdown(app);
            SettingsAction::Handled
        }
    }
}

// ---------------------------------------------------------------------------
// Dropdown navigation (dropdown open)
// ---------------------------------------------------------------------------

fn handle_dropdown_key(app: &mut App, key: KeyEvent) -> SettingsAction {
    let Some(command) = lookup(DROPDOWN_KEYS, key.code) else {
        return SettingsAction::Handled;
    };
    let options = options_for_row(app, app.navigation.settings_state.selected_row);
    let count = options.len();
    let idx = app.navigation.settings_state.dropdown_index;

    match command {
        DropdownCommand::Up => {
            app.navigation.settings_state.dropdown_index = idx.saturating_sub(1);
            SettingsAction::Handled
        }
        DropdownCommand::Down => {
            if idx + 1 < count {
                app.navigation.settings_state.dropdown_index = idx + 1;
            }
            SettingsAction::Handled
        }
        DropdownCommand::Confirm => {
            let selected = options
                .get(app.navigation.settings_state.dropdown_index)
                .cloned()
                .unwrap_or_default();
            apply_setting(app, app.navigation.settings_state.selected_row, &selected);
            app.navigation.settings_state.dropdown_open = false;
            SettingsAction::Handled
        }
        DropdownCommand::Cancel => {
            app.navigation.settings_state.dropdown_open = false;
            SettingsAction::Handled
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn open_dropdown(app: &mut App) {
    let row = app.navigation.settings_state.selected_row;
    let options = options_for_row(app, row);
    let current = current_value_for_row(app, row);

    // Pre-select the current value in the dropdown
    let idx = options.iter().position(|o| *o == current).unwrap_or(0);

    app.navigation.settings_state.dropdown_open = true;
    app.navigation.settings_state.dropdown_index = idx;
}

/// Return option strings for a given row index.
fn options_for_row(app: &App, row: usize) -> Vec<String> {
    match row {
        0 => THEME_OPTIONS.iter().map(|s| s.to_string()).collect(),
        1 => {
            let mut opts = vec![AUTO_GROUP_LABEL.to_string()];
            for group in app.system.groups() {
                if let Some(coordinator) = group.coordinator() {
                    opts.push(coordinator.name.clone());
                }
            }
            opts
        }
        2 => ALBUM_ART_OPTIONS.iter().map(|s| s.to_string()).collect(),
        _ => vec![],
    }
}

/// Return the current display value for a row.
fn current_value_for_row(app: &App, row: usize) -> String {
    match row {
        0 => app.config.theme.clone(),
        1 => app
            .config
            .default_group
            .clone()
            .unwrap_or_else(|| AUTO_GROUP_LABEL.to_string()),
        2 => app.config.album_art_mode.to_string(),
        _ => String::new(),
    }
}

/// Apply a setting change and persist to disk.
fn apply_setting(app: &mut App, row: usize, value: &str) {
    match row {
        0 => {
            app.config.theme = value.to_string();
            app.theme = Theme::from_name(value);
        }
        1 => {
            if value == AUTO_GROUP_LABEL {
                app.config.default_group = None;
            } else {
                app.config.default_group = Some(value.to_string());
            }
        }
        2 => {
            app.config.album_art_mode = match value {
                "halfblock" => AlbumArtMode::Halfblock,
                "off" => AlbumArtMode::Off,
                _ => AlbumArtMode::Image,
            };
            app.toast = Some(Toast::info(
                "Album art change takes effect on restart".to_string(),
            ));
        }
        _ => {}
    }

    if let Err(e) = app.config.save() {
        app.toast = Some(Toast::error(format!("failed to save config: {e}")));
    }
}
