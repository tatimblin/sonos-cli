//! TUI application state and navigation types.

use std::cell::RefCell;

use ratatui_image::picker::Picker;

use crate::config::Config;
use crate::tui::image_loader::ImageLoader;
use crate::tui::theme::Theme;
use crate::tui::widgets::speaker_list::PickUpState;
use sonos_sdk::{GroupId, SonosSystem, SpeakerId};

/// Top-level TUI state. Owns the SDK handle and all UI state.
///
/// Screens read from `&App`; event handlers write to `&mut App`.
pub struct App {
    pub system: SonosSystem,
    pub navigation: Navigation,
    pub should_quit: bool,
    pub dirty: bool,
    #[allow(dead_code)] // used in future milestones
    pub config: Config,
    pub theme: Theme,
    /// Inline status message (e.g. errors from speaker actions). Cleared on next key press.
    pub status_message: Option<String>,
    /// Terminal width cached from last render/resize, used for grid navigation.
    pub terminal_width: u16,
    /// Terminal image protocol picker, detected before entering raw mode.
    /// `None` when album art is disabled or terminal detection failed.
    /// `RefCell` because `new_resize_protocol()` requires `&mut Picker`.
    pub picker: RefCell<Option<Picker>>,
    /// Background image fetcher and cache for album art.
    pub image_loader: ImageLoader,
}

impl App {
    pub fn new(config: Config, theme: Theme, picker: Option<Picker>) -> anyhow::Result<Self> {
        let system = SonosSystem::new()?;
        Ok(Self {
            system,
            navigation: Navigation::new(),
            should_quit: false,
            dirty: true, // first frame always renders
            config,
            theme,
            status_message: None,
            terminal_width: 80, // updated on first render/resize
            picker: RefCell::new(picker),
            image_loader: ImageLoader::new(),
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
                tab_focused: false,
                groups_state: HomeGroupsState::default(),
                speakers_state: SpeakerListScreenState::default(),
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
    Home {
        tab: HomeTab,
        tab_focused: bool,
        groups_state: HomeGroupsState,
        speakers_state: SpeakerListScreenState,
    },
    GroupView {
        group_id: GroupId,
        tab: GroupTab,
        tab_focused: bool,
        speakers_state: SpeakerListScreenState,
    },
    #[allow(dead_code)] // used in future milestones
    SpeakerDetail { speaker_id: SpeakerId },
}

/// UI state for the Home > Groups tab.
#[derive(Clone, Debug, Default)]
pub struct HomeGroupsState {
    pub selected_index: usize,
}

/// Shared UI state for any speaker list (Home > Speakers or GroupView > Speakers).
#[derive(Clone, Debug, Default)]
pub struct SpeakerListScreenState {
    pub selected_index: usize,
    pub pick_up: Option<PickUpState>,
}

impl Screen {
    pub fn speakers_state(&self) -> Option<&SpeakerListScreenState> {
        match self {
            Screen::Home { speakers_state, .. } => Some(speakers_state),
            Screen::GroupView { speakers_state, .. } => Some(speakers_state),
            _ => None,
        }
    }

    pub fn speakers_state_mut(&mut self) -> Option<&mut SpeakerListScreenState> {
        match self {
            Screen::Home { speakers_state, .. } => Some(speakers_state),
            Screen::GroupView { speakers_state, .. } => Some(speakers_state),
            _ => None,
        }
    }
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
        if let Screen::Home { ref mut tab, .. } = nav.current_mut() {
            *tab = HomeTab::Speakers;
        }
        match nav.current() {
            Screen::Home { tab, .. } => assert_eq!(*tab, HomeTab::Speakers),
            _ => panic!("expected Home screen"),
        }
    }
}
