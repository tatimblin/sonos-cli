//! Speaker list key handling — navigation, volume adjustment, pick-up/drop regrouping,
//! and playback controls (play/pause, next, previous).

use crossterm::event::KeyEvent;

use crate::tui::app::{App, Toast};
use crate::tui::keymap::{lookup, PickUpCommand, SpeakerCommand, PICK_UP_KEYS, SPEAKER_KEYS};
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
    let start = from + 1;
    entries
        .get(start..)?
        .iter()
        .position(|entry| {
            matches!(
                entry,
                ListEntry::AddToGroupRow(_) | ListEntry::CreateNewGroupRow
            )
        })
        .map(|offset| start + offset)
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
    let Some(command) = lookup(SPEAKER_KEYS, key.code) else {
        return SpeakerListAction::Handled;
    };
    let selected = app.navigation.speakers_state.selection.index_in(entries);

    match command {
        SpeakerCommand::Up => {
            if let Some(prev) = prev_entry(selected) {
                app.navigation
                    .speakers_state
                    .selection
                    .select(entries, prev);
            } else {
                return SpeakerListAction::FocusTabBar;
            }
            SpeakerListAction::Handled
        }
        SpeakerCommand::Down => {
            if let Some(next) = next_entry(entries, selected) {
                app.navigation
                    .speakers_state
                    .selection
                    .select(entries, next);
            }
            SpeakerListAction::Handled
        }
        SpeakerCommand::VolumeDown => {
            handle_volume_adjust(app, entries, selected, -2);
            SpeakerListAction::Handled
        }
        SpeakerCommand::VolumeUp => {
            handle_volume_adjust(app, entries, selected, 2);
            SpeakerListAction::Handled
        }
        SpeakerCommand::PickUp => {
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
                    app.navigation
                        .speakers_state
                        .selection
                        .select(&pickup_entries, first);
                }
            }
            SpeakerListAction::Handled
        }
        SpeakerCommand::ToggleMute => {
            handle_mute_toggle(app, entries, selected);
            SpeakerListAction::Handled
        }
        SpeakerCommand::PlayPause => {
            handle_play_pause(app, entries, selected);
            SpeakerListAction::Handled
        }
        SpeakerCommand::Next => {
            handle_playback_action(app, entries, selected, PlaybackAction::Next);
            SpeakerListAction::Handled
        }
        SpeakerCommand::Previous => {
            handle_playback_action(app, entries, selected, PlaybackAction::Previous);
            SpeakerListAction::Handled
        }
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

/// Toggle mute on the selected group header (group mute) or speaker row.
fn handle_mute_toggle(app: &mut App, entries: &[ListEntry], selected: usize) {
    if selected >= entries.len() {
        return;
    }
    let result = match &entries[selected] {
        ListEntry::GroupHeader(group_id) => app.system.group_by_id(group_id).map(|group| {
            let muted = group.mute.get().is_some_and(|m| m.is_muted());
            group.set_mute(!muted)
        }),
        ListEntry::SpeakerRow(speaker_id) => app.system.speaker_by_id(speaker_id).map(|speaker| {
            let muted = speaker.mute.get().is_some_and(|m| m.is_muted());
            speaker.set_mute(!muted)
        }),
        _ => None,
    };
    if let Some(Err(e)) = result {
        app.toast = Some(Toast::error(format!("{e}")));
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

    let Some(command) = lookup(PICK_UP_KEYS, key.code) else {
        return SpeakerListAction::Handled;
    };
    let selected = app.navigation.speakers_state.selection.index_in(entries);

    match command {
        PickUpCommand::Up => {
            if let Some(prev) = prev_action_row(entries, selected) {
                app.navigation
                    .speakers_state
                    .selection
                    .select(entries, prev);
            }
            SpeakerListAction::Handled
        }
        PickUpCommand::Down => {
            if let Some(next) = next_action_row(entries, selected) {
                app.navigation
                    .speakers_state
                    .selection
                    .select(entries, next);
            }
            SpeakerListAction::Handled
        }
        PickUpCommand::Drop => {
            match &entries[selected] {
                ListEntry::AddToGroupRow(target_gid) => {
                    let same_group = pick_up
                        .original_group_id
                        .as_ref()
                        .is_some_and(|og| og == target_gid);

                    if same_group {
                        app.navigation.speakers_state.cancel_pick_up();
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

            // Follow the moved speaker — the selection resolves its new row
            // once the topology change lands.
            app.navigation.speakers_state.cancel_pick_up();

            SpeakerListAction::Handled
        }
        PickUpCommand::Cancel => {
            app.navigation.speakers_state.cancel_pick_up();
            SpeakerListAction::Handled
        }
    }
}

#[cfg(test)]
mod tests {
    //! Only keys that resolve without a SOAP call are covered here: the
    //! in-memory system's speakers sit at unroutable IPs, so volume, mute and
    //! playback keys (and regrouping) would hit the network.

    use super::*;
    use crate::config::Config;
    use crate::tui::theme::Theme;
    use crossterm::event::KeyCode;
    use sonos_sdk::SonosSystem;

    /// Two standalone groups. `groups()` order is not guaranteed, so tests
    /// locate entries by kind rather than by name.
    fn app() -> App {
        let system = SonosSystem::with_groups(&["Kitchen", "Office"]);
        App::with_system(system, Config::default(), Theme::default_theme(), None)
    }

    fn press(app: &mut App, code: KeyCode) -> SpeakerListAction {
        handle_key(app, KeyEvent::from(code))
    }

    fn entries(app: &App) -> Vec<ListEntry> {
        build_list_entries(&app.system, app.navigation.speakers_state.pick_up.as_ref())
    }

    fn selected(app: &App) -> usize {
        app.navigation
            .speakers_state
            .selection
            .index_in(&entries(app))
    }

    #[test]
    fn down_moves_selection_and_stops_at_the_end() {
        let mut app = app();
        let len = entries(&app).len();
        assert_eq!(len, 4, "header + row per group");

        press(&mut app, KeyCode::Down);
        assert_eq!(selected(&app), 1);
        for _ in 0..len {
            press(&mut app, KeyCode::Down);
        }
        assert_eq!(selected(&app), len - 1);
    }

    #[test]
    fn up_from_the_top_focuses_the_tab_bar() {
        let mut app = app();
        assert!(matches!(
            press(&mut app, KeyCode::Up),
            SpeakerListAction::FocusTabBar
        ));
        press(&mut app, KeyCode::Down);
        assert!(matches!(
            press(&mut app, KeyCode::Up),
            SpeakerListAction::Handled
        ));
        assert_eq!(selected(&app), 0);
    }

    #[test]
    fn space_on_group_header_does_not_pick_up() {
        let mut app = app();
        press(&mut app, KeyCode::Char(' '));
        assert!(app.navigation.speakers_state.pick_up.is_none());
        assert!(app.toast.is_none());
    }

    #[test]
    fn space_on_speaker_row_picks_up_and_selects_home_row() {
        let mut app = app();
        press(&mut app, KeyCode::Down);
        let ListEntry::SpeakerRow(speaker_id) = entries(&app)[1].clone() else {
            panic!("entry 1 should be a speaker row");
        };
        let name = app.system.speaker_by_id(&speaker_id).unwrap().name.clone();

        press(&mut app, KeyCode::Char(' '));

        let pick_up = app.navigation.speakers_state.pick_up.clone().unwrap();
        assert_eq!(pick_up.speaker_id, speaker_id);
        let home = pick_up.original_group_id.expect("speaker has a group");
        assert_eq!(
            entries(&app)[selected(&app)],
            ListEntry::AddToGroupRow(home),
            "cursor lands on the speaker's own group"
        );
        assert_eq!(
            app.toast.as_ref().map(|t| t.message.as_str()),
            Some(format!("Picked up: {name}").as_str())
        );
    }

    #[test]
    fn pickup_down_jumps_between_action_rows_and_esc_cancels() {
        let mut app = app();
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Char(' '));
        assert_eq!(selected(&app), 2, "first group's AddToGroupRow");

        press(&mut app, KeyCode::Down);
        assert!(matches!(
            entries(&app)[selected(&app)],
            ListEntry::AddToGroupRow(_)
        ));
        assert_eq!(selected(&app), 5, "skips the second group's header and row");

        press(&mut app, KeyCode::Down);
        assert_eq!(entries(&app)[selected(&app)], ListEntry::CreateNewGroupRow);

        press(&mut app, KeyCode::Esc);
        assert!(app.navigation.speakers_state.pick_up.is_none());
    }

    #[test]
    fn cancelling_pickup_returns_the_cursor_to_the_picked_up_speaker() {
        let mut app = app();
        for _ in 0..3 {
            press(&mut app, KeyCode::Down);
        }
        let picked = entries(&app)[3].clone();
        assert!(matches!(picked, ListEntry::SpeakerRow(_)));

        press(&mut app, KeyCode::Char(' '));
        press(&mut app, KeyCode::Up);
        press(&mut app, KeyCode::Esc);

        assert_eq!(entries(&app)[selected(&app)], picked);
    }

    #[test]
    fn dropping_on_home_group_ends_pickup_without_regrouping() {
        let mut app = app();
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Char(' '));
        let before = app.system.groups().len();

        press(&mut app, KeyCode::Char(' '));

        assert!(app.navigation.speakers_state.pick_up.is_none());
        assert_eq!(app.system.groups().len(), before);
    }
}
