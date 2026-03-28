//! Home > Groups tab — responsive grid of live group cards.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use sonos_sdk::PlaybackState;

use crate::tui::app::{App, HomeGroupsState};
use crate::tui::widgets::group_card::{self, GroupCardData, PlaybackIcon};

/// Card height (border + 7 content lines).
const CARD_HEIGHT: u16 = 9;

/// Render the Groups tab content.
pub fn render(frame: &mut Frame, area: Rect, app: &App, state: &HomeGroupsState) {
    let groups = app.system.groups();

    if groups.is_empty() {
        let paragraph = Paragraph::new("No groups found")
            .alignment(Alignment::Center)
            .style(app.theme.muted);
        frame.render_widget(paragraph, area);
        return;
    }

    let cols = if area.width >= 100 { 2usize } else { 1 };
    let rows = groups.len().div_ceil(cols);

    // Build row constraints
    let row_constraints: Vec<Constraint> =
        (0..rows).map(|_| Constraint::Length(CARD_HEIGHT)).collect();

    let row_areas = Layout::vertical(row_constraints).split(area);

    for (row_idx, row_area) in row_areas.iter().enumerate() {
        if row_area.y + row_area.height > area.y + area.height {
            break; // don't render past visible area
        }

        let col_constraints: Vec<Constraint> = (0..cols)
            .map(|_| Constraint::Ratio(1, cols as u32))
            .collect();
        let col_areas = Layout::horizontal(col_constraints).split(*row_area);

        for (col_idx, col_area) in col_areas.iter().enumerate() {
            let group_idx = row_idx * cols + col_idx;
            if group_idx >= groups.len() {
                break;
            }

            let group = &groups[group_idx];
            let selected = group_idx == state.selected_index;

            let coordinator = group.coordinator();
            if coordinator.is_none() {
                let name = format!("Group {}", group.id);
                group_card::render_unavailable_card(frame, *col_area, &name, selected, &app.theme);
                continue;
            }
            let coordinator = coordinator.unwrap();

            // Watch properties — subscribes and returns current value
            let playback_state = app.watch(&coordinator.playback_state);
            let current_track = app.watch(&coordinator.current_track);
            let position = app.watch(&coordinator.position);
            let group_volume = app.watch_group(&group.volume);

            let playback_icon = match playback_state.as_ref() {
                Some(PlaybackState::Playing) => PlaybackIcon::Playing,
                Some(PlaybackState::Paused) => PlaybackIcon::Paused,
                _ => PlaybackIcon::Stopped,
            };

            let (track_title, track_artist) = current_track
                .as_ref()
                .filter(|t| !t.is_empty())
                .map(|t| {
                    (
                        t.title.clone().unwrap_or_default(),
                        t.artist.clone().unwrap_or_default(),
                    )
                })
                .unwrap_or_default();

            let volume = group_volume.map(|v| v.value()).unwrap_or(0);

            // Use interpolated progress if available, otherwise SDK position
            let (progress, elapsed_ms, duration_ms) =
                if let Some(ps) = app.progress_states.get(&group.id) {
                    let elapsed = ps.interpolated_position_ms();
                    let duration = ps.last_duration_ms;
                    let ratio = if duration > 0 {
                        elapsed as f64 / duration as f64
                    } else {
                        0.0
                    };
                    (ratio, elapsed, duration)
                } else if let Some(pos) = position.as_ref() {
                    (pos.progress(), pos.position_ms, pos.duration_ms)
                } else {
                    (0.0, 0, 0)
                };

            // Speaker count text
            let members = group.members();
            let speaker_count_text = if members.len() <= 1 {
                coordinator.model_name.clone()
            } else {
                format!("{} + {}", coordinator.model_name, members.len() - 1)
            };

            let data = GroupCardData {
                group_name: coordinator.name.clone(),
                playback_state: playback_icon,
                track_title,
                track_artist,
                volume,
                progress,
                elapsed_ms,
                duration_ms,
                speaker_count_text,
                selected,
            };

            group_card::render_group_card(frame, *col_area, &data, &app.theme);
        }
    }
}
