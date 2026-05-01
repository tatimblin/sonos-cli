//! Key handlers for the Speakers and Settings tabs.

use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::app::{App, Tab};
use crate::tui::types::SpeakerListAction;
use crate::tui::widgets::speaker_list;

pub fn handle_key(app: &mut App, key: KeyEvent) {
    app.status_message = None;

    if app.navigation.tab_focused {
        match key.code {
            KeyCode::Left | KeyCode::Right => {
                app.navigation.tab = match app.navigation.tab {
                    Tab::Speakers => Tab::Settings,
                    Tab::Settings => Tab::Speakers,
                };
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
        Tab::Settings => {}
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
