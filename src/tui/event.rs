//! TUI event loop and key handling.
//!
//! Uses `event::poll(250ms)` so the SDK event drain runs even without keyboard
//! input. The `dirty` flag skips rendering on idle poll timeouts.

use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use crate::tui::app::{App, GroupTab, HomeTab, Screen};
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
        Screen::Home { tab } => handle_home_key(app, key, &tab),
        Screen::GroupView { group_id, tab } => handle_group_key(app, key, &group_id, &tab),
        Screen::SpeakerDetail { .. } => handle_speaker_key(app, key),
    }
}

fn handle_home_key(app: &mut App, key: KeyEvent, tab: &HomeTab) {
    match key.code {
        KeyCode::Left | KeyCode::Right => {
            let new_tab = match tab {
                HomeTab::Groups => HomeTab::Speakers,
                HomeTab::Speakers => HomeTab::Groups,
            };
            *app.navigation.current_mut() = Screen::Home { tab: new_tab };
        }
        KeyCode::Enter => {
            // Milestone 7+: drill into selected group/speaker
        }
        _ => {}
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
