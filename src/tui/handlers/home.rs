//! Key handlers for the Home screen (Groups & Speakers tabs).

use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::app::{App, GroupTab, HomeSpeakersState, HomeTab, ModalState, Screen};
use crate::tui::screens::home_speakers;

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
                    });
                }
            }
        }
        _ => {}
    }
}

fn handle_home_speakers_key(app: &mut App, key: KeyEvent) {
    let speaker_count = home_speakers::speakers_in_display_order(app).len();

    match key.code {
        KeyCode::Up => {
            if let Screen::Home {
                ref mut speakers_state,
                ref mut tab_focused,
                ..
            } = app.navigation.current_mut()
            {
                if let Some(ref mut modal) = speakers_state.modal {
                    if modal.selected_index > 0 {
                        modal.selected_index -= 1;
                    }
                    return;
                }
                if speakers_state.selected_index > 0 {
                    speakers_state.selected_index -= 1;
                } else {
                    // Already at top — focus the tab bar
                    *tab_focused = true;
                }
            }
        }
        KeyCode::Down => {
            if let Screen::Home {
                ref mut speakers_state,
                ..
            } = app.navigation.current_mut()
            {
                if let Some(ref mut modal) = speakers_state.modal {
                    if modal.selected_index + 1 < modal.items.len() {
                        modal.selected_index += 1;
                    }
                    return;
                }
                if speaker_count > 0 && speakers_state.selected_index + 1 < speaker_count {
                    speakers_state.selected_index += 1;
                }
            }
        }
        KeyCode::Char('n') => {
            handle_speakers_create_group(app);
        }
        KeyCode::Char('d') => {
            handle_speakers_ungroup(app);
        }
        KeyCode::Enter => {
            handle_speakers_enter(app);
        }
        _ => {}
    }
}

fn handle_speakers_create_group(app: &mut App) {
    let display_ids = home_speakers::speakers_in_display_order(app);
    let selected = match app.navigation.current() {
        Screen::Home { speakers_state, .. } => speakers_state.selected_index,
        _ => return,
    };
    let Some(speaker_id) = display_ids.get(selected) else {
        return;
    };
    if let Some(speaker) = app.system.speaker_by_id(speaker_id) {
        match app.system.create_group(&speaker, &[]) {
            Ok(result) if result.is_success() => {
                app.status_message = Some(format!("Created group with {}", speaker.name));
            }
            Ok(_) => {
                app.status_message = Some("Group created with partial failures".to_string());
            }
            Err(e) => {
                app.status_message = Some(format!("error: {e}"));
            }
        }
    }
}

fn handle_speakers_ungroup(app: &mut App) {
    let display_ids = home_speakers::speakers_in_display_order(app);
    let selected = match app.navigation.current() {
        Screen::Home { speakers_state, .. } => speakers_state.selected_index,
        _ => return,
    };
    let Some(speaker_id) = display_ids.get(selected) else {
        return;
    };
    if let Some(speaker) = app.system.speaker_by_id(speaker_id) {
        if let Some(group) = speaker.group() {
            if group.is_coordinator(&speaker.id) && group.member_count() > 1 {
                app.status_message =
                    Some("Cannot ungroup coordinator. Remove other members first.".to_string());
                return;
            }
            if group.is_standalone() {
                app.status_message = Some("Speaker is already standalone.".to_string());
                return;
            }
        }
        match speaker.leave_group() {
            Ok(_) => {
                app.status_message = Some(format!("{} ungrouped", speaker.name));
            }
            Err(e) => {
                app.status_message = Some(format!("error: {e}"));
            }
        }
    }
}

fn handle_speakers_enter(app: &mut App) {
    let has_modal = matches!(
        app.navigation.current(),
        Screen::Home {
            speakers_state: HomeSpeakersState { modal: Some(_), .. },
            ..
        }
    );

    if has_modal {
        confirm_group_picker(app);
        return;
    }

    // Open group picker modal
    let groups = app.system.groups();
    let non_standalone: Vec<String> = groups
        .iter()
        .filter(|g| !g.is_standalone())
        .filter_map(|g| g.coordinator().map(|c| c.name.clone()))
        .collect();

    if non_standalone.is_empty() {
        app.status_message = Some("No groups available to join.".to_string());
        return;
    }

    if let Screen::Home {
        ref mut speakers_state,
        ..
    } = app.navigation.current_mut()
    {
        speakers_state.modal = Some(ModalState {
            title: "Move to group".to_string(),
            items: non_standalone,
            selected_index: 0,
        });
    }
}

fn confirm_group_picker(app: &mut App) {
    let (group_name, speaker_idx) = {
        let screen = app.navigation.current();
        match screen {
            Screen::Home { speakers_state, .. } => {
                if let Some(ref modal) = speakers_state.modal {
                    let name = modal.items.get(modal.selected_index).cloned();
                    (name, speakers_state.selected_index)
                } else {
                    (None, 0)
                }
            }
            _ => (None, 0),
        }
    };

    if let Some(group_name) = group_name {
        let display_ids = home_speakers::speakers_in_display_order(app);
        if let Some(speaker) = display_ids
            .get(speaker_idx)
            .and_then(|id| app.system.speaker_by_id(id))
        {
            if let Some(group) = app.system.group(&group_name) {
                match group.add_speaker(&speaker) {
                    Ok(()) => {
                        app.status_message =
                            Some(format!("{} moved to {}", speaker.name, group_name));
                    }
                    Err(e) => {
                        app.status_message = Some(format!("error: {e}"));
                    }
                }
            }
        }
    }

    if let Screen::Home {
        ref mut speakers_state,
        ..
    } = app.navigation.current_mut()
    {
        speakers_state.modal = None;
    }
}
