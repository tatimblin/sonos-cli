//! TUI color theme system.
//!
//! Every widget references `app.theme` — no hardcoded colors or characters.
//! Styles and glyphs are pre-computed in the constructor so render functions
//! pay zero allocation cost.

use ratatui::style::{Color, Modifier, Style};

/// UI characters used by the TUI. Themes declare both styles and glyphs,
/// so custom themes can swap characters alongside colors.
#[derive(Clone, Debug)]
pub struct Glyphs {
    // Playback state icons
    pub playing: &'static str,
    pub paused: &'static str,
    pub stopped: &'static str,

    // Speaker list tree connectors
    pub connector_branch: &'static str,
    pub connector_last: &'static str,

    // Cursor / selection indicator
    pub cursor: &'static str,

    // Dot leader fill character
    pub leader_char: char,

    // Model separator in speaker rows
    pub model_separator: &'static str,

    // Group divider
    pub divider_left: &'static str,
    pub divider_fill: &'static str,
    pub divider_right: &'static str,

    // Drop zone borders
    pub zone_tl: &'static str,
    pub zone_tr: &'static str,
    pub zone_bl: &'static str,
    pub zone_br: &'static str,
    pub zone_horiz: &'static str,
    pub zone_vert: &'static str,

    // Media controls
    pub control_prev: &'static str,
    pub control_next: &'static str,

    // Header / tabs
    pub logo: &'static str,
    pub tab_active_left: &'static str,
    pub tab_active_right: &'static str,
    pub tab_active_indicator: &'static str,

    // Settings
    pub dropdown_indicator: &'static str,
    pub dropdown_active: &'static str,
    pub settings_cursor: &'static str,

    // Separator line
    pub separator: char,

    // Progress bar cursor
    pub progress_cursor: &'static str,

    // Album art placeholder
    pub music_note: &'static str,
}

impl Glyphs {
    pub fn default_glyphs() -> Self {
        Self {
            playing: "\u{25b6}", // ▶
            paused: "\u{23f8}",  // ⏸
            stopped: "\u{25a0}", // ■

            connector_branch: "\u{251c}\u{2500} ", // ├─
            connector_last: "\u{2514}\u{2500} ",   // └─

            cursor: "\u{276f}", // ❯

            leader_char: ':',

            model_separator: " \u{2022} ", // •

            divider_left: "+",
            divider_fill: "\u{2500}", // ─
            divider_right: "+",

            zone_tl: "\u{256d}",    // ╭
            zone_tr: "\u{256e}",    // ╮
            zone_bl: "\u{2570}",    // ╰
            zone_br: "\u{256f}",    // ╯
            zone_horiz: "\u{2500}", // ─
            zone_vert: "\u{2502}",  // │

            control_prev: "\u{23ee}", // ⏮
            control_next: "\u{23ed}", // ⏭

            logo: "\u{266a}  S O N O S", // ♪  S O N O S

            tab_active_left: "[",
            tab_active_right: "]",
            tab_active_indicator: "\u{25b8}", // ▸

            dropdown_indicator: "\u{25bc}", // ▼
            dropdown_active: "\u{25b8}",    // ▸
            settings_cursor: "\u{25c0}",    // ◀

            separator: '\u{2500}', // ─

            progress_cursor: "\u{257a}", // ╺

            music_note: "\u{266a}", // ♪
        }
    }
}

/// Semantic styles used by the TUI. Grows as screens need new roles.
#[derive(Clone, Debug)]
pub struct Theme {
    // Layout chrome
    pub header: Style,
    pub legend: Style,
    pub muted: Style,

    // Track info
    pub track_info: Style,
    pub bottom_bar_border: Style,
    pub bottom_bar_controls: Style,

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

    // Speakers tab
    pub group_header: Style,
    pub speaker_cursor: Style,
    pub speaker_name: Style,
    pub leader: Style,

    // General
    pub accent: Style,
    pub error: Style,

    // UI characters
    pub glyphs: Glyphs,
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

            track_info: Style::new().fg(Color::Gray),
            bottom_bar_border: Style::new().fg(Color::DarkGray),
            bottom_bar_controls: Style::new().fg(Color::White),

            playing_icon: Style::new().fg(Color::Green),
            paused_icon: Style::new().fg(Color::Yellow),
            stopped_icon: Style::new().fg(Color::DarkGray),

            volume_filled: Style::new().fg(Color::Cyan),
            volume_empty: Style::new().fg(Color::DarkGray),

            progress_filled: Style::new().fg(Color::White),
            progress_empty: Style::new().fg(Color::DarkGray),
            progress_cursor: Style::new().fg(Color::White),
            progress_time: Style::new().fg(Color::DarkGray),

            group_header: Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
            speaker_cursor: Style::new().fg(Color::Cyan),
            speaker_name: Style::new().fg(Color::Gray),
            leader: Style::new().fg(Color::DarkGray),

            accent: Style::new().fg(Color::Cyan),
            error: Style::new().fg(Color::Red),

            glyphs: Glyphs::default_glyphs(),
        }
    }

    pub fn light() -> Self {
        Self {
            header: Style::new().fg(Color::Black).add_modifier(Modifier::BOLD),
            legend: Style::new().fg(Color::Gray),
            muted: Style::new().fg(Color::Gray),

            track_info: Style::new().fg(Color::DarkGray),
            bottom_bar_border: Style::new().fg(Color::Gray),
            bottom_bar_controls: Style::new().fg(Color::Black),

            playing_icon: Style::new().fg(Color::Green),
            paused_icon: Style::new().fg(Color::Yellow),
            stopped_icon: Style::new().fg(Color::Gray),

            volume_filled: Style::new().fg(Color::Blue),
            volume_empty: Style::new().fg(Color::Gray),

            progress_filled: Style::new().fg(Color::Black),
            progress_empty: Style::new().fg(Color::Gray),
            progress_cursor: Style::new().fg(Color::Black),
            progress_time: Style::new().fg(Color::Gray),

            group_header: Style::new()
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            speaker_cursor: Style::new().fg(Color::Blue),
            speaker_name: Style::new().fg(Color::DarkGray),
            leader: Style::new().fg(Color::Gray),

            accent: Style::new().fg(Color::Blue),
            error: Style::new().fg(Color::Red),

            glyphs: Glyphs::default_glyphs(),
        }
    }

    pub fn neon() -> Self {
        Self {
            header: Style::new().fg(Color::Magenta).add_modifier(Modifier::BOLD),
            legend: Style::new().fg(Color::DarkGray),
            muted: Style::new().fg(Color::DarkGray),

            track_info: Style::new().fg(Color::LightMagenta),
            bottom_bar_border: Style::new().fg(Color::DarkGray),
            bottom_bar_controls: Style::new().fg(Color::Cyan),

            playing_icon: Style::new().fg(Color::Green),
            paused_icon: Style::new().fg(Color::Yellow),
            stopped_icon: Style::new().fg(Color::DarkGray),

            volume_filled: Style::new().fg(Color::Magenta),
            volume_empty: Style::new().fg(Color::DarkGray),

            progress_filled: Style::new().fg(Color::Cyan),
            progress_empty: Style::new().fg(Color::DarkGray),
            progress_cursor: Style::new().fg(Color::Cyan),
            progress_time: Style::new().fg(Color::DarkGray),

            group_header: Style::new()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            speaker_cursor: Style::new().fg(Color::Cyan),
            speaker_name: Style::new().fg(Color::LightMagenta),
            leader: Style::new().fg(Color::DarkGray),

            accent: Style::new().fg(Color::Cyan),
            error: Style::new().fg(Color::LightRed),

            glyphs: Glyphs::default_glyphs(),
        }
    }

    pub fn sonos() -> Self {
        let orange = Color::Rgb(255, 120, 0);
        Self {
            header: Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
            legend: Style::new().fg(Color::DarkGray),
            muted: Style::new().fg(Color::DarkGray),

            track_info: Style::new().fg(Color::Gray),
            bottom_bar_border: Style::new().fg(Color::DarkGray),
            bottom_bar_controls: Style::new().fg(Color::White),

            playing_icon: Style::new().fg(orange),
            paused_icon: Style::new().fg(Color::Yellow),
            stopped_icon: Style::new().fg(Color::DarkGray),

            volume_filled: Style::new().fg(orange),
            volume_empty: Style::new().fg(Color::DarkGray),

            progress_filled: Style::new().fg(Color::White),
            progress_empty: Style::new().fg(Color::DarkGray),
            progress_cursor: Style::new().fg(orange),
            progress_time: Style::new().fg(Color::DarkGray),

            group_header: Style::new()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            speaker_cursor: Style::new().fg(orange),
            speaker_name: Style::new().fg(Color::Gray),
            leader: Style::new().fg(Color::DarkGray),

            accent: Style::new().fg(orange),
            error: Style::new().fg(Color::Red),

            glyphs: Glyphs::default_glyphs(),
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

    #[test]
    fn glyphs_are_populated() {
        let theme = Theme::dark();
        assert_eq!(theme.glyphs.playing, "\u{25b6}");
        assert_eq!(theme.glyphs.connector_last, "\u{2514}\u{2500} ");
    }
}
