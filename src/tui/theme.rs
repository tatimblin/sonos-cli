//! TUI color theme system.
//!
//! Every widget references `app.theme` — no hardcoded colors. Styles are
//! pre-computed in the constructor so render functions pay zero allocation cost.

use ratatui::style::{Color, Modifier, Style};

/// Semantic styles used by the TUI. Grows as screens need new roles.
#[derive(Clone, Debug)]
pub struct Theme {
    // Layout chrome
    pub header: Style,
    pub legend: Style,
    pub muted: Style,

    // Group cards
    pub card_border: Style,
    pub card_border_selected: Style,
    pub card_title: Style,
    pub track_info: Style,

    // Playback state icons
    pub playing_icon: Style,
    pub paused_icon: Style,
    pub stopped_icon: Style,

    // Volume bar
    pub volume_filled: Style,
    pub volume_empty: Style,

    // Progress bar
    pub progress_filled: Style,
    pub progress_empty: Style,
    pub progress_cursor: Style,
    pub progress_time: Style,

    // Mini-player (unused in current layout, kept for Group View)
    #[allow(dead_code)]
    pub mini_player_border: Style,
    #[allow(dead_code)]
    pub mini_player_title: Style,

    // Speakers tab
    pub group_header: Style,
    pub speaker_cursor: Style,
    pub speaker_name: Style,

    // Modal
    pub modal_border: Style,
    pub modal_title: Style,
    pub modal_selected: Style,

    // General
    pub accent: Style,
    pub error: Style,
}

impl Theme {
    /// Resolve a theme by name from config. Unknown names fall back to dark.
    pub fn from_name(name: &str) -> Self {
        match name {
            "light" => Self::light(),
            "neon" => Self::neon(),
            "sonos" => Self::sonos(),
            _ => Self::dark(),
        }
    }

    pub fn dark() -> Self {
        Self {
            header: Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
            legend: Style::new().fg(Color::DarkGray),
            muted: Style::new().fg(Color::DarkGray),

            card_border: Style::new().fg(Color::DarkGray),
            card_border_selected: Style::new()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            card_title: Style::new()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            track_info: Style::new().fg(Color::Gray),

            playing_icon: Style::new().fg(Color::Green),
            paused_icon: Style::new().fg(Color::Yellow),
            stopped_icon: Style::new().fg(Color::DarkGray),

            volume_filled: Style::new().fg(Color::Cyan),
            volume_empty: Style::new().fg(Color::DarkGray),

            progress_filled: Style::new().fg(Color::White),
            progress_empty: Style::new().fg(Color::DarkGray),
            progress_cursor: Style::new().fg(Color::White),
            progress_time: Style::new().fg(Color::DarkGray),

            mini_player_border: Style::new().fg(Color::DarkGray),
            mini_player_title: Style::new()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),

            group_header: Style::new()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            speaker_cursor: Style::new().fg(Color::Cyan),
            speaker_name: Style::new().fg(Color::Gray),

            modal_border: Style::new().fg(Color::White),
            modal_title: Style::new()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            modal_selected: Style::new()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD),

            accent: Style::new().fg(Color::Cyan),
            error: Style::new().fg(Color::Red),
        }
    }

    pub fn light() -> Self {
        Self {
            header: Style::new().fg(Color::Black).add_modifier(Modifier::BOLD),
            legend: Style::new().fg(Color::Gray),
            muted: Style::new().fg(Color::Gray),

            card_border: Style::new().fg(Color::Gray),
            card_border_selected: Style::new()
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
            card_title: Style::new()
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
            track_info: Style::new().fg(Color::DarkGray),

            playing_icon: Style::new().fg(Color::Green),
            paused_icon: Style::new().fg(Color::Yellow),
            stopped_icon: Style::new().fg(Color::Gray),

            volume_filled: Style::new().fg(Color::Blue),
            volume_empty: Style::new().fg(Color::Gray),

            progress_filled: Style::new().fg(Color::Black),
            progress_empty: Style::new().fg(Color::Gray),
            progress_cursor: Style::new().fg(Color::Black),
            progress_time: Style::new().fg(Color::Gray),

            mini_player_border: Style::new().fg(Color::Gray),
            mini_player_title: Style::new()
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),

            group_header: Style::new()
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            speaker_cursor: Style::new().fg(Color::Blue),
            speaker_name: Style::new().fg(Color::DarkGray),

            modal_border: Style::new().fg(Color::Black),
            modal_title: Style::new()
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
            modal_selected: Style::new()
                .fg(Color::White)
                .bg(Color::Black)
                .add_modifier(Modifier::BOLD),

            accent: Style::new().fg(Color::Blue),
            error: Style::new().fg(Color::Red),
        }
    }

    pub fn neon() -> Self {
        Self {
            header: Style::new()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
            legend: Style::new().fg(Color::DarkGray),
            muted: Style::new().fg(Color::DarkGray),

            card_border: Style::new().fg(Color::DarkGray),
            card_border_selected: Style::new()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
            card_title: Style::new()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            track_info: Style::new().fg(Color::LightMagenta),

            playing_icon: Style::new().fg(Color::Green),
            paused_icon: Style::new().fg(Color::Yellow),
            stopped_icon: Style::new().fg(Color::DarkGray),

            volume_filled: Style::new().fg(Color::Magenta),
            volume_empty: Style::new().fg(Color::DarkGray),

            progress_filled: Style::new().fg(Color::Cyan),
            progress_empty: Style::new().fg(Color::DarkGray),
            progress_cursor: Style::new().fg(Color::Cyan),
            progress_time: Style::new().fg(Color::DarkGray),

            mini_player_border: Style::new().fg(Color::Magenta),
            mini_player_title: Style::new()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),

            group_header: Style::new()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            speaker_cursor: Style::new().fg(Color::Cyan),
            speaker_name: Style::new().fg(Color::LightMagenta),

            modal_border: Style::new().fg(Color::Magenta),
            modal_title: Style::new()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            modal_selected: Style::new()
                .fg(Color::Black)
                .bg(Color::Magenta)
                .add_modifier(Modifier::BOLD),

            accent: Style::new().fg(Color::Cyan),
            error: Style::new().fg(Color::LightRed),
        }
    }

    pub fn sonos() -> Self {
        let orange = Color::Rgb(255, 120, 0);
        Self {
            header: Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
            legend: Style::new().fg(Color::DarkGray),
            muted: Style::new().fg(Color::DarkGray),

            card_border: Style::new().fg(Color::DarkGray),
            card_border_selected: Style::new()
                .fg(orange)
                .add_modifier(Modifier::BOLD),
            card_title: Style::new()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            track_info: Style::new().fg(Color::Gray),

            playing_icon: Style::new().fg(orange),
            paused_icon: Style::new().fg(Color::Yellow),
            stopped_icon: Style::new().fg(Color::DarkGray),

            volume_filled: Style::new().fg(orange),
            volume_empty: Style::new().fg(Color::DarkGray),

            progress_filled: Style::new().fg(Color::White),
            progress_empty: Style::new().fg(Color::DarkGray),
            progress_cursor: Style::new().fg(orange),
            progress_time: Style::new().fg(Color::DarkGray),

            mini_player_border: Style::new().fg(Color::DarkGray),
            mini_player_title: Style::new()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),

            group_header: Style::new()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            speaker_cursor: Style::new().fg(orange),
            speaker_name: Style::new().fg(Color::Gray),

            modal_border: Style::new().fg(orange),
            modal_title: Style::new()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            modal_selected: Style::new()
                .fg(Color::Black)
                .bg(orange)
                .add_modifier(Modifier::BOLD),

            accent: Style::new().fg(orange),
            error: Style::new().fg(Color::Red),
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

    #[test]
    fn from_name_resolves_all_themes() {
        let _ = Theme::from_name("light");
        let _ = Theme::from_name("neon");
        let _ = Theme::from_name("sonos");
    }
}
