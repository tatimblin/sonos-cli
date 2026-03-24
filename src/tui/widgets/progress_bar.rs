//! Track progress bar: `━━━━━━━●──── 2:31/5:55`
#![allow(dead_code)]

use ratatui::style::Style;
use ratatui::text::{Line, Span};

/// Format milliseconds as `M:SS` or `H:MM:SS` for tracks over 1 hour.
pub fn format_time(ms: u64) -> String {
    if ms == 0 {
        return "--:--".to_string();
    }
    let total_seconds = ms / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

/// Render a progress bar as a `Line` within the given character width.
///
/// Pattern: `━━━━━━━●──── 2:31/5:55`
#[allow(clippy::too_many_arguments)]
pub fn render_progress_bar(
    progress: f64,
    elapsed_ms: u64,
    duration_ms: u64,
    width: u16,
    filled_style: Style,
    empty_style: Style,
    cursor_style: Style,
    time_style: Style,
) -> Line<'static> {
    let elapsed_str = format_time(elapsed_ms);
    let duration_str = format_time(duration_ms);
    let time_label = format!(" {elapsed_str}/{duration_str}");
    let time_width = time_label.len() as u16;
    let bar_width = width.saturating_sub(time_width + 1) as usize;

    if bar_width == 0 {
        return Line::from(Span::styled(time_label, time_style));
    }

    let progress = progress.clamp(0.0, 1.0);
    let cursor_pos = (bar_width as f64 * progress) as usize;
    let filled_count = cursor_pos.min(bar_width);
    let empty_count = bar_width.saturating_sub(filled_count + 1);

    let filled_str: String = "━".repeat(filled_count);
    let cursor_str = if filled_count < bar_width { "●" } else { "" };
    let empty_str: String = "─".repeat(empty_count);

    Line::from(vec![
        Span::styled(filled_str, filled_style),
        Span::styled(cursor_str.to_string(), cursor_style),
        Span::styled(empty_str, empty_style),
        Span::styled(time_label, time_style),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_time_zero() {
        assert_eq!(format_time(0), "--:--");
    }

    #[test]
    fn format_time_minutes() {
        assert_eq!(format_time(151_000), "2:31");
    }

    #[test]
    fn format_time_hours() {
        assert_eq!(format_time(3_661_000), "1:01:01");
    }

    #[test]
    fn format_time_under_minute() {
        assert_eq!(format_time(45_000), "0:45");
    }

    #[test]
    fn progress_bar_zero() {
        let line = render_progress_bar(
            0.0, 0, 0, 30,
            Style::default(), Style::default(), Style::default(), Style::default(),
        );
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains("--:--/--:--"));
    }

    #[test]
    fn progress_bar_half() {
        let line = render_progress_bar(
            0.5, 151_000, 355_000, 30,
            Style::default(), Style::default(), Style::default(), Style::default(),
        );
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains("2:31/5:55"));
        assert!(text.contains('━'));
        assert!(text.contains('─'));
    }
}
