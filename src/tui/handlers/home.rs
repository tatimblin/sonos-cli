//! Key handlers for the Speakers and Settings tabs.

use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::app::{App, Tab};
use crate::tui::handlers::{settings, speaker_list};
use crate::tui::types::{SettingsAction, SpeakerListAction};

pub fn handle_key(app: &mut App, key: KeyEvent) {
    app.status_message = None;

    if app.navigation.tab_focused {
        match key.code {
            KeyCode::Left | KeyCode::Right => {
                let prev_tab = app.navigation.tab.clone();
                app.navigation.tab = match app.navigation.tab {
                    Tab::Speakers => Tab::Settings,
                    Tab::Settings => Tab::Speakers,
                };
                // Reset dropdown state when switching away from Settings
                if prev_tab == Tab::Settings {
                    app.navigation.settings_state.dropdown_open = false;
                }
                app.navigation.tab_focused = false;
            }
            KeyCode::Down | KeyCode::Enter => {
                app.navigation.tab_focused = false;
            }
            _ => {}
        }
        return;
    }

    match app.navigation.tab {
        Tab::Speakers => handle_speakers_key(app, key),
        Tab::Settings => handle_settings_key(app, key),
    }
}

fn handle_speakers_key(app: &mut App, key: KeyEvent) {
    match speaker_list::handle_key(app, key) {
        SpeakerListAction::FocusTabBar => {
            app.navigation.tab_focused = true;
        }
        SpeakerListAction::Handled => {}
    }
}

fn handle_settings_key(app: &mut App, key: KeyEvent) {
    match settings::handle_key(app, key) {
        SettingsAction::FocusTabBar => {
            app.navigation.tab_focused = true;
        }
        SettingsAction::Handled => {}
    }
}
