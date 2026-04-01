//! Key handlers for the GroupView and SpeakerDetail screens.

use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::app::{App, GroupTab, Screen};

pub fn handle_group_key(
    app: &mut App,
    key: KeyEvent,
    group_id: &sonos_sdk::GroupId,
    tab: &GroupTab,
) {
    // Tab-level keys (work on all tabs)
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
            return;
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
            return;
        }
        _ => {}
    }

    // Tab-specific keys
    match tab {
        GroupTab::NowPlaying => handle_now_playing_key(app, key, group_id),
        GroupTab::Speakers => {} // Milestone 8 (speakers tab)
        GroupTab::Queue => {}    // Milestone 8 (queue tab)
    }
}

fn handle_now_playing_key(app: &mut App, key: KeyEvent, group_id: &sonos_sdk::GroupId) {
    let group = app.system.group_by_id(group_id);
    let coordinator = group.as_ref().and_then(|g| g.coordinator());

    match key.code {
        // Space: toggle play/pause
        KeyCode::Char(' ') => {
            if let Some(ref coord) = coordinator {
                let state = coord.playback_state.get();
                let result = match state {
                    Some(sonos_sdk::PlaybackState::Playing) => coord.pause(),
                    _ => coord.play(),
                };
                if let Err(e) = result {
                    app.status_message = Some(format!("Playback error: {e}"));
                }
            }
        }
        // Up: volume up
        KeyCode::Up => {
            if let Some(ref group) = group {
                if let Err(e) = group.set_relative_volume(2) {
                    app.status_message = Some(format!("Volume error: {e}"));
                }
            }
        }
        // Down: volume down
        KeyCode::Down => {
            if let Some(ref group) = group {
                if let Err(e) = group.set_relative_volume(-2) {
                    app.status_message = Some(format!("Volume error: {e}"));
                }
            }
        }
        // n: next track
        KeyCode::Char('n') => {
            if let Some(ref coord) = coordinator {
                if let Err(e) = coord.next() {
                    app.status_message = Some(format!("Next track error: {e}"));
                }
            }
        }
        // p: previous track
        KeyCode::Char('p') => {
            if let Some(ref coord) = coordinator {
                if let Err(e) = coord.previous() {
                    app.status_message = Some(format!("Previous track error: {e}"));
                }
            }
        }
        _ => {}
    }
}

pub fn handle_speaker_key(_app: &mut App, _key: KeyEvent) {
    // Milestone 9: speaker detail key handling
}
