//! Volume bar widget: `■■■■■■■■···· 80%`

use ratatui::style::Style;
use ratatui::text::{Line, Span};

// Pre-computed bar strings — sliced per-frame instead of allocating via `.repeat()`.
// 100 chars each covers any reasonable terminal width.
// ■ = 3 bytes UTF-8, · = 2 bytes UTF-8.
pub(crate) const FILLED: &str = "■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■";
pub(crate) const EMPTY: &str = "····································································································";

pub(crate) const FILLED_CHAR_BYTES: usize = 3; // ■ U+25A0
pub(crate) const EMPTY_CHAR_BYTES: usize = 2; // · U+00B7

/// Render a volume bar as a `Line` within the given character width.
///
/// Pattern: `■■■■■■■■···· 80%`
pub fn render_volume_bar(
    level: u16,
    width: u16,
    filled_style: Style,
    empty_style: Style,
) -> Line<'static> {
    let label = format!(" {level}%");
    let label_width = label.len() as u16;
    let bar_width = width.saturating_sub(label_width + 1) as usize;

    if bar_width == 0 {
        return Line::from(Span::styled(label, filled_style));
    }

    let filled_count = if level >= 100 {
        bar_width
    } else {
        (bar_width as u32 * level as u32 / 100) as usize
    };
    let empty_count = bar_width.saturating_sub(filled_count);

    let filled_count = filled_count.min(100);
    let empty_count = empty_count.min(100);

    Line::from(vec![
        Span::styled(&FILLED[..filled_count * FILLED_CHAR_BYTES], filled_style),
        Span::styled(&EMPTY[..empty_count * EMPTY_CHAR_BYTES], empty_style),
        Span::styled(label, filled_style),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_volume_all_empty() {
        let line = render_volume_bar(0, 20, Style::default(), Style::default());
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains("0%"));
        assert!(!text.contains('■'));
    }

    #[test]
    fn full_volume_all_filled() {
        let line = render_volume_bar(100, 20, Style::default(), Style::default());
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains("100%"));
        assert!(!text.contains('·'));
    }

    #[test]
    fn half_volume_roughly_half() {
        let line = render_volume_bar(50, 20, Style::default(), Style::default());
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains("50%"));
        assert!(text.contains('■'));
        assert!(text.contains('·'));
    }
}
