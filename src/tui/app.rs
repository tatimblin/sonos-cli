//! TUI application state and navigation types.

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Instant;

use crate::config::Config;
use crate::tui::theme::Theme;
use sonos_sdk::property::{GroupPropertyHandle, PropertyHandle};
use sonos_sdk::{GroupId, SonosSystem, SpeakerId};
use sonos_state::property::SonosProperty;

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
    /// Per-group progress interpolation state for smooth animation.
    pub progress_states: HashMap<GroupId, ProgressState>,
    /// Current frame's watch handles. Widgets push into this during render.
    /// Cleared before each render cycle; widgets repopulate via app.watch().
    watch_handles: RefCell<Vec<Box<dyn Any>>>,
    /// Inline status message (e.g. errors from speaker actions). Cleared on next key press.
    pub status_message: Option<String>,
    /// Terminal width cached from last render/resize, used for grid navigation.
    pub terminal_width: u16,
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
            progress_states: HashMap::new(),
            watch_handles: RefCell::new(Vec::new()),
            status_message: None,
            terminal_width: 80, // updated on first render/resize
        })
    }

    /// Watch a speaker property — returns current value, keeps subscription alive.
    ///
    /// Call this in widget rendering code wherever you need a property value.
    /// The returned WatchHandle is stored internally and kept alive until the
    /// next render cycle.
    ///
    /// Returns `None` on cold cache (first watch before any event arrives).
    /// The SDK subscription delivers data within ~50-200ms, triggering a
    /// re-render with the populated value. Widgets already handle `None`.
    pub fn watch<P>(&self, prop: &PropertyHandle<P>) -> Option<P>
    where
        P: SonosProperty + Clone + 'static,
    {
        tracing::trace!("App::watch called for speaker property");
        match prop.watch() {
            Ok(wh) => {
                let val = wh.value().cloned();
                self.watch_handles.borrow_mut().push(Box::new(wh));
                val
            }
            Err(e) => {
                tracing::warn!("App::watch failed: {e}, falling back to get()");
                prop.get()
            }
        }
    }

    /// Watch a group property — returns current value, keeps subscription alive.
    pub fn watch_group<P>(&self, prop: &GroupPropertyHandle<P>) -> Option<P>
    where
        P: SonosProperty + Clone + 'static,
    {
        tracing::trace!("App::watch_group called for group property");
        match prop.watch() {
            Ok(wh) => {
                let val = wh.value().cloned();
                self.watch_handles.borrow_mut().push(Box::new(wh));
                val
            }
            Err(e) => {
                tracing::warn!("App::watch_group failed: {e}, falling back to get()");
                prop.get()
            }
        }
    }

    /// Drop all watch handles — called by event loop before render.
    /// Starts grace periods; widgets re-acquire during draw(), cancelling them.
    pub fn clear_watch_handles(&mut self) {
        self.watch_handles.get_mut().clear();
    }
}

/// Client-side progress interpolation state for a single group.
#[derive(Clone, Debug)]
pub struct ProgressState {
    pub last_position_ms: u64,
    pub last_duration_ms: u64,
    pub wall_clock_at_last_update: Instant,
    pub is_playing: bool,
}

impl ProgressState {
    pub fn new(position_ms: u64, duration_ms: u64, is_playing: bool) -> Self {
        Self {
            last_position_ms: position_ms,
            last_duration_ms: duration_ms,
            wall_clock_at_last_update: Instant::now(),
            is_playing,
        }
    }

    /// Compute interpolated position in milliseconds.
    ///
    /// Caps interpolation at 10s ahead to limit drift from system sleep/stalls.
    pub fn interpolated_position_ms(&self) -> u64 {
        if !self.is_playing {
            return self.last_position_ms;
        }
        let elapsed = self.wall_clock_at_last_update.elapsed().as_millis() as u64;
        let capped_elapsed = elapsed.min(10_000);
        (self.last_position_ms + capped_elapsed).min(self.last_duration_ms)
    }

    /// Compute interpolated progress ratio (0.0–1.0).
    #[allow(dead_code)] // used in future milestones
    pub fn interpolated_progress(&self) -> f64 {
        if self.last_duration_ms == 0 {
            return 0.0;
        }
        self.interpolated_position_ms() as f64 / self.last_duration_ms as f64
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
                speakers_state: HomeSpeakersState::default(),
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
        speakers_state: HomeSpeakersState,
    },
    GroupView {
        group_id: GroupId,
        tab: GroupTab,
    },
    #[allow(dead_code)] // used in future milestones
    SpeakerDetail {
        speaker_id: SpeakerId,
    },
}

/// UI state for the Home > Groups tab.
#[derive(Clone, Debug, Default)]
pub struct HomeGroupsState {
    pub selected_index: usize,
}

/// UI state for the Home > Speakers tab.
#[derive(Clone, Debug, Default)]
pub struct HomeSpeakersState {
    pub selected_index: usize,
    /// Active modal (e.g. group picker for move-to-group).
    pub modal: Option<ModalState>,
}

/// State for a modal overlay (e.g. group picker).
#[derive(Clone, Debug)]
pub struct ModalState {
    pub title: String,
    pub items: Vec<String>,
    pub selected_index: usize,
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
