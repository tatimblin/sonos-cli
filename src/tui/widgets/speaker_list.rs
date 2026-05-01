//! Speaker list widget — render-only component.
//!
//! Takes pre-computed `SpeakerScreenData` and `Theme`, outputs to frame.
//! No hooks, no SDK, no key handling.
//!
//! Two rendering modes:
//! - **Normal**: flat list of group headers and speaker rows with volume bars
//! - **Drop zone**: collapsed groups shown as bordered drop zones during pick-up

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

    let vol_width = 16.min(area.width.saturating_sub(50));

    let mut lines: Vec<Line> = Vec::new();

    for (idx, (entry, entry_data)) in data.entries.iter().zip(data.entry_data.iter()).enumerate() {
        let is_selected = idx == selected_index;

        match entry {
            ListEntry::GroupHeader(_) => {
                let (icon, icon_style) = match &entry_data.playback_state {
                    Some(PlaybackState::Playing) => ("\u{25b6} ", theme.playing_icon),
                    Some(PlaybackState::Paused) => ("\u{23f8} ", theme.paused_icon),
                    _ => ("\u{25a0} ", theme.stopped_icon),
                };

                let track_info = entry_data.track_info.as_deref().unwrap_or("");

                let name_style = if is_selected {
                    theme.speaker_cursor
                } else {
                    theme.group_header
                };

                let mut spans = vec![
                    Span::styled(icon, icon_style),
                    Span::styled(entry_data.name.clone(), name_style),
                ];

                if !track_info.is_empty() {
                    spans.push(Span::raw("  "));
                    spans.push(Span::styled(track_info.to_string(), theme.track_info));
                }

                if let Some(vol) = entry_data.group_volume {
                    append_volume_spans(&mut spans, vol, is_selected, vol_width, theme);
                }

                lines.push(Line::from(spans));
            }
            ListEntry::SpeakerRow(_) => {
                let cursor = if is_selected { "  \u{25b8} " } else { "    " };

                let name_style = if is_selected {
                    theme.speaker_cursor
                } else {
                    theme.speaker_name
                };

                let mut spans = vec![
                    Span::styled(cursor.to_string(), name_style),
                    Span::styled(entry_data.name.clone(), name_style),
                ];

                if let Some(vol) = entry_data.speaker_volume {
                    append_volume_spans(&mut spans, vol, is_selected, vol_width, theme);
                }

                lines.push(Line::from(spans));
            }
            ListEntry::UngroupedHeader => {
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![Span::styled(
                    " NOT IN A GROUP ",
                    theme.group_header,
                )]));
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

    let paragraph = Paragraph::new(lines);
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

    // Inner width for zone content (minus border chars on each side)
    let inner_w = width.saturating_sub(2);

    for (i, zone) in data.zones.iter().enumerate() {
        let is_active = i == data.active_zone_index;

        // Active: solid heavy border ┏━┓┃┗━┛
        // Inactive: dashed light border ┎┄┒┆┖┄┚
        let (tl, horiz, tr, vert, bl, br) = if is_active {
            (
                "\u{250F}", "\u{2501}", "\u{2513}", "\u{2503}", "\u{2517}", "\u{251B}",
            )
        } else {
            (
                "\u{250E}", "\u{2504}", "\u{2512}", "\u{2506}", "\u{2516}", "\u{2504}",
            )
        };

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

        // Horizontal border fill (reused for top and bottom)
        let horiz_fill = horiz.repeat(inner_w);

        // Top border
        lines.push(Line::from(vec![
            Span::styled(tl.to_string(), border_style),
            Span::styled(horiz_fill.clone(), border_style),
            Span::styled(tr.to_string(), border_style),
        ]));

        // Inner lines with stripe fill
        let inner_height = zone.inner_height.max(1);
        let label = build_zone_label(zone);
        let content_style = if is_active { theme.accent } else { theme.muted };

        for row in 0..inner_height {
            let content = if row == 0 {
                label_over_stripes(&label, inner_w)
            } else {
                stripe_line(inner_w)
            };

            let right_indicator = if is_active && row == 0 {
                Span::styled("\u{25C0}", theme.accent)
            } else {
                Span::styled(vert.to_string(), border_style)
            };

            lines.push(Line::from(vec![
                Span::styled(vert.to_string(), border_style),
                Span::styled(content, content_style),
                right_indicator,
            ]));
        }

        // Bottom border
        lines.push(Line::from(vec![
            Span::styled(bl.to_string(), border_style),
            Span::styled(horiz_fill, border_style),
            Span::styled(br.to_string(), border_style),
        ]));

        // Spacing between zones
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
        DropZoneKind::ExistingGroup(_) => {
            if zone.remaining_members.is_empty() {
                "Drop here (empty)".to_string()
            } else {
                let members = zone.remaining_members.join(" + ");
                format!("Drop here \u{2014} {members}")
            }
        }
    }
}

/// Generate a stripe line of the given width using diagonal backslash characters.
fn stripe_line(width: usize) -> String {
    let pattern = "\u{2572} "; // followed by space
    let repeated = pattern.repeat(width / 2 + 1);
    repeated.chars().take(width).collect()
}

/// Generate a stripe line with a centered label cut into it.
fn label_over_stripes(label: &str, width: usize) -> String {
    if label.len() >= width {
        return label.chars().take(width).collect();
    }
    let stripes = stripe_line(width);
    let pad = (width.saturating_sub(label.len())) / 2;
    let mut chars: Vec<char> = stripes.chars().collect();
    for (i, ch) in label.chars().enumerate() {
        let pos = pad + i;
        if pos < chars.len() {
            chars[pos] = ch;
        }
    }
    chars.into_iter().collect()
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

/// Append volume bar (when selected) or percentage text to a span list.
fn append_volume_spans(
    spans: &mut Vec<Span>,
    vol: u16,
    is_selected: bool,
    width: u16,
    theme: &Theme,
) {
    spans.push(Span::raw("  "));
    if is_selected {
        let vol_line =
            volume_bar::render_volume_bar(vol, width, theme.volume_filled, theme.volume_empty);
        spans.extend(vol_line.spans);
    } else {
        spans.push(Span::styled(format!("{vol}%"), theme.muted));
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stripe_line_produces_alternating_pattern() {
        let s = stripe_line(6);
        assert_eq!(s, "\u{2572} \u{2572} \u{2572} ");
    }

    #[test]
    fn stripe_line_odd_width() {
        let s = stripe_line(5);
        assert_eq!(s.chars().count(), 5);
    }

    #[test]
    fn stripe_line_zero() {
        let s = stripe_line(0);
        assert!(s.is_empty());
    }

    #[test]
    fn label_over_stripes_centers_text() {
        let s = label_over_stripes("hi", 10);
        assert_eq!(s.chars().count(), 10);
        assert!(s.contains("hi"));
    }

    #[test]
    fn label_over_stripes_long_label() {
        let s = label_over_stripes("this is a very long label", 10);
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
    fn build_zone_label_empty_group() {
        let zone = DropZone {
            kind: DropZoneKind::ExistingGroup(sonos_sdk::GroupId::new("test:1")),
            group_name: "Living Room".to_string(),
            remaining_members: Vec::new(),
            inner_height: 1,
        };
        assert_eq!(build_zone_label(&zone), "Drop here (empty)");
    }

    #[test]
    fn build_zone_label_with_members() {
        let zone = DropZone {
            kind: DropZoneKind::ExistingGroup(sonos_sdk::GroupId::new("test:1")),
            group_name: "Living Room".to_string(),
            remaining_members: vec!["Kitchen".to_string(), "Bedroom".to_string()],
            inner_height: 2,
        };
        assert_eq!(
            build_zone_label(&zone),
            "Drop here \u{2014} Kitchen + Bedroom"
        );
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
