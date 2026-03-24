//! TUI rendering — layout dispatch, screen stubs, breadcrumb, and key legend.
//!
//! All rendering lives in this one file for M6. When a screen or widget grows
//! beyond ~80 lines, extract it to `tui/screens/<name>.rs` or
//! `tui/widgets/<name>.rs`.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use sonos_sdk::SonosSystem;

use crate::tui::app::{App, GroupTab, HomeTab, Screen};

/// Top-level render dispatch. Splits the frame into header / content / legend.
pub fn render(frame: &mut Frame, app: &App) {
    let [header_area, content_area, legend_area] = Layout::vertical([
        Constraint::Length(1), // breadcrumb header
        Constraint::Min(0),    // content area
        Constraint::Length(1), // key legend
    ])
    .areas(frame.area());

    render_breadcrumb(frame, header_area, app);

    match app.navigation.current() {
        Screen::Home { tab, .. } => render_home(frame, content_area, app, tab),
        Screen::GroupView { group_id, tab } => {
            render_group_view(frame, content_area, app, group_id, tab)
        }
        Screen::SpeakerDetail { speaker_id } => {
            render_speaker_detail(frame, content_area, app, speaker_id)
        }
    }

    render_key_legend(frame, legend_area, app);
}

// ---------------------------------------------------------------------------
// Breadcrumb header
// ---------------------------------------------------------------------------

fn screen_label(screen: &Screen, system: &SonosSystem) -> String {
    match screen {
        Screen::Home { .. } => "SONOS".to_string(),
        Screen::GroupView { group_id, .. } => system
            .group_by_id(group_id)
            .and_then(|g| g.coordinator())
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "Group".to_string()),
        Screen::SpeakerDetail { speaker_id } => system
            .speaker_by_id(speaker_id)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "Speaker".to_string()),
    }
}

fn render_breadcrumb(frame: &mut Frame, area: Rect, app: &App) {
    let labels: Vec<String> = app
        .navigation
        .stack
        .iter()
        .map(|screen| screen_label(screen, &app.system))
        .collect();
    let breadcrumb = labels.join(" > ");

    let mut spans = vec![Span::styled(&breadcrumb, app.theme.header)];

    if let Some(tab_text) = current_tab_text(app.navigation.current()) {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(tab_text, app.theme.muted));
    }

    let paragraph = Paragraph::new(Line::from(spans)).style(app.theme.header);
    frame.render_widget(paragraph, area);
}

fn current_tab_text(screen: &Screen) -> Option<String> {
    match screen {
        Screen::Home { tab, .. } => {
            let groups = if *tab == HomeTab::Groups {
                "[Groups]"
            } else {
                " Groups "
            };
            let speakers = if *tab == HomeTab::Speakers {
                "[Speakers]"
            } else {
                " Speakers "
            };
            Some(format!("{groups} {speakers}"))
        }
        Screen::GroupView { tab, .. } => {
            let np = if *tab == GroupTab::NowPlaying {
                "[NowPlaying]"
            } else {
                " NowPlaying "
            };
            let sp = if *tab == GroupTab::Speakers {
                "[Speakers]"
            } else {
                " Speakers "
            };
            let q = if *tab == GroupTab::Queue {
                "[Queue]"
            } else {
                " Queue "
            };
            Some(format!("{np} {sp} {q}"))
        }
        Screen::SpeakerDetail { .. } => None,
    }
}

// ---------------------------------------------------------------------------
// Key legend
// ---------------------------------------------------------------------------

fn render_key_legend(frame: &mut Frame, area: Rect, app: &App) {
    let text = match app.navigation.current() {
        Screen::Home {
            tab: HomeTab::Groups,
            ..
        } => "Tab Switch  ↑↓←→ Select  Enter Open group  q Quit",
        Screen::Home {
            tab: HomeTab::Speakers,
            ..
        } => "Tab Switch  ↑↓ Navigate  n New group  d Ungroup  Enter Move  q Quit",
        Screen::GroupView {
            tab: GroupTab::NowPlaying,
            ..
        } => "←→ Tabs  Esc Back",
        Screen::GroupView {
            tab: GroupTab::Speakers,
            ..
        } => "←→ Tabs  Enter Open speaker  Esc Back",
        Screen::GroupView {
            tab: GroupTab::Queue,
            ..
        } => "←→ Tabs  Esc Back",
        Screen::SpeakerDetail { .. } => "Esc Back",
    };

    let paragraph = Paragraph::new(text).style(app.theme.legend);
    frame.render_widget(paragraph, area);
}

// ---------------------------------------------------------------------------
// Screen stubs — replaced with real content in M7+
// ---------------------------------------------------------------------------

fn render_home(frame: &mut Frame, area: Rect, app: &App, tab: &HomeTab) {
    let text = match tab {
        HomeTab::Groups => "Groups — Milestone 7",
        HomeTab::Speakers => "Speakers — Milestone 7",
    };
    let paragraph = Paragraph::new(text)
        .alignment(Alignment::Center)
        .style(app.theme.muted);
    frame.render_widget(paragraph, area);
}

fn render_group_view(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    _group_id: &sonos_sdk::GroupId,
    tab: &GroupTab,
) {
    let text = match tab {
        GroupTab::NowPlaying => "Now Playing — Milestone 7",
        GroupTab::Speakers => "Group Speakers — Milestone 7",
        GroupTab::Queue => "Queue — Milestone 7",
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
    let paragraph = Paragraph::new("Speaker Detail — Milestone 8")
        .alignment(Alignment::Center)
        .style(app.theme.muted);
    frame.render_widget(paragraph, area);
}
