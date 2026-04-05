//! Track progress bar utilities.

use ratatui::style::Style;
use ratatui::text::Span;

// Pre-computed bar strings — sliced per-frame instead of allocating via `.repeat()`.
// All chars below are 3 bytes in UTF-8. 100 chars covers any reasonable terminal width.
pub(crate) const PROG_FILLED: &str = "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━";
pub(crate) const PROG_EMPTY: &str = "────────────────────────────────────────────────────────────────────────────────────────────────────────";
pub(crate) const PROG_CHAR_BYTES: usize = 3; // ━ U+2501, ─ U+2500

/// Render a progress bar as spans. Caller composes into their layout.
///
/// Returns filled + cursor + empty spans. The `cursor` character (e.g. `"●"`, `"╺"`)
/// is shown between filled and empty sections when `Some`. Pass `None` for no cursor.
pub fn render_bar_spans(
    progress: f64,
    width: usize,
    cursor: Option<&str>,
    filled_style: Style,
    cursor_style: Style,
    empty_style: Style,
) -> Vec<Span<'static>> {
    let progress = progress.clamp(0.0, 1.0);
    let cursor_width = if cursor.is_some() { 1 } else { 0 };
    let bar_width = width.saturating_sub(cursor_width);
    let filled = (bar_width as f64 * progress) as usize;
    let empty = bar_width.saturating_sub(filled);

    let filled = filled.min(100);
    let empty = empty.min(100);

    let mut spans = Vec::with_capacity(3);
    spans.push(Span::styled(
        &PROG_FILLED[..filled * PROG_CHAR_BYTES],
        filled_style,
    ));
    if let Some(c) = cursor {
        spans.push(Span::styled(c.to_string(), cursor_style));
    }
    spans.push(Span::styled(
        &PROG_EMPTY[..empty * PROG_CHAR_BYTES],
        empty_style,
    ));
    spans
}

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
    fn bar_spans_zero_progress() {
        let spans = render_bar_spans(
            0.0,
            10,
            Some("●"),
            Style::default(),
            Style::default(),
            Style::default(),
        );
        assert_eq!(spans.len(), 3);
        // filled should be empty string, cursor present, empty should fill
        assert_eq!(spans[0].content.as_ref(), "");
        assert_eq!(spans[1].content.as_ref(), "●");
    }

    #[test]
    fn bar_spans_full_progress() {
        let spans = render_bar_spans(
            1.0,
            10,
            Some("●"),
            Style::default(),
            Style::default(),
            Style::default(),
        );
        assert_eq!(spans.len(), 3);
        // empty should be empty string
        assert_eq!(spans[2].content.as_ref(), "");
    }

    #[test]
    fn bar_spans_no_cursor() {
        let spans = render_bar_spans(
            0.5,
            10,
            None,
            Style::default(),
            Style::default(),
            Style::default(),
        );
        assert_eq!(spans.len(), 2); // no cursor span
    }
}
