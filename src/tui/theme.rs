//! TUI color theme system.
//!
//! Every widget references `app.theme` — no hardcoded colors. Styles are
//! pre-computed in the constructor so render functions pay zero allocation cost.

use ratatui::style::{Color, Modifier, Style};

/// Semantic styles used by the TUI. Grows as screens need new roles.
#[derive(Clone, Debug)]
pub struct Theme {
    /// Breadcrumb header bar.
    pub header: Style,
    /// Key legend bar at the bottom.
    pub legend: Style,
    /// Dimmed/inactive placeholder text.
    pub muted: Style,
}

impl Theme {
    /// Resolve a theme by name from config. Unknown names fall back to dark.
    pub fn from_name(_name: &str) -> Self {
        // Milestone 7+: match on "light", "neon", "sonos" when visual surface exists
        Self::dark()
    }

    pub fn dark() -> Self {
        Self {
            header: Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
            legend: Style::new().fg(Color::DarkGray),
            muted: Style::new().fg(Color::DarkGray),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_name_unknown_falls_back_to_dark() {
        let theme = Theme::from_name("nonexistent");
        let dark = Theme::dark();
        // Same styles — from_name("anything") returns dark for M6
        assert_eq!(theme.header, dark.header);
        assert_eq!(theme.legend, dark.legend);
        assert_eq!(theme.muted, dark.muted);
    }

    #[test]
    fn from_name_dark_returns_dark() {
        let theme = Theme::from_name("dark");
        let dark = Theme::dark();
        assert_eq!(theme.header, dark.header);
    }
}
