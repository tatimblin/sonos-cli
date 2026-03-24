//! Key handlers for the GroupView and SpeakerDetail screens.

use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::app::{App, GroupTab, Screen};

pub fn handle_group_key(
    app: &mut App,
    key: KeyEvent,
    group_id: &sonos_sdk::GroupId,
    tab: &GroupTab,
) {
    match key.code {
        KeyCode::Left => {
            let new_tab = match tab {
                GroupTab::NowPlaying => GroupTab::Queue,
                GroupTab::Speakers => GroupTab::NowPlaying,
                GroupTab::Queue => GroupTab::Speakers,
            };
            *app.navigation.current_mut() = Screen::GroupView {
                group_id: group_id.clone(),
                tab: new_tab,
            };
        }
        KeyCode::Right => {
            let new_tab = match tab {
                GroupTab::NowPlaying => GroupTab::Speakers,
                GroupTab::Speakers => GroupTab::Queue,
                GroupTab::Queue => GroupTab::NowPlaying,
            };
            *app.navigation.current_mut() = Screen::GroupView {
                group_id: group_id.clone(),
                tab: new_tab,
            };
        }
        _ => {}
    }
}

pub fn handle_speaker_key(_app: &mut App, _key: KeyEvent) {
    // Milestone 9: speaker detail key handling
}
