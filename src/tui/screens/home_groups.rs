//! Home > Groups tab — responsive grid of live group cards.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use sonos_sdk::PlaybackState;

use crate::tui::app::HomeGroupsState;
use crate::tui::hooks::{ProgressState, RenderContext};
use crate::tui::widgets::group_card::{self, GroupCardData, PlaybackIcon};

/// Card height (border + 7 content lines).
const CARD_HEIGHT: u16 = 9;

/// Render the Groups tab content.
pub fn render(frame: &mut Frame, area: Rect, ctx: &mut RenderContext, state: &HomeGroupsState) {
    let groups = ctx.app.system.groups();

    if groups.is_empty() {
        let paragraph = Paragraph::new("No groups found")
            .alignment(Alignment::Center)
            .style(ctx.app.theme.muted);
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
                group_card::render_unavailable_card(
                    frame,
                    *col_area,
                    &name,
                    selected,
                    &ctx.app.theme,
                );
                continue;
            }
            let coordinator = coordinator.unwrap();

            // Hooks: use_watch returns owned values (borrow released immediately)
            let playback_state = ctx.hooks.use_watch(&coordinator.playback_state);
            let current_track = ctx.hooks.use_watch(&coordinator.current_track);
            let position = ctx.hooks.use_watch(&coordinator.position);
            let group_volume = ctx.hooks.use_watch_group(&group.volume);

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

            let is_playing = playback_state
                .as_ref()
                .map_or(false, |p| *p == PlaybackState::Playing);

            // Hooks: use_animation before use_state (borrow ordering)
            let group_id_str = group.id.to_string();
            ctx.hooks
                .use_animation(&format!("{group_id_str}:tick"), is_playing);

            // Hooks: use_state for progress interpolation (must be last)
            let progress_key = format!("{group_id_str}:progress");
            let progress_state =
                ctx.hooks
                    .use_state::<ProgressState>(&progress_key, ProgressState::default);

            // Update progress state from SDK values
            if let Some(pos) = position.as_ref() {
                progress_state.update(pos.position_ms, pos.duration_ms, is_playing);
            } else {
                // No position data yet — just update playing state
                progress_state.is_playing = is_playing;
            }

            let elapsed = progress_state.interpolated_position_ms();
            let duration = progress_state.last_duration_ms;
            let progress = if duration > 0 {
                elapsed as f64 / duration as f64
            } else {
                0.0
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
                elapsed_ms: elapsed,
                duration_ms: duration,
                speaker_count_text,
                selected,
            };

            group_card::render_group_card(frame, *col_area, &data, &ctx.app.theme);
        }
    }
}
