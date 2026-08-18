//! Speaker list key handling — navigation, volume adjustment, pick-up/drop regrouping,
//! and playback controls (play/pause, next, previous).

use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::app::{App, Toast};
use crate::tui::types::{
    build_list_entries, group_for_entry, ListEntry, PickUpState, SpeakerListAction,
};

/// Handle a key event for the speaker list. Returns an action for the caller.
pub fn handle_key(app: &mut App, key: KeyEvent) -> SpeakerListAction {
    let is_pick_up = app.navigation.speakers_state.pick_up.is_some();
    let entries = build_list_entries(&app.system, app.navigation.speakers_state.pick_up.as_ref());

    if entries.is_empty() {
        return SpeakerListAction::Handled;
    }

    if is_pick_up {
        handle_pick_up_key(app, key, &entries)
    } else {
        handle_normal_key(app, key, &entries)
    }
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

/// Find the next selectable action row (AddToGroupRow where !is_home, or CreateNewGroupRow).
fn next_action_row(entries: &[ListEntry], from: usize) -> Option<usize> {
    for i in (from + 1)..entries.len() {
        match &entries[i] {
            ListEntry::AddToGroupRow(_) | ListEntry::CreateNewGroupRow => return Some(i),
            _ => {}
        }
    }
    None
}

/// Find the previous selectable action row.
fn prev_action_row(entries: &[ListEntry], from: usize) -> Option<usize> {
    for i in (0..from).rev() {
        match &entries[i] {
            ListEntry::AddToGroupRow(_) | ListEntry::CreateNewGroupRow => return Some(i),
            _ => {}
        }
    }
    None
}

/// Find the first selectable action row in the list.
fn home_action_row(app: &App, entries: &[ListEntry]) -> Option<usize> {
    let original_group_id = app
        .navigation
        .speakers_state
        .pick_up
        .as_ref()
        .and_then(|pu| pu.original_group_id.as_ref());

    for (i, entry) in entries.iter().enumerate() {
        if let ListEntry::AddToGroupRow(gid) = entry {
            if original_group_id.is_some_and(|og| og == gid) {
                return Some(i);
            }
        }
    }
    None
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
                let speaker = app.system.speaker_by_id(speaker_id);
                let original_group = speaker
                    .as_ref()
                    .and_then(|s| s.group())
                    .map(|g| g.id.clone());
                let speaker_name = speaker
                    .map(|s| s.name.clone())
                    .unwrap_or_else(|| "Speaker".to_string());

                app.toast = Some(Toast::info(format!("Picked up: {speaker_name}")));
                app.navigation.speakers_state.pick_up = Some(PickUpState {
                    speaker_id: speaker_id.clone(),
                    original_group_id: original_group,
                });

                // Rebuild entries with pickup active to find the first action row
                let pickup_entries =
                    build_list_entries(&app.system, app.navigation.speakers_state.pick_up.as_ref());
                if let Some(first) = home_action_row(app, &pickup_entries) {
                    app.navigation.speakers_state.selected_index = first;
                }
            }
            SpeakerListAction::Handled
        }
        KeyCode::Char('p') => {
            handle_play_pause(app, entries, selected);
            SpeakerListAction::Handled
        }
        KeyCode::Char('n') => {
            handle_playback_action(app, entries, selected, PlaybackAction::Next);
            SpeakerListAction::Handled
        }
        KeyCode::Char('b') => {
            handle_playback_action(app, entries, selected, PlaybackAction::Previous);
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
                    app.toast = Some(Toast::error(format!("{e}")));
                }
            }
        }
        ListEntry::SpeakerRow(speaker_id) => {
            if let Some(speaker) = app.system.speaker_by_id(speaker_id) {
                if let Err(e) = speaker.set_relative_volume(delta) {
                    app.toast = Some(Toast::error(format!("{e}")));
                }
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Playback controls
// ---------------------------------------------------------------------------

enum PlaybackAction {
    Next,
    Previous,
}

/// Resolve the coordinator speaker for the group containing the selected entry.
fn resolve_coordinator(
    app: &App,
    entries: &[ListEntry],
    selected: usize,
) -> Option<sonos_sdk::Speaker> {
    let group_id = group_for_entry(entries, selected);

    let group = group_id
        .and_then(|gid| app.system.group_by_id(&gid))
        .or_else(|| {
            if selected < entries.len() {
                if let ListEntry::SpeakerRow(sid) = &entries[selected] {
                    return app.system.speaker_by_id(sid).and_then(|s| s.group());
                }
            }
            None
        });

    group.and_then(|g| g.coordinator())
}

fn handle_play_pause(app: &mut App, entries: &[ListEntry], selected: usize) {
    let Some(coordinator) = resolve_coordinator(app, entries, selected) else {
        return;
    };

    let is_playing = coordinator
        .playback_state
        .get()
        .is_some_and(|ps| ps.is_playing());

    let result = if is_playing {
        coordinator.pause()
    } else {
        coordinator.play()
    };

    if let Err(e) = result {
        app.toast = Some(Toast::error(format!("{e}")));
    }
}

fn handle_playback_action(
    app: &mut App,
    entries: &[ListEntry],
    selected: usize,
    action: PlaybackAction,
) {
    let Some(coordinator) = resolve_coordinator(app, entries, selected) else {
        return;
    };

    let result = match action {
        PlaybackAction::Next => coordinator.next(),
        PlaybackAction::Previous => coordinator.previous(),
    };

    if let Err(e) = result {
        app.toast = Some(Toast::error(format!("{e}")));
    }
}

// ---------------------------------------------------------------------------
// Pick-up mode handling
// ---------------------------------------------------------------------------

fn handle_pick_up_key(app: &mut App, key: KeyEvent, entries: &[ListEntry]) -> SpeakerListAction {
    let pick_up = match app.navigation.speakers_state.pick_up.clone() {
        Some(p) => p,
        None => return SpeakerListAction::Handled,
    };

    let selected = app
        .navigation
        .speakers_state
        .selected_index
        .min(entries.len().saturating_sub(1));

    match key.code {
        KeyCode::Up => {
            if let Some(prev) = prev_action_row(entries, selected) {
                app.navigation.speakers_state.selected_index = prev;
            }
            SpeakerListAction::Handled
        }
        KeyCode::Down => {
            if let Some(next) = next_action_row(entries, selected) {
                app.navigation.speakers_state.selected_index = next;
            }
            SpeakerListAction::Handled
        }
        KeyCode::Char(' ') => {
            match &entries[selected] {
                ListEntry::AddToGroupRow(target_gid) => {
                    let same_group = pick_up
                        .original_group_id
                        .as_ref()
                        .is_some_and(|og| og == target_gid);

                    if same_group {
                        app.navigation.speakers_state.pick_up = None;
                        return SpeakerListAction::Handled;
                    }

                    if let Some(speaker) = app.system.speaker_by_id(&pick_up.speaker_id) {
                        if let Some(group) = app.system.group_by_id(target_gid) {
                            match group.add_speaker(&speaker) {
                                Ok(()) => {
                                    let group_name = group
                                        .coordinator()
                                        .map(|c| c.name.clone())
                                        .unwrap_or_else(|| "group".to_string());
                                    app.toast = Some(Toast::info(format!(
                                        "{} moved to {}",
                                        speaker.name, group_name
                                    )));
                                }
                                Err(e) => {
                                    app.toast = Some(Toast::error(format!("{e}")));
                                }
                            }
                        } else {
                            app.toast =
                                Some(Toast::error("target group no longer exists".to_string()));
                        }
                    }
                }
                ListEntry::CreateNewGroupRow => {
                    if let Some(speaker) = app.system.speaker_by_id(&pick_up.speaker_id) {
                        match speaker.leave_group() {
                            Ok(_) => {
                                app.toast = Some(Toast::info(format!(
                                    "{} is now standalone",
                                    speaker.name
                                )));
                            }
                            Err(e) => {
                                app.toast = Some(Toast::error(format!("{e}")));
                            }
                        }
                    }
                }
                _ => {
                    // Space on a non-action row during pickup — ignore
                    return SpeakerListAction::Handled;
                }
            }

            // Find the speaker in the rebuilt list and set cursor there
            let new_entries = build_list_entries(&app.system, None);
            let new_index = new_entries
                .iter()
                .position(|e| matches!(e, ListEntry::SpeakerRow(sid) if *sid == pick_up.speaker_id))
                .unwrap_or(0);
            app.navigation.speakers_state.selected_index = new_index;
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
