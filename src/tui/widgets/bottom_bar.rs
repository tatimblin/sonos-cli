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
// Wide layout (>= 100 cols) — 3 rows, 3 columns (Spotify-style)
// ---------------------------------------------------------------------------
//            Left               Center                    Right
// Row 0: [art] Title          controls              Group Name
// Row 1: [art] Artist    time ━━━━━━━━━━ time     vol_bar vol%
// Row 2: [art] Album
// ---------------------------------------------------------------------------

fn render_wide(frame: &mut Frame, area: Rect, data: &mut BottomBarData, theme: &Theme) {
    let art_width: u16 = 6;
    let art_area = Rect::new(area.x, area.y, art_width, 3);

    album_art::render_album_art(
        frame,
        art_area,
        data.album_art_protocol.as_mut(),
        theme.bottom_bar_border,
        theme.muted,
        theme.glyphs.music_note,
    );

    let content_x = area.x + art_width + 1;
    let content_w = area.width.saturating_sub(art_width + 1);
    if content_w == 0 {
        return;
    }

    // Three-column split: left (metadata) | center (controls+progress) | right (group+volume)
    let right_w: u16 = 22.min(content_w / 3);
    let left_w = (content_w.saturating_sub(right_w)) * 3 / 10;
    let center_w = content_w.saturating_sub(left_w + right_w);

    let left_x = content_x;
    let center_x = left_x + left_w;
    let right_x = center_x + center_w;

    // -- Left column: Title (row 0), Artist (row 1), Album (row 2) --

    let title = data.track_title.as_deref().unwrap_or("No track");
    let title_display = truncate_str(title, left_w.saturating_sub(1) as usize);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(title_display, theme.header))),
        Rect::new(left_x, area.y, left_w, 1),
    );

    let artist = data.track_artist.as_deref().unwrap_or("");
    let artist_display = truncate_str(artist, left_w.saturating_sub(1) as usize);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(artist_display, theme.muted))),
        Rect::new(left_x, area.y + 1, left_w, 1),
    );

    if area.height >= 3 {
        let album = data.track_album.as_deref().unwrap_or("");
        if !album.is_empty() {
            let album_display = truncate_str(album, left_w.saturating_sub(1) as usize);
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(album_display, theme.muted))),
                Rect::new(left_x, area.y + 2, left_w, 1),
            );
        }
    }

    // -- Center column: Controls (row 0), Progress bar (row 1) --

    let controls_str = build_controls_str(data.playback_state.as_ref(), theme);
    let controls_width: u16 = 11;
    let controls_pad = center_w.saturating_sub(controls_width) / 2;
    let mut controls_spans = vec![Span::raw(" ".repeat(controls_pad as usize))];
    controls_spans.push(Span::styled(controls_str, theme.bottom_bar_controls));
    frame.render_widget(
        Paragraph::new(Line::from(controls_spans)),
        Rect::new(center_x, area.y, center_w, 1),
    );

    let pos_str = progress_bar::format_time(data.position_ms);
    let dur_str = progress_bar::format_time(data.duration_ms);
    let time_label_width = (pos_str.chars().count() + dur_str.chars().count() + 2) as u16;
    let bar_width = center_w.saturating_sub(time_label_width + 2) as usize;

    let bar_spans = progress_bar::render_bar_spans(
        data.progress,
        bar_width.saturating_sub(1),
        Some(theme.glyphs.progress_cursor),
        theme.progress_filled,
        theme.progress_cursor,
        theme.progress_empty,
    );

    let mut progress_spans = vec![Span::styled(format!("{pos_str} "), theme.progress_time)];
    progress_spans.extend(bar_spans);
    progress_spans.push(Span::styled(format!(" {dur_str}"), theme.progress_time));
    frame.render_widget(
        Paragraph::new(Line::from(progress_spans)),
        Rect::new(center_x, area.y + 1, center_w, 1),
    );

    // -- Right column: Group name (row 0), Volume bar (row 1) --

    let group = &data.group_name;
    let group_display = truncate_str(group, right_w as usize);
    let group_chars = group_display.chars().count() as u16;
    let group_pad = right_w.saturating_sub(group_chars);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(" ".repeat(group_pad as usize)),
            Span::styled(group_display, theme.track_info),
        ])),
        Rect::new(right_x, area.y, right_w, 1),
    );

    let vol_width = right_w.min(20);
    let vol_line = volume_bar::render_volume_bar(
        data.volume,
        vol_width,
        theme.volume_filled,
        theme.volume_empty,
    );
    let vol_pad = right_w.saturating_sub(vol_width);
    let mut vol_spans = vec![Span::raw(" ".repeat(vol_pad as usize))];
    vol_spans.extend(vol_line.spans);
    frame.render_widget(
        Paragraph::new(Line::from(vol_spans)),
        Rect::new(right_x, area.y + 1, right_w, 1),
    );
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
        theme.glyphs.music_note,
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
        Span::styled(title_display, theme.header),
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
        Span::styled(artist_display, theme.muted),
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

        let controls_str = build_controls_str(data.playback_state.as_ref(), theme);
        let controls_width = 11u16;

        let pos_str = progress_bar::format_time(data.position_ms);
        let dur_str = progress_bar::format_time(data.duration_ms);
        let time_width = (pos_str.chars().count() + dur_str.chars().count() + 2) as u16;

        let bar_width = row2_w.saturating_sub(controls_width + time_width + 4) as usize;

        let bar_spans = progress_bar::render_bar_spans(
            data.progress,
            bar_width.saturating_sub(1),
            Some(theme.glyphs.progress_cursor),
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
    let icon = playback_icon(data.playback_state.as_ref(), theme);
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

fn playback_icon<'a>(state: Option<&PlaybackState>, theme: &'a Theme) -> &'a str {
    let g = &theme.glyphs;
    match state {
        Some(PlaybackState::Playing) => g.playing,
        Some(PlaybackState::Paused) => g.paused,
        _ => g.stopped,
    }
}

fn build_controls_str(state: Option<&PlaybackState>, theme: &Theme) -> String {
    let g = &theme.glyphs;
    let play_pause = playback_icon(state, theme);
    format!(" {}  {play_pause}  {} ", g.control_prev, g.control_next)
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
        let theme = Theme::dark();
        assert_eq!(
            playback_icon(Some(&PlaybackState::Playing), &theme),
            theme.glyphs.playing
        );
    }

    #[test]
    fn playback_icon_paused() {
        let theme = Theme::dark();
        assert_eq!(
            playback_icon(Some(&PlaybackState::Paused), &theme),
            theme.glyphs.paused
        );
    }

    #[test]
    fn playback_icon_none() {
        let theme = Theme::dark();
        assert_eq!(playback_icon(None, &theme), theme.glyphs.stopped);
    }

    #[test]
    fn build_controls_str_includes_icons() {
        let theme = Theme::dark();
        let controls = build_controls_str(Some(&PlaybackState::Playing), &theme);
        assert!(controls.contains(theme.glyphs.control_prev));
        assert!(controls.contains(theme.glyphs.control_next));
        assert!(controls.contains(theme.glyphs.playing));
    }
}
