//! TUI rendering — framed layout with header, content, separators, and key legend.
//!
//! Screen rendering is delegated to `tui/screens/` modules. Widget rendering
//! lives in `tui/widgets/`.

use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use sonos_sdk::SonosSystem;

use crate::tui::app::{App, GroupTab, HomeTab, Screen};
use crate::tui::hooks::RenderContext;
use crate::tui::screens::{home_groups, home_speakers};

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

    match ctx.app.navigation.current() {
        Screen::Home {
            tab,
            groups_state,
            speakers_state,
            ..
        } => match tab {
            HomeTab::Groups => {
                let groups_state = groups_state.clone();
                home_groups::render(frame, content_area, ctx, &groups_state);
            }
            HomeTab::Speakers => {
                let speakers_state = speakers_state.clone();
                home_speakers::render(frame, content_area, ctx, &speakers_state);
            }
        },
        Screen::GroupView { group_id, tab } => {
            let group_id = group_id.clone();
            let tab = tab.clone();
            render_group_view(frame, content_area, ctx.app, &group_id, &tab);
        }
        Screen::SpeakerDetail { speaker_id } => {
            let speaker_id = speaker_id.clone();
            render_speaker_detail(frame, content_area, ctx.app, &speaker_id);
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
            cell.set_char('─').set_style(style);
        }
    }
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let screen = app.navigation.current();

    let logo = build_logo(screen, &app.system);
    let tab_spans = build_tab_spans(screen, &app.theme);

    let logo_width = logo.chars().count();
    let tab_width: usize = tab_spans.iter().map(|s| s.content.chars().count()).sum();
    let padding = (area.width as usize).saturating_sub(logo_width + tab_width);

    let mut spans = vec![Span::styled(logo, app.theme.header)];
    spans.push(Span::raw(" ".repeat(padding)));
    spans.extend(tab_spans);

    let paragraph = Paragraph::new(Line::from(spans));
    frame.render_widget(paragraph, area);
}

fn build_logo(screen: &Screen, system: &SonosSystem) -> String {
    let base = "♪  S O N O S";

    match screen {
        Screen::Home { .. } => base.to_string(),
        Screen::GroupView { group_id, .. } => {
            let name = system
                .group_by_id(group_id)
                .and_then(|g| g.coordinator())
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "Group".to_string());
            format!("{base}  ›  {name}")
        }
        Screen::SpeakerDetail { speaker_id } => {
            let name = system
                .speaker_by_id(speaker_id)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "Speaker".to_string());
            format!("{base}  ›  {name}")
        }
    }
}

fn build_tab_spans(screen: &Screen, theme: &crate::tui::theme::Theme) -> Vec<Span<'static>> {
    match screen {
        Screen::Home {
            tab, tab_focused, ..
        } => {
            let tabs = [
                ("Groups", *tab == HomeTab::Groups),
                ("Speakers", *tab == HomeTab::Speakers),
            ];
            render_tab_labels(&tabs, *tab_focused, theme)
        }
        Screen::GroupView { tab, .. } => {
            let tabs = [
                ("NowPlaying", *tab == GroupTab::NowPlaying),
                ("Speakers", *tab == GroupTab::Speakers),
                ("Queue", *tab == GroupTab::Queue),
            ];
            render_tab_labels(&tabs, false, theme)
        }
        Screen::SpeakerDetail { .. } => vec![],
    }
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
    let text = match app.navigation.current() {
        Screen::Home {
            tab: HomeTab::Groups,
            ..
        } => "↑↓←→ Navigate   ⏎ Open   ␣ Play/Pause   ⎋ Quit",
        Screen::Home {
            tab: HomeTab::Speakers,
            ..
        } => "↑↓ Navigate   n New group   d Ungroup   ⏎ Move   ⎋ Quit",
        Screen::GroupView {
            tab: GroupTab::NowPlaying,
            ..
        } => "←→ Tabs   ⎋ Back",
        Screen::GroupView {
            tab: GroupTab::Speakers,
            ..
        } => "←→ Tabs   ⏎ Open speaker   ⎋ Back",
        Screen::GroupView {
            tab: GroupTab::Queue,
            ..
        } => "←→ Tabs   ⎋ Back",
        Screen::SpeakerDetail { .. } => "⎋ Back",
    };

    let paragraph = Paragraph::new(text).style(app.theme.legend);
    frame.render_widget(paragraph, area);
}

// ---------------------------------------------------------------------------
// Screen stubs — replaced with real content in M8+
// ---------------------------------------------------------------------------

fn render_group_view(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    _group_id: &sonos_sdk::GroupId,
    tab: &GroupTab,
) {
    let text = match tab {
        GroupTab::NowPlaying => "Now Playing — Milestone 8",
        GroupTab::Speakers => "Group Speakers — Milestone 8",
        GroupTab::Queue => "Queue — Milestone 8",
    };
    let paragraph = Paragraph::new(text)
        .alignment(Alignment::Center)
        .style(app.theme.muted);
    frame.render_widget(paragraph, area);
}

fn render_speaker_detail(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    _speaker_id: &sonos_sdk::SpeakerId,
) {
    let paragraph = Paragraph::new("Speaker Detail — Milestone 9")
        .alignment(Alignment::Center)
        .style(app.theme.muted);
    frame.render_widget(paragraph, area);
}
