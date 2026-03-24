//! TUI event loop, key handling, and property watch lifecycle.
//!
//! Uses `event::poll(50ms)` so the SDK event drain runs even without keyboard
//! input. The `dirty` flag skips rendering on idle poll timeouts.
//! Progress bars animate via client-side interpolation when any group is Playing.

use std::time::{Duration, Instant};

use crate::tui::app::{App, HomeTab, ProgressState, Screen};
use crate::tui::handlers;
use crate::tui::ui;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

/// Main event loop. Initialises the terminal, runs until quit, then restores.
///
/// Terminal is always restored, even on error — prevents leaving the shell in raw mode.
pub fn run_event_loop(mut app: App) -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    let result = run_event_loop_inner(&mut app, &mut terminal);
    ratatui::restore();
    result
}

fn run_event_loop_inner(
    app: &mut App,
    terminal: &mut ratatui::DefaultTerminal,
) -> anyhow::Result<()> {
    let change_iter = app.system.iter();

    // Set up initial watches if starting on Groups tab
    setup_watches_if_groups_tab(app);

    // Get initial terminal width
    let initial_size = terminal.size()?;
    app.terminal_width = initial_size.width;

    // Throttle animation renders — 250ms is plenty for a progress bar
    let mut last_animation_render: Option<Instant> = None;

    loop {
        // 1. Render (only when state changed)
        if app.dirty {
            terminal.draw(|frame| ui::render(frame, app))?;
            app.dirty = false;
        }

        // 2. Poll for terminal events (non-blocking, 50ms timeout for animation)
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    let was_groups_tab = is_on_groups_tab(app);
                    handle_key(app, key);
                    let is_groups_tab = is_on_groups_tab(app);

                    // Handle watch lifecycle on tab/screen transitions
                    if was_groups_tab && !is_groups_tab {
                        teardown_watches(app);
                    } else if !was_groups_tab && is_groups_tab {
                        setup_watches(app);
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
            handle_change_event(app, &sdk_event);
            app.dirty = true;
        }

        // 4. Animation tick — throttle to ~4fps (250ms) for progress bar smoothness
        if has_active_animation(app) {
            let should_animate = last_animation_render
                .map(|t| t.elapsed() >= Duration::from_millis(250))
                .unwrap_or(true);
            if should_animate {
                app.dirty = true;
                last_animation_render = Some(Instant::now());
            }
        }

        if app.should_quit {
            break;
        }
    }

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
        let is_playing = playback.as_ref().map(|s| s.is_playing()).unwrap_or(false);
        let (pos_ms, dur_ms) = position
            .as_ref()
            .map(|p| (p.position_ms, p.duration_ms))
            .unwrap_or((0, 0));

        app.progress_states.insert(
            group.id.clone(),
            ProgressState::new(pos_ms, dur_ms, is_playing),
        );
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
        } => handlers::home::handle_home_key(app, key, &tab, tab_focused),
        Screen::GroupView { group_id, tab } => {
            handlers::group::handle_group_key(app, key, &group_id, &tab)
        }
        Screen::SpeakerDetail { .. } => handlers::group::handle_speaker_key(app, key),
    }
}
