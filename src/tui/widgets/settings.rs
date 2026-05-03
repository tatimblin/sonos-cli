//! Settings widget — render-only component.
//!
//! Takes pre-computed `SettingsData` and `Theme`, outputs to frame.
//! No hooks, no SDK, no key handling.

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::tui::theme::Theme;
use crate::tui::types::SettingsData;

/// Label column width (right-aligned labels).
const LABEL_WIDTH: usize = 16;

/// Render the settings form from pre-computed data.
pub fn render(frame: &mut Frame, area: Rect, data: &SettingsData, theme: &Theme) {
    let mut lines: Vec<Line> = Vec::new();

    // Title
    lines.push(Line::from(vec![Span::styled("  Settings", theme.accent)]));
    lines.push(Line::raw(""));

    // Render each settings row
    for (i, item) in data.items.iter().enumerate() {
        let is_selected = i == data.selected_row;
        let is_dropdown_row = is_selected && data.dropdown_open;

        // Build label (right-aligned to LABEL_WIDTH)
        let label = format!("{:>width$}:  ", item.label, width = LABEL_WIDTH);

        // Build value display
        let value_display = format!("[ {} {} ]", item.value, theme.glyphs.dropdown_indicator);

        let mut spans = Vec::new();

        if is_selected && !data.dropdown_open {
            spans.push(Span::styled(
                format!("{} ", theme.glyphs.settings_cursor),
                theme.accent,
            ));
        } else {
            spans.push(Span::raw("  "));
        }

        if is_selected {
            spans.push(Span::styled(label, theme.accent));
            spans.push(Span::styled(value_display, theme.accent));
        } else {
            spans.push(Span::styled(label, theme.muted));
            spans.push(Span::styled(value_display, theme.muted));
        }

        lines.push(Line::from(spans));

        // Render dropdown options if this row's dropdown is open
        if is_dropdown_row {
            for (j, option) in item.options.iter().enumerate() {
                let is_active = j == data.dropdown_index;
                let prefix = " ".repeat(LABEL_WIDTH + 5);

                if is_active {
                    let line = Line::from(vec![Span::styled(
                        format!("{prefix}{} {option}", theme.glyphs.dropdown_active),
                        theme.accent,
                    )]);
                    lines.push(line);
                } else {
                    let line = Line::from(vec![Span::styled(
                        format!("{prefix}  {option}"),
                        theme.muted,
                    )]);
                    lines.push(line);
                }
            }
        }
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}
