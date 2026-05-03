//! TUI application state and navigation types.

use std::cell::RefCell;
use std::time::{Duration, Instant};

use ratatui_image::picker::Picker;

use crate::config::Config;
use crate::tui::image_loader::ImageLoader;
use crate::tui::theme::Theme;
use crate::tui::types::PickUpState;
use sonos_sdk::SonosSystem;

const TOAST_DURATION: Duration = Duration::from_secs(3);

/// A temporary notification that auto-dismisses after `TOAST_DURATION`.
pub struct Toast {
    pub message: String,
    pub is_error: bool,
    created_at: Instant,
}

impl Toast {
    pub fn info(message: String) -> Self {
        Self {
            message,
            is_error: false,
            created_at: Instant::now(),
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            message,
            is_error: true,
            created_at: Instant::now(),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= TOAST_DURATION
    }
}

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
    /// Toast notification shown in the header. Auto-expires after 3 seconds.
    pub toast: Option<Toast>,
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
            dirty: true,
            config,
            theme,
            toast: None,
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
    pub settings_state: SettingsScreenState,
}

/// UI state for the Settings screen dropdown form.
#[derive(Clone, Debug, Default)]
pub struct SettingsScreenState {
    /// Which row (0 = theme, 1 = default group, 2 = album art) is selected.
    pub selected_row: usize,
    /// Whether the dropdown for the selected row is open.
    pub dropdown_open: bool,
    /// Index within the open dropdown's option list.
    pub dropdown_index: usize,
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
