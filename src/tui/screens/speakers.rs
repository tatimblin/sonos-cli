//! Speakers tab screen — data assembly via hooks, delegates rendering to widget.

use ratatui::layout::Rect;
use ratatui::Frame;

use crate::tui::helpers;
use crate::tui::hooks::RenderContext;
use crate::tui::types::{build_list_entries, EntryRenderData, ListEntry, SpeakerListData};
use crate::tui::widgets::speaker_list;

/// Render the Speakers tab content.
pub fn render(frame: &mut Frame, area: Rect, ctx: &mut RenderContext) {
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
    let data = SpeakerListData {
        entries,
        entry_data,
        selected_index: state.selected_index,
        pick_up: state.pick_up.clone(),
        status_message: ctx.app.status_message.clone(),
    };

    speaker_list::render(frame, area, &data, &ctx.app.theme);
}
