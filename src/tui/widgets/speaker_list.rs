//! Speaker list widget — render-only component.
//!
//! Takes pre-computed `SpeakerListData` and `Theme`, outputs to frame.
//! No hooks, no SDK, no key handling.
//!
//! Group headers are single-line with dot leader fill:
//!   `▶ Bedroom ::::: BOY IN RED — Artist ::::::::::::: 30%`
//!   `■ Kitchen ::::::::::::::::::::::::::::::::::::::: 21%`
//!
//! Speaker rows use tree connectors with model names:
//!   `├─ Bathroom • Sonos Connect:AMP                  46%`
//!   `└─ Bedroom • Sonos Amp                           28%`
//!
//! During pickup mode, inline action rows appear:
//!   `└─ Add to group`       (selectable drop target)
//!   `└─ Already in group`   (dimmed, home group)
//!   `► Create new group`    (standalone target)

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use sonos_sdk::PlaybackState;

use crate::tui::theme::Theme;
use crate::tui::types::{ListEntry, SpeakerListData};
use crate::tui::widgets::volume_bar;

/// Render the speaker list from pre-computed data.
pub fn render(frame: &mut Frame, area: Rect, data: &SpeakerListData, theme: &Theme) {
    render_normal(frame, area, data, theme);
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

    let cursor_prefix_width = 2;
    let is_pickup = data.picked_up_speaker_id.is_some();

    let mut lines: Vec<Line> = Vec::new();
    let mut entry_visual_starts: Vec<usize> = Vec::new();
    let mut entry_visual_heights: Vec<usize> = Vec::new();

    // Status line during pickup mode
    if let Some(ref speaker_name) = data.picked_up_speaker_name {
        let left = format!("Picked up: {speaker_name}");
        let right = "\u{2423} drop  Esc cancel";
        let pad = total_width.saturating_sub(left.len() + right.len());
        lines.push(Line::from(vec![
            Span::styled(left, theme.accent),
            Span::raw(" ".repeat(pad)),
            Span::styled(right, theme.muted),
        ]));
        lines.push(Line::raw(""));
    }

    for (idx, (entry, entry_data)) in data.entries.iter().zip(data.entry_data.iter()).enumerate() {
        let is_selected = idx == selected_index;

        // Insert blank separator + divider between groups (not before the first entry)
        let is_group_header = matches!(entry, ListEntry::GroupHeader(_));
        if idx > 0 && is_group_header {
            lines.push(Line::raw(""));
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

                let name_chars = entry_data.name.chars().count();
                let icon_chars = icon.chars().count();

                let track_text = entry_data.track_info.as_deref().unwrap_or("");
                let track_chars = track_text.chars().count();

                let fixed_chars =
                    cursor_prefix_width + icon_chars + 1 + name_chars + 1 + vol_chars + 1;
                let available = total_width.saturating_sub(fixed_chars);

                let mut spans = vec![
                    Span::styled(cursor_str, theme.speaker_cursor),
                    Span::raw(" "),
                    Span::styled(format!("{icon} "), icon_style),
                    Span::styled(entry_data.name.clone(), name_style),
                    Span::raw(" "),
                ];

                if !track_text.is_empty() && available > track_chars + 4 {
                    let leaders_total = available.saturating_sub(track_chars + 2);
                    let leaders_before = 5.min(leaders_total / 2);
                    let leaders_after = leaders_total.saturating_sub(leaders_before);

                    spans.push(Span::styled(
                        std::iter::repeat_n(g.leader_char, leaders_before).collect::<String>(),
                        theme.leader,
                    ));
                    spans.push(Span::styled(format!(" {track_text} "), theme.track_info));
                    spans.push(Span::styled(
                        std::iter::repeat_n(g.leader_char, leaders_after).collect::<String>(),
                        theme.leader,
                    ));
                } else {
                    spans.push(Span::styled(
                        std::iter::repeat_n(g.leader_char, available).collect::<String>(),
                        theme.leader,
                    ));
                }

                spans.push(Span::styled(format!(" {vol_text}"), theme.muted));

                lines.push(Line::from(spans));
                entry_visual_starts.push(start_line);
                entry_visual_heights.push(lines.len() - start_line);
            }
            ListEntry::SpeakerRow(speaker_id) => {
                let connector = if entry_data.is_last_in_group {
                    g.connector_last
                } else {
                    g.connector_branch
                };

                let is_picked_up = data
                    .picked_up_speaker_id
                    .as_ref()
                    .is_some_and(|id| id == speaker_id);

                let name_style = if is_picked_up {
                    theme.picked_up
                } else if is_selected {
                    theme.speaker_cursor
                } else {
                    theme.speaker_name
                };

                let cursor_str = if is_selected { g.cursor } else { " " };

                let mut spans = vec![
                    Span::styled(cursor_str, theme.speaker_cursor),
                    Span::raw(" "),
                    Span::styled(" ".repeat(2), theme.muted),
                ];

                if is_picked_up {
                    spans.push(Span::styled(connector, theme.picked_up));
                    spans.push(Span::styled(entry_data.name.clone(), theme.picked_up));
                    if let Some(ref model) = entry_data.model_name {
                        spans.push(Span::styled(
                            g.model_separator.to_string(),
                            theme.picked_up,
                        ));
                        spans.push(Span::styled(model.clone(), theme.picked_up));
                    }
                    let content_width: usize =
                        spans.iter().map(|s| s.content.chars().count()).sum();
                    let remaining = total_width.saturating_sub(content_width);
                    if remaining > 0 {
                        spans.push(Span::styled(" ".repeat(remaining), theme.picked_up));
                    }
                } else {
                    spans.push(Span::styled(connector, theme.muted));
                    spans.push(Span::styled(entry_data.name.clone(), name_style));

                    if let Some(ref model) = entry_data.model_name {
                        spans.push(Span::styled(g.model_separator.to_string(), theme.muted));
                        spans.push(Span::styled(model.clone(), theme.muted));
                    }

                    if let Some(vol) = entry_data.speaker_volume {
                        append_volume_spans(
                            &mut spans,
                            vol,
                            is_selected && !is_pickup,
                            vol_width,
                            total_width,
                            theme,
                        );
                    }
                }

                lines.push(Line::from(spans));
                entry_visual_starts.push(start_line);
                entry_visual_heights.push(1);
            }
            ListEntry::AddToGroupRow(_) => {
                let cursor_str = if is_selected { g.cursor } else { " " };

                let style = if entry_data.is_home_group {
                    theme.muted
                } else if is_selected {
                    theme.accent
                } else {
                    theme.muted
                };

                let cursor_style = if is_selected && !entry_data.is_home_group {
                    theme.speaker_cursor
                } else {
                    theme.muted
                };

                lines.push(Line::from(vec![
                    Span::styled(cursor_str, cursor_style),
                    Span::raw(" "),
                    Span::styled(" ".repeat(2), theme.muted),
                    Span::styled(g.connector_last, style),
                    Span::styled(entry_data.name.clone(), style),
                ]));

                entry_visual_starts.push(start_line);
                entry_visual_heights.push(1);
            }
            ListEntry::CreateNewGroupRow => {
                // Divider before "Create new group"
                lines.push(Line::raw(""));
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

                let cursor_str = if is_selected { g.cursor } else { " " };
                let style = if is_selected {
                    theme.accent
                } else {
                    theme.muted
                };
                let cursor_style = if is_selected {
                    theme.speaker_cursor
                } else {
                    theme.muted
                };

                lines.push(Line::from(vec![
                    Span::styled(cursor_str, cursor_style),
                    Span::raw(" "),
                    Span::styled(" ".repeat(2), theme.muted),
                    Span::styled(format!("{} ", g.tab_active_indicator), style),
                    Span::styled(entry_data.name.clone(), style),
                ]));

                entry_visual_starts.push(start_line);
                entry_visual_heights.push(lines.len() - start_line);
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

    let scroll_offset = if viewport_height == 0 || entry_visual_starts.is_empty() {
        0
    } else if selected_index < entry_visual_starts.len() {
        let top = entry_visual_starts[selected_index];
        let bottom = top + entry_visual_heights[selected_index];

        let visible_top = if selected_index > 0
            && matches!(&data.entries[selected_index], ListEntry::GroupHeader(_))
        {
            top.saturating_sub(3)
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

