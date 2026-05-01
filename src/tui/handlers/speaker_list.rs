//! Speaker list key handling — navigation, volume adjustment, pick-up/drop regrouping,
//! and playback controls (play/pause, next, previous).

use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::app::App;
use crate::tui::types::{
    build_list_entries, group_for_entry, DropZoneKind, ListEntry, PickUpState, SpeakerListAction,
};

/// Handle a key event for the speaker list. Returns an action for the caller.
pub fn handle_key(app: &mut App, key: KeyEvent) -> SpeakerListAction {
    let is_pick_up = app.navigation.speakers_state.pick_up.is_some();

    if is_pick_up {
        return handle_pick_up_key(app, key);
    }

    let entries = build_list_entries(&app.system);
    if entries.is_empty() {
        return SpeakerListAction::Handled;
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
                let speaker = app.system.speaker_by_id(speaker_id);
                let original_group = speaker
                    .as_ref()
                    .and_then(|s| s.group())
                    .map(|g| g.id.clone());
                let speaker_name = speaker
                    .map(|s| s.name.clone())
                    .unwrap_or_else(|| "Speaker".to_string());

                // Find the initial zone index: match the speaker's current group
                let groups = app.system.groups();
                let initial_zone = original_group
                    .as_ref()
                    .and_then(|og| groups.iter().position(|g| g.id == *og))
                    .unwrap_or(0);

                app.navigation.speakers_state.pick_up = Some(PickUpState {
                    speaker_id: speaker_id.clone(),
                    speaker_name,
                    original_group_id: original_group,
                    active_zone_index: initial_zone,
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

fn handle_pick_up_key(app: &mut App, key: KeyEvent) -> SpeakerListAction {
    let pick_up = match app.navigation.speakers_state.pick_up.clone() {
        Some(p) => p,
        None => return SpeakerListAction::Handled,
    };

    // Build the current zone list to navigate
    let groups = app.system.groups();
    let zone_count = groups.len() + 1; // +1 for "Add new group"
    let active = pick_up.active_zone_index.min(zone_count.saturating_sub(1));

    match key.code {
        KeyCode::Up => {
            if active > 0 {
                if let Some(ref mut pu) = app.navigation.speakers_state.pick_up {
                    pu.active_zone_index = active - 1;
                }
            }
            SpeakerListAction::Handled
        }
        KeyCode::Down => {
            if active + 1 < zone_count {
                if let Some(ref mut pu) = app.navigation.speakers_state.pick_up {
                    pu.active_zone_index = active + 1;
                }
            }
            SpeakerListAction::Handled
        }
        KeyCode::Char(' ') => {
            // Determine the target zone kind
            let target_kind = if active < groups.len() {
                DropZoneKind::ExistingGroup(groups[active].id.clone())
            } else {
                DropZoneKind::NewGroup
            };

            // Check if dropping on the same group (no-op)
            let same_group = match (&target_kind, &pick_up.original_group_id) {
                (DropZoneKind::ExistingGroup(target_gid), Some(orig_gid)) => target_gid == orig_gid,
                _ => false,
            };

            if same_group {
                // Space-Space = safe no-op, just cancel pick-up
                app.navigation.speakers_state.pick_up = None;
                return SpeakerListAction::Handled;
            }

            // Perform the drop
            if let Some(speaker) = app.system.speaker_by_id(&pick_up.speaker_id) {
                match target_kind {
                    DropZoneKind::ExistingGroup(target_gid) => {
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
                        } else {
                            // Target group dissolved during pick-up
                            app.status_message =
                                Some("error: target group no longer exists".to_string());
                        }
                    }
                    DropZoneKind::NewGroup => match speaker.leave_group() {
                        Ok(_) => {
                            app.status_message =
                                Some(format!("{} is now standalone", speaker.name));
                        }
                        Err(e) => {
                            app.status_message = Some(format!("error: {e}"));
                        }
                    },
                }
            }

            // Find the speaker in the rebuilt list and set cursor there
            let new_entries = build_list_entries(&app.system);
            let new_index = new_entries
                .iter()
                .position(|e| matches!(e, ListEntry::SpeakerRow(sid) if *sid == pick_up.speaker_id))
                .unwrap_or(0);
            app.navigation.speakers_state.selected_index = new_index;
            app.navigation.speakers_state.pick_up = None;

            SpeakerListAction::Handled
        }
        KeyCode::Esc => {
            // Esc restores original selected_index (already preserved)
            app.navigation.speakers_state.pick_up = None;
            SpeakerListAction::Handled
        }
        _ => {
            // All other keys swallowed during pick-up
            SpeakerListAction::Handled
        }
    }
}
