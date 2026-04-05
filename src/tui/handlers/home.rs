//! Key handlers for the Home screen (Groups & Speakers tabs).

use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::app::{App, GroupTab, HomeTab, Screen, SpeakerListScreenState};
use crate::tui::widgets::speaker_list::{self, SpeakerListAction, SpeakerListMode};

pub fn handle_home_key(app: &mut App, key: KeyEvent, tab: &HomeTab, tab_focused: bool) {
    // Clear status message on any key press
    app.status_message = None;

    // When tab bar is focused, Left/Right switch tabs, Down/Enter return to content
    if tab_focused {
        match key.code {
            KeyCode::Left | KeyCode::Right => {
                if let Screen::Home {
                    ref mut tab,
                    ref mut tab_focused,
                    ..
                } = app.navigation.current_mut()
                {
                    *tab = match tab {
                        HomeTab::Groups => HomeTab::Speakers,
                        HomeTab::Speakers => HomeTab::Groups,
                    };
                    *tab_focused = false;
                }
            }
            KeyCode::Down | KeyCode::Enter => {
                if let Screen::Home {
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

    match tab {
        HomeTab::Groups => handle_home_groups_key(app, key),
        HomeTab::Speakers => handle_home_speakers_key(app, key),
    }
}

fn handle_home_groups_key(app: &mut App, key: KeyEvent) {
    let groups = app.system.groups();
    let total = groups.len();
    let cols = if app.terminal_width >= 100 { 2 } else { 1 };

    match key.code {
        KeyCode::Up => {
            if let Screen::Home {
                ref mut groups_state,
                ref mut tab_focused,
                ..
            } = app.navigation.current_mut()
            {
                if total > 0 && groups_state.selected_index >= cols {
                    groups_state.selected_index -= cols;
                } else {
                    // Already at top row — focus the tab bar
                    *tab_focused = true;
                }
            }
        }
        KeyCode::Down => {
            if let Screen::Home {
                ref mut groups_state,
                ..
            } = app.navigation.current_mut()
            {
                if total > 0 {
                    let new_idx = groups_state.selected_index + cols;
                    if new_idx < total {
                        groups_state.selected_index = new_idx;
                    }
                }
            }
        }
        KeyCode::Left => {
            if let Screen::Home {
                ref mut groups_state,
                ..
            } = app.navigation.current_mut()
            {
                if cols > 1 && groups_state.selected_index % cols > 0 {
                    groups_state.selected_index -= 1;
                }
            }
        }
        KeyCode::Right => {
            if let Screen::Home {
                ref mut groups_state,
                ..
            } = app.navigation.current_mut()
            {
                if cols > 1 && groups_state.selected_index % cols < cols - 1 {
                    let new_idx = groups_state.selected_index + 1;
                    if new_idx < total {
                        groups_state.selected_index = new_idx;
                    }
                }
            }
        }
        KeyCode::Enter => {
            if total > 0 {
                let selected = match app.navigation.current() {
                    Screen::Home { groups_state, .. } => groups_state.selected_index,
                    _ => 0,
                };
                if let Some(group) = groups.get(selected) {
                    app.navigation.push(Screen::GroupView {
                        group_id: group.id.clone(),
                        tab: GroupTab::default(),
                        tab_focused: false,
                        speakers_state: SpeakerListScreenState::default(),
                    });
                }
            }
        }
        _ => {}
    }
}

fn handle_home_speakers_key(app: &mut App, key: KeyEvent) {
    let mode = SpeakerListMode::FullList;
    match speaker_list::handle_key(app, key, &mode) {
        SpeakerListAction::NavigateToGroup(group_id) => {
            app.navigation.push(Screen::GroupView {
                group_id,
                tab: GroupTab::default(),
                tab_focused: false,
                speakers_state: SpeakerListScreenState::default(),
            });
        }
        SpeakerListAction::NavigateToSpeaker(speaker_id) => {
            app.navigation.push(Screen::SpeakerDetail { speaker_id });
        }
        SpeakerListAction::FocusTabBar => {
            if let Screen::Home {
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
