//! Speakers tab screen — data assembly via hooks, delegates rendering to widget.

use ratatui::layout::Rect;
use ratatui::Frame;

use crate::tui::helpers;
use crate::tui::hooks::RenderContext;
use crate::tui::types::{
    build_list_entries, DropZone, DropZoneData, DropZoneKind, EntryRenderData, ListEntry,
    SpeakerListData, SpeakerScreenData,
};
use crate::tui::widgets::speaker_list;

/// Render the Speakers tab content.
pub fn render(frame: &mut Frame, area: Rect, ctx: &mut RenderContext) {
    let state = &ctx.app.navigation.speakers_state;

    let data = if let Some(ref pick_up) = state.pick_up {
        // Build drop zone data from current groups
        let groups = ctx.app.system.groups();
        let mut zones: Vec<DropZone> = Vec::new();

        for group in &groups {
            let coordinator = group.coordinator();

            // Subscribe to topology changes so the list refreshes after regrouping
            if let Some(ref coord) = coordinator {
                ctx.hooks.use_watch(&coord.group_membership);
            }

            let members = group.members();
            let group_name = coordinator
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "Unknown Group".to_string());

            // Remaining members: all members except the picked-up speaker
            let remaining: Vec<String> = members
                .iter()
                .filter(|m| m.id != pick_up.speaker_id)
                .map(|m| m.name.clone())
                .collect();

            // Inner height = original member count (including picked-up speaker if it was here)
            let inner_height = members.len().max(1);

            zones.push(DropZone {
                kind: DropZoneKind::ExistingGroup(group.id.clone()),
                group_name,
                remaining_members: remaining,
                inner_height,
            });
        }

        // Always add "Add new group" zone at the bottom
        zones.push(DropZone {
            kind: DropZoneKind::NewGroup,
            group_name: "Add new group".to_string(),
            remaining_members: Vec::new(),
            inner_height: 1,
        });

        // Clamp active_zone_index
        let active = pick_up.active_zone_index.min(zones.len().saturating_sub(1));

        SpeakerScreenData::PickUp(DropZoneData {
            zones,
            active_zone_index: active,
            speaker_name: pick_up.speaker_name.clone(),
            status_message: ctx.app.status_message.clone(),
        })
    } else {
        // Normal mode: build entry list
        let entries = build_list_entries(&ctx.app.system);
        let mut entry_data: Vec<EntryRenderData> = Vec::with_capacity(entries.len());

        for entry in &entries {
            match entry {
                ListEntry::SpeakerRow(speaker_id) => {
                    let speaker = ctx.app.system.speaker_by_id(speaker_id);
                    let vol = speaker
                        .as_ref()
                        .and_then(|s| ctx.hooks.use_watch(&s.volume))
                        .map(|v| v.value() as u16);
                    // Subscribe to topology changes so the list refreshes after regrouping
                    if let Some(ref s) = speaker {
                        ctx.hooks.use_watch(&s.group_membership);
                    }
                    let name = speaker
                        .map(|s| s.name.clone())
                        .unwrap_or_else(|| "Unknown".to_string());
                    entry_data.push(EntryRenderData {
                        name,
                        speaker_volume: vol,
                        group_volume: None,
                        playback_state: None,
                        track_info: None,
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
                        speaker_volume: None,
                        group_volume: gvol,
                        playback_state: pb,
                        track_info: track,
                    });
                }
                ListEntry::UngroupedHeader => {
                    entry_data.push(EntryRenderData {
                        name: String::new(),
                        speaker_volume: None,
                        group_volume: None,
                        playback_state: None,
                        track_info: None,
                    });
                }
            }
        }

        let state = &ctx.app.navigation.speakers_state;
        SpeakerScreenData::Normal(SpeakerListData {
            entries,
            entry_data,
            selected_index: state.selected_index,
            status_message: ctx.app.status_message.clone(),
        })
    };

    speaker_list::render(frame, area, &data, &ctx.app.theme);
}
