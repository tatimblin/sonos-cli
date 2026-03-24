//! TUI event loop and key handling.
//!
//! Uses `event::poll(250ms)` so the SDK event drain runs even without keyboard
//! input. The `dirty` flag skips rendering on idle poll timeouts.

use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use crate::tui::app::{App, GroupTab, HomeTab, HomeSpeakersState, Screen};
use crate::tui::ui;

/// Main event loop. Initialises the terminal, runs until quit, then restores.
pub fn run_event_loop(mut app: App) -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    let change_iter = app.system.iter();

    loop {
        // 1. Render (only when state changed)
        if app.dirty {
            terminal.draw(|frame| ui::render(frame, &app))?;
            app.dirty = false;
        }

        // 2. Poll for terminal events (non-blocking, 250ms timeout)
        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key) => {
                    handle_key(&mut app, key);
                    app.dirty = true;
                }
                Event::Resize(_, _) => {
                    app.dirty = true;
                }
                _ => {}
            }
        }

        // 3. Drain all pending SDK events (non-blocking)
        for _event in change_iter.try_iter() {
            // Milestone 7+ will handle change events here
            app.dirty = true;
        }

        if app.should_quit {
            break;
        }
    }

    ratatui::restore();
    Ok(())
}

// ---------------------------------------------------------------------------
// Key handling — global first, then screen-specific
// ---------------------------------------------------------------------------

fn handle_key(app: &mut App, key: KeyEvent) {
    // Ignore key releases — only handle presses
    if key.kind != event::KeyEventKind::Press {
        return;
    }

    // Global keys (work on every screen)
    match key.code {
        KeyCode::Char('q') if app.navigation.at_root() => {
            app.should_quit = true;
            return;
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
            return;
        }
        KeyCode::Esc => {
            if !app.navigation.pop() {
                app.should_quit = true;
            }
            return;
        }
        _ => {}
    }

    // Screen-specific keys
    match app.navigation.current().clone() {
        Screen::Home { tab, .. } => handle_home_key(app, key, &tab),
        Screen::GroupView { group_id, tab } => handle_group_key(app, key, &group_id, &tab),
        Screen::SpeakerDetail { .. } => handle_speaker_key(app, key),
    }
}

fn handle_home_key(app: &mut App, key: KeyEvent, tab: &HomeTab) {
    // Clear status message on any key press
    app.status_message = None;

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
        KeyCode::Tab | KeyCode::BackTab => {
            if let Screen::Home { ref mut tab, .. } = app.navigation.current_mut() {
                *tab = HomeTab::Speakers;
            }
        }
        KeyCode::Up => {
            if let Screen::Home {
                ref mut groups_state,
                ..
            } = app.navigation.current_mut()
            {
                if total > 0 && groups_state.selected_index >= cols {
                    groups_state.selected_index -= cols;
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
    let speaker_count = build_speaker_list_len(app);

    match key.code {
        KeyCode::Tab | KeyCode::BackTab => {
            if let Screen::Home { ref mut tab, .. } = app.navigation.current_mut() {
                *tab = HomeTab::Groups;
            }
        }
        KeyCode::Up => {
            if let Screen::Home {
                ref mut speakers_state,
                ..
            } = app.navigation.current_mut()
            {
                // Handle modal navigation
                if let Some(ref mut modal) = speakers_state.modal {
                    if modal.selected_index > 0 {
                        modal.selected_index -= 1;
                    }
                    return;
                }
                if speakers_state.selected_index > 0 {
                    speakers_state.selected_index -= 1;
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
        KeyCode::Esc => {
            // Close modal if open, otherwise handled by global Esc
            if let Screen::Home {
                ref mut speakers_state,
                ..
            } = app.navigation.current_mut()
            {
                if speakers_state.modal.is_some() {
                    speakers_state.modal = None;
                    return;
                }
            }
            // Fall through to global Esc handler by re-popping
            if !app.navigation.pop() {
                app.should_quit = true;
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

/// Count the total selectable speakers in the speaker list.
fn build_speaker_list_len(app: &App) -> usize {
    app.system.speakers().len()
}

fn handle_speakers_create_group(app: &mut App) {
    let speakers = app.system.speakers();
    let selected = match app.navigation.current() {
        Screen::Home {
            speakers_state, ..
        } => speakers_state.selected_index,
        _ => return,
    };
    if let Some(speaker) = speakers.get(selected) {
        match app.system.create_group(speaker, &[]) {
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
    let speakers = app.system.speakers();
    let selected = match app.navigation.current() {
        Screen::Home {
            speakers_state, ..
        } => speakers_state.selected_index,
        _ => return,
    };
    if let Some(speaker) = speakers.get(selected) {
        // Check if this speaker is a coordinator with members
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
    // If modal is open, confirm selection
    let has_modal = matches!(
        app.navigation.current(),
        Screen::Home {
            speakers_state: HomeSpeakersState {
                modal: Some(_),
                ..
            },
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
        speakers_state.modal = Some(crate::tui::app::ModalState {
            title: "Move to group".to_string(),
            items: non_standalone,
            selected_index: 0,
        });
    }
}

fn confirm_group_picker(app: &mut App) {
    // Extract modal selection info
    let (group_name, speaker_idx) = {
        let screen = app.navigation.current();
        match screen {
            Screen::Home {
                speakers_state, ..
            } => {
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
        let speakers = app.system.speakers();
        if let Some(speaker) = speakers.get(speaker_idx) {
            if let Some(group) = app.system.group(&group_name) {
                match group.add_speaker(speaker) {
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

    // Close modal
    if let Screen::Home {
        ref mut speakers_state,
        ..
    } = app.navigation.current_mut()
    {
        speakers_state.modal = None;
    }
}

fn handle_group_key(app: &mut App, key: KeyEvent, group_id: &sonos_sdk::GroupId, tab: &GroupTab) {
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

fn handle_speaker_key(_app: &mut App, _key: KeyEvent) {
    // Milestone 8+: speaker detail key handling
}
