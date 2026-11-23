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
//! Satellites nest under their speaker and show its surround or sub levels;
//! the selected row shows a bar for the level `←→` edits:
//!   `│  ├─ Left surround • Sonos One             ♪ +3  TV +8`
//!   `│  └─ Right surround • Sonos One   ■■■■■··· ♪ +3  TV +8`
//!
//! During pickup mode, inline action rows appear:
//!   `└─ Add to group`       (selectable drop target)
//!   `└─ Already in group`   (dimmed, home group)
//!   `► Create new group`    (standalone target)

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use sonos_sdk::PlaybackState;

use crate::tui::theme::Theme;
use crate::tui::types::{
    group_for_entry, GroupHeaderData, LevelFocus, ListEntry, RowData, SatelliteLevels,
    SatelliteRowData, SpeakerListData, SpeakerRowData,
};
use crate::tui::widgets::volume_bar;

/// Columns taken by the cursor glyph and the space after it.
const CURSOR_PREFIX_WIDTH: usize = 2;

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

    let viewport_height = area.height as usize;
    let vol_width = 16.min(area.width.saturating_sub(50));
    let total_width = area.width as usize;

    let is_pickup = data.picked_up_speaker_id.is_some();
    let focused_group = group_for_entry(&data.entries, selected_index);

    let mut lines: Vec<Line> = Vec::new();
    let mut entry_visual_starts: Vec<usize> = Vec::new();
    let mut entry_visual_heights: Vec<usize> = Vec::new();

    for (idx, (entry, row)) in data.entries.iter().zip(data.rows.iter()).enumerate() {
        let is_selected = idx == selected_index;

        // Blank separator + divider between groups (not before the first entry)
        if idx > 0 && matches!(entry, ListEntry::GroupHeader(_)) {
            lines.extend(divider_lines(total_width, theme));
        }

        let start_line = lines.len();

        match row {
            RowData::GroupHeader(header) => {
                let is_focused_group = matches!(
                    (entry, &focused_group),
                    (ListEntry::GroupHeader(gid), Some(fg)) if fg == gid
                );
                lines.push(render_group_header(
                    header,
                    is_selected,
                    is_focused_group,
                    total_width,
                    theme,
                ));
            }
            RowData::Speaker(speaker) => {
                let is_picked_up = matches!(
                    (entry, &data.picked_up_speaker_id),
                    (ListEntry::SpeakerRow(sid), Some(picked)) if picked == sid
                );
                let shows_volume_bar = is_selected && !is_pickup;
                lines.push(render_speaker_row(
                    speaker,
                    is_selected,
                    is_picked_up,
                    shows_volume_bar,
                    vol_width,
                    total_width,
                    theme,
                ));
            }
            RowData::Satellite(satellite) => {
                lines.push(render_satellite_row(
                    satellite,
                    is_selected,
                    data.level_focus,
                    total_width,
                    theme,
                ));
            }
            RowData::AddToGroup { is_home } => {
                lines.push(render_add_to_group_row(*is_home, is_selected, theme));
            }
            RowData::CreateNewGroup => {
                // "Create new group" sits below its own divider
                lines.extend(divider_lines(total_width, theme));
                lines.push(render_create_group_row(is_selected, theme));
            }
        }

        entry_visual_starts.push(start_line);
        entry_visual_heights.push(lines.len() - start_line);
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
// Row renderers
// ===========================================================================

/// `❯ ▶ Kitchen ::::: Track — Artist :::::::::  30%`
fn render_group_header(
    header: &GroupHeaderData,
    is_selected: bool,
    is_focused_group: bool,
    total_width: usize,
    theme: &Theme,
) -> Line<'static> {
    let g = &theme.glyphs;

    let (icon, icon_style) = match &header.playback_state {
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

    let mut spans = cursor_prefix(is_selected, theme.speaker_cursor, theme);
    spans.push(Span::styled(format!("{icon} "), icon_style));
    spans.push(Span::styled(header.name.clone(), name_style));
    spans.push(Span::raw(" "));

    let mut right = Vec::new();
    if header.is_muted {
        right.push(Span::styled(format!(" {}", g.muted), theme.error));
    }
    right.push(Span::styled(
        format!(" {}", format_volume(header.volume)),
        theme.muted,
    ));

    let available = fill_width(&spans, &right, total_width);
    let leaders = |count: usize| -> String { std::iter::repeat_n(g.leader_char, count).collect() };

    let track_text = header.track_info.as_deref().unwrap_or("");
    let track_chars = track_text.chars().count();

    if !track_text.is_empty() && available > track_chars + 4 {
        let leaders_total = available.saturating_sub(track_chars + 2);
        let leaders_before = 5.min(leaders_total / 2);
        let leaders_after = leaders_total.saturating_sub(leaders_before);

        spans.push(Span::styled(leaders(leaders_before), leader_style));
        spans.push(Span::styled(format!(" {track_text} "), theme.track_info));
        spans.push(Span::styled(leaders(leaders_after), leader_style));
    } else {
        spans.push(Span::styled(leaders(available), leader_style));
    }

    spans.extend(right);
    Line::from(spans)
}

/// `  ├─ Kitchen • Sonos One          46%`, or a volume bar when selected.
fn render_speaker_row(
    speaker: &SpeakerRowData,
    is_selected: bool,
    is_picked_up: bool,
    shows_volume_bar: bool,
    vol_width: u16,
    total_width: usize,
    theme: &Theme,
) -> Line<'static> {
    let g = &theme.glyphs;
    let connector = if speaker.is_last_in_group {
        g.connector_last
    } else {
        g.connector_branch
    };

    let mut spans = cursor_prefix(is_selected, theme.speaker_cursor, theme);
    spans.push(indent(theme));

    if is_picked_up {
        // Reverse-highlighted across the full width; the volume is dropped.
        let style = theme.picked_up;
        spans.push(Span::styled(connector, style));
        spans.push(Span::styled(speaker.name.clone(), style));
        if let Some(ref model) = speaker.model_name {
            spans.push(Span::styled(g.model_separator.to_string(), style));
            spans.push(Span::styled(model.clone(), style));
        }
        let remaining = fill_width(&spans, &[], total_width);
        if remaining > 0 {
            spans.push(Span::styled(" ".repeat(remaining), style));
        }
        return Line::from(spans);
    }

    let name_style = if is_selected {
        theme.speaker_cursor
    } else {
        theme.speaker_name
    };

    spans.push(Span::styled(connector, theme.muted));
    spans.push(Span::styled(speaker.name.clone(), name_style));
    if let Some(ref model) = speaker.model_name {
        spans.push(Span::styled(g.model_separator.to_string(), theme.muted));
        spans.push(Span::styled(model.clone(), theme.muted));
    }

    if let Some(vol) = speaker.volume {
        let right = volume_spans(vol, speaker.is_muted, shows_volume_bar, vol_width, theme);
        push_right_aligned(&mut spans, right, total_width);
    }

    Line::from(spans)
}

/// Columns of the bar on a selected satellite row.
const LEVEL_BAR_WIDTH: usize = 8;

/// `     │  ├─ Left surround • Sonos One          ♪ +3  TV +8`
///
/// Selected, the level `←→` edits gets a bar and the accent colour; the other
/// stays a muted number.
fn render_satellite_row(
    satellite: &SatelliteRowData,
    is_selected: bool,
    focus: LevelFocus,
    total_width: usize,
    theme: &Theme,
) -> Line<'static> {
    let g = &theme.glyphs;
    let continuation = if satellite.main_is_last_in_group {
        "   "
    } else {
        g.connector_pipe
    };
    let connector = if satellite.is_last {
        g.connector_last
    } else {
        g.connector_branch
    };
    let name_style = if is_selected {
        theme.speaker_cursor
    } else {
        theme.muted
    };

    let mut spans = cursor_prefix(is_selected, theme.speaker_cursor, theme);
    spans.push(indent(theme));
    spans.push(Span::styled(continuation, theme.muted));
    spans.push(Span::styled(connector, theme.muted));
    spans.push(Span::styled(satellite.label.clone(), name_style));
    if let Some(ref model) = satellite.model_name {
        spans.push(Span::styled(g.model_separator.to_string(), theme.muted));
        spans.push(Span::styled(model.clone(), theme.muted));
    }

    let right = match satellite.levels {
        SatelliteLevels::Surround {
            enabled: Some(false),
            ..
        }
        | SatelliteLevels::Sub {
            enabled: Some(false),
            ..
        } => vec![Span::styled("off", theme.muted)],
        SatelliteLevels::Surround { music, tv, .. } => {
            let music_label = format!("{} {}", g.music_note, format_level(music));
            let tv_label = format!("TV {}", format_level(tv));
            let mut right = level_spans(
                music,
                music_label,
                is_selected && focus == LevelFocus::Music,
                theme,
            );
            right.push(Span::raw("  "));
            right.extend(level_spans(
                tv,
                tv_label,
                is_selected && focus == LevelFocus::Tv,
                theme,
            ));
            right
        }
        SatelliteLevels::Sub { level, .. } => {
            level_spans(level, format_level(level), is_selected, theme)
        }
        SatelliteLevels::Unknown => Vec::new(),
    };
    push_right_aligned(&mut spans, right, total_width);

    Line::from(spans)
}

/// One level: `■■■■■··· ♪ +3` in the accent colour when it is being edited,
/// otherwise just the muted label.
fn level_spans(
    level: Option<i8>,
    label: String,
    is_editing: bool,
    theme: &Theme,
) -> Vec<Span<'static>> {
    if !is_editing {
        return vec![Span::styled(label, theme.muted)];
    }
    let mut spans = Vec::new();
    if let Some(level) = level {
        let (filled, empty) = level_bar(level, LEVEL_BAR_WIDTH);
        spans.push(Span::styled(filled, theme.volume_filled));
        spans.push(Span::styled(empty, theme.volume_empty));
        spans.push(Span::raw(" "));
    }
    spans.push(Span::styled(label, theme.accent));
    spans
}

/// A level from -15 to +15 as a bar `width` columns wide: 0 fills half.
fn level_bar(level: i8, width: usize) -> (String, String) {
    let filled = ((i32::from(level.clamp(-15, 15)) + 15) as usize * width + 15) / 30;
    ("\u{25a0}".repeat(filled), "\u{00b7}".repeat(width - filled))
}

/// `+3`, `-2`, `+0`; `-` when not known yet.
fn format_level(level: Option<i8>) -> String {
    match level {
        Some(v) => format!("{v:+}"),
        None => "-".to_string(),
    }
}

/// `  └─ Add to group`, or dimmed `└─ Already in group` for the home group.
fn render_add_to_group_row(is_home: bool, is_selected: bool, theme: &Theme) -> Line<'static> {
    let (label, style, cursor_style) = if is_home {
        ("Already in group", theme.muted, theme.muted)
    } else if is_selected {
        ("Add to group", theme.accent, theme.speaker_cursor)
    } else {
        ("Add to group", theme.muted, theme.muted)
    };

    let mut spans = cursor_prefix(is_selected, cursor_style, theme);
    spans.push(indent(theme));
    spans.push(Span::styled(theme.glyphs.connector_last, style));
    spans.push(Span::styled(label, style));
    Line::from(spans)
}

/// `  ▸ Create new group`
fn render_create_group_row(is_selected: bool, theme: &Theme) -> Line<'static> {
    let (style, cursor_style) = if is_selected {
        (theme.accent, theme.speaker_cursor)
    } else {
        (theme.muted, theme.muted)
    };

    let mut spans = cursor_prefix(is_selected, cursor_style, theme);
    spans.push(indent(theme));
    spans.push(Span::styled(
        format!("{} ", theme.glyphs.tab_active_indicator),
        style,
    ));
    spans.push(Span::styled("Create new group", style));
    Line::from(spans)
}

// ===========================================================================
// Shared helpers
// ===========================================================================

/// The cursor glyph (or a blank when unselected) and the space after it.
fn cursor_prefix(is_selected: bool, style: Style, theme: &Theme) -> Vec<Span<'static>> {
    let cursor = if is_selected {
        theme.glyphs.cursor
    } else {
        " "
    };
    vec![Span::styled(cursor, style), Span::raw(" ")]
}

/// Indent that nests speaker and action rows under their group header.
fn indent(theme: &Theme) -> Span<'static> {
    Span::styled(" ".repeat(2), theme.muted)
}

/// A blank line, a centered divider, and another blank line.
fn divider_lines(total_width: usize, theme: &Theme) -> [Line<'static>; 3] {
    let g = &theme.glyphs;
    let indent_width = CURSOR_PREFIX_WIDTH + 2;
    let divider_inner_w = total_width.saturating_sub(indent_width + 2);
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
    [
        Line::raw(""),
        Line::from(vec![
            Span::raw(" ".repeat(indent_width)),
            Span::styled(divider, theme.muted),
        ]),
        Line::raw(""),
    ]
}

fn spans_width(spans: &[Span]) -> usize {
    spans.iter().map(|s| s.content.chars().count()).sum()
}

/// Columns left between `left` and `right` when `right` ends flush with
/// `total_width`.
fn fill_width(left: &[Span], right: &[Span], total_width: usize) -> usize {
    total_width.saturating_sub(spans_width(left) + spans_width(right))
}

/// Pad `spans` with blanks so `right`, appended after, ends flush with `total_width`.
fn push_right_aligned(
    spans: &mut Vec<Span<'static>>,
    right: Vec<Span<'static>>,
    total_width: usize,
) {
    let pad = fill_width(spans, &right, total_width);
    if pad > 0 {
        spans.push(Span::raw(" ".repeat(pad)));
    }
    spans.extend(right);
}

/// Format volume as a fixed 4-char right-aligned string: `" 0%"`, `"60%"`, `"100%"`.
fn format_volume(vol: Option<u16>) -> String {
    match vol {
        Some(v) => format!("{v:>3}%"),
        None => "   -".to_string(),
    }
}

/// A speaker's volume: a bar when `as_bar`, otherwise the percentage.
///
/// When `is_muted`, the mute glyph precedes the volume and the bar renders dimmed.
fn volume_spans(
    vol: u16,
    is_muted: bool,
    as_bar: bool,
    bar_width: u16,
    theme: &Theme,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    if is_muted {
        spans.push(Span::styled(
            format!("{} ", theme.glyphs.muted),
            theme.error,
        ));
    }
    if as_bar {
        let filled = if is_muted {
            theme.volume_empty
        } else {
            theme.volume_filled
        };
        spans.extend(
            volume_bar::render_volume_bar(vol, bar_width, filled, theme.volume_empty).spans,
        );
    } else {
        spans.push(Span::styled(format!("{vol:>3}%"), theme.muted));
    }
    spans
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
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

    fn group_header(name: &str, volume: u16) -> GroupHeaderData {
        GroupHeaderData {
            name: name.to_string(),
            volume: Some(volume),
            is_muted: false,
            playback_state: Some(PlaybackState::Stopped),
            track_info: None,
        }
    }

    fn header(name: &str, volume: u16) -> RowData {
        RowData::GroupHeader(group_header(name, volume))
    }

    fn row(name: &str, model: &str, volume: u16, is_last: bool) -> RowData {
        RowData::Speaker(SpeakerRowData {
            name: name.to_string(),
            model_name: Some(model.to_string()),
            volume: Some(volume),
            is_muted: false,
            is_last_in_group: is_last,
        })
    }

    fn set_muted(row: &mut RowData) {
        match row {
            RowData::GroupHeader(header) => header.is_muted = true,
            RowData::Speaker(speaker) => speaker.is_muted = true,
            _ => panic!("only headers and speaker rows can be muted"),
        }
    }

    /// One group "Kitchen" (playing, 30%) with members Kitchen (46%) and
    /// Bathroom (28%). The header is selected unless a test changes it.
    fn kitchen_group() -> SpeakerListData {
        let mut kitchen = group_header("Kitchen", 30);
        kitchen.playback_state = Some(PlaybackState::Playing);
        kitchen.track_info = Some("Boy In Red \u{2014} Artist".to_string());
        SpeakerListData {
            entries: vec![
                ListEntry::GroupHeader(gid(0)),
                ListEntry::SpeakerRow(sid(0)),
                ListEntry::SpeakerRow(sid(1)),
            ],
            rows: vec![
                RowData::GroupHeader(kitchen),
                row("Kitchen", "Sonos One", 46, false),
                row("Bathroom", "Sonos Amp", 28, true),
            ],
            selected_index: 0,
            picked_up_speaker_id: None,
            level_focus: LevelFocus::Music,
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

    fn surround(label: &str, music: i8, tv: i8, is_last: bool) -> RowData {
        RowData::Satellite(SatelliteRowData {
            label: label.to_string(),
            model_name: Some("Sonos One".to_string()),
            levels: SatelliteLevels::Surround {
                enabled: Some(true),
                music: Some(music),
                tv: Some(tv),
            },
            is_last,
            main_is_last_in_group: true,
        })
    }

    /// Basement: a Playbar with two surrounds. Nothing is selected under the
    /// header unless a test changes it.
    fn basement() -> SpeakerListData {
        let satellite = |n: u8| ListEntry::SatelliteRow {
            main: sid(0),
            satellite: sid(n),
        };
        SpeakerListData {
            entries: vec![
                ListEntry::GroupHeader(gid(0)),
                ListEntry::SpeakerRow(sid(0)),
                satellite(1),
                satellite(2),
            ],
            rows: vec![
                header("Basement", 25),
                row("Basement", "Sonos Playbar", 25, true),
                surround("Left surround", 3, 8, false),
                surround("Right surround", 3, 8, true),
            ],
            selected_index: 0,
            picked_up_speaker_id: None,
            level_focus: LevelFocus::Music,
        }
    }

    #[test]
    fn satellite_rows_nest_under_their_speaker_with_both_levels() {
        let lines = render_lines(&basement());
        assert!(
            lines[1].starts_with("    \u{2514}\u{2500} Basement \u{2022} Sonos Playbar"),
            "{:?}",
            lines[1]
        );
        assert!(
            lines[2].starts_with("       \u{251c}\u{2500} Left surround \u{2022} Sonos One "),
            "{:?}",
            lines[2]
        );
        assert!(lines[2].ends_with(" \u{266a} +3  TV +8"), "{:?}", lines[2]);
        assert!(
            lines[3].starts_with("       \u{2514}\u{2500} Right surround"),
            "{:?}",
            lines[3]
        );
        assert_flush_right(&lines[2]);
    }

    #[test]
    fn selected_surround_row_shows_a_bar_for_the_level_being_edited() {
        let mut data = basement();
        data.selected_index = 2;
        let music = &render_lines(&data)[2];
        assert!(music.starts_with("\u{276f}"), "{music:?}");
        // +3 of -15..+15 in 8 columns: 5 filled.
        assert!(
            music.ends_with(" \u{25a0}\u{25a0}\u{25a0}\u{25a0}\u{25a0}\u{00b7}\u{00b7}\u{00b7} \u{266a} +3  TV +8"),
            "{music:?}"
        );

        data.level_focus = LevelFocus::Tv;
        let tv = &render_lines(&data)[2];
        // +8: 6 filled, and the music level is a bare number.
        assert!(
            tv.ends_with(" \u{266a} +3  \u{25a0}\u{25a0}\u{25a0}\u{25a0}\u{25a0}\u{25a0}\u{00b7}\u{00b7} TV +8"),
            "{tv:?}"
        );
        assert_flush_right(tv);
    }

    #[test]
    fn switched_off_satellites_say_off() {
        let mut data = basement();
        if let RowData::Satellite(sat) = &mut data.rows[2] {
            sat.levels = SatelliteLevels::Surround {
                enabled: Some(false),
                music: Some(3),
                tv: Some(8),
            };
        }
        data.rows[3] = RowData::Satellite(SatelliteRowData {
            label: "Sub".to_string(),
            model_name: Some("Sonos Sub Mini".to_string()),
            levels: SatelliteLevels::Sub {
                enabled: Some(true),
                level: Some(2),
            },
            is_last: true,
            main_is_last_in_group: true,
        });
        let lines = render_lines(&data);
        assert!(lines[2].ends_with(" off"), "{:?}", lines[2]);
        assert!(lines[3].ends_with(" +2"), "{:?}", lines[3]);
    }

    #[test]
    fn satellites_of_a_middle_speaker_continue_the_branch() {
        let mut data = basement();
        for row in &mut data.rows[2..] {
            if let RowData::Satellite(sat) = row {
                sat.main_is_last_in_group = false;
            }
        }
        let line = &render_lines(&data)[2];
        assert!(
            line.starts_with("    \u{2502}  \u{251c}\u{2500} Left surround"),
            "{line:?}"
        );
    }

    #[test]
    fn empty_list_says_no_speakers() {
        let data = SpeakerListData {
            entries: vec![],
            rows: vec![],
            selected_index: 0,
            picked_up_speaker_id: None,
            level_focus: LevelFocus::Music,
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
        data.rows[0] = header("Kitchen", 5);
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
        set_muted(&mut data.rows[0]);
        set_muted(&mut data.rows[2]);
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
        set_muted(&mut data.rows[1]);
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
        data.rows.push(header("Office", 10));
        data.rows.push(row("Office", "Sonos Era 100", 10, true));
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
            rows: vec![
                header("Kitchen", 30),
                row("Kitchen", "Sonos One", 46, false),
                row("Bathroom", "Sonos Amp", 28, false),
                RowData::AddToGroup { is_home: true },
                header("Office", 10),
                row("Office", "Sonos Era 100", 10, false),
                RowData::AddToGroup { is_home: false },
                RowData::CreateNewGroup,
            ],
            selected_index: 6,
            picked_up_speaker_id: Some(sid(1)),
            level_focus: LevelFocus::Music,
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
