//! Speaker list widget — render-only component.
//!
//! Takes pre-computed `SpeakerScreenData` and `Theme`, outputs to frame.
//! No hooks, no SDK, no key handling.
//!
//! Two rendering modes:
//! - **Normal**: flat list of group headers and speaker rows with volume bars
//! - **Drop zone**: collapsed groups shown as bordered drop zones during pick-up
//!
//! Group headers are single-line with dot leader fill:
//!   `▶ Bedroom ::::: BOY IN RED — Artist ::::::::::::: 30%`
//!   `■ Kitchen ::::::::::::::::::::::::::::::::::::::: 21%`
//!
//! Speaker rows use tree connectors with model names:
//!   `├─ Bathroom • Sonos Connect:AMP                  46%`
//!   `└─ Bedroom • Sonos Amp                           28%`

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use sonos_sdk::PlaybackState;

use crate::tui::theme::Theme;
use crate::tui::types::{
    DropZone, DropZoneData, DropZoneKind, ListEntry, SpeakerListData, SpeakerScreenData,
};
use crate::tui::widgets::volume_bar;

/// Render the speaker list from pre-computed data.
pub fn render(frame: &mut Frame, area: Rect, data: &SpeakerScreenData, theme: &Theme) {
    match data {
        SpeakerScreenData::Normal(list_data) => render_normal(frame, area, list_data, theme),
        SpeakerScreenData::PickUp(zone_data) => render_drop_zones(frame, area, zone_data, theme),
    }
}

// ===========================================================================
// Normal mode rendering
// ===========================================================================

fn render_normal(frame: &mut Frame, area: Rect, data: &SpeakerListData, theme: &Theme) {
    if data.entries.is_empty() {
        let paragraph = Paragraph::new("No speakers found")
            .alignment(ratatui::layout::Alignment::Center)
            .style(theme.muted);
        frame.render_widget(paragraph, area);
        return;
    }

    let selected_index = data
        .selected_index
        .min(data.entries.len().saturating_sub(1));

    let viewport_height = area.height as usize;
    let vol_width = 16.min(area.width.saturating_sub(50));
    let total_width = area.width as usize;
    let g = &theme.glyphs;

    // Cursor prefix width: "❯ " = 2 chars for selected, "  " for others
    let cursor_prefix_width = 2;

    let mut lines: Vec<Line> = Vec::new();
    let mut entry_visual_starts: Vec<usize> = Vec::new();
    let mut entry_visual_heights: Vec<usize> = Vec::new();

    for (idx, (entry, entry_data)) in data.entries.iter().zip(data.entry_data.iter()).enumerate() {
        let is_selected = idx == selected_index;

        // Insert blank separator + divider between groups (not before the first entry)
        let is_group_header = matches!(entry, ListEntry::GroupHeader(_));
        if idx > 0 && is_group_header {
            lines.push(Line::raw(""));
            // Divider line: +────────────────+
            let divider_inner_w = total_width.saturating_sub(cursor_prefix_width + 2 + 2);
            let divider = format!(
                "{}{}{}",
                g.divider_left,
                g.divider_fill.repeat(divider_inner_w),
                g.divider_right,
            );
            lines.push(Line::from(vec![
                Span::raw(" ".repeat(cursor_prefix_width + 2)),
                Span::styled(divider, theme.muted),
            ]));
            lines.push(Line::raw(""));
        }

        let start_line = lines.len();

        match entry {
            ListEntry::GroupHeader(_) => {
                // Single-line header with dot leaders:
                // [cursor] [icon] Name ::::: Track — Artist ::::: vol%
                let (icon, icon_style) = match &entry_data.playback_state {
                    Some(PlaybackState::Playing) => (g.playing, theme.playing_icon),
                    Some(PlaybackState::Paused) => (g.paused, theme.paused_icon),
                    _ => (g.stopped, theme.stopped_icon),
                };

                let name_style = if is_selected {
                    theme.speaker_cursor
                } else {
                    theme.group_header
                };

                let cursor_str = if is_selected { g.cursor } else { " " };

                let vol_text = format_volume(entry_data.group_volume);
                let vol_chars = vol_text.chars().count();

                // Fixed parts: cursor(1) + space(1) + icon(1) + space(1) + name + space(1) + vol + space(1)
                let name_chars = entry_data.name.chars().count();
                let icon_chars = icon.chars().count();

                let track_text = entry_data.track_info.as_deref().unwrap_or("");
                let track_chars = track_text.chars().count();

                // Available space for leaders and track info
                let fixed_chars = cursor_prefix_width + icon_chars + 1 + name_chars + 1 + vol_chars + 1;
                let available = total_width.saturating_sub(fixed_chars);

                let mut spans = vec![
                    Span::styled(cursor_str, theme.speaker_cursor),
                    Span::raw(" "),
                    Span::styled(format!("{icon} "), icon_style),
                    Span::styled(entry_data.name.clone(), name_style),
                    Span::raw(" "),
                ];

                if !track_text.is_empty() && available > track_chars + 4 {
                    // Leaders before track, track, leaders after track, then volume
                    let leaders_total = available.saturating_sub(track_chars + 2); // 2 spaces around track
                    let leaders_before = 5.min(leaders_total / 2);
                    let leaders_after = leaders_total.saturating_sub(leaders_before);

                    spans.push(Span::styled(
                        std::iter::repeat(g.leader_char).take(leaders_before).collect::<String>(),
                        theme.leader,
                    ));
                    spans.push(Span::styled(format!(" {track_text} "), theme.track_info));
                    spans.push(Span::styled(
                        std::iter::repeat(g.leader_char).take(leaders_after).collect::<String>(),
                        theme.leader,
                    ));
                } else {
                    // No track or not enough space: leaders fill to volume
                    spans.push(Span::styled(
                        std::iter::repeat(g.leader_char).take(available).collect::<String>(),
                        theme.leader,
                    ));
                }

                spans.push(Span::styled(format!(" {vol_text}"), theme.muted));

                lines.push(Line::from(spans));
                entry_visual_starts.push(start_line);
                entry_visual_heights.push(lines.len() - start_line);
            }
            ListEntry::SpeakerRow(_) => {
                let connector = if entry_data.is_last_in_group {
                    g.connector_last
                } else {
                    g.connector_branch
                };

                let name_style = if is_selected {
                    theme.speaker_cursor
                } else {
                    theme.speaker_name
                };

                let cursor_str = if is_selected { g.cursor } else { " " };

                let mut spans = vec![
                    Span::styled(cursor_str, theme.speaker_cursor),
                    Span::raw(" "),
                    Span::styled(" ".repeat(2), theme.muted), // indent to align with group name
                    Span::styled(connector, theme.muted),
                    Span::styled(entry_data.name.clone(), name_style),
                ];

                // Model name
                if let Some(ref model) = entry_data.model_name {
                    spans.push(Span::styled(g.model_separator.to_string(), theme.muted));
                    spans.push(Span::styled(model.clone(), theme.muted));
                }

                if let Some(vol) = entry_data.speaker_volume {
                    append_volume_spans(&mut spans, vol, is_selected, vol_width, total_width, theme);
                }

                lines.push(Line::from(spans));

                entry_visual_starts.push(start_line);
                entry_visual_heights.push(1);
            }
        }
    }

    if let Some(ref msg) = data.status_message {
        lines.push(Line::raw(""));
        let style = if msg.starts_with("error:") {
            theme.error
        } else {
            theme.accent
        };
        lines.push(Line::from(vec![Span::styled(format!(" {msg}"), style)]));
    }

    // Scrolling: find the visual position of the selected entry and ensure it's visible.
    let scroll_offset = if viewport_height == 0 || entry_visual_starts.is_empty() {
        0
    } else if selected_index < entry_visual_starts.len() {
        let top = entry_visual_starts[selected_index];
        let bottom = top + entry_visual_heights[selected_index];

        // Include blank separator + divider before group headers
        let visible_top = if selected_index > 0
            && matches!(&data.entries[selected_index], ListEntry::GroupHeader(_))
        {
            top.saturating_sub(3) // blank + divider + blank
        } else {
            top
        };

        if bottom <= viewport_height {
            0
        } else if visible_top + viewport_height >= bottom {
            visible_top
        } else {
            bottom.saturating_sub(viewport_height)
        }
    } else {
        0
    };

    let paragraph = Paragraph::new(lines).scroll((scroll_offset as u16, 0));
    frame.render_widget(paragraph, area);
}

// ===========================================================================
// Drop zone mode rendering
// ===========================================================================

fn render_drop_zones(frame: &mut Frame, area: Rect, data: &DropZoneData, theme: &Theme) {
    let width = area.width as usize;
    if width < 4 || area.height < 3 {
        return;
    }

    let g = &theme.glyphs;
    let mut lines: Vec<Line> = Vec::new();

    // Status line: "Picked up: {name}" left, "Space drop  Esc cancel" right
    let left = format!("Picked up: {}", data.speaker_name);
    let right = "\u{2423} drop  Esc cancel";
    let pad = width.saturating_sub(left.len() + right.len());
    lines.push(Line::from(vec![
        Span::styled(left, theme.accent),
        Span::raw(" ".repeat(pad)),
        Span::styled(right, theme.muted),
    ]));

    lines.push(Line::raw("")); // blank separator

    // Inner width for zone content (minus border chars and indent)
    let indent = 4; // align with speaker list indent
    let inner_w = width.saturating_sub(indent + 2); // -2 for border chars

    for (i, zone) in data.zones.iter().enumerate() {
        let is_active = i == data.active_zone_index;

        let border_style = if is_active { theme.accent } else { theme.muted };
        let header_style = if is_active {
            theme.accent
        } else {
            theme.group_header
        };

        // Group header line (above the box)
        lines.push(Line::from(vec![Span::styled(
            zone.group_name.clone(),
            header_style,
        )]));

        let horiz_fill = g.zone_horiz.repeat(inner_w);

        // Top border: ╭────────╮
        lines.push(Line::from(vec![
            Span::raw(" ".repeat(indent)),
            Span::styled(g.zone_tl.to_string(), border_style),
            Span::styled(horiz_fill.clone(), border_style),
            Span::styled(g.zone_tr.to_string(), border_style),
        ]));

        // Inner lines
        let inner_height = zone.inner_height.max(1);
        let label = build_zone_label(zone);
        let content_style = if is_active { theme.accent } else { theme.muted };

        for row in 0..inner_height {
            let content = if row == 0 {
                center_label(&label, inner_w)
            } else {
                " ".repeat(inner_w)
            };

            lines.push(Line::from(vec![
                Span::raw(" ".repeat(indent)),
                Span::styled(g.zone_vert.to_string(), border_style),
                Span::styled(content, content_style),
                Span::styled(g.zone_vert.to_string(), border_style),
            ]));
        }

        // Bottom border: ╰────────╯
        lines.push(Line::from(vec![
            Span::raw(" ".repeat(indent)),
            Span::styled(g.zone_bl.to_string(), border_style),
            Span::styled(horiz_fill, border_style),
            Span::styled(g.zone_br.to_string(), border_style),
        ]));

        lines.push(Line::raw(""));
    }

    // Status message if present
    if let Some(ref msg) = data.status_message {
        let style = if msg.starts_with("error:") {
            theme.error
        } else {
            theme.accent
        };
        lines.push(Line::from(vec![Span::styled(format!(" {msg}"), style)]));
    }

    // Compute scroll offset to keep active zone visible
    let viewport_height = area.height;
    let scroll_offset = compute_scroll_offset(&data.zones, data.active_zone_index, viewport_height);

    let paragraph = Paragraph::new(lines).scroll((scroll_offset, 0));
    frame.render_widget(paragraph, area);
}

/// Build the label text for a drop zone.
fn build_zone_label(zone: &DropZone) -> String {
    match &zone.kind {
        DropZoneKind::NewGroup => "Add new group".to_string(),
        DropZoneKind::ExistingGroup(_) => "Drop Speaker".to_string(),
    }
}

/// Center a label within a given width, padding with spaces.
fn center_label(label: &str, width: usize) -> String {
    let label_len = label.chars().count();
    if label_len >= width {
        return label.chars().take(width).collect();
    }
    let pad_left = (width - label_len) / 2;
    let pad_right = width - label_len - pad_left;
    format!(
        "{}{}{}",
        " ".repeat(pad_left),
        label,
        " ".repeat(pad_right)
    )
}

/// Compute a scroll offset so the active zone is visible.
fn compute_scroll_offset(zones: &[DropZone], active_index: usize, viewport_height: u16) -> u16 {
    // 2 lines for status + blank, then per zone: header + top border + inner_height + bottom border + blank
    let mut line = 2u16;
    let mut active_start = 0u16;
    let mut active_end = 0u16;

    for (i, zone) in zones.iter().enumerate() {
        let zone_start = line;
        let zone_lines = 1 + 1 + (zone.inner_height.max(1) as u16) + 1 + 1;
        let zone_end = zone_start + zone_lines;

        if i == active_index {
            active_start = zone_start;
            active_end = zone_end;
        }
        line = zone_end;
    }

    if active_end <= viewport_height {
        return 0;
    }
    if active_start == 0 {
        return 0;
    }

    // Center the active zone in the viewport
    let zone_height = active_end - active_start;
    if zone_height < viewport_height {
        active_start.saturating_sub((viewport_height - zone_height) / 2)
    } else {
        active_start
    }
}

// ===========================================================================
// Shared helpers
// ===========================================================================

/// Format volume as a fixed 4-char right-aligned string: `" 0%"`, `"60%"`, `"100%"`.
fn format_volume(vol: Option<u16>) -> String {
    match vol {
        Some(v) => format!("{v:>3}%"),
        None => "   -".to_string(),
    }
}

/// Append right-aligned volume bar (when selected) or percentage text to a span list.
fn append_volume_spans(
    spans: &mut Vec<Span>,
    vol: u16,
    is_selected: bool,
    bar_width: u16,
    total_width: usize,
    theme: &Theme,
) {
    // Calculate current content width
    let content_width: usize = spans.iter().map(|s| s.content.chars().count()).sum();

    if is_selected {
        // Right-align the volume bar
        let vol_display_width = bar_width as usize;
        let pad = total_width.saturating_sub(content_width + vol_display_width);
        if pad > 0 {
            spans.push(Span::raw(" ".repeat(pad)));
        }
        let vol_line =
            volume_bar::render_volume_bar(vol, bar_width, theme.volume_filled, theme.volume_empty);
        spans.extend(vol_line.spans);
    } else {
        // Right-align the volume percentage
        let vol_text = format!("{vol:>3}%");
        let vol_chars = vol_text.chars().count();
        let pad = total_width.saturating_sub(content_width + vol_chars);
        if pad > 0 {
            spans.push(Span::raw(" ".repeat(pad)));
        }
        spans.push(Span::styled(vol_text, theme.muted));
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_label_centers_text() {
        let s = center_label("hi", 10);
        assert_eq!(s.chars().count(), 10);
        assert!(s.contains("hi"));
    }

    #[test]
    fn center_label_long_label() {
        let s = center_label("this is a very long label", 10);
        assert_eq!(s.chars().count(), 10);
    }

    #[test]
    fn build_zone_label_new_group() {
        let zone = DropZone {
            kind: DropZoneKind::NewGroup,
            group_name: "Add new group".to_string(),
            remaining_members: Vec::new(),
            inner_height: 1,
        };
        assert_eq!(build_zone_label(&zone), "Add new group");
    }

    #[test]
    fn build_zone_label_existing_group() {
        let zone = DropZone {
            kind: DropZoneKind::ExistingGroup(sonos_sdk::GroupId::new("test:1")),
            group_name: "Living Room".to_string(),
            remaining_members: Vec::new(),
            inner_height: 1,
        };
        assert_eq!(build_zone_label(&zone), "Drop Speaker");
    }

    #[test]
    fn scroll_offset_no_scroll_needed() {
        let zones = vec![DropZone {
            kind: DropZoneKind::NewGroup,
            group_name: "Test".to_string(),
            remaining_members: Vec::new(),
            inner_height: 1,
        }];
        let offset = compute_scroll_offset(&zones, 0, 20);
        assert_eq!(offset, 0);
    }
}
