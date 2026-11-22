//! Key handlers for the Speakers and Settings tabs.

use crossterm::event::KeyEvent;

use crate::tui::app::{App, Tab};
use crate::tui::handlers::{settings, speaker_list};
use crate::tui::keymap::{lookup, TabBarCommand, TAB_BAR_KEYS};
use crate::tui::types::KeyOutcome;

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if app.navigation.tab_focused {
        match lookup(TAB_BAR_KEYS, key.code) {
            Some(TabBarCommand::SwitchTab) => {
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
            Some(TabBarCommand::Leave) => {
                app.navigation.tab_focused = false;
            }
            None => {}
        }
        return;
    }

    let outcome = match app.navigation.tab {
        Tab::Speakers => speaker_list::handle_key(app, key),
        Tab::Settings => settings::handle_key(app, key),
    };
    match outcome {
        KeyOutcome::FocusTabBar => {
            app.navigation.tab_focused = true;
        }
        KeyOutcome::Handled => {}
    }
}
