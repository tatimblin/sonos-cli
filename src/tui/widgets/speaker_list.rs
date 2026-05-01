//! Speaker list widget — render-only component.
//!
//! Takes pre-computed `SpeakerListData` and `Theme`, outputs to frame.
//! No hooks, no SDK, no key handling.
//!
//! ## Visual layout
//!
//! Group headers span 2 visual lines:
//!   Line 1: `▶ Group Name          60%  ◀` (icon + name + volume number + cursor)
//!   Line 2: `  Song Title — Artist`        (track info, indented)
//!
//! Speaker rows use tree connectors:
//!   `├ Speaker Name        ■■■■■···· 40%  ◀` (non-last member, volume bar when selected)
//!   `└ Speaker Name        50%`               (last member, volume number when not selected)
//!
//! Blank lines separate groups. Scrolling keeps the selected entry visible.

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use sonos_sdk::PlaybackState;

use crate::tui::theme::Theme;
use crate::tui::types::{build_display_order, ListEntry, SpeakerListData};
use crate::tui::widgets::volume_bar;

/// Render the speaker list from pre-computed data.
pub fn render(frame: &mut Frame, area: Rect, data: &SpeakerListData, theme: &Theme) {
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
    let is_pick_up = data.pick_up.is_some();
    let pick_up_speaker_id = data.pick_up.as_ref().map(|p| p.speaker_id.clone());

    let display_order = build_display_order(&data.entries, &data.pick_up);

    let viewport_height = area.height as usize;
    let vol_width = 16.min(area.width.saturating_sub(50));
    let total_width = area.width as usize;

    // Build visual lines with entry index tracking for scroll calculation.
    // Each entry maps to one or more visual lines. We track which visual line
    // each entry starts at, and how tall it is.
    let mut lines: Vec<Line> = Vec::new();
    let mut entry_visual_starts: Vec<usize> = Vec::new(); // visual line where each display_order entry starts
    let mut entry_visual_heights: Vec<usize> = Vec::new();

    for (display_pos, &orig_idx) in display_order.iter().enumerate() {
        let entry = &data.entries[orig_idx];
        let entry_data = &data.entry_data[orig_idx];

        let is_selected = if is_pick_up {
            pick_up_speaker_id
                .as_ref()
                .is_some_and(|pid| matches!(entry, ListEntry::SpeakerRow(sid) if sid == pid))
        } else {
            orig_idx == selected_index
        };

        // Insert blank separator line between groups (not before the first entry)
        let is_group_header = matches!(entry, ListEntry::GroupHeader(_));
        if display_pos > 0 && is_group_header {
            lines.push(Line::raw(""));
        }

        let start_line = lines.len();

        match entry {
            ListEntry::GroupHeader(_) => {
                // Line 1: icon + name + volume number + cursor indicator
                let (icon, icon_style) = match &entry_data.playback_state {
                    Some(PlaybackState::Playing) => ("\u{25b6} ", theme.playing_icon),
                    Some(PlaybackState::Paused) => ("\u{23f8} ", theme.paused_icon),
                    _ => ("\u{25a0} ", theme.stopped_icon),
                };

                let name_style = if is_selected {
                    theme.speaker_cursor
                } else {
                    theme.group_header
                };

                let vol_text = format_volume(entry_data.group_volume);

                let mut spans = vec![
                    Span::styled(icon, icon_style),
                    Span::styled(entry_data.name.clone(), name_style),
                ];

                spans.push(Span::styled(format!("  {vol_text}"), theme.muted));

                if is_selected {
                    append_cursor_indicator(&mut spans, total_width, theme);
                }

                lines.push(Line::from(spans));

                // Line 2: track info (indented to align with name after icon)
                let track_info = entry_data.track_info.as_deref().unwrap_or("");
                if !track_info.is_empty() {
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(track_info.to_string(), theme.track_info),
                    ]));
                } else {
                    // Always emit 2 lines for group headers for consistent layout
                    lines.push(Line::raw(""));
                }

                entry_visual_starts.push(start_line);
                entry_visual_heights.push(lines.len() - start_line);
            }
            ListEntry::SpeakerRow(_) => {
                let connector = if entry_data.is_last_in_group {
                    "\u{2514} " // └
                } else {
                    "\u{251c} " // ├
                };

                let name_style = if is_selected {
                    theme.speaker_cursor
                } else {
                    theme.speaker_name
                };

                let mut spans = vec![
                    Span::styled(connector, theme.muted),
                    Span::styled(entry_data.name.clone(), name_style),
                ];

                if let Some(vol) = entry_data.speaker_volume {
                    append_volume_spans(&mut spans, vol, is_selected, vol_width, theme);
                }

                if is_selected {
                    append_cursor_indicator(&mut spans, total_width, theme);
                }

                lines.push(Line::from(spans));

                entry_visual_starts.push(start_line);
                entry_visual_heights.push(1);
            }
        }
    }

    // Append pick-up mode hint
    if let Some(ref pick_up) = data.pick_up {
        let name = data
            .entries
            .iter()
            .zip(data.entry_data.iter())
            .find(|(e, _)| matches!(e, ListEntry::SpeakerRow(sid) if *sid == pick_up.speaker_id))
            .map(|(_, d)| d.name.as_str())
            .unwrap_or("Speaker");
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![Span::styled(
            format!(" Moving {name} \u{2014} Space to drop, Esc to cancel"),
            theme.accent,
        )]));
    }

    // Append status message
    if data.pick_up.is_none() {
        if let Some(ref msg) = data.status_message {
            lines.push(Line::raw(""));
            let style = if msg.starts_with("error:") {
                theme.error
            } else {
                theme.accent
            };
            lines.push(Line::from(vec![Span::styled(format!(" {msg}"), style)]));
        }
    }

    // Scrolling: find the display position of the selected entry and ensure it's visible.
    let scroll_offset = if viewport_height == 0 || entry_visual_starts.is_empty() {
        0
    } else {
        let selected_pos = display_order.iter().enumerate().find_map(|(pos, &oi)| {
            let matched = if is_pick_up {
                pick_up_speaker_id.as_ref().is_some_and(
                    |pid| matches!(&data.entries[oi], ListEntry::SpeakerRow(sid) if sid == pid),
                )
            } else {
                oi == selected_index
            };
            matched.then_some(pos)
        });

        match selected_pos {
            Some(pos) if pos < entry_visual_starts.len() => {
                let top = entry_visual_starts[pos];
                let bottom = top + entry_visual_heights[pos];

                // Include blank separator before group headers
                let visible_top = if pos > 0
                    && matches!(&data.entries[display_order[pos]], ListEntry::GroupHeader(_))
                {
                    top.saturating_sub(1)
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
            }
            _ => 0,
        }
    };

    let paragraph = Paragraph::new(lines).scroll((scroll_offset as u16, 0));
    frame.render_widget(paragraph, area);
}

/// Append right-aligned `◀` cursor indicator with padding to fill the row.
fn append_cursor_indicator(spans: &mut Vec<Span>, total_width: usize, theme: &Theme) {
    let content_width: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    let cursor_width = 2; // " ◀"
    let pad = total_width.saturating_sub(content_width + cursor_width);
    if pad > 0 {
        spans.push(Span::raw(" ".repeat(pad)));
    }
    spans.push(Span::styled("\u{25c0}", theme.speaker_cursor));
}

/// Format volume as a fixed 4-char right-aligned string: `" 0%"`, `"60%"`, `"100%"`.
fn format_volume(vol: Option<u16>) -> String {
    match vol {
        Some(v) => format!("{v:>3}%"),
        None => "   -".to_string(),
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
        spans.push(Span::styled(format!("{vol:>3}%"), theme.muted));
    }
}
