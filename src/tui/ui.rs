//! TUI rendering — framed layout with header, content, separators, and key legend.

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::tui::app::{App, Navigation, Tab};
use crate::tui::hooks::RenderContext;
use crate::tui::screens;

/// Top-level render dispatch. Draws header, separators, content, bottom bar, and key legend.
///
/// Layout with bottom bar (height >= 12, Speakers tab):
/// ```text
/// Header (1 line)
/// Separator (1 line)
/// Content area (dynamic)
/// Separator (1 line)
/// Bottom bar (3 lines) or (1 line if width < 60)
/// Separator (1 line)
/// Key legend (1 line)
/// ```
///
/// Without bottom bar (Settings tab or height < 12):
/// ```text
/// Header (1 line)
/// Separator (1 line)
/// Content area (dynamic)
/// Separator (1 line)
/// Key legend (1 line)
/// ```
pub fn render(frame: &mut Frame, ctx: &mut RenderContext) {
    let area = frame.area();
    if area.height < 4 || area.width < 20 {
        return;
    }

    // Drive animation tick while toast is active
    let toast_active = ctx.app.toast.as_ref().is_some_and(|t| !t.is_expired());
    ctx.hooks.use_animation("toast", toast_active);

    // Horizontal padding (1 char each side)
    let padded_x = area.x + 1;
    let padded_w = area.width.saturating_sub(2);

    // Determine whether to show the bottom bar
    let show_bottom_bar = ctx.app.navigation.tab == Tab::Speakers && area.height >= 12;

    // Bottom bar height: 3 for normal, 1 for minimal (< 60 cols)
    let bar_height: u16 = if show_bottom_bar {
        if area.width < 60 {
            1
        } else {
            3
        }
    } else {
        0
    };
    // Extra overhead when bar is shown: bar + separator above it
    let bar_overhead = if show_bottom_bar { bar_height + 1 } else { 0 };

    // Header (first row)
    let header_area = Rect::new(padded_x, area.y, padded_w, 1);
    render_header(frame, header_area, ctx.app);

    // Separator between header and content
    draw_separator(
        frame,
        area.y + 1,
        area.x,
        area.x + area.width - 1,
        ctx.app.theme.glyphs.separator,
        ctx.app.theme.muted,
    );

    // Content area: header(1) + sep(1) = 2 from top; sep(1) + legend(1) + bar_overhead from bottom
    let content_height = area.height.saturating_sub(4 + bar_overhead);
    let content_area = Rect::new(padded_x, area.y + 2, padded_w, content_height);

    match ctx.app.navigation.tab {
        Tab::Speakers => {
            // Compute bottom bar area if applicable
            let bar_area = if show_bottom_bar {
                // bar sits between content separator and legend separator
                // Layout from bottom: legend(1) + sep(1) + bar(bar_height) + sep(1)
                let bar_y = area.y + area.height - 1 - 1 - bar_height;
                Some(Rect::new(padded_x, bar_y, padded_w, bar_height))
            } else {
                None
            };

            screens::speakers::render(frame, content_area, bar_area, ctx);
        }
        Tab::Settings => {
            screens::settings::render(frame, content_area, ctx);
        }
    }

    // Separator between content and bottom bar (or footer)
    let content_sep_y = area.y + 2 + content_height;
    draw_separator(
        frame,
        content_sep_y,
        area.x,
        area.x + area.width - 1,
        ctx.app.theme.glyphs.separator,
        ctx.app.theme.muted,
    );

    // Separator between bottom bar and footer (only when bar is shown)
    if show_bottom_bar {
        draw_separator(
            frame,
            area.y + area.height - 2,
            area.x,
            area.x + area.width - 1,
            ctx.app.theme.glyphs.separator,
            ctx.app.theme.muted,
        );
    }

    // Key legend (last row)
    let footer_area = Rect::new(padded_x, area.y + area.height - 1, padded_w, 1);
    render_key_legend(frame, footer_area, ctx.app);
}

/// Draw a full-width horizontal separator line.
fn draw_separator(frame: &mut Frame, y: u16, left: u16, right: u16, sep_char: char, style: Style) {
    let buf = frame.buffer_mut();
    for x in left..=right {
        if let Some(cell) = buf.cell_mut((x, y)) {
            cell.set_char(sep_char).set_style(style);
        }
    }
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let logo = app.theme.glyphs.logo;
    let tab_spans = build_tab_spans(&app.navigation, &app.theme);

    let logo_width = logo.chars().count();
    let tab_width: usize = tab_spans.iter().map(|s| s.content.chars().count()).sum();

    let toast_spans = app.toast.as_ref().filter(|t| !t.is_expired()).map(|t| {
        let style = if t.is_error {
            app.theme.error
        } else {
            app.theme.accent
        };
        vec![Span::styled(
            format!("{} {}", app.theme.glyphs.toast_prefix, t.message),
            style,
        )]
    });

    let toast_width: usize = toast_spans
        .as_ref()
        .map(|spans| spans.iter().map(|s| s.content.chars().count()).sum())
        .unwrap_or(0);

    let fixed_width = logo_width + toast_width + tab_width;
    let total = area.width as usize;

    let mut spans = vec![Span::styled(logo, app.theme.header)];

    if let Some(ts) = toast_spans {
        let left_pad = total.saturating_sub(fixed_width) / 2;
        let right_pad = total.saturating_sub(fixed_width + left_pad);
        spans.push(Span::raw(" ".repeat(left_pad)));
        spans.extend(ts);
        spans.push(Span::raw(" ".repeat(right_pad)));
    } else {
        let padding = total.saturating_sub(logo_width + tab_width);
        spans.push(Span::raw(" ".repeat(padding)));
    }

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
    let g = &theme.glyphs;
    let mut spans = Vec::new();
    for (i, (label, is_active)) in tabs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("      "));
        }
        if *is_active {
            let style = if focused { theme.accent } else { theme.header };
            spans.push(Span::styled(
                format!(
                    "{}{}{label}{}",
                    g.tab_active_left, g.tab_active_indicator, g.tab_active_right
                ),
                style,
            ));
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
        Tab::Speakers => "\u{2191}\u{2193} Navigate   \u{2190}\u{2192} Volume   \u{2423} Pick up/Drop   p Play/Pause  n Next  b Prev   \u{238b} Quit",
        Tab::Settings => {
            if app.navigation.settings_state.dropdown_open {
                "\u{2191}\u{2193} Select  Enter Confirm  \u{238b} Cancel"
            } else {
                "\u{2191}\u{2193} Navigate  Enter Open  \u{2190}\u{2192} Tabs  \u{238b} Quit"
            }
        }
    };

    let paragraph = Paragraph::new(text).style(app.theme.legend);
    frame.render_widget(paragraph, area);
}
