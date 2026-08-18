//! Track progress bar utilities.

use ratatui::style::{Color, Style};
use ratatui::text::Span;

use crate::tui::theme::colors_equal;

// Pre-computed bar strings — sliced per-frame instead of allocating via `.repeat()`.
// All chars below are 3 bytes in UTF-8. 100 chars covers any reasonable terminal width.
pub(crate) const PROG_FILLED: &str = "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━";
pub(crate) const PROG_EMPTY: &str = "────────────────────────────────────────────────────────────────────────────────────────────────────────";
pub(crate) const PROG_CHAR_BYTES: usize = 3; // ━ U+2501, ─ U+2500

/// Render a progress bar as spans. Caller composes into their layout.
///
/// Returns filled + cursor + empty spans. The `cursor` character (e.g. `"●"`, `"╺"`)
/// is shown between filled and empty sections when `Some`. Pass `None` for no cursor.
///
/// When `gradient_start` and `gradient_end` differ (both must be `Color::Rgb`), each
/// filled character gets a linearly interpolated color. When they are equal, a single
/// filled span is emitted (zero overhead).
// Eight arguments, one over clippy's threshold. They are all irreducible inputs to
// a single rendering decision — geometry (progress, width), the optional cursor,
// three styles, and the two gradient endpoints — so grouping them into a struct
// would move the argument list to the call sites without removing anything. Left
// flat deliberately rather than restructured.
#[allow(clippy::too_many_arguments)]
pub fn render_bar_spans(
    progress: f64,
    width: usize,
    cursor: Option<&str>,
    filled_style: Style,
    cursor_style: Style,
    empty_style: Style,
    gradient_start: Color,
    gradient_end: Color,
) -> Vec<Span<'static>> {
    let progress = progress.clamp(0.0, 1.0);
    let cursor_width = if cursor.is_some() { 1 } else { 0 };
    let bar_width = width.saturating_sub(cursor_width);
    let filled = ((bar_width as f64 * progress) as usize).min(100);
    let empty = bar_width.saturating_sub(filled).min(100);

    let use_gradient = !colors_equal(gradient_start, gradient_end)
        && matches!(gradient_start, Color::Rgb(..))
        && matches!(gradient_end, Color::Rgb(..))
        && filled > 0;

    let mut spans = Vec::with_capacity(if use_gradient { filled + 2 } else { 3 });

    if use_gradient {
        for i in 0..filled {
            let t = if filled <= 1 {
                0.0
            } else {
                i as f64 / (filled - 1) as f64
            };
            let color = gradient_color(gradient_start, gradient_end, t);
            spans.push(Span::styled(
                &PROG_FILLED[i * PROG_CHAR_BYTES..(i + 1) * PROG_CHAR_BYTES],
                Style::new().fg(color),
            ));
        }
    } else {
        spans.push(Span::styled(
            &PROG_FILLED[..filled * PROG_CHAR_BYTES],
            filled_style,
        ));
    }

    if let Some(c) = cursor {
        spans.push(Span::styled(c.to_string(), cursor_style));
    }
    spans.push(Span::styled(
        &PROG_EMPTY[..empty * PROG_CHAR_BYTES],
        empty_style,
    ));
    spans
}

fn gradient_color(start: Color, end: Color, t: f64) -> Color {
    if let (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) = (start, end) {
        Color::Rgb(lerp(r1, r2, t), lerp(g1, g2, t), lerp(b1, b2, t))
    } else {
        start
    }
}

fn lerp(a: u8, b: u8, t: f64) -> u8 {
    (a as f64 + (b as f64 - a as f64) * t).round() as u8
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

    const NO_GRAD: Color = Color::White;

    #[test]
    fn bar_spans_zero_progress() {
        let spans = render_bar_spans(
            0.0,
            10,
            Some("●"),
            Style::default(),
            Style::default(),
            Style::default(),
            NO_GRAD,
            NO_GRAD,
        );
        assert_eq!(spans.len(), 3);
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
            NO_GRAD,
            NO_GRAD,
        );
        assert_eq!(spans.len(), 3);
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
            NO_GRAD,
            NO_GRAD,
        );
        assert_eq!(spans.len(), 2);
    }

    #[test]
    fn bar_spans_same_gradient_produces_single_filled_span() {
        let same = Color::Rgb(100, 100, 100);
        let spans = render_bar_spans(
            0.5,
            10,
            Some("●"),
            Style::new().fg(same),
            Style::default(),
            Style::default(),
            same,
            same,
        );
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].style.fg, Some(same));
    }

    #[test]
    fn bar_spans_gradient_produces_per_char_spans() {
        let start = Color::Rgb(255, 0, 0);
        let end = Color::Rgb(0, 0, 255);
        let spans = render_bar_spans(
            0.5,
            10,
            Some("●"),
            Style::default(),
            Style::default(),
            Style::default(),
            start,
            end,
        );
        // 9 bar chars at 50% = 4 filled + cursor + empty
        let filled_count = spans.len() - 2; // minus cursor and empty
        assert!(filled_count >= 3);
        // First span should be near start color
        assert_eq!(spans[0].style.fg, Some(Color::Rgb(255, 0, 0)));
        // Last filled span should be near end color
        let last_filled = &spans[filled_count - 1];
        if let Some(Color::Rgb(r, _g, b)) = last_filled.style.fg {
            assert!(b > r, "last filled span should be more blue than red");
        }
    }

    #[test]
    fn bar_spans_gradient_zero_progress_no_filled_spans() {
        let start = Color::Rgb(255, 0, 0);
        let end = Color::Rgb(0, 0, 255);
        let spans = render_bar_spans(
            0.0,
            10,
            Some("●"),
            Style::default(),
            Style::default(),
            Style::default(),
            start,
            end,
        );
        // No filled chars → falls back to single empty filled span + cursor + empty
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].content.as_ref(), "");
    }

    #[test]
    fn bar_spans_gradient_full_progress() {
        let start = Color::Rgb(255, 0, 0);
        let end = Color::Rgb(0, 0, 255);
        let spans = render_bar_spans(
            1.0,
            10,
            Some("●"),
            Style::default(),
            Style::default(),
            Style::default(),
            start,
            end,
        );
        let last = spans.last().unwrap();
        assert_eq!(last.content.as_ref(), "");
    }

    #[test]
    fn bar_spans_non_rgb_gradient_falls_back() {
        let spans = render_bar_spans(
            0.5,
            10,
            Some("●"),
            Style::new().fg(Color::White),
            Style::default(),
            Style::default(),
            Color::White,
            Color::Cyan,
        );
        // Non-RGB gradient should produce single filled span
        assert_eq!(spans.len(), 3);
    }

    #[test]
    fn bar_spans_gradient_single_filled_char() {
        let start = Color::Rgb(255, 0, 0);
        let end = Color::Rgb(0, 0, 255);
        // Very small progress on a small bar → 1 filled char
        let spans = render_bar_spans(
            0.2,
            6,
            Some("●"),
            Style::default(),
            Style::default(),
            Style::default(),
            start,
            end,
        );
        // 1 filled char should use start color
        assert_eq!(spans[0].style.fg, Some(start));
    }

    #[test]
    fn lerp_endpoints() {
        assert_eq!(lerp(0, 255, 0.0), 0);
        assert_eq!(lerp(0, 255, 1.0), 255);
    }

    #[test]
    fn lerp_midpoint() {
        assert_eq!(lerp(0, 200, 0.5), 100);
    }

    #[test]
    fn gradient_color_rgb() {
        let c = gradient_color(Color::Rgb(0, 0, 0), Color::Rgb(100, 200, 50), 0.5);
        assert_eq!(c, Color::Rgb(50, 100, 25));
    }

    #[test]
    fn gradient_color_non_rgb_returns_start() {
        let c = gradient_color(Color::White, Color::Rgb(100, 100, 100), 0.5);
        assert_eq!(c, Color::White);
    }
}
