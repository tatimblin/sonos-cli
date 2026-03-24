//! TUI rendering — layout dispatch, breadcrumb, key legend, and mini-player.
//!
//! Screen rendering is delegated to `tui/screens/` modules. Widget rendering
//! lives in `tui/widgets/`.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use sonos_sdk::SonosSystem;

use crate::tui::app::{App, GroupTab, HomeGroupsState, HomeTab, HomeSpeakersState, Screen};
use crate::tui::screens::{home_groups, home_speakers};
use crate::tui::widgets::group_card::PlaybackIcon;
use crate::tui::widgets::mini_player::{self, MiniPlayerData};

/// Top-level render dispatch. Splits the frame into header / content / [mini-player] / legend.
pub fn render(frame: &mut Frame, app: &App) {
    let is_home = matches!(app.navigation.current(), Screen::Home { .. });

    let areas = if is_home && !app.system.groups().is_empty() {
        // 4-region layout with mini-player
        let [header, content, mini, legend] = Layout::vertical([
            Constraint::Length(1),  // breadcrumb header
            Constraint::Min(0),    // content area
            Constraint::Length(3), // mini-player (top border + 2 lines)
            Constraint::Length(1), // key legend
        ])
        .areas(frame.area());
        (header, content, Some(mini), legend)
    } else {
        // 3-region layout without mini-player
        let [header, content, legend] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .areas(frame.area());
        (header, content, None, legend)
    };

    let (header_area, content_area, mini_player_area, legend_area) = areas;

    render_breadcrumb(frame, header_area, app);

    match app.navigation.current() {
        Screen::Home {
            tab,
            groups_state,
            speakers_state,
            ..
        } => {
            match tab {
                HomeTab::Groups => home_groups::render(frame, content_area, app, groups_state),
                HomeTab::Speakers => home_speakers::render(frame, content_area, app, speakers_state),
            }

            if let Some(mini_area) = mini_player_area {
                render_mini_player(frame, mini_area, app, tab, groups_state, speakers_state);
            }
        }
        Screen::GroupView { group_id, tab } => {
            render_group_view(frame, content_area, app, group_id, tab);
        }
        Screen::SpeakerDetail { speaker_id } => {
            render_speaker_detail(frame, content_area, app, speaker_id);
        }
    }

    render_key_legend(frame, legend_area, app);
}

// ---------------------------------------------------------------------------
// Mini-player
// ---------------------------------------------------------------------------

fn render_mini_player(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    tab: &HomeTab,
    groups_state: &HomeGroupsState,
    speakers_state: &HomeSpeakersState,
) {
    let groups = app.system.groups();
    if groups.is_empty() {
        return;
    }

    // Determine focused group based on active tab
    let focused_group = match tab {
        HomeTab::Groups => groups.get(groups_state.selected_index),
        HomeTab::Speakers => {
            // Find the group of the selected speaker
            let speakers = app.system.speakers();
            speakers
                .get(speakers_state.selected_index)
                .and_then(|s| s.group())
                .or_else(|| groups.first().cloned())
                .as_ref()
                .and_then(|g| groups.iter().find(|og| og.id == g.id))
        }
    };

    let Some(group) = focused_group else {
        return;
    };

    let coordinator = match group.coordinator() {
        Some(c) => c,
        None => return,
    };

    let playback_state = coordinator.playback_state.get();
    let current_track = coordinator.current_track.get();
    let group_volume = group.volume.get();

    let playback_icon = match playback_state.as_ref() {
        Some(sonos_sdk::PlaybackState::Playing) => PlaybackIcon::Playing,
        Some(sonos_sdk::PlaybackState::Paused) => PlaybackIcon::Paused,
        _ => PlaybackIcon::Stopped,
    };

    let track_display = current_track
        .as_ref()
        .filter(|t| !t.is_empty())
        .map(|t| t.display())
        .unwrap_or_default();

    let volume = group_volume.map(|v| v.value()).unwrap_or(0);

    let (progress, elapsed_ms, duration_ms) = if let Some(ps) = app.progress_states.get(&group.id)
    {
        let elapsed = ps.interpolated_position_ms();
        let duration = ps.last_duration_ms;
        let ratio = if duration > 0 {
            elapsed as f64 / duration as f64
        } else {
            0.0
        };
        (ratio, elapsed, duration)
    } else {
        let position = coordinator.position.get();
        match position.as_ref() {
            Some(pos) => (pos.progress(), pos.position_ms, pos.duration_ms),
            None => (0.0, 0, 0),
        }
    };

    let data = MiniPlayerData {
        group_name: coordinator.name.clone(),
        playback_state: playback_icon,
        track_display,
        volume,
        progress,
        elapsed_ms,
        duration_ms,
    };

    mini_player::render_mini_player(frame, area, &data, &app.theme);
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

    let tab_spans = current_tab_spans(app.navigation.current(), &app.theme);
    if !tab_spans.is_empty() {
        spans.push(Span::raw("  "));
        spans.extend(tab_spans);
    }

    let paragraph = Paragraph::new(Line::from(spans)).style(app.theme.header);
    frame.render_widget(paragraph, area);
}

fn current_tab_spans(screen: &Screen, theme: &crate::tui::theme::Theme) -> Vec<Span<'static>> {
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
            spans.push(Span::raw(" "));
        }
        let style = if *is_active && focused {
            // Tab bar focused + this is the active tab: highlighted
            theme.accent
        } else if *is_active {
            // Active tab but tab bar not focused: just brackets, muted
            theme.muted
        } else {
            // Inactive tab: dimmer
            theme.muted
        };
        let text = if *is_active {
            format!("[{label}]")
        } else {
            format!(" {label} ")
        };
        spans.push(Span::styled(text, style));
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
        } => "↑↓←→ Navigate  Enter Open group  q Quit",
        Screen::Home {
            tab: HomeTab::Speakers,
            ..
        } => "↑↓ Navigate  n New group  d Ungroup  Enter Move  q Quit",
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
