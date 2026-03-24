//! Group card widget — renders a single group's state within a bordered box.

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::tui::theme::Theme;
use crate::tui::widgets::{progress_bar, volume_bar};

/// Data needed to render a single group card.
pub struct GroupCardData {
    pub group_name: String,
    pub playback_state: PlaybackIcon,
    pub track_display: String,
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
pub fn render_group_card(frame: &mut Frame, area: Rect, data: &GroupCardData, theme: &Theme) {
    let (border_type, border_style) = if data.selected {
        (BorderType::Double, theme.card_border_selected)
    } else {
        (BorderType::Plain, theme.card_border)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let icon_style = match data.playback_state {
        PlaybackIcon::Playing => theme.playing_icon,
        PlaybackIcon::Paused => theme.paused_icon,
        PlaybackIcon::Stopped => theme.stopped_icon,
    };

    // Line 1: group name + playback state
    let prefix = if data.selected { "● " } else { "  " };
    let title_line = Line::from(vec![
        Span::styled(prefix, theme.card_title),
        Span::styled(data.group_name.clone(), theme.card_title),
        Span::raw("  "),
        Span::styled(data.playback_state.symbol(), icon_style),
        Span::raw(" "),
        Span::styled(data.playback_state.label(), icon_style),
    ]);

    // Line 2: track info
    let track_text = if data.track_display.is_empty() {
        "Nothing playing".to_string()
    } else {
        data.track_display.clone()
    };
    let track_line = Line::from(vec![
        Span::raw("  "),
        Span::styled(track_text, theme.track_info),
    ]);

    // Line 3: volume bar
    let vol_line = volume_bar::render_volume_bar(
        data.volume,
        inner.width.saturating_sub(2),
        theme.volume_filled,
        theme.volume_empty,
    );
    let vol_line = prepend_spaces(vol_line, 2);

    // Line 4: progress bar
    let prog_line = progress_bar::render_progress_bar(
        data.progress,
        data.elapsed_ms,
        data.duration_ms,
        inner.width.saturating_sub(2),
        theme.progress_filled,
        theme.progress_empty,
        theme.progress_cursor,
        theme.progress_time,
    );
    let prog_line = prepend_spaces(prog_line, 2);

    // Line 5: speaker count
    let speaker_line = Line::from(vec![
        Span::raw("  "),
        Span::styled(data.speaker_count_text.clone(), theme.muted),
    ]);

    let lines: Vec<Line> = vec![title_line, track_line, vol_line, prog_line, speaker_line];
    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

fn prepend_spaces(mut line: Line<'static>, count: usize) -> Line<'static> {
    line.spans
        .insert(0, Span::raw(" ".repeat(count)));
    line
}

/// Render a placeholder for an unavailable group.
pub fn render_unavailable_card(frame: &mut Frame, area: Rect, group_name: &str, selected: bool, theme: &Theme) {
    let (border_type, border_style) = if selected {
        (BorderType::Double, theme.card_border_selected)
    } else {
        (BorderType::Plain, theme.card_border)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let prefix = if selected { "● " } else { "  " };
    let lines = vec![
        Line::from(vec![
            Span::styled(prefix, theme.card_title),
            Span::styled(group_name.to_string(), theme.card_title),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("Unavailable", theme.muted),
        ]),
    ];
    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}
