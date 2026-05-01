//! TUI rendering — framed layout with header, content, separators, and key legend.

use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::tui::app::{App, Navigation, Tab};
use crate::tui::hooks::RenderContext;
use crate::tui::widgets::speaker_list;


/// Top-level render dispatch. Draws header, separators, content, and key legend.
pub fn render(frame: &mut Frame, ctx: &mut RenderContext) {
    let area = frame.area();
    if area.height < 4 || area.width < 20 {
        return;
    }

    // Horizontal padding (1 char each side)
    let padded_x = area.x + 1;
    let padded_w = area.width.saturating_sub(2);

    // Header (first row)
    let header_area = Rect::new(padded_x, area.y, padded_w, 1);
    render_header(frame, header_area, ctx.app);

    // Separator between header and content
    draw_separator(
        frame,
        area.y + 1,
        area.x,
        area.x + area.width - 1,
        ctx.app.theme.muted,
    );

    // Content area
    let content_area = Rect::new(
        padded_x,
        area.y + 2,
        padded_w,
        area.height.saturating_sub(4),
    );

    match ctx.app.navigation.tab {
        Tab::Speakers => {
            let state = ctx.app.navigation.speakers_state.clone();
            speaker_list::render(frame, content_area, ctx, &state);
        }
        Tab::Settings => {
            let paragraph = Paragraph::new("Settings \u{2014} coming soon")
                .alignment(Alignment::Center)
                .style(ctx.app.theme.muted);
            frame.render_widget(paragraph, content_area);
        }
    }

    // Separator between content and footer
    draw_separator(
        frame,
        area.y + area.height - 2,
        area.x,
        area.x + area.width - 1,
        ctx.app.theme.muted,
    );

    // Key legend (last row)
    let footer_area = Rect::new(padded_x, area.y + area.height - 1, padded_w, 1);
    render_key_legend(frame, footer_area, ctx.app);
}

/// Draw a full-width horizontal separator line.
fn draw_separator(frame: &mut Frame, y: u16, left: u16, right: u16, style: Style) {
    let buf = frame.buffer_mut();
    for x in left..=right {
        if let Some(cell) = buf.cell_mut((x, y)) {
            cell.set_char('\u{2500}').set_style(style);
        }
    }
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let logo = "\u{266a}  S O N O S";
    let tab_spans = build_tab_spans(&app.navigation, &app.theme);

    let logo_width = logo.chars().count();
    let tab_width: usize = tab_spans.iter().map(|s| s.content.chars().count()).sum();
    let padding = (area.width as usize).saturating_sub(logo_width + tab_width);

    let mut spans = vec![Span::styled(logo, app.theme.header)];
    spans.push(Span::raw(" ".repeat(padding)));
    spans.extend(tab_spans);

    let paragraph = Paragraph::new(Line::from(spans));
    frame.render_widget(paragraph, area);
}

fn build_tab_spans(nav: &Navigation, theme: &crate::tui::theme::Theme) -> Vec<Span<'static>> {
    let tabs = [
        ("Speakers", nav.tab == Tab::Speakers),
        ("Settings", nav.tab == Tab::Settings),
    ];
    render_tab_labels(&tabs, nav.tab_focused, theme)
}

fn render_tab_labels(
    tabs: &[(&str, bool)],
    focused: bool,
    theme: &crate::tui::theme::Theme,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (i, (label, is_active)) in tabs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("      "));
        }
        if *is_active {
            let style = if focused { theme.accent } else { theme.header };
            spans.push(Span::styled(format!("[▸{label}]"), style));
        } else {
            let style = theme.muted;
            spans.push(Span::styled(label.to_string(), style));
        }
    }
    spans
}

// ---------------------------------------------------------------------------
// Key legend
// ---------------------------------------------------------------------------

fn render_key_legend(frame: &mut Frame, area: Rect, app: &App) {
    let text = match app.navigation.tab {
        Tab::Speakers => "\u{2191}\u{2193} Navigate   \u{2190}\u{2192} Volume   \u{2423} Pick up/Drop   ? Help   \u{238b} Quit",
        Tab::Settings => "\u{2190}\u{2192} Tabs   \u{238b} Quit",
    };

    let paragraph = Paragraph::new(text).style(app.theme.legend);
    frame.render_widget(paragraph, area);
}
