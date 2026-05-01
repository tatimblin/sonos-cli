//! Speaker list key handling — navigation, volume adjustment, pick-up/drop regrouping,
//! and playback controls (play/pause, next, previous).

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

fn next_selectable(entries: &[ListEntry], from: usize) -> Option<usize> {
    ((from + 1)..entries.len()).find(|&i| entries[i].is_selectable())
}

fn prev_selectable(entries: &[ListEntry], from: usize) -> Option<usize> {
    (0..from).rev().find(|&i| entries[i].is_selectable())
}

fn handle_normal_key(app: &mut App, key: KeyEvent, entries: &[ListEntry]) -> SpeakerListAction {
    let selected = app
        .navigation
        .speakers_state
        .selected_index
        .min(entries.len().saturating_sub(1));

    match key.code {
        KeyCode::Up => {
            if let Some(prev) = prev_selectable(entries, selected) {
                app.navigation.speakers_state.selected_index = prev;
            } else {
                return SpeakerListAction::FocusTabBar;
            }
            SpeakerListAction::Handled
        }
        KeyCode::Down => {
            if let Some(next) = next_selectable(entries, selected) {
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
    // Try group header first
    let group_id = group_for_entry(entries, selected);

    let group = group_id
        .and_then(|gid| app.system.group_by_id(&gid))
        .or_else(|| {
            // Standalone speaker: resolve via the speaker's own group
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

    // Toggle: if playing, pause; otherwise play
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
        app.status_message = Some(format!("error: {e}"));
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
        app.status_message = Some(format!("error: {e}"));
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
            if let Some(prev) = prev_selectable(entries, drop_index) {
                if let Some(ref mut pu) = app.navigation.speakers_state.pick_up {
                    pu.drop_index = prev;
                }
            }
            SpeakerListAction::Handled
        }
        KeyCode::Down => {
            if let Some(next) = next_selectable(entries, drop_index) {
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
