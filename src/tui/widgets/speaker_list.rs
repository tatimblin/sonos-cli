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
//! Muted speakers and groups show the mute glyph before the volume:
//!   `├─ Bathroom • Sonos Connect:AMP                × 46%`
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
use crate::tui::types::{group_for_entry, ListEntry, SpeakerListData};
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
    let focused_group = group_for_entry(&data.entries, selected_index);

    let mut lines: Vec<Line> = Vec::new();
    let mut entry_visual_starts: Vec<usize> = Vec::new();
    let mut entry_visual_heights: Vec<usize> = Vec::new();

    for (idx, (entry, entry_data)) in data.entries.iter().zip(data.entry_data.iter()).enumerate() {
        let is_selected = idx == selected_index;

        // Insert blank separator + divider between groups (not before the first entry)
        let is_group_header = matches!(entry, ListEntry::GroupHeader(_));
        if idx > 0 && is_group_header {
            lines.push(Line::raw(""));
            let divider_inner_w = total_width.saturating_sub(cursor_prefix_width + 2 + 2);
            let left_pad = divider_inner_w / 2;
            let right_pad = divider_inner_w.saturating_sub(left_pad + 1);
            let divider = format!(
                "{}{}{}{}{}",
                g.divider_left,
                " ".repeat(left_pad),
                g.divider_fill,
                " ".repeat(right_pad),
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
            ListEntry::GroupHeader(group_id) => {
                let is_focused_group = focused_group.as_ref().is_some_and(|fg| fg == group_id);

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

                let leader_style = if is_focused_group {
                    theme.accent_secondary
                } else {
                    theme.leader
                };

                let cursor_str = if is_selected { g.cursor } else { " " };

                let vol_text = format_volume(entry_data.group_volume);
                let mute_text = if entry_data.is_muted {
                    format!(" {}", g.muted)
                } else {
                    String::new()
                };
                let vol_chars = vol_text.chars().count() + mute_text.chars().count();

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
                        leader_style,
                    ));
                    spans.push(Span::styled(format!(" {track_text} "), theme.track_info));
                    spans.push(Span::styled(
                        std::iter::repeat_n(g.leader_char, leaders_after).collect::<String>(),
                        leader_style,
                    ));
                } else {
                    spans.push(Span::styled(
                        std::iter::repeat_n(g.leader_char, available).collect::<String>(),
                        leader_style,
                    ));
                }

                if !mute_text.is_empty() {
                    spans.push(Span::styled(mute_text, theme.error));
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
                        spans.push(Span::styled(g.model_separator.to_string(), theme.picked_up));
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
                            entry_data.is_muted,
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
                let left_pad = divider_inner_w / 2;
                let right_pad = divider_inner_w.saturating_sub(left_pad + 1);
                let divider = format!(
                    "{}{}{}{}{}",
                    g.divider_left,
                    " ".repeat(left_pad),
                    g.divider_fill,
                    " ".repeat(right_pad),
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
///
/// When `is_muted`, the mute glyph precedes the volume and the bar renders dimmed.
fn append_volume_spans(
    spans: &mut Vec<Span>,
    vol: u16,
    is_muted: bool,
    is_selected: bool,
    bar_width: u16,
    total_width: usize,
    theme: &Theme,
) {
    // Calculate current content width
    let content_width: usize = spans.iter().map(|s| s.content.chars().count()).sum();

    let mute_text = if is_muted {
        format!("{} ", theme.glyphs.muted)
    } else {
        String::new()
    };
    let mute_chars = mute_text.chars().count();

    let vol_spans = if is_selected {
        // Right-align the volume bar
        let filled = if is_muted {
            theme.volume_empty
        } else {
            theme.volume_filled
        };
        volume_bar::render_volume_bar(vol, bar_width, filled, theme.volume_empty).spans
    } else {
        // Right-align the volume percentage
        vec![Span::styled(format!("{vol:>3}%"), theme.muted)]
    };
    let vol_display_width: usize = vol_spans.iter().map(|s| s.content.chars().count()).sum();

    let pad = total_width.saturating_sub(content_width + mute_chars + vol_display_width);
    if pad > 0 {
        spans.push(Span::raw(" ".repeat(pad)));
    }
    if is_muted {
        spans.push(Span::styled(mute_text, theme.error));
    }
    spans.extend(vol_spans);
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::types::EntryRenderData;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use sonos_sdk::{GroupId, SpeakerId};

    const WIDTH: u16 = 80;

    fn gid(n: u8) -> GroupId {
        GroupId::new(format!("RINCON_{n:03}:1"))
    }

    fn sid(n: u8) -> SpeakerId {
        SpeakerId::new(format!("RINCON_{n:03}"))
    }

    fn header(name: &str, volume: u16) -> EntryRenderData {
        EntryRenderData {
            name: name.to_string(),
            model_name: None,
            speaker_volume: None,
            group_volume: Some(volume),
            is_muted: false,
            playback_state: Some(PlaybackState::Stopped),
            track_info: None,
            is_last_in_group: false,
            is_home_group: false,
        }
    }

    fn row(name: &str, model: &str, volume: u16, is_last: bool) -> EntryRenderData {
        EntryRenderData {
            name: name.to_string(),
            model_name: Some(model.to_string()),
            speaker_volume: Some(volume),
            group_volume: None,
            is_muted: false,
            playback_state: None,
            track_info: None,
            is_last_in_group: is_last,
            is_home_group: false,
        }
    }

    fn action(name: &str, is_home: bool) -> EntryRenderData {
        EntryRenderData {
            name: name.to_string(),
            model_name: None,
            speaker_volume: None,
            group_volume: None,
            is_muted: false,
            playback_state: None,
            track_info: None,
            is_last_in_group: true,
            is_home_group: is_home,
        }
    }

    /// One group "Kitchen" (playing, 30%) with members Kitchen (46%) and
    /// Bathroom (28%). The header is selected unless a test changes it.
    fn kitchen_group() -> SpeakerListData {
        let mut kitchen = header("Kitchen", 30);
        kitchen.playback_state = Some(PlaybackState::Playing);
        kitchen.track_info = Some("Boy In Red \u{2014} Artist".to_string());
        SpeakerListData {
            entries: vec![
                ListEntry::GroupHeader(gid(0)),
                ListEntry::SpeakerRow(sid(0)),
                ListEntry::SpeakerRow(sid(1)),
            ],
            entry_data: vec![
                kitchen,
                row("Kitchen", "Sonos One", 46, false),
                row("Bathroom", "Sonos Amp", 28, true),
            ],
            selected_index: 0,
            picked_up_speaker_id: None,
        }
    }

    /// Render into a `WIDTH`-column test terminal and return each row as text
    /// with trailing blanks trimmed.
    fn render_lines(data: &SpeakerListData) -> Vec<String> {
        let height = 20;
        let mut terminal = Terminal::new(TestBackend::new(WIDTH, height)).unwrap();
        let theme = Theme::default_theme();
        terminal
            .draw(|frame| render(frame, frame.area(), data, &theme))
            .unwrap();
        let buffer = terminal.backend().buffer();
        (0..height)
            .map(|y| {
                (0..WIDTH)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect()
    }

    /// Right-aligned content ends exactly at the last column.
    fn assert_flush_right(line: &str) {
        assert_eq!(
            line.chars().count(),
            WIDTH as usize,
            "expected right-aligned content in {line:?}"
        );
    }

    #[test]
    fn empty_list_says_no_speakers() {
        let data = SpeakerListData {
            entries: vec![],
            entry_data: vec![],
            selected_index: 0,
            picked_up_speaker_id: None,
        };
        assert!(render_lines(&data)
            .iter()
            .any(|l| l.contains("No speakers found")));
    }

    #[test]
    fn group_header_shows_icon_name_track_and_volume() {
        let lines = render_lines(&kitchen_group());
        let header = &lines[0];
        assert!(
            header.starts_with("\u{276f} \u{25b6} Kitchen :"),
            "{header:?}"
        );
        assert!(
            header.contains(": Boy In Red \u{2014} Artist :"),
            "{header:?}"
        );
        assert!(header.ends_with(":  30%"), "{header:?}");
        assert_flush_right(header);
    }

    #[test]
    fn stopped_group_header_fills_with_leaders() {
        let mut data = kitchen_group();
        data.entry_data[0] = header("Kitchen", 5);
        let header = &render_lines(&data)[0];
        assert!(
            header.starts_with("\u{276f} \u{25a0} Kitchen :"),
            "{header:?}"
        );
        assert!(header.ends_with("::::   5%"), "{header:?}");
        assert_flush_right(header);
    }

    #[test]
    fn speaker_rows_use_tree_connectors_and_right_aligned_volume() {
        let lines = render_lines(&kitchen_group());
        assert!(
            lines[1].starts_with("    \u{251c}\u{2500} Kitchen \u{2022} Sonos One "),
            "{:?}",
            lines[1]
        );
        assert!(lines[1].ends_with(" 46%"), "{:?}", lines[1]);
        assert_flush_right(&lines[1]);
        assert!(
            lines[2].starts_with("    \u{2514}\u{2500} Bathroom \u{2022} Sonos Amp "),
            "{:?}",
            lines[2]
        );
        assert!(lines[2].ends_with(" 28%"), "{:?}", lines[2]);
        assert_flush_right(&lines[2]);
    }

    #[test]
    fn selected_speaker_row_shows_volume_bar() {
        let mut data = kitchen_group();
        data.selected_index = 1;
        let lines = render_lines(&data);
        assert!(lines[1].starts_with('\u{276f}'), "{:?}", lines[1]);
        // 16-column bar: 11 cells (46% → 5 filled) then the label.
        let bar = format!("{}{} 46%", "\u{25a0}".repeat(5), "\u{b7}".repeat(6));
        assert!(lines[1].ends_with(&format!(" {bar}")), "{:?}", lines[1]);
        assert_flush_right(&lines[1]);
        // The header loses the cursor; unselected rows keep plain text.
        assert!(lines[0].starts_with("  \u{25b6} Kitchen"), "{:?}", lines[0]);
        assert!(!lines[2].contains('\u{25a0}'), "{:?}", lines[2]);
    }

    #[test]
    fn muted_header_and_rows_show_mute_glyph_before_volume() {
        let mut data = kitchen_group();
        data.entry_data[0].is_muted = true;
        data.entry_data[2].is_muted = true;
        let lines = render_lines(&data);
        assert!(lines[0].ends_with(": \u{d7}  30%"), "{:?}", lines[0]);
        assert_flush_right(&lines[0]);
        assert!(!lines[1].contains('\u{d7}'), "{:?}", lines[1]);
        assert!(lines[2].ends_with(" \u{d7}  28%"), "{:?}", lines[2]);
        assert_flush_right(&lines[2]);
    }

    #[test]
    fn muted_selected_row_shows_mute_glyph_before_bar() {
        let mut data = kitchen_group();
        data.entry_data[1].is_muted = true;
        data.selected_index = 1;
        let line = &render_lines(&data)[1];
        assert!(line.contains(" \u{d7} \u{25a0}"), "{line:?}");
        assert!(line.ends_with(" 46%"), "{line:?}");
        assert_flush_right(line);
    }

    #[test]
    fn groups_are_separated_by_a_divider() {
        let mut data = kitchen_group();
        data.entries.push(ListEntry::GroupHeader(gid(2)));
        data.entries.push(ListEntry::SpeakerRow(sid(2)));
        data.entry_data.push(header("Office", 10));
        data.entry_data
            .push(row("Office", "Sonos Era 100", 10, true));
        let lines = render_lines(&data);
        assert_eq!(lines[3], "");
        assert!(!lines[4].is_empty(), "divider expected");
        assert_eq!(lines[5], "");
        assert!(
            lines[6].starts_with("  \u{25a0} Office :"),
            "{:?}",
            lines[6]
        );
        assert!(
            lines[7].contains("\u{2514}\u{2500} Office"),
            "{:?}",
            lines[7]
        );
    }

    #[test]
    fn pickup_mode_shows_action_rows() {
        // Bathroom picked up from Kitchen; Office is a second group.
        let data = SpeakerListData {
            entries: vec![
                ListEntry::GroupHeader(gid(0)),
                ListEntry::SpeakerRow(sid(0)),
                ListEntry::SpeakerRow(sid(1)),
                ListEntry::AddToGroupRow(gid(0)),
                ListEntry::GroupHeader(gid(2)),
                ListEntry::SpeakerRow(sid(2)),
                ListEntry::AddToGroupRow(gid(2)),
                ListEntry::CreateNewGroupRow,
            ],
            entry_data: vec![
                header("Kitchen", 30),
                row("Kitchen", "Sonos One", 46, false),
                row("Bathroom", "Sonos Amp", 28, false),
                action("Already in group", true),
                header("Office", 10),
                row("Office", "Sonos Era 100", 10, false),
                action("Add to group", false),
                action("Create new group", false),
            ],
            selected_index: 6,
            picked_up_speaker_id: Some(sid(1)),
        };
        let lines = render_lines(&data);

        // The picked-up speaker drops its volume (it is padded to full width
        // for the reverse highlight instead).
        assert!(
            lines[2].contains("\u{251c}\u{2500} Bathroom \u{2022} Sonos Amp"),
            "{:?}",
            lines[2]
        );
        assert!(!lines[2].contains('%'), "{:?}", lines[2]);
        assert!(lines[1].ends_with(" 46%"), "{:?}", lines[1]);

        assert_eq!(lines[3], "    \u{2514}\u{2500} Already in group");
        assert!(lines[7].contains("Office"), "{:?}", lines[7]);
        assert_eq!(lines[9], "\u{276f}   \u{2514}\u{2500} Add to group");
        // "Create new group" sits below its own divider.
        assert_eq!(lines[10], "");
        assert!(!lines[11].is_empty(), "divider expected");
        assert_eq!(lines[12], "");
        assert_eq!(lines[13], "    \u{25b8} Create new group");
    }
}
