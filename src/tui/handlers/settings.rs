//! Settings screen key handling — row navigation, dropdown open/close, config persistence.

use crossterm::event::KeyEvent;

use crate::config::AlbumArtMode;
use crate::tui::app::{App, Toast};
use crate::tui::keymap::{lookup, DropdownCommand, SettingsCommand, DROPDOWN_KEYS, SETTINGS_KEYS};
use crate::tui::theme::Theme;
use crate::tui::types::{KeyOutcome, SettingsRow, AUTO_GROUP_LABEL};

/// Handle a key event for the settings form. Returns an outcome for the caller.
pub fn handle_key(app: &mut App, key: KeyEvent) -> KeyOutcome {
    if app.navigation.settings_state.dropdown_open {
        handle_dropdown_key(app, key)
    } else {
        handle_row_key(app, key)
    }
}

// ---------------------------------------------------------------------------
// Row navigation (dropdown closed)
// ---------------------------------------------------------------------------

fn handle_row_key(app: &mut App, key: KeyEvent) -> KeyOutcome {
    let Some(command) = lookup(SETTINGS_KEYS, key.code) else {
        return KeyOutcome::Handled;
    };
    let row = app.navigation.settings_state.selected_row;

    match command {
        SettingsCommand::Up => {
            if row == 0 {
                return KeyOutcome::FocusTabBar;
            }
            app.navigation.settings_state.selected_row = row.saturating_sub(1);
            KeyOutcome::Handled
        }
        SettingsCommand::Down => {
            if row + 1 < SettingsRow::ALL.len() {
                app.navigation.settings_state.selected_row = row + 1;
            }
            KeyOutcome::Handled
        }
        SettingsCommand::Open => {
            open_dropdown(app);
            KeyOutcome::Handled
        }
    }
}

// ---------------------------------------------------------------------------
// Dropdown navigation (dropdown open)
// ---------------------------------------------------------------------------

fn handle_dropdown_key(app: &mut App, key: KeyEvent) -> KeyOutcome {
    let Some(command) = lookup(DROPDOWN_KEYS, key.code) else {
        return KeyOutcome::Handled;
    };
    let Some(row) = SettingsRow::at(app.navigation.settings_state.selected_row) else {
        return KeyOutcome::Handled;
    };
    let options = row.options(&app.system);
    let count = options.len();
    let idx = app.navigation.settings_state.dropdown_index;

    match command {
        DropdownCommand::Up => {
            app.navigation.settings_state.dropdown_index = idx.saturating_sub(1);
            KeyOutcome::Handled
        }
        DropdownCommand::Down => {
            if idx + 1 < count {
                app.navigation.settings_state.dropdown_index = idx + 1;
            }
            KeyOutcome::Handled
        }
        DropdownCommand::Confirm => {
            let selected = options
                .get(app.navigation.settings_state.dropdown_index)
                .cloned()
                .unwrap_or_default();
            apply_setting(app, row, &selected);
            app.navigation.settings_state.dropdown_open = false;
            KeyOutcome::Handled
        }
        DropdownCommand::Cancel => {
            app.navigation.settings_state.dropdown_open = false;
            KeyOutcome::Handled
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn open_dropdown(app: &mut App) {
    let Some(row) = SettingsRow::at(app.navigation.settings_state.selected_row) else {
        return;
    };
    let options = row.options(&app.system);
    let current = row.current_value(&app.config);

    // Pre-select the current value in the dropdown
    let idx = options.iter().position(|o| *o == current).unwrap_or(0);

    app.navigation.settings_state.dropdown_open = true;
    app.navigation.settings_state.dropdown_index = idx;
}

/// Apply a setting change and persist to disk.
fn apply_setting(app: &mut App, row: SettingsRow, value: &str) {
    match row {
        SettingsRow::Theme => {
            app.config.theme = value.to_string();
            app.theme = Theme::from_name(value);
        }
        SettingsRow::DefaultGroup => {
            if value == AUTO_GROUP_LABEL {
                app.config.default_group = None;
            } else {
                app.config.default_group = Some(value.to_string());
            }
        }
        SettingsRow::AlbumArt => {
            app.config.album_art_mode = match value {
                "halfblock" => AlbumArtMode::Halfblock,
                "off" => AlbumArtMode::Off,
                _ => AlbumArtMode::Image,
            };
        }
    }

    if let Err(e) = app.config.save() {
        app.toast = Some(Toast::error(format!("failed to save config: {e}")));
    }
}
