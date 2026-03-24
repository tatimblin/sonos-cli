//! Mini-player bar — 2-line widget showing the focused group's now-playing info.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::tui::theme::Theme;
use crate::tui::widgets::group_card::PlaybackIcon;
use crate::tui::widgets::{progress_bar, volume_bar};

/// Data needed to render the mini-player.
pub struct MiniPlayerData {
    pub group_name: String,
    pub playback_state: PlaybackIcon,
    pub track_display: String,
    pub volume: u16,
    pub progress: f64,
    pub elapsed_ms: u64,
    pub duration_ms: u64,
}

/// Render the mini-player within the given 2-line area.
pub fn render_mini_player(frame: &mut Frame, area: Rect, data: &MiniPlayerData, theme: &Theme) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(theme.mini_player_border);
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

    // Line 1: group name + icon + track info
    let track_text = if data.track_display.is_empty() {
        "Nothing playing".to_string()
    } else {
        data.track_display.clone()
    };

    let line1 = Line::from(vec![
        Span::raw(" "),
        Span::styled(data.playback_state.symbol(), icon_style),
        Span::raw(" "),
        Span::styled(data.group_name.clone(), theme.mini_player_title),
        Span::raw("  "),
        Span::styled(track_text, theme.track_info),
    ]);

    // Line 2: progress bar + volume
    let [progress_area, volume_area] =
        Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
            .areas(Rect::new(inner.x, inner.y + 1.min(inner.height.saturating_sub(1)), inner.width, 1));

    let prog_line = progress_bar::render_progress_bar(
        data.progress,
        data.elapsed_ms,
        data.duration_ms,
        progress_area.width.saturating_sub(1),
        theme.progress_filled,
        theme.progress_empty,
        theme.progress_cursor,
        theme.progress_time,
    );
    let prog_line = Line::from({
        let mut spans = vec![Span::raw(" ")];
        spans.extend(prog_line.spans);
        spans
    });

    let vol_line = volume_bar::render_volume_bar(
        data.volume,
        volume_area.width.saturating_sub(1),
        theme.volume_filled,
        theme.volume_empty,
    );

    let line1_area = Rect::new(inner.x, inner.y, inner.width, 1);
    frame.render_widget(Paragraph::new(line1), line1_area);

    if inner.height >= 2 {
        frame.render_widget(Paragraph::new(prog_line), progress_area);
        frame.render_widget(Paragraph::new(vol_line), volume_area);
    }
}
