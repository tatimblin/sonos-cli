//! TUI color theme system.
//!
//! Every widget references `app.theme` — no hardcoded colors or characters.
//! Styles and glyphs are pre-computed in the constructor so render functions
//! pay zero allocation cost.

use ratatui::style::{Color, Modifier, Style};

/// Check whether two `Color` values are the same variant and value.
pub(crate) fn colors_equal(a: Color, b: Color) -> bool {
    match (a, b) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => r1 == r2 && g1 == g2 && b1 == b2,
        _ => std::mem::discriminant(&a) == std::mem::discriminant(&b),
    }
}

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

    // Toast notification
    pub toast_prefix: &'static str,

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

            progress_cursor: "\u{25cf}", // ●

            toast_prefix: "\u{25cf}", // ●

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

    // Pickup mode
    pub picked_up: Style,

    // General
    pub accent: Style,
    pub accent_secondary: Style,
    pub error: Style,

    // Progress bar gradient (start == end means no gradient)
    pub progress_gradient_start: Color,
    pub progress_gradient_end: Color,

    // UI characters
    pub glyphs: Glyphs,
}

impl Theme {
    /// Resolve a theme by name from config. Unknown names fall back to default.
    pub fn from_name(name: &str) -> Self {
        match name {
            "bw" => Self::bw(),
            "minimal" => Self::minimal(),
            "dance_party" => Self::dance_party(),
            _ => Self::default_theme(),
        }
    }

    pub fn default_theme() -> Self {
        Self {
            header: Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
            legend: Style::new().fg(Color::DarkGray),
            muted: Style::new().fg(Color::DarkGray),

            track_info: Style::new().fg(Color::Gray),
            bottom_bar_controls: Style::new().fg(Color::White),

            playing_icon: Style::new().fg(Color::Green),
            paused_icon: Style::new().fg(Color::Yellow),
            stopped_icon: Style::new().fg(Color::DarkGray),

            volume_filled: Style::new().fg(Color::Cyan),
            volume_empty: Style::new().fg(Color::DarkGray),

            progress_filled: Style::new().fg(Color::Gray),
            progress_empty: Style::new().fg(Color::DarkGray),
            progress_cursor: Style::new().fg(Color::White),
            progress_time: Style::new().fg(Color::DarkGray),

            group_header: Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
            speaker_cursor: Style::new().fg(Color::Cyan),
            speaker_name: Style::new().fg(Color::Gray),
            leader: Style::new().fg(Color::DarkGray),

            picked_up: Style::new().bg(Color::DarkGray),

            accent: Style::new().fg(Color::Cyan),
            accent_secondary: Style::new().fg(Color::Blue),
            error: Style::new().fg(Color::Red),

            progress_gradient_start: Color::Gray,
            progress_gradient_end: Color::Gray,

            glyphs: Glyphs::default_glyphs(),
        }
    }

    pub fn bw() -> Self {
        Self {
            header: Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
            legend: Style::new().fg(Color::DarkGray),
            muted: Style::new().fg(Color::DarkGray),

            track_info: Style::new().fg(Color::Gray),
            bottom_bar_controls: Style::new().fg(Color::White),

            playing_icon: Style::new()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            paused_icon: Style::new().fg(Color::Gray),
            stopped_icon: Style::new().fg(Color::DarkGray),

            volume_filled: Style::new().fg(Color::White),
            volume_empty: Style::new().fg(Color::DarkGray),

            progress_filled: Style::new().fg(Color::White),
            progress_empty: Style::new().fg(Color::DarkGray),
            progress_cursor: Style::new()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            progress_time: Style::new().fg(Color::DarkGray),

            group_header: Style::new()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            speaker_cursor: Style::new()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            speaker_name: Style::new().fg(Color::Gray),
            leader: Style::new().fg(Color::DarkGray),

            picked_up: Style::new().bg(Color::DarkGray),

            accent: Style::new().fg(Color::White),
            accent_secondary: Style::new().fg(Color::Gray),
            error: Style::new()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),

            progress_gradient_start: Color::White,
            progress_gradient_end: Color::White,

            glyphs: Glyphs::default_glyphs(),
        }
    }

    pub fn minimal() -> Self {
        Self {
            header: Style::new().fg(Color::Gray),
            legend: Style::new().fg(Color::DarkGray),
            muted: Style::new().fg(Color::DarkGray),

            track_info: Style::new().fg(Color::DarkGray),
            bottom_bar_controls: Style::new().fg(Color::Gray),

            playing_icon: Style::new().fg(Color::Green),
            paused_icon: Style::new().fg(Color::Yellow),
            stopped_icon: Style::new().fg(Color::DarkGray),

            volume_filled: Style::new().fg(Color::Gray),
            volume_empty: Style::new().fg(Color::DarkGray),

            progress_filled: Style::new().fg(Color::DarkGray),
            progress_empty: Style::new().fg(Color::DarkGray),
            progress_cursor: Style::new().fg(Color::Gray),
            progress_time: Style::new().fg(Color::DarkGray),

            group_header: Style::new()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            speaker_cursor: Style::new().fg(Color::Gray),
            speaker_name: Style::new().fg(Color::DarkGray),
            leader: Style::new().fg(Color::DarkGray),

            picked_up: Style::new().bg(Color::DarkGray),

            accent: Style::new().fg(Color::Gray),
            accent_secondary: Style::new().fg(Color::DarkGray),
            error: Style::new().fg(Color::Red),

            progress_gradient_start: Color::DarkGray,
            progress_gradient_end: Color::DarkGray,

            glyphs: Glyphs {
                connector_branch: "   ",
                connector_last: "   ",
                leader_char: ' ',
                divider_left: "",
                divider_fill: " ",
                divider_right: "",
                model_separator: "  ",
                cursor: "\u{203a}",              // ›
                progress_cursor: "\u{2022}",     // •
                logo: "sonos",
                tab_active_left: "",
                tab_active_right: "",
                tab_active_indicator: "",
                ..Glyphs::default_glyphs()
            },
        }
    }

    pub fn dance_party() -> Self {
        let hot_pink = Color::Rgb(255, 100, 255);
        Self {
            header: Style::new().fg(hot_pink).add_modifier(Modifier::BOLD),
            legend: Style::new().fg(Color::Rgb(100, 100, 180)),
            muted: Style::new().fg(Color::Rgb(90, 70, 120)),

            track_info: Style::new().fg(Color::Rgb(255, 200, 100)),
            bottom_bar_controls: Style::new().fg(Color::Rgb(100, 255, 255)),

            playing_icon: Style::new().fg(Color::Rgb(50, 255, 50)),
            paused_icon: Style::new().fg(Color::Rgb(255, 255, 50)),
            stopped_icon: Style::new().fg(Color::Rgb(120, 50, 150)),

            volume_filled: Style::new().fg(Color::Rgb(255, 50, 150)),
            volume_empty: Style::new().fg(Color::Rgb(60, 30, 80)),

            progress_filled: Style::new().fg(Color::Rgb(255, 50, 150)),
            progress_empty: Style::new().fg(Color::Rgb(40, 20, 60)),
            progress_cursor: Style::new().fg(Color::Rgb(255, 255, 100)),
            progress_time: Style::new().fg(Color::Rgb(100, 100, 180)),

            group_header: Style::new()
                .fg(Color::Rgb(255, 150, 50))
                .add_modifier(Modifier::BOLD),
            speaker_cursor: Style::new().fg(hot_pink),
            speaker_name: Style::new().fg(Color::Rgb(200, 150, 255)),
            leader: Style::new().fg(Color::Rgb(60, 30, 80)),

            picked_up: Style::new().bg(Color::Rgb(80, 30, 100)),

            accent: Style::new().fg(hot_pink),
            accent_secondary: Style::new().fg(Color::Rgb(100, 255, 200)),
            error: Style::new().fg(Color::Rgb(255, 80, 80)),

            progress_gradient_start: Color::Rgb(255, 50, 150),
            progress_gradient_end: Color::Rgb(50, 200, 255),

            glyphs: Glyphs {
                playing: "\u{266b}",             // ♫
                paused: "\u{1f4a4}",             // 💤
                stopped: "\u{2716}",             // ✖
                cursor: "\u{2605}",              // ★
                music_note: "\u{266b}",          // ♫
                logo: "\u{2605} D A N C E  P A R T Y \u{2605}",
                progress_cursor: "\u{25c6}",     // ◆
                toast_prefix: "\u{2605}",        // ★
                control_prev: "\u{25c4}\u{25c4}", // ◄◄
                control_next: "\u{25ba}\u{25ba}", // ►►
                ..Glyphs::default_glyphs()
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_name_unknown_falls_back_to_default() {
        let theme = Theme::from_name("nonexistent");
        let default = Theme::default_theme();
        assert_eq!(theme.header, default.header);
    }

    #[test]
    fn from_name_resolves_all_themes() {
        let _ = Theme::from_name("default");
        let _ = Theme::from_name("bw");
        let _ = Theme::from_name("minimal");
        let _ = Theme::from_name("dance_party");
    }

    #[test]
    fn colors_equal_same_rgb() {
        assert!(colors_equal(Color::Rgb(1, 2, 3), Color::Rgb(1, 2, 3)));
    }

    #[test]
    fn colors_equal_different_rgb() {
        assert!(!colors_equal(Color::Rgb(1, 2, 3), Color::Rgb(4, 5, 6)));
    }

    #[test]
    fn colors_equal_named() {
        assert!(colors_equal(Color::White, Color::White));
        assert!(!colors_equal(Color::White, Color::Cyan));
    }
}
