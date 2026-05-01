//! Speaker list widget — render-only component.
//!
//! Takes pre-computed `SpeakerListData` and `Theme`, outputs to frame.
//! No hooks, no SDK, no key handling.

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

    let vol_width = 16.min(area.width.saturating_sub(50));

    let mut lines: Vec<Line> = Vec::new();

    for &orig_idx in &display_order {
        let entry = &data.entries[orig_idx];
        let entry_data = &data.entry_data[orig_idx];

        let is_selected = if is_pick_up {
            pick_up_speaker_id
                .as_ref()
                .is_some_and(|pid| matches!(entry, ListEntry::SpeakerRow(sid) if sid == pid))
        } else {
            orig_idx == selected_index
        };

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

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
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
