//! TUI event loop, key handling, and SDK event processing.
//!
//! Uses `event::poll(50ms)` so the SDK event drain runs even without keyboard
//! input. The `dirty` flag skips rendering on idle poll timeouts.
//! Progress bars animate via client-side interpolation when any group is Playing.
//!
//! Watch lifecycle is managed by the hooks system: widgets call
//! `ctx.hooks.use_watch()` during render to subscribe to properties.
//! Handles are refreshed each frame (WatchHandle is a snapshot) and
//! cleaned up via mark-and-sweep when widgets stop rendering.

use std::time::{Duration, Instant};

use crate::tui::app::{App, Tab};
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
                Event::Resize(_, _) => {
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

        // 3b. Poll image loader for completed album art fetches.
        if app.image_loader.poll() {
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
// Key handling — global first, then tab-specific
// ---------------------------------------------------------------------------

fn handle_key(app: &mut App, key: KeyEvent) {
    if key.kind != event::KeyEventKind::Press {
        return;
    }

    // Settings dropdown intercepts Esc before global quit
    if app.navigation.tab == Tab::Settings
        && key.code == KeyCode::Esc
        && handlers::settings::is_dropdown_open(app)
    {
        handlers::home::handle_key(app, key);
        return;
    }

    match key.code {
        KeyCode::Char('q') => {
            app.should_quit = true;
            return;
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
            return;
        }
        KeyCode::Esc => {
            if app.navigation.speakers_state.pick_up.is_some() {
                app.navigation.speakers_state.pick_up = None;
                return;
            }
            app.should_quit = true;
            return;
        }
        _ => {}
    }

    handlers::home::handle_key(app, key);
}
