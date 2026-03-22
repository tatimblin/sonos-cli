//! TUI application state and navigation types.

// Fields and variants used by tests now and by M7+ screens later.
#![allow(dead_code)]

use crate::config::Config;
use crate::tui::theme::Theme;
use sonos_sdk::{GroupId, SonosSystem, SpeakerId};

/// Top-level TUI state. Owns the SDK handle and all UI state.
///
/// Screens read from `&App`; event handlers write to `&mut App`.
pub struct App {
    pub system: SonosSystem,
    pub navigation: Navigation,
    pub should_quit: bool,
    pub dirty: bool,
    pub config: Config,
    pub theme: Theme,
}

impl App {
    pub fn new(config: Config, theme: Theme) -> anyhow::Result<Self> {
        let system = SonosSystem::new()?;
        Ok(Self {
            system,
            navigation: Navigation::new(),
            should_quit: false,
            dirty: true, // first frame always renders
            config,
            theme,
        })
    }
}

/// Stack-based navigation. The bottom of the stack is always Home.
pub struct Navigation {
    pub stack: Vec<Screen>,
}

impl Default for Navigation {
    fn default() -> Self {
        Self::new()
    }
}

impl Navigation {
    pub fn new() -> Self {
        Self {
            stack: vec![Screen::Home {
                tab: HomeTab::default(),
            }],
        }
    }

    pub fn current(&self) -> &Screen {
        self.stack.last().expect("navigation stack is never empty")
    }

    pub fn current_mut(&mut self) -> &mut Screen {
        self.stack
            .last_mut()
            .expect("navigation stack is never empty")
    }

    pub fn push(&mut self, screen: Screen) {
        self.stack.push(screen);
    }

    /// Returns true if a screen was popped. Returns false if at root.
    pub fn pop(&mut self) -> bool {
        if self.stack.len() > 1 {
            self.stack.pop();
            true
        } else {
            false
        }
    }

    pub fn at_root(&self) -> bool {
        self.stack.len() == 1
    }
}

#[derive(Clone, Debug)]
pub enum Screen {
    Home { tab: HomeTab },
    GroupView { group_id: GroupId, tab: GroupTab },
    SpeakerDetail { speaker_id: SpeakerId },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum HomeTab {
    #[default]
    Groups,
    Speakers,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum GroupTab {
    #[default]
    NowPlaying,
    Speakers,
    Queue,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_starts_at_home() {
        let nav = Navigation::new();
        assert!(nav.at_root());
        assert!(matches!(nav.current(), Screen::Home { .. }));
    }

    #[test]
    fn push_adds_to_stack() {
        let mut nav = Navigation::new();
        nav.push(Screen::SpeakerDetail {
            speaker_id: SpeakerId::new("RINCON_TEST"),
        });
        assert!(!nav.at_root());
        assert!(matches!(nav.current(), Screen::SpeakerDetail { .. }));
    }

    #[test]
    fn pop_returns_to_previous() {
        let mut nav = Navigation::new();
        nav.push(Screen::SpeakerDetail {
            speaker_id: SpeakerId::new("RINCON_TEST"),
        });
        assert!(nav.pop());
        assert!(nav.at_root());
        assert!(matches!(nav.current(), Screen::Home { .. }));
    }

    #[test]
    fn pop_at_root_returns_false() {
        let mut nav = Navigation::new();
        assert!(!nav.pop());
        assert!(nav.at_root());
    }

    #[test]
    fn current_mut_allows_tab_switch() {
        let mut nav = Navigation::new();
        *nav.current_mut() = Screen::Home {
            tab: HomeTab::Speakers,
        };
        match nav.current() {
            Screen::Home { tab } => assert_eq!(*tab, HomeTab::Speakers),
            _ => panic!("expected Home screen"),
        }
    }
}
