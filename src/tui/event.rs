//! TUI event loop, key handling, and property watch lifecycle.
//!
//! Uses `event::poll(50ms)` so the SDK event drain runs even without keyboard
//! input. The `dirty` flag skips rendering on idle poll timeouts.
//! Progress bars animate via client-side interpolation when any group is Playing.

use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crate::tui::app::{App, GroupTab, HomeTab, HomeSpeakersState, ProgressState, Screen};
use crate::tui::ui;

/// Main event loop. Initialises the terminal, runs until quit, then restores.
pub fn run_event_loop(mut app: App) -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    let change_iter = app.system.iter();

    // Set up initial watches if starting on Groups tab
    setup_watches_if_groups_tab(&mut app);

    loop {
        // 1. Render (only when state changed)
        if app.dirty {
            let size = terminal.size()?;
            app.terminal_width = size.width;
            terminal.draw(|frame| ui::render(frame, &app))?;
            app.dirty = false;
        }

        // 2. Poll for terminal events (non-blocking, 50ms timeout for animation)
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    let was_groups_tab = is_on_groups_tab(&app);
                    handle_key(&mut app, key);
                    let is_groups_tab = is_on_groups_tab(&app);

                    // Handle watch lifecycle on tab/screen transitions
                    if was_groups_tab && !is_groups_tab {
                        teardown_watches(&mut app);
                    } else if !was_groups_tab && is_groups_tab {
                        setup_watches(&mut app);
                    }

                    app.dirty = true;
                }
                Event::Resize(w, _) => {
                    app.terminal_width = w;
                    app.dirty = true;
                }
                _ => {}
            }
        }

        // 3. Drain all pending SDK events (non-blocking)
        for sdk_event in change_iter.try_iter() {
            handle_change_event(&mut app, &sdk_event);
            app.dirty = true;
        }

        // 4. Animation tick — mark dirty if any group is currently playing
        if has_active_animation(&app) {
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
// Property watch lifecycle
// ---------------------------------------------------------------------------

fn is_on_groups_tab(app: &App) -> bool {
    matches!(
        app.navigation.current(),
        Screen::Home {
            tab: HomeTab::Groups,
            ..
        }
    )
}

/// Set up watches if the app is currently on the Groups tab.
fn setup_watches_if_groups_tab(app: &mut App) {
    if is_on_groups_tab(app) {
        setup_watches(app);
    }
}

/// Watch playback properties for all group coordinators.
fn setup_watches(app: &mut App) {
    let groups = app.system.groups();

    for group in &groups {
        let Some(coordinator) = group.coordinator() else {
            continue;
        };

        // Watch speaker properties on the coordinator individually
        // (different handle types prevent abstraction):

        // current_track
        if let Ok(status) = coordinator.current_track.watch() {
            app.watch_registry
                .insert((coordinator.id.clone(), "current_track"));
            if status.current.is_none() {
                let _ = coordinator.current_track.fetch();
            }
        }

        // playback_state
        if let Ok(status) = coordinator.playback_state.watch() {
            app.watch_registry
                .insert((coordinator.id.clone(), "playback_state"));
            if status.current.is_none() {
                let _ = coordinator.playback_state.fetch();
            }
        }

        // position
        if let Ok(status) = coordinator.position.watch() {
            app.watch_registry
                .insert((coordinator.id.clone(), "position"));
            if status.current.is_none() {
                let _ = coordinator.position.fetch();
            }
        }

        // group volume
        if let Ok(status) = group.volume.watch() {
            app.watch_registry
                .insert((coordinator.id.clone(), "group_volume"));
            if status.current.is_none() {
                let _ = group.volume.fetch();
            }
        }

        // Initialize progress state
        let position = coordinator.position.get();
        let playback = coordinator.playback_state.get();
        let is_playing = playback
            .as_ref()
            .map(|s| s.is_playing())
            .unwrap_or(false);
        let (pos_ms, dur_ms) = position
            .as_ref()
            .map(|p| (p.position_ms, p.duration_ms))
            .unwrap_or((0, 0));

        app.progress_states
            .insert(group.id.clone(), ProgressState::new(pos_ms, dur_ms, is_playing));

    }
}

/// Unwatch all currently watched properties.
fn teardown_watches(app: &mut App) {
    for (speaker_id, property_key) in app.watch_registry.drain() {
        if let Some(speaker) = app.system.speaker_by_id(&speaker_id) {
            match property_key {
                "current_track" => speaker.current_track.unwatch(),
                "playback_state" => speaker.playback_state.unwatch(),
                "position" => speaker.position.unwatch(),
                "group_volume" => {
                    if let Some(group) = app.system.group_for_speaker(&speaker_id) {
                        group.volume.unwatch();
                    }
                }
                _ => {}
            }
        }
    }

    app.progress_states.clear();
}

// ---------------------------------------------------------------------------
// Change event handling
// ---------------------------------------------------------------------------

fn handle_change_event(app: &mut App, event: &sonos_sdk::ChangeEvent) {
    match event.property_key {
        "position" => {
            // Find which group this coordinator belongs to, update progress state
            if let Some(speaker) = app.system.speaker_by_id(&event.speaker_id) {
                if let Some(pos) = speaker.position.get() {
                    if let Some(group) = speaker.group() {
                        if let Some(ps) = app.progress_states.get_mut(&group.id) {
                            ps.last_position_ms = pos.position_ms;
                            ps.last_duration_ms = pos.duration_ms;
                            ps.wall_clock_at_last_update = Instant::now();
                        }
                    }
                }
            }
        }
        "playback_state" => {
            if let Some(speaker) = app.system.speaker_by_id(&event.speaker_id) {
                if let Some(state) = speaker.playback_state.get() {
                    if let Some(group) = speaker.group() {
                        if let Some(ps) = app.progress_states.get_mut(&group.id) {
                            let now_playing = state.is_playing();
                            if ps.is_playing && !now_playing {
                                // Freeze at current interpolated position
                                ps.last_position_ms = ps.interpolated_position_ms();
                                ps.wall_clock_at_last_update = Instant::now();
                            }
                            ps.is_playing = now_playing;
                            if now_playing {
                                ps.wall_clock_at_last_update = Instant::now();
                            }
                        }
                    }
                }
            }
        }
        "group_membership" => {
            // Topology changed — teardown and re-establish watches
            if is_on_groups_tab(app) {
                teardown_watches(app);
                setup_watches(app);
            }
        }
        _ => {
            // Other events (current_track, group_volume, etc.) — just mark dirty
            // The render functions read fresh values from SDK cache
        }
    }
}

/// Check if any group has an active playing animation.
fn has_active_animation(app: &App) -> bool {
    app.progress_states.values().any(|ps| ps.is_playing)
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
            // Check for modal first in speakers tab
            if let Screen::Home {
                tab: HomeTab::Speakers,
                ref speakers_state,
                ..
            } = app.navigation.current()
            {
                if speakers_state.modal.is_some() {
                    if let Screen::Home {
                        ref mut speakers_state,
                        ..
                    } = app.navigation.current_mut()
                    {
                        speakers_state.modal = None;
                    }
                    return;
                }
            }

            if !app.navigation.pop() {
                app.should_quit = true;
            }
            return;
        }
        _ => {}
    }

    // Screen-specific keys
    match app.navigation.current().clone() {
        Screen::Home {
            tab, tab_focused, ..
        } => handle_home_key(app, key, &tab, tab_focused),
        Screen::GroupView { group_id, tab } => handle_group_key(app, key, &group_id, &tab),
        Screen::SpeakerDetail { .. } => handle_speaker_key(app, key),
    }
}

fn handle_home_key(app: &mut App, key: KeyEvent, tab: &HomeTab, tab_focused: bool) {
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
                    ref mut tab_focused, ..
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
    let speaker_count = app.system.speakers().len();

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
    // Milestone 9: speaker detail key handling
}
