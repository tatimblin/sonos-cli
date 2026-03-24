//! Group card widget — renders a single group's state within a bordered box.

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::tui::theme::Theme;
use crate::tui::widgets::progress_bar;

// Pre-computed bar strings — sliced per-frame instead of allocating via `.repeat()`.
// All chars below are 3 bytes in UTF-8. 100 chars covers any reasonable terminal width.
const VOL_FILLED: &str = "■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■";
const VOL_EMPTY: &str = "····································································································";
const PROG_FILLED: &str = "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━";
const PROG_EMPTY: &str = "────────────────────────────────────────────────────────────────────────────────────────────────────────";
const SPACES: &str = "                                                                                                    ";

const VOL_FILLED_CHAR_BYTES: usize = 3; // ■ U+25A0
const VOL_EMPTY_CHAR_BYTES: usize = 2; // · U+00B7
const PROG_CHAR_BYTES: usize = 3; // ━ U+2501, ─ U+2500

/// Data needed to render a single group card.
pub struct GroupCardData {
    pub group_name: String,
    pub playback_state: PlaybackIcon,
    pub track_title: String,
    pub track_artist: String,
    pub volume: u16,
    pub progress: f64,
    pub elapsed_ms: u64,
    pub duration_ms: u64,
    pub speaker_count_text: String,
    pub selected: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaybackIcon {
    Playing,
    Paused,
    Stopped,
}

impl PlaybackIcon {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Playing => "▶",
            Self::Paused => "⏸",
            Self::Stopped => "■",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Playing => "Playing",
            Self::Paused => "Paused",
            Self::Stopped => "Stopped",
        }
    }
}

/// Render a group card within the given area.
#[allow(clippy::too_many_lines)]
pub fn render_group_card(frame: &mut Frame, area: Rect, data: &GroupCardData, theme: &Theme) {
    let (border_type, border_style) = if data.selected {
        (BorderType::Thick, theme.card_border_selected)
    } else {
        (BorderType::Plain, theme.card_border)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(border_style);

    let raw_inner = block.inner(area);
    frame.render_widget(block, area);

    // Horizontal padding (1 char each side)
    let inner = Rect::new(
        raw_inner.x + 1,
        raw_inner.y,
        raw_inner.width.saturating_sub(2),
        raw_inner.height,
    );

    if inner.height == 0 || inner.width < 10 {
        return;
    }

    let w = inner.width as usize;

    let icon_style = match data.playback_state {
        PlaybackIcon::Playing => theme.playing_icon,
        PlaybackIcon::Paused => theme.paused_icon,
        PlaybackIcon::Stopped => theme.stopped_icon,
    };

    // Line 1: ●/○ Name          ▶ Playing
    let prefix = if data.selected { "● " } else { "○ " };
    let left = format!("{prefix}{}", data.group_name);
    let right = format!(
        "{} {}",
        data.playback_state.symbol(),
        data.playback_state.label()
    );
    let left_width = left.chars().count();
    let right_width = right.chars().count();
    let pad = w.saturating_sub(left_width + right_width).min(100);
    let line1 = Line::from(vec![
        Span::styled(left, theme.card_title),
        Span::raw(&SPACES[..pad]),
        Span::styled(right, icon_style),
    ]);

    // Line 2: empty
    let line2 = Line::raw("");

    // Line 3: track title (indented)
    let title = if data.track_title.is_empty() {
        "Nothing playing"
    } else {
        &data.track_title
    };
    let line3 = Line::from(Span::styled(format!("  {title}"), theme.track_info));

    // Line 4: artist (indented)
    let line4 = if data.track_artist.is_empty() {
        Line::raw("")
    } else {
        Line::from(Span::styled(
            format!("  {}", data.track_artist),
            theme.muted,
        ))
    };

    // Line 5: empty
    let line5 = Line::raw("");

    // Line 6: ▶  ━━━━━━━━╺────────  2:31/5:55
    let elapsed_str = progress_bar::format_time(data.elapsed_ms);
    let duration_str = progress_bar::format_time(data.duration_ms);
    let time_text = format!("  {elapsed_str}/{duration_str}");
    // prefix: "  " + icon(1) + "  " = 5 display cols
    let prog_prefix_width = 5;
    let prog_bar_width = w.saturating_sub(prog_prefix_width + time_text.len());
    let progress = data.progress.clamp(0.0, 1.0);
    let cursor_pos = (prog_bar_width as f64 * progress) as usize;
    let has_track = !data.track_title.is_empty();
    let filled_count = cursor_pos.min(prog_bar_width);
    let cursor_width = if has_track { 1 } else { 0 };
    let empty_count = prog_bar_width.saturating_sub(filled_count + cursor_width);
    let cursor = if has_track && filled_count < prog_bar_width {
        "●"
    } else {
        ""
    };
    let filled_count = filled_count.min(100);
    let empty_count = empty_count.min(100);
    let line6 = Line::from(vec![
        Span::raw("  "),
        Span::styled(data.playback_state.symbol(), icon_style),
        Span::raw("  "),
        Span::styled(
            &PROG_FILLED[..filled_count * PROG_CHAR_BYTES],
            theme.progress_filled,
        ),
        Span::styled(cursor, theme.progress_cursor),
        Span::styled(
            &PROG_EMPTY[..empty_count * PROG_CHAR_BYTES],
            theme.progress_empty,
        ),
        Span::styled(time_text, theme.progress_time),
    ]);

    // Line 7: Speaker text          🔊 ████░░░░
    // Each half gets max 50% of the width.
    let half_w = w / 2;
    let spk_text = &data.speaker_count_text;
    let spk_display = format!("  {spk_text}");
    let spk_len = spk_display.chars().count();
    let spk_pad = half_w.saturating_sub(spk_len).min(100);
    // volume prefix: 🔊(2) + " " = 3 display cols; suffix: " XX%"
    let vol_label = format!(" {}%", data.volume);
    let vol_prefix_width = 3;
    let vol_bar_width = half_w.saturating_sub(vol_prefix_width + vol_label.len());
    let vol_filled = ((vol_bar_width as f64) * (data.volume as f64) / 100.0) as usize;
    let vol_empty = vol_bar_width.saturating_sub(vol_filled);
    let vol_filled = vol_filled.min(100);
    let vol_empty = vol_empty.min(100);
    let line7 = Line::from(vec![
        Span::styled(spk_display, theme.muted),
        Span::raw(&SPACES[..spk_pad]),
        Span::raw("🔊 "),
        Span::styled(
            &VOL_FILLED[..vol_filled * VOL_FILLED_CHAR_BYTES],
            theme.volume_filled,
        ),
        Span::styled(
            &VOL_EMPTY[..vol_empty * VOL_EMPTY_CHAR_BYTES],
            theme.volume_empty,
        ),
        Span::styled(vol_label, theme.muted),
    ]);

    let lines = vec![line1, line2, line3, line4, line5, line6, line7];
    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

/// Render a placeholder for an unavailable group.
pub fn render_unavailable_card(
    frame: &mut Frame,
    area: Rect,
    group_name: &str,
    selected: bool,
    theme: &Theme,
) {
    let (border_type, border_style) = if selected {
        (BorderType::Thick, theme.card_border_selected)
    } else {
        (BorderType::Plain, theme.card_border)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(Span::styled(format!("○ {group_name}"), theme.card_title)),
        Line::raw(""),
        Line::from(Span::styled("  Unavailable", theme.muted)),
    ];
    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}
