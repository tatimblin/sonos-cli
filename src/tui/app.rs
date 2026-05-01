//! TUI application state and navigation types.

use std::cell::RefCell;

use ratatui_image::picker::Picker;

use crate::config::Config;
use crate::tui::image_loader::ImageLoader;
use crate::tui::theme::Theme;
use crate::tui::widgets::speaker_list::PickUpState;
use sonos_sdk::SonosSystem;

/// Top-level TUI state. Owns the SDK handle and all UI state.
///
/// Screens read from `&App`; event handlers write to `&mut App`.
pub struct App {
    pub system: SonosSystem,
    pub navigation: Navigation,
    pub should_quit: bool,
    pub dirty: bool,
    #[allow(dead_code)]
    pub config: Config,
    pub theme: Theme,
    /// Inline status message (e.g. errors from speaker actions). Cleared on next key press.
    pub status_message: Option<String>,
    /// Terminal image protocol picker, detected before entering raw mode.
    /// `None` when album art is disabled or terminal detection failed.
    /// `RefCell` because `new_resize_protocol()` requires `&mut Picker`.
    #[allow(dead_code)] // retained for bottom player bar
    pub picker: RefCell<Option<Picker>>,
    /// Background image fetcher and cache for album art.
    #[allow(dead_code)] // retained for bottom player bar
    pub image_loader: ImageLoader,
}

impl App {
    pub fn new(config: Config, theme: Theme, picker: Option<Picker>) -> anyhow::Result<Self> {
        let system = SonosSystem::new()?;
        Ok(Self {
            system,
            navigation: Navigation::new(),
            should_quit: false,
            dirty: true,
            config,
            theme,
            status_message: None,
            picker: RefCell::new(picker),
            image_loader: ImageLoader::new(),
        })
    }
}

/// Simple two-tab navigation. No stack, no drill-in.
#[derive(Default)]
pub struct Navigation {
    pub tab: Tab,
    pub tab_focused: bool,
    pub speakers_state: SpeakerListScreenState,
}

impl Navigation {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Shared UI state for the speaker list.
#[derive(Clone, Debug, Default)]
pub struct SpeakerListScreenState {
    pub selected_index: usize,
    pub pick_up: Option<PickUpState>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Speakers,
    Settings,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_starts_on_speakers() {
        let nav = Navigation::new();
        assert_eq!(nav.tab, Tab::Speakers);
        assert!(!nav.tab_focused);
    }

    #[test]
    fn tab_toggles() {
        let mut nav = Navigation::new();
        nav.tab = Tab::Settings;
        assert_eq!(nav.tab, Tab::Settings);
        nav.tab = Tab::Speakers;
        assert_eq!(nav.tab, Tab::Speakers);
    }
}
