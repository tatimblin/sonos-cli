//! Speaker list key handling — navigation, volume adjustment, pick-up/drop regrouping.

use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::app::App;
use crate::tui::types::{
    build_list_entries, group_for_entry, ListEntry, PickUpState, SpeakerListAction,
};

/// Handle a key event for the speaker list. Returns an action for the caller.
pub fn handle_key(app: &mut App, key: KeyEvent) -> SpeakerListAction {
    let pick_up = app.navigation.speakers_state.pick_up.clone();
    let entries = build_list_entries(&app.system);

    if entries.is_empty() {
        return SpeakerListAction::Handled;
    }

    if pick_up.is_some() {
        return handle_pick_up_key(app, key, &entries);
    }

    handle_normal_key(app, key, &entries)
}

fn next_entry(entries: &[ListEntry], from: usize) -> Option<usize> {
    let next = from + 1;
    if next < entries.len() {
        Some(next)
    } else {
        None
    }
}

fn prev_entry(from: usize) -> Option<usize> {
    if from > 0 {
        Some(from - 1)
    } else {
        None
    }
}

fn handle_normal_key(app: &mut App, key: KeyEvent, entries: &[ListEntry]) -> SpeakerListAction {
    let selected = app
        .navigation
        .speakers_state
        .selected_index
        .min(entries.len().saturating_sub(1));

    match key.code {
        KeyCode::Up => {
            if let Some(prev) = prev_entry(selected) {
                app.navigation.speakers_state.selected_index = prev;
            } else {
                return SpeakerListAction::FocusTabBar;
            }
            SpeakerListAction::Handled
        }
        KeyCode::Down => {
            if let Some(next) = next_entry(entries, selected) {
                app.navigation.speakers_state.selected_index = next;
            }
            SpeakerListAction::Handled
        }
        KeyCode::Left => {
            handle_volume_adjust(app, entries, selected, -2);
            SpeakerListAction::Handled
        }
        KeyCode::Right => {
            handle_volume_adjust(app, entries, selected, 2);
            SpeakerListAction::Handled
        }
        KeyCode::Char(' ') => {
            if selected >= entries.len() {
                return SpeakerListAction::Handled;
            }
            if let ListEntry::SpeakerRow(speaker_id) = &entries[selected] {
                let original_group = app
                    .system
                    .speaker_by_id(speaker_id)
                    .and_then(|s| s.group())
                    .map(|g| g.id.clone());

                app.navigation.speakers_state.pick_up = Some(PickUpState {
                    speaker_id: speaker_id.clone(),
                    original_group_id: original_group,
                    drop_index: selected,
                });
            }
            SpeakerListAction::Handled
        }
        _ => SpeakerListAction::Handled,
    }
}

fn handle_volume_adjust(app: &mut App, entries: &[ListEntry], selected: usize, delta: i8) {
    if selected >= entries.len() {
        return;
    }
    match &entries[selected] {
        ListEntry::GroupHeader(group_id) => {
            if let Some(group) = app.system.group_by_id(group_id) {
                if let Err(e) = group.set_relative_volume(delta as i16) {
                    app.status_message = Some(format!("error: {e}"));
                }
            }
        }
        ListEntry::SpeakerRow(speaker_id) => {
            if let Some(speaker) = app.system.speaker_by_id(speaker_id) {
                if let Err(e) = speaker.set_relative_volume(delta) {
                    app.status_message = Some(format!("error: {e}"));
                }
            }
        }
    }
}

fn handle_pick_up_key(app: &mut App, key: KeyEvent, entries: &[ListEntry]) -> SpeakerListAction {
    let pick_up = match app.navigation.speakers_state.pick_up.clone() {
        Some(p) => p,
        None => return SpeakerListAction::Handled,
    };

    let drop_index = pick_up.drop_index.min(entries.len().saturating_sub(1));

    match key.code {
        KeyCode::Up => {
            if let Some(prev) = prev_entry(drop_index) {
                if let Some(ref mut pu) = app.navigation.speakers_state.pick_up {
                    pu.drop_index = prev;
                }
            }
            SpeakerListAction::Handled
        }
        KeyCode::Down => {
            if let Some(next) = next_entry(entries, drop_index) {
                if let Some(ref mut pu) = app.navigation.speakers_state.pick_up {
                    pu.drop_index = next;
                }
            }
            SpeakerListAction::Handled
        }
        KeyCode::Char(' ') => {
            let target_group = group_for_entry(entries, drop_index);
            let same_group = pick_up.original_group_id.as_ref() == target_group.as_ref();

            if same_group {
                app.navigation.speakers_state.pick_up = None;
                return SpeakerListAction::Handled;
            }

            if let Some(speaker) = app.system.speaker_by_id(&pick_up.speaker_id) {
                match target_group {
                    Some(target_gid) => {
                        if let Some(group) = app.system.group_by_id(&target_gid) {
                            match group.add_speaker(&speaker) {
                                Ok(()) => {
                                    let group_name = group
                                        .coordinator()
                                        .map(|c| c.name.clone())
                                        .unwrap_or_else(|| "group".to_string());
                                    app.status_message =
                                        Some(format!("{} moved to {}", speaker.name, group_name));
                                }
                                Err(e) => {
                                    app.status_message = Some(format!("error: {e}"));
                                }
                            }
                        }
                    }
                    None => match speaker.leave_group() {
                        Ok(_) => {
                            app.status_message = Some(format!("{} ungrouped", speaker.name));
                        }
                        Err(e) => {
                            app.status_message = Some(format!("error: {e}"));
                        }
                    },
                }
            }

            app.navigation.speakers_state.pick_up = None;
            SpeakerListAction::Handled
        }
        KeyCode::Esc => {
            app.navigation.speakers_state.pick_up = None;
            SpeakerListAction::Handled
        }
        _ => SpeakerListAction::Handled,
    }
}
