//! Speakers tab screen — data assembly via hooks, delegates rendering to widget.

use ratatui::layout::Rect;
use ratatui::Frame;
use sonos_sdk::{GroupId, PlaybackState, SpeakerId};

use crate::tui::helpers;
use crate::tui::hooks::{ProgressState, RenderContext};
use crate::tui::image_loader::ArtProtocolState;
use crate::tui::types::{
    build_list_entries, group_for_selection, BottomBarData, GroupHeaderData, ListEntry, RowData,
    SpeakerListData, SpeakerRowData,
};
use crate::tui::widgets::{bottom_bar, speaker_list};

/// Render the Speakers tab content area and bottom bar.
///
/// `content_area` is for the speaker list; `bar_area` is for the bottom player bar.
/// When `bar_area` is `None` (terminal too small), the bottom bar is suppressed.
pub fn render(
    frame: &mut Frame,
    content_area: Rect,
    bar_area: Option<Rect>,
    ctx: &mut RenderContext,
) {
    let app = ctx.app;
    let pick_up = app.navigation.speakers_state.pick_up.as_ref();

    let entries = build_list_entries(&app.system, pick_up);
    let rows = entries
        .iter()
        .enumerate()
        .map(|(i, entry)| match entry {
            ListEntry::GroupHeader(group_id) => {
                RowData::GroupHeader(group_header_data(group_id, ctx))
            }
            ListEntry::SpeakerRow(speaker_id) => {
                let is_last_in_group = !matches!(
                    entries.get(i + 1),
                    Some(ListEntry::SpeakerRow(_)) | Some(ListEntry::AddToGroupRow(_))
                );
                RowData::Speaker(speaker_row_data(speaker_id, is_last_in_group, ctx))
            }
            ListEntry::AddToGroupRow(group_id) => RowData::AddToGroup {
                is_home: pick_up
                    .and_then(|pu| pu.original_group_id.as_ref())
                    .is_some_and(|og| og == group_id),
            },
            ListEntry::CreateNewGroupRow => RowData::CreateNewGroup,
        })
        .collect();

    let selected_index = app.navigation.speakers_state.selection.index_in(&entries);

    let data = SpeakerListData {
        entries,
        rows,
        selected_index,
        picked_up_speaker_id: pick_up.map(|pu| pu.speaker_id.clone()),
    };

    speaker_list::render(frame, content_area, &data, &ctx.app.theme);

    // Bottom bar: assemble data for the focused group's coordinator (hidden during pickup)
    if let Some(bar_rect) = bar_area {
        if data.picked_up_speaker_id.is_none() {
            let mut bar_data = assemble_bottom_bar(
                &data.entries,
                data.selected_index,
                bar_rect.width >= 100,
                ctx,
            );
            bottom_bar::render(frame, bar_rect, &mut bar_data, &ctx.app.theme);

            if bar_data.album_art_protocol.is_some() {
                let art_state = ctx
                    .hooks
                    .use_state::<ArtProtocolState>("bottom_bar_art", ArtProtocolState::default);
                art_state.protocol = bar_data.album_art_protocol.take();
            }
        }
    }
}

/// Watch a group's volume, mute, and its coordinator's playback for the header row.
fn group_header_data(group_id: &GroupId, ctx: &mut RenderContext) -> GroupHeaderData {
    let group = ctx.app.system.group_by_id(group_id);
    let coordinator = group.as_ref().and_then(|g| g.coordinator());

    let volume = group
        .as_ref()
        .and_then(|g| ctx.hooks.use_watch_group_or_fetch(&g.volume))
        .map(|v| v.value());
    let is_muted = group
        .as_ref()
        .and_then(|g| ctx.hooks.use_watch_group_or_fetch(&g.mute))
        .is_some_and(|m| m.is_muted());

    let playback_state = coordinator
        .as_ref()
        .and_then(|c| ctx.hooks.use_watch(&c.playback_state));

    let current_track = coordinator
        .as_ref()
        .and_then(|c| ctx.hooks.use_watch(&c.current_track));

    GroupHeaderData {
        name: coordinator
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "Unknown Group".to_string()),
        volume,
        is_muted,
        playback_state,
        track_info: helpers::track_summary(&current_track),
    }
}

/// Watch a speaker's volume, mute, and group membership for its row.
fn speaker_row_data(
    speaker_id: &SpeakerId,
    is_last_in_group: bool,
    ctx: &mut RenderContext,
) -> SpeakerRowData {
    let speaker = ctx.app.system.speaker_by_id(speaker_id);
    let volume = speaker
        .as_ref()
        .and_then(|s| ctx.hooks.use_watch(&s.volume))
        .map(|v| v.value() as u16);
    let is_muted = speaker
        .as_ref()
        .and_then(|s| ctx.hooks.use_watch(&s.mute))
        .is_some_and(|m| m.is_muted());
    if let Some(ref s) = speaker {
        ctx.hooks.use_watch(&s.group_membership);
    }

    SpeakerRowData {
        name: speaker
            .as_ref()
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "Unknown".to_string()),
        model_name: speaker.as_ref().map(|s| s.model_name.clone()),
        volume,
        is_muted,
        is_last_in_group,
    }
}

/// Assemble `BottomBarData` from the focused group's coordinator.
/// What a position fetch is keyed on: coordinator, track URI, playback state.
type PositionSyncKey = (SpeakerId, Option<String>, Option<PlaybackState>);

fn assemble_bottom_bar(
    entries: &[ListEntry],
    selected_index: usize,
    is_wide: bool,
    ctx: &mut RenderContext,
) -> BottomBarData {
    let group = group_for_selection(&ctx.app.system, entries, selected_index);
    let coordinator = group.as_ref().and_then(|g| g.coordinator());

    // Group name
    let group_name = coordinator
        .as_ref()
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "No Group".to_string());

    // Watch playback properties from the coordinator
    let playback_state = coordinator
        .as_ref()
        .and_then(|c| ctx.hooks.use_watch(&c.playback_state));

    let current_track = coordinator
        .as_ref()
        .and_then(|c| ctx.hooks.use_watch(&c.current_track));

    // Sonos never sends the playback position in events, so fetch it when the
    // group's track or playback state changes (and on first sight, via
    // `use_watch_or_fetch`). `ProgressState` interpolates between fetches.
    if let Some(c) = coordinator.as_ref() {
        let sync_key = (
            c.id.clone(),
            current_track.as_ref().and_then(|t| t.uri.clone()),
            playback_state.clone(),
        );
        let last_sync = ctx
            .hooks
            .use_state::<Option<PositionSyncKey>>("bottom_bar_position_sync", || None);
        if last_sync.as_ref() != Some(&sync_key) {
            let first_sight = last_sync.is_none();
            *last_sync = Some(sync_key);
            if !first_sight {
                if let Err(e) = c.position.fetch() {
                    tracing::debug!("position fetch failed for {}: {e}", c.name);
                }
            }
        }
    }
    let position = coordinator
        .as_ref()
        .and_then(|c| ctx.hooks.use_watch_or_fetch(&c.position));

    // Group volume
    let volume = group
        .as_ref()
        .and_then(|g| ctx.hooks.use_watch_group_or_fetch(&g.volume))
        .map(|v| v.value())
        .unwrap_or(0);

    // Animation: enable progress interpolation when playing
    let is_playing = playback_state.as_ref().is_some_and(|ps| ps.is_playing());
    ctx.hooks.use_animation("bottom_bar_progress", is_playing);

    // Progress interpolation via use_state
    let pos_ms = position.as_ref().map(|p| p.position_ms).unwrap_or(0);
    let dur_ms = position.as_ref().map(|p| p.duration_ms).unwrap_or(0);

    let progress_state = ctx
        .hooks
        .use_state::<ProgressState>("bottom_bar_progress", ProgressState::default);
    let now = std::time::Instant::now();
    progress_state.update(pos_ms, dur_ms, is_playing, now);

    let interpolated_pos = progress_state.interpolated_position_ms(now);
    let progress = if dur_ms > 0 {
        (interpolated_pos as f64 / dur_ms as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let track_title = current_track.as_ref().and_then(|t| t.title.clone());
    let track_artist = current_track.as_ref().and_then(|t| t.artist.clone());
    let track_album = current_track.as_ref().and_then(|t| t.album.clone());
    let album_art_uri = current_track.as_ref().and_then(|t| t.album_art_uri.clone());

    // Request album art loading if we have a URI and a coordinator IP
    if let Some(ref uri) = album_art_uri {
        if let Some(ref coord) = coordinator {
            ctx.app.image_loader.request(uri, coord.ip);
        }
    }

    // Manage art protocol state via use_state — must be called after use_watch/use_animation.
    // We take the protocol out temporarily for rendering; it gets put back after render
    // (see caller). If not put back, ensure_protocol recreates it from the image cache.
    let art_state = ctx
        .hooks
        .use_state::<ArtProtocolState>("bottom_bar_art", ArtProtocolState::default);
    art_state.ensure_protocol(&album_art_uri, &ctx.app.image_loader, &ctx.app.picker);
    let protocol = art_state.protocol.take();

    let group_volume_active = selected_index < entries.len()
        && matches!(&entries[selected_index], ListEntry::GroupHeader(_));

    BottomBarData {
        group_name,
        track_title,
        track_artist,
        track_album,
        album_art_protocol: protocol,
        playback_state,
        progress,
        position_ms: interpolated_pos,
        duration_ms: dur_ms,
        volume,
        is_wide,
        group_volume_active,
    }
}
