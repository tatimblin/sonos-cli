//! Bottom player bar widget — Spotify-style persistent playback display.
//!
//! Render-only: takes `BottomBarData` and `Theme`, outputs to frame.
//! Supports wide (>= 100 cols), narrow (60..100), and minimal (< 60) layouts.

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use sonos_sdk::PlaybackState;

use crate::tui::theme::Theme;
use crate::tui::types::BottomBarData;
use crate::tui::widgets::album_art;
use crate::tui::widgets::progress_bar;
use crate::tui::widgets::volume_bar;

/// Render the bottom player bar. Area must be exactly 3 rows tall for
/// wide/narrow layouts, or 1 row for minimal layout.
pub fn render(frame: &mut Frame, area: Rect, data: &mut BottomBarData, theme: &Theme) {
    if area.width < 20 {
        return;
    }

    if area.height == 1 {
        render_minimal(frame, area, data, theme);
        return;
    }

    if area.height < 3 {
        return;
    }

    if data.is_wide {
        render_wide(frame, area, data, theme);
    } else {
        render_narrow(frame, area, data, theme);
    }
}

// ---------------------------------------------------------------------------
// Wide layout (>= 100 cols) — 3 rows, 3 columns
// ---------------------------------------------------------------------------
// Row 0: [art] Title                    controls                  Group Name
// Row 1: [art] Artist      time ━━━━━━━━━━━━╺──────── time    vol_bar vol%
// Row 2: [art]
// ---------------------------------------------------------------------------

fn render_wide(frame: &mut Frame, area: Rect, data: &mut BottomBarData, theme: &Theme) {
    // Album art: 5 wide x 3 tall (includes border)
    let art_width: u16 = 6;
    let art_area = Rect::new(area.x, area.y, art_width, 3);

    album_art::render_album_art(
        frame,
        art_area,
        data.album_art_protocol.as_mut(),
        theme.bottom_bar_border,
        theme.muted,
    );

    let text_x = area.x + art_width + 1;
    let text_w = area.width.saturating_sub(art_width + 1);
    if text_w == 0 {
        return;
    }

    // Compute column widths: left (metadata), center (controls+progress), right (group+volume)
    let vol_width: u16 = 20;
    let controls_width: u16 = 11; // "  ⏮  ▶  ⏭  "

    // Row 0: Title (left), controls (center), group name (right)
    let title = data.track_title.as_deref().unwrap_or("No track");
    let group = &data.group_name;

    let group_width = group.chars().count() as u16;
    let title_width = text_w.saturating_sub(controls_width + group_width + 4);

    let title_display = truncate_str(title, title_width as usize);

    let controls_str = build_controls_str(data.playback_state.as_ref());

    // Distribute remaining space: half before controls, half after
    let title_chars = title_display.chars().count() as u16;
    let total_used = title_chars + controls_width + group_width;
    let remaining = text_w.saturating_sub(total_used);
    let pad1 = (remaining / 2).max(1);
    let pad2 = remaining.saturating_sub(pad1).max(1);

    let row0 = Line::from(vec![
        Span::styled(title_display, theme.card_title),
        Span::raw(" ".repeat(pad1 as usize)),
        Span::styled(controls_str, theme.bottom_bar_controls),
        Span::raw(" ".repeat(pad2 as usize)),
        Span::styled(group.clone(), theme.track_info),
    ]);
    let row0_area = Rect::new(text_x, area.y, text_w, 1);
    frame.render_widget(Paragraph::new(row0), row0_area);

    // Row 1: Artist, time + progress bar + time, volume bar
    let artist = data.track_artist.as_deref().unwrap_or("");

    let pos_str = progress_bar::format_time(data.position_ms);
    let dur_str = progress_bar::format_time(data.duration_ms);
    let time_label_width = (pos_str.chars().count() + dur_str.chars().count() + 2) as u16; // " " on each side

    let progress_bar_width = text_w.saturating_sub(vol_width + time_label_width + 4) as usize;
    let artist_max =
        text_w.saturating_sub(vol_width + time_label_width + progress_bar_width as u16 + 4);
    let artist_display = truncate_str(artist, artist_max as usize);
    let artist_chars = artist_display.chars().count() as u16;

    // Build progress spans
    let bar_spans = progress_bar::render_bar_spans(
        data.progress,
        progress_bar_width.saturating_sub(1),
        Some("\u{257A}"),
        theme.progress_filled,
        theme.progress_cursor,
        theme.progress_empty,
    );

    let bar_section_width = (progress_bar_width + time_label_width as usize + 1) as u16;
    let pad_after_artist = text_w.saturating_sub(artist_chars + bar_section_width + vol_width + 1);

    let mut row1_spans = vec![
        Span::styled(artist_display, theme.track_info),
        Span::raw(" ".repeat(pad_after_artist as usize)),
        Span::styled(format!("{pos_str} "), theme.progress_time),
    ];
    row1_spans.extend(bar_spans);
    row1_spans.push(Span::styled(format!(" {dur_str}"), theme.progress_time));
    row1_spans.push(Span::raw("  "));

    // Volume bar
    let vol_line = volume_bar::render_volume_bar(
        data.volume,
        vol_width,
        theme.volume_filled,
        theme.volume_empty,
    );
    row1_spans.extend(vol_line.spans);

    let row1_area = Rect::new(text_x, area.y + 1, text_w, 1);
    frame.render_widget(Paragraph::new(Line::from(row1_spans)), row1_area);

    // Row 2: empty (art occupies this row)
}

// ---------------------------------------------------------------------------
// Narrow layout (60..100 cols) — 3 rows
// ---------------------------------------------------------------------------
// Row 0: [art] Title                                 Group Name
// Row 1: [art] Artist                             vol_bar vol%
// Row 2:        controls   time ━━━━━━━━━━━━━╺──────── time
// ---------------------------------------------------------------------------

fn render_narrow(frame: &mut Frame, area: Rect, data: &mut BottomBarData, theme: &Theme) {
    let art_width: u16 = 6;
    let art_area = Rect::new(area.x, area.y, art_width, 3.min(area.height));

    album_art::render_album_art(
        frame,
        art_area,
        data.album_art_protocol.as_mut(),
        theme.bottom_bar_border,
        theme.muted,
    );

    let text_x = area.x + art_width + 1;
    let text_w = area.width.saturating_sub(art_width + 1);
    if text_w == 0 {
        return;
    }

    let vol_width: u16 = 18.min(text_w / 3);

    // Row 0: Title + group name
    let title = data.track_title.as_deref().unwrap_or("No track");
    let group = &data.group_name;

    let group_width = group.chars().count() as u16;
    let title_max = text_w.saturating_sub(group_width + 2);
    let title_display = truncate_str(title, title_max as usize);
    let title_chars = title_display.chars().count() as u16;
    let pad = text_w.saturating_sub(title_chars + group_width);

    let row0 = Line::from(vec![
        Span::styled(title_display, theme.card_title),
        Span::raw(" ".repeat(pad as usize)),
        Span::styled(group.clone(), theme.track_info),
    ]);
    frame.render_widget(Paragraph::new(row0), Rect::new(text_x, area.y, text_w, 1));

    // Row 1: Artist + volume bar
    let artist = data.track_artist.as_deref().unwrap_or("");
    let artist_max = text_w.saturating_sub(vol_width + 2);
    let artist_display = truncate_str(artist, artist_max as usize);
    let artist_chars = artist_display.chars().count() as u16;
    let pad = text_w.saturating_sub(artist_chars + vol_width);

    let vol_line = volume_bar::render_volume_bar(
        data.volume,
        vol_width,
        theme.volume_filled,
        theme.volume_empty,
    );

    let mut row1_spans = vec![
        Span::styled(artist_display, theme.track_info),
        Span::raw(" ".repeat(pad as usize)),
    ];
    row1_spans.extend(vol_line.spans);

    frame.render_widget(
        Paragraph::new(Line::from(row1_spans)),
        Rect::new(text_x, area.y + 1, text_w, 1),
    );

    // Row 2: Controls + progress bar (full width, below art)
    if area.height >= 3 {
        let row2_x = area.x + 1;
        let row2_w = area.width.saturating_sub(2);

        let controls_str = build_controls_str(data.playback_state.as_ref());
        let controls_width = 11u16;

        let pos_str = progress_bar::format_time(data.position_ms);
        let dur_str = progress_bar::format_time(data.duration_ms);
        let time_width = (pos_str.chars().count() + dur_str.chars().count() + 2) as u16;

        let bar_width = row2_w.saturating_sub(controls_width + time_width + 4) as usize;

        let bar_spans = progress_bar::render_bar_spans(
            data.progress,
            bar_width.saturating_sub(1),
            Some("\u{257A}"),
            theme.progress_filled,
            theme.progress_cursor,
            theme.progress_empty,
        );

        let mut row2_spans = vec![
            Span::raw("  "),
            Span::styled(controls_str, theme.bottom_bar_controls),
            Span::raw("  "),
            Span::styled(format!("{pos_str} "), theme.progress_time),
        ];
        row2_spans.extend(bar_spans);
        row2_spans.push(Span::styled(format!(" {dur_str}"), theme.progress_time));

        frame.render_widget(
            Paragraph::new(Line::from(row2_spans)),
            Rect::new(row2_x, area.y + 2, row2_w, 1),
        );
    }
}

// ---------------------------------------------------------------------------
// Minimal layout (< 60 cols) — 1 row
// ---------------------------------------------------------------------------
// [play_icon] Track Title — Group Name
// ---------------------------------------------------------------------------

fn render_minimal(frame: &mut Frame, area: Rect, data: &BottomBarData, theme: &Theme) {
    let icon = playback_icon(data.playback_state.as_ref());
    let title = data.track_title.as_deref().unwrap_or("No track");
    let group = &data.group_name;

    let text = format!("{icon} {title} \u{2014} {group}");
    let display = truncate_str(&text, area.width as usize);

    let paragraph = Paragraph::new(Line::from(vec![Span::styled(display, theme.track_info)]));
    frame.render_widget(paragraph, area);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn playback_icon(state: Option<&PlaybackState>) -> &'static str {
    match state {
        Some(PlaybackState::Playing) => "\u{25b6}", // ▶
        Some(PlaybackState::Paused) => "\u{23f8}",  // ⏸
        _ => "\u{25a0}",                            // ■
    }
}

fn build_controls_str(state: Option<&PlaybackState>) -> String {
    let play_pause = playback_icon(state);
    format!(" \u{23ee}  {play_pause}  \u{23ed} ") // ⏮  ▶  ⏭
}

/// Truncate a string to fit within `max_chars` characters, appending ellipsis if needed.
fn truncate_str(s: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else if max_chars <= 1 {
        "\u{2026}".to_string()
    } else {
        let truncated: String = s.chars().take(max_chars - 1).collect();
        format!("{truncated}\u{2026}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_str_short_string() {
        assert_eq!(truncate_str("hello", 10), "hello");
    }

    #[test]
    fn truncate_str_exact_fit() {
        assert_eq!(truncate_str("hello", 5), "hello");
    }

    #[test]
    fn truncate_str_needs_truncation() {
        let result = truncate_str("hello world", 8);
        assert_eq!(result, "hello w\u{2026}");
    }

    #[test]
    fn truncate_str_zero_width() {
        assert_eq!(truncate_str("hello", 0), "");
    }

    #[test]
    fn truncate_str_width_one() {
        assert_eq!(truncate_str("hello", 1), "\u{2026}");
    }

    #[test]
    fn playback_icon_playing() {
        assert_eq!(playback_icon(Some(&PlaybackState::Playing)), "\u{25b6}");
    }

    #[test]
    fn playback_icon_paused() {
        assert_eq!(playback_icon(Some(&PlaybackState::Paused)), "\u{23f8}");
    }

    #[test]
    fn playback_icon_none() {
        assert_eq!(playback_icon(None), "\u{25a0}");
    }

    #[test]
    fn build_controls_str_includes_icons() {
        let controls = build_controls_str(Some(&PlaybackState::Playing));
        assert!(controls.contains('\u{23ee}')); // ⏮
        assert!(controls.contains('\u{23ed}')); // ⏭
        assert!(controls.contains('\u{25b6}')); // ▶
    }
}
