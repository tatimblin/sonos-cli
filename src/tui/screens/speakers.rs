//! Speakers tab screen — data assembly via hooks, delegates rendering to widget.

use ratatui::layout::Rect;
use ratatui::Frame;

use crate::tui::helpers;
use crate::tui::hooks::{ProgressState, RenderContext};
use crate::tui::types::{
    build_list_entries, group_for_entry, BottomBarData, EntryRenderData, ListEntry,
    SpeakerListData,
};
use crate::tui::widgets::album_art::ArtProtocolState;
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
    let state = &ctx.app.navigation.speakers_state;
    let pick_up = state.pick_up.as_ref();

    let entries = build_list_entries(&ctx.app.system, pick_up);
    let mut entry_data: Vec<EntryRenderData> = Vec::with_capacity(entries.len());

    for (i, entry) in entries.iter().enumerate() {
        match entry {
            ListEntry::SpeakerRow(speaker_id) => {
                let speaker = ctx.app.system.speaker_by_id(speaker_id);
                let vol = speaker
                    .as_ref()
                    .and_then(|s| ctx.hooks.use_watch(&s.volume))
                    .map(|v| v.value() as u16);
                if let Some(ref s) = speaker {
                    ctx.hooks.use_watch(&s.group_membership);
                }
                let name = speaker
                    .as_ref()
                    .map(|s| s.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                let model = speaker.as_ref().map(|s| s.model_name.clone());

                let is_last = !matches!(
                    entries.get(i + 1),
                    Some(ListEntry::SpeakerRow(_)) | Some(ListEntry::AddToGroupRow(_))
                );

                entry_data.push(EntryRenderData {
                    name,
                    model_name: model,
                    speaker_volume: vol,
                    group_volume: None,
                    playback_state: None,
                    track_info: None,
                    is_last_in_group: is_last,
                    is_home_group: false,
                });
            }
            ListEntry::GroupHeader(group_id) => {
                let group = ctx.app.system.group_by_id(group_id);
                let coordinator = group.as_ref().and_then(|g| g.coordinator());

                let gvol = group
                    .as_ref()
                    .and_then(|g| ctx.hooks.use_watch_group(&g.volume))
                    .map(|v| v.value());

                let pb = coordinator
                    .as_ref()
                    .and_then(|c| ctx.hooks.use_watch(&c.playback_state));

                let current_track = coordinator
                    .as_ref()
                    .and_then(|c| ctx.hooks.use_watch(&c.current_track));
                let track = helpers::track_summary(&current_track);

                let name = coordinator
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| "Unknown Group".to_string());

                entry_data.push(EntryRenderData {
                    name,
                    model_name: None,
                    speaker_volume: None,
                    group_volume: gvol,
                    playback_state: pb,
                    track_info: track,
                    is_last_in_group: false,
                    is_home_group: false,
                });
            }
            ListEntry::AddToGroupRow(group_id) => {
                let is_home = pick_up
                    .and_then(|pu| pu.original_group_id.as_ref())
                    .is_some_and(|og| og == group_id);

                let name = if is_home {
                    "Already in group".to_string()
                } else {
                    "Add to group".to_string()
                };

                entry_data.push(EntryRenderData {
                    name,
                    model_name: None,
                    speaker_volume: None,
                    group_volume: None,
                    playback_state: None,
                    track_info: None,
                    is_last_in_group: true,
                    is_home_group: is_home,
                });
            }
            ListEntry::CreateNewGroupRow => {
                entry_data.push(EntryRenderData {
                    name: "Create new group".to_string(),
                    model_name: None,
                    speaker_volume: None,
                    group_volume: None,
                    playback_state: None,
                    track_info: None,
                    is_last_in_group: false,
                    is_home_group: false,
                });
            }
        }
    }

    let state = &ctx.app.navigation.speakers_state;
    let selected_index = state.selected_index.min(entries.len().saturating_sub(1));

    let data = SpeakerListData {
        entries,
        entry_data,
        selected_index,
        status_message: ctx.app.status_message.clone(),
        picked_up_speaker_name: pick_up.map(|pu| pu.speaker_name.clone()),
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

/// Assemble `BottomBarData` from the focused group's coordinator.
fn assemble_bottom_bar(
    entries: &[ListEntry],
    selected_index: usize,
    is_wide: bool,
    ctx: &mut RenderContext,
) -> BottomBarData {
    // Find the group for the currently focused entry
    let group_id = group_for_entry(entries, selected_index);

    // For standalone speakers (ungrouped section), resolve via the speaker's own group
    let resolved_group_id = group_id.or_else(|| {
        if selected_index < entries.len() {
            if let ListEntry::SpeakerRow(sid) = &entries[selected_index] {
                return ctx
                    .app
                    .system
                    .speaker_by_id(sid)
                    .and_then(|s| s.group())
                    .map(|g| g.id.clone());
            }
        }
        None
    });

    let group = resolved_group_id
        .as_ref()
        .and_then(|gid| ctx.app.system.group_by_id(gid));
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

    let position = coordinator
        .as_ref()
        .and_then(|c| ctx.hooks.use_watch(&c.position));

    // Group volume
    let volume = group
        .as_ref()
        .and_then(|g| ctx.hooks.use_watch_group(&g.volume))
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
    progress_state.update(pos_ms, dur_ms, is_playing);

    let interpolated_pos = progress_state.interpolated_position_ms();
    let progress = if dur_ms > 0 {
        (interpolated_pos as f64 / dur_ms as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };

    // Extract track metadata with URI fallback
    let track_title = current_track.as_ref().and_then(|t| {
        t.title.clone().or_else(|| {
            t.uri
                .as_deref()
                .map(helpers::uri_source_label)
                .filter(|s| !s.is_empty())
                .map(String::from)
        })
    });
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
    }
}
