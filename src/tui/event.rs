//! TUI event loop, key handling, and SDK event processing.
//!
//! Uses `event::poll(50ms)` so the SDK event drain runs even without keyboard
//! input. The `dirty` flag skips rendering on idle poll timeouts.
//! Progress bars animate via client-side interpolation when any group is Playing.
//!
//! Watch lifecycle is managed by the hooks system: widgets call
//! `ctx.hooks.use_watch()` during render to subscribe to properties.
//! Handles persist across frames (subscription tokens); `prop.get()`
//! reads the live cache. Mark-and-sweep cleans up on screen transitions.

use std::time::{Duration, Instant};

use crate::tui::app::{App, HomeTab, Screen};
use crate::tui::handlers;
use crate::tui::hooks::{Hooks, RenderContext};
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
    tracing::debug!("TUI event loop started, got change_iter");

    // Get initial terminal width
    let initial_size = terminal.size()?;
    app.terminal_width = initial_size.width;

    // Hooks system — owns widget state, watch handles, and animation registrations
    let mut hooks = Hooks::new();

    // Throttle animation renders — 250ms is plenty for a progress bar
    let mut last_animation_render: Option<Instant> = None;
    let mut frame_count: u64 = 0;

    loop {
        // 1. Render (only when state changed)
        //    Hooks manage watch subscriptions via persistent handles.
        //    Mark-and-sweep evicts state for widgets that stopped rendering.
        if app.dirty {
            frame_count += 1;
            if frame_count <= 3 {
                tracing::debug!("TUI render frame {frame_count}");
            }
            hooks.begin_frame();
            terminal.draw(|frame| {
                let mut ctx = RenderContext {
                    app,
                    hooks: &mut hooks,
                };
                ui::render(frame, &mut ctx);
            })?;
            hooks.end_frame();
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

        // 3. Drain all pending SDK events — dirty-marking only.
        //    State updates happen in the render phase via use_watch + use_state.
        for _sdk_event in change_iter.try_iter() {
            app.dirty = true;
        }

        // 4. Animation tick — throttle to ~4fps (250ms) for progress bar smoothness
        if hooks.has_active_animations() {
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
