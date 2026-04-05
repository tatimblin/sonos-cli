//! Key handlers for the GroupView and SpeakerDetail screens.

use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::app::{App, GroupTab, Screen, SpeakerListScreenState};
use crate::tui::widgets::speaker_list::{self, SpeakerListAction, SpeakerListMode};

pub fn handle_group_key(
    app: &mut App,
    key: KeyEvent,
    group_id: &sonos_sdk::GroupId,
    tab: &GroupTab,
    tab_focused: bool,
) {
    // When tab bar is focused, Left/Right switch tabs, Down/Enter return to content
    if tab_focused {
        match key.code {
            KeyCode::Left | KeyCode::Right => {
                let new_tab = match key.code {
                    KeyCode::Left => match tab {
                        GroupTab::NowPlaying => GroupTab::Queue,
                        GroupTab::Speakers => GroupTab::NowPlaying,
                        GroupTab::Queue => GroupTab::Speakers,
                    },
                    _ => match tab {
                        GroupTab::NowPlaying => GroupTab::Speakers,
                        GroupTab::Speakers => GroupTab::Queue,
                        GroupTab::Queue => GroupTab::NowPlaying,
                    },
                };
                if let Screen::GroupView {
                    ref mut tab,
                    ref mut tab_focused,
                    ..
                } = app.navigation.current_mut()
                {
                    *tab = new_tab;
                    *tab_focused = false;
                }
            }
            KeyCode::Down | KeyCode::Enter => {
                if let Screen::GroupView {
                    ref mut tab_focused,
                    ..
                } = app.navigation.current_mut()
                {
                    *tab_focused = false;
                }
            }
            _ => {}
        }
        return;
    }

    // Tab-specific keys
    match tab {
        GroupTab::NowPlaying => {
            // NowPlaying uses Left/Right for tab switching (no volume on this tab)
            match key.code {
                KeyCode::Left | KeyCode::Right => {
                    let new_tab = match key.code {
                        KeyCode::Left => GroupTab::Queue,
                        _ => GroupTab::Speakers,
                    };
                    if let Screen::GroupView { ref mut tab, .. } = app.navigation.current_mut() {
                        *tab = new_tab;
                    }
                    return;
                }
                _ => {}
            }
            handle_now_playing_key(app, key, group_id);
        }
        GroupTab::Speakers => {
            let mode = SpeakerListMode::GroupScoped {
                group_id: group_id.clone(),
            };
            match speaker_list::handle_key(app, key, &mode) {
                SpeakerListAction::NavigateToSpeaker(speaker_id) => {
                    app.navigation.push(Screen::SpeakerDetail { speaker_id });
                }
                SpeakerListAction::NavigateToGroup(gid) => {
                    app.navigation.push(Screen::GroupView {
                        group_id: gid,
                        tab: GroupTab::default(),
                        tab_focused: false,
                        speakers_state: SpeakerListScreenState::default(),
                    });
                }
                SpeakerListAction::FocusTabBar => {
                    if let Screen::GroupView {
                        ref mut tab_focused,
                        ..
                    } = app.navigation.current_mut()
                    {
                        *tab_focused = true;
                    }
                }
                SpeakerListAction::Handled => {}
            }
        }
        GroupTab::Queue => {
            // Left/Right switches tabs on Queue (no content interaction yet)
            match key.code {
                KeyCode::Left => {
                    if let Screen::GroupView { ref mut tab, .. } = app.navigation.current_mut() {
                        *tab = GroupTab::Speakers;
                    }
                }
                KeyCode::Right => {
                    if let Screen::GroupView { ref mut tab, .. } = app.navigation.current_mut() {
                        *tab = GroupTab::NowPlaying;
                    }
                }
                _ => {}
            }
        }
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
