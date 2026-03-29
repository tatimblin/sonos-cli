//! TUI event loop, key handling, and SDK event processing.
//!
//! Uses `event::poll(50ms)` so the SDK event drain runs even without keyboard
//! input. The `dirty` flag skips rendering on idle poll timeouts.
//! Progress bars animate via client-side interpolation when any group is Playing.
//!
//! Watch lifecycle is declarative: widgets call `app.watch()` during render to
//! subscribe to properties. The event loop clears handles before each draw,
//! starting grace periods. `draw()` immediately re-acquires handles via
//! `app.watch()`, cancelling the grace periods.

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

    // Get initial terminal width
    let initial_size = terminal.size()?;
    app.terminal_width = initial_size.width;

    // Throttle animation renders — 250ms is plenty for a progress bar
    let mut last_animation_render: Option<Instant> = None;

    loop {
        // 1. Render (only when state changed)
        //    Clear old handles → grace periods start.
        //    draw() → widgets call app.watch() → grace periods cancelled.
        if app.dirty {
            app.clear_watch_handles();
            terminal.draw(|frame| ui::render(frame, app))?;
            app.dirty = false;
        }

        // 2. Poll for terminal events (non-blocking, 50ms timeout for animation)
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    handle_key(app, key);
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
// Change event handling
// ---------------------------------------------------------------------------

fn handle_change_event(app: &mut App, event: &sonos_sdk::ChangeEvent) {
    match event.property_key {
        "position" => {
            if let Some(speaker) = app.system.speaker_by_id(&event.speaker_id) {
                if let Some(pos) = speaker.position.get() {
                    if let Some(group) = speaker.group() {
                        let ps = app
                            .progress_states
                            .entry(group.id.clone())
                            .or_insert_with(|| ProgressState::new(0, 0, false));
                        ps.last_position_ms = pos.position_ms;
                        ps.last_duration_ms = pos.duration_ms;
                        ps.wall_clock_at_last_update = Instant::now();
                    }
                }
            }
        }
        "playback_state" => {
            if let Some(speaker) = app.system.speaker_by_id(&event.speaker_id) {
                if let Some(state) = speaker.playback_state.get() {
                    if let Some(group) = speaker.group() {
                        let ps = app
                            .progress_states
                            .entry(group.id.clone())
                            .or_insert_with(|| ProgressState::new(0, 0, false));
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
        "group_membership" => {
            // Topology changed — next render will re-watch new groups automatically.
            app.progress_states.clear();
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
