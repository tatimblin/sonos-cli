//! TUI event loop, key handling, and SDK event processing.
//!
//! Uses `event::poll(50ms)` so the SDK event drain runs even without keyboard
//! input. The `dirty` flag skips rendering on idle poll timeouts.
//! Progress bars animate via client-side interpolation when any group is Playing.
//!
//! Watch lifecycle is managed by the hooks system: widgets call
//! `ctx.hooks.use_watch()` during render to subscribe to properties.
//! A handle is acquired once and held across frames (`WatchHandle` reads the
//! store live), then cleaned up via mark-and-sweep when widgets stop rendering.

use std::time::{Duration, Instant};

use crate::tui::app::App;
use crate::tui::handlers;
use crate::tui::hooks::{Hooks, RenderContext};
use crate::tui::keymap::{self, GlobalCommand, Mode, GLOBAL_KEYS};
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
        // 0. Expire stale toasts
        if app.toast.as_ref().is_some_and(|t| t.is_expired()) {
            app.toast = None;
            app.dirty = true;
        }

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
// Key handling — dispatched from the binding tables in `keymap.rs`
// ---------------------------------------------------------------------------

fn handle_key(app: &mut App, key: KeyEvent) {
    if key.kind != event::KeyEventKind::Press {
        return;
    }

    // Ctrl-C always quits — an emergency exit, even mid pick-up.
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.should_quit = true;
        return;
    }

    // The mode's own table wins. Unbound keys fall through to the global
    // table, unless the mode is exclusive (pick-up) and swallows them.
    let mode = Mode::of(app);
    if !mode.binds(key.code) {
        if !mode.is_exclusive()
            && keymap::lookup(GLOBAL_KEYS, key.code) == Some(GlobalCommand::Quit)
        {
            app.should_quit = true;
        }
        return;
    }

    handlers::home::handle_key(app, key);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::tui::app::Tab;
    use crate::tui::theme::Theme;
    use sonos_sdk::SonosSystem;

    fn app() -> App {
        let system = SonosSystem::with_groups(&["Kitchen", "Office"]);
        App::with_system(system, Config::default(), Theme::default_theme(), None)
    }

    fn press(app: &mut App, code: KeyCode) {
        handle_key(app, KeyEvent::from(code));
    }

    /// Pick up the first group's speaker.
    fn pick_up(app: &mut App) {
        press(app, KeyCode::Down);
        press(app, KeyCode::Char(' '));
        assert!(app.navigation.speakers_state.pick_up.is_some());
    }

    #[test]
    fn q_and_esc_quit_from_the_speaker_list() {
        for code in [KeyCode::Char('q'), KeyCode::Esc] {
            let mut app = app();
            press(&mut app, code);
            assert!(app.should_quit, "{code:?}");
        }
    }

    #[test]
    fn pickup_mode_only_accepts_its_keys() {
        let mut app = app();
        pick_up(&mut app);

        // Quit, playback and volume keys are swallowed. (Volume and playback
        // would otherwise hit the network and leave a toast behind.)
        app.toast = None;
        for code in [KeyCode::Char('q'), KeyCode::Char('p'), KeyCode::Left] {
            press(&mut app, code);
            assert!(!app.should_quit, "{code:?} must not quit mid pick-up");
            assert!(app.navigation.speakers_state.pick_up.is_some(), "{code:?}");
            assert!(app.toast.is_none(), "{code:?} must not reach the handler");
        }

        press(&mut app, KeyCode::Esc);
        assert!(app.navigation.speakers_state.pick_up.is_none());
        assert!(!app.should_quit, "Esc cancels pick-up rather than quitting");
    }

    #[test]
    fn ctrl_c_quits_mid_pickup() {
        let mut app = app();
        pick_up(&mut app);
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        );
        assert!(app.should_quit);
    }

    #[test]
    fn esc_closes_the_settings_dropdown_but_q_still_quits() {
        let mut app = app();
        app.navigation.tab = Tab::Settings;
        press(&mut app, KeyCode::Enter);
        assert!(app.navigation.settings_state.dropdown_open);

        press(&mut app, KeyCode::Esc);
        assert!(!app.navigation.settings_state.dropdown_open);
        assert!(!app.should_quit);

        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('q'));
        assert!(app.should_quit);
    }
}
