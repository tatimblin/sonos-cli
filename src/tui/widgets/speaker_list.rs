//! Speaker list widget — renders grouped speakers with volume, playback state,
//! and pick-up/drop regrouping.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use sonos_sdk::{GroupId, PlaybackState, SonosSystem, SpeakerId};

use crate::tui::app::{App, SpeakerListScreenState};
use crate::tui::hooks::RenderContext;
use crate::tui::theme::Theme;
use crate::tui::widgets::volume_bar;

// ============================================================================
// Types
// ============================================================================

/// A single row in the flat list. Navigation and rendering dispatch on this.
#[derive(Clone, Debug, PartialEq)]
pub enum ListEntry {
    GroupHeader(GroupId),
    SpeakerRow(SpeakerId),
    UngroupedHeader,
}

impl ListEntry {
    fn is_selectable(&self) -> bool {
        !matches!(self, ListEntry::UngroupedHeader)
    }
}

/// State for a speaker being moved between groups.
#[derive(Clone, Debug)]
pub struct PickUpState {
    pub speaker_id: SpeakerId,
    pub original_group_id: Option<GroupId>,
    pub drop_index: usize,
}

/// Action returned from `handle_key` so callers can respond.
pub enum SpeakerListAction {
    Handled,
    FocusTabBar,
}

/// Per-entry render data collected during the watch-subscription pass.
struct EntryRenderData {
    speaker_volume: Option<u16>,
    group_volume: Option<u16>,
    playback_state: Option<PlaybackState>,
    track_info: Option<String>,
}

// ============================================================================
// List building
// ============================================================================

/// Build the flat list of entries from the current system state.
pub fn build_list_entries(system: &SonosSystem) -> Vec<ListEntry> {
    let groups = system.groups();
    let mut entries = Vec::new();

    // Multi-member groups first
    for group in &groups {
        if group.is_standalone() {
            continue;
        }
        entries.push(ListEntry::GroupHeader(group.id.clone()));
        for member in group.members() {
            entries.push(ListEntry::SpeakerRow(member.id.clone()));
        }
    }

    // Standalone speakers
    let standalones: Vec<_> = groups
        .iter()
        .filter(|g| g.is_standalone())
        .filter_map(|g| g.coordinator())
        .collect();

    if !standalones.is_empty() {
        entries.push(ListEntry::UngroupedHeader);
        for speaker in standalones {
            entries.push(ListEntry::SpeakerRow(speaker.id.clone()));
        }
    }

    entries
}

/// Determine which group a list entry at `index` belongs to.
fn group_for_entry(entries: &[ListEntry], index: usize) -> Option<GroupId> {
    if index >= entries.len() {
        return None;
    }
    for i in (0..=index).rev() {
        match &entries[i] {
            ListEntry::GroupHeader(gid) => return Some(gid.clone()),
            ListEntry::UngroupedHeader => return None,
            _ => continue,
        }
    }
    None
}

/// Build display order for pick-up mode: the picked-up speaker is removed from its
/// original position and inserted at the drop position, so it visually moves through
/// the list with other entries shifting to fill the gap.
fn build_display_order(entries: &[ListEntry], pick_up: &Option<PickUpState>) -> Vec<usize> {
    let identity = || (0..entries.len()).collect();

    let Some(pick_up) = pick_up else {
        return identity();
    };

    let Some(orig_idx) = entries
        .iter()
        .position(|e| matches!(e, ListEntry::SpeakerRow(sid) if *sid == pick_up.speaker_id))
    else {
        return identity();
    };

    if orig_idx == pick_up.drop_index {
        return identity();
    }

    let mut order: Vec<usize> = (0..entries.len()).collect();
    order.remove(orig_idx);

    let insert_at = if orig_idx < pick_up.drop_index {
        pick_up.drop_index - 1
    } else {
        pick_up.drop_index
    };
    let insert_at = insert_at.min(order.len());
    order.insert(insert_at, orig_idx);

    order
}

// ============================================================================
// Rendering
// ============================================================================

/// Render the speaker list widget.
pub fn render(
    frame: &mut Frame,
    area: Rect,
    ctx: &mut RenderContext,
    state: &SpeakerListScreenState,
) {
    let speakers = ctx.app.system.speakers();

    if speakers.is_empty() {
        let paragraph = Paragraph::new("No speakers found")
            .alignment(ratatui::layout::Alignment::Center)
            .style(ctx.app.theme.muted);
        frame.render_widget(paragraph, area);
        return;
    }

    let entries = build_list_entries(&ctx.app.system);

    if entries.is_empty() {
        let paragraph = Paragraph::new("No speakers in group")
            .alignment(ratatui::layout::Alignment::Center)
            .style(ctx.app.theme.muted);
        frame.render_widget(paragraph, area);
        return;
    }

    // Watch volumes for all speakers and group properties for all groups in the list.
    let mut render_data: Vec<EntryRenderData> = Vec::with_capacity(entries.len());

    for entry in &entries {
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
                render_data.push(EntryRenderData {
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

                let track = coordinator
                    .as_ref()
                    .and_then(|c| ctx.hooks.use_watch(&c.current_track))
                    .filter(|t| !t.is_empty())
                    .map(|t| {
                        let title = t.title.as_deref().unwrap_or("Unknown");
                        let artist = t.artist.as_deref().unwrap_or("Unknown");
                        format!("{title} \u{00b7} {artist}")
                    });

                render_data.push(EntryRenderData {
                    speaker_volume: None,
                    group_volume: gvol,
                    playback_state: pb,
                    track_info: track,
                });
            }
            ListEntry::UngroupedHeader => {
                render_data.push(EntryRenderData {
                    speaker_volume: None,
                    group_volume: None,
                    playback_state: None,
                    track_info: None,
                });
            }
        }
    }

    let selected_index = state.selected_index.min(entries.len().saturating_sub(1));
    let is_pick_up = state.pick_up.is_some();
    let pick_up_speaker_id = state.pick_up.as_ref().map(|p| p.speaker_id.clone());

    let display_order = build_display_order(&entries, &state.pick_up);

    let vol_width = 16.min(area.width.saturating_sub(50));

    let mut lines: Vec<Line> = Vec::new();

    for &orig_idx in &display_order {
        let entry = &entries[orig_idx];
        let data = &render_data[orig_idx];

        let is_selected = if is_pick_up {
            pick_up_speaker_id
                .as_ref()
                .is_some_and(|pid| matches!(entry, ListEntry::SpeakerRow(sid) if sid == pid))
        } else {
            orig_idx == selected_index
        };

        match entry {
            ListEntry::GroupHeader(group_id) => {
                let group_name = ctx
                    .app
                    .system
                    .group_by_id(group_id)
                    .and_then(|g| g.coordinator())
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| "Unknown Group".to_string());

                let (icon, icon_style) = match &data.playback_state {
                    Some(PlaybackState::Playing) => ("\u{25b6} ", ctx.app.theme.playing_icon),
                    Some(PlaybackState::Paused) => ("\u{23f8} ", ctx.app.theme.paused_icon),
                    _ => ("\u{25a0} ", ctx.app.theme.stopped_icon),
                };

                let track_info = data.track_info.as_deref().unwrap_or("");

                let name_style = if is_selected {
                    ctx.app.theme.speaker_cursor
                } else {
                    ctx.app.theme.group_header
                };

                let mut spans = vec![
                    Span::styled(icon, icon_style),
                    Span::styled(group_name, name_style),
                ];

                if !track_info.is_empty() {
                    spans.push(Span::raw("  "));
                    spans.push(Span::styled(
                        track_info.to_string(),
                        ctx.app.theme.track_info,
                    ));
                }

                if let Some(vol) = data.group_volume {
                    append_volume_spans(&mut spans, vol, is_selected, vol_width, &ctx.app.theme);
                }

                lines.push(Line::from(spans));
            }
            ListEntry::SpeakerRow(speaker_id) => {
                let speaker_name = ctx
                    .app
                    .system
                    .speaker_by_id(speaker_id)
                    .map(|s| s.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string());

                let cursor = if is_selected { "  \u{25b8} " } else { "    " };

                let name_style = if is_selected {
                    ctx.app.theme.speaker_cursor
                } else {
                    ctx.app.theme.speaker_name
                };

                let mut spans = vec![
                    Span::styled(cursor.to_string(), name_style),
                    Span::styled(speaker_name, name_style),
                ];

                if let Some(vol) = data.speaker_volume {
                    append_volume_spans(&mut spans, vol, is_selected, vol_width, &ctx.app.theme);
                }

                lines.push(Line::from(spans));
            }
            ListEntry::UngroupedHeader => {
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![Span::styled(
                    " NOT IN A GROUP ",
                    ctx.app.theme.group_header,
                )]));
            }
        }
    }

    if let Some(ref pick_up) = state.pick_up {
        let name = ctx
            .app
            .system
            .speaker_by_id(&pick_up.speaker_id)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "Speaker".to_string());
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![Span::styled(
            format!(" Moving {name} \u{2014} Space to drop, Esc to cancel"),
            ctx.app.theme.accent,
        )]));
    }

    if state.pick_up.is_none() {
        if let Some(ref msg) = ctx.app.status_message {
            lines.push(Line::raw(""));
            let style = if msg.starts_with("error:") {
                ctx.app.theme.error
            } else {
                ctx.app.theme.accent
            };
            lines.push(Line::from(vec![Span::styled(format!(" {msg}"), style)]));
        }
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

/// Append volume bar (when selected) or percentage text to a span list.
fn append_volume_spans(
    spans: &mut Vec<Span>,
    vol: u16,
    is_selected: bool,
    width: u16,
    theme: &Theme,
) {
    spans.push(Span::raw("  "));
    if is_selected {
        let vol_line =
            volume_bar::render_volume_bar(vol, width, theme.volume_filled, theme.volume_empty);
        spans.extend(vol_line.spans);
    } else {
        spans.push(Span::styled(format!("{vol}%"), theme.muted));
    }
}

// ============================================================================
// Key handling
// ============================================================================

/// Handle a key event for the speaker list. Returns an action for the caller.
pub fn handle_key(app: &mut App, key: KeyEvent) -> SpeakerListAction {
    let pick_up = app.navigation.speakers_state.pick_up.clone();
    let entries = build_list_entries(&app.system);

    if entries.is_empty() {
        return SpeakerListAction::Handled;
    }

    if pick_up.is_some() {
        return handle_pick_up_key(app, key, &entries);
    }

    handle_normal_key(app, key, &entries)
}

fn next_selectable(entries: &[ListEntry], from: usize) -> Option<usize> {
    ((from + 1)..entries.len()).find(|&i| entries[i].is_selectable())
}

fn prev_selectable(entries: &[ListEntry], from: usize) -> Option<usize> {
    (0..from).rev().find(|&i| entries[i].is_selectable())
}

fn handle_normal_key(app: &mut App, key: KeyEvent, entries: &[ListEntry]) -> SpeakerListAction {
    let selected = app
        .navigation
        .speakers_state
        .selected_index
        .min(entries.len().saturating_sub(1));

    match key.code {
        KeyCode::Up => {
            if let Some(prev) = prev_selectable(entries, selected) {
                app.navigation.speakers_state.selected_index = prev;
            } else {
                return SpeakerListAction::FocusTabBar;
            }
            SpeakerListAction::Handled
        }
        KeyCode::Down => {
            if let Some(next) = next_selectable(entries, selected) {
                app.navigation.speakers_state.selected_index = next;
            }
            SpeakerListAction::Handled
        }
        KeyCode::Left => {
            handle_volume_adjust(app, entries, selected, -2);
            SpeakerListAction::Handled
        }
        KeyCode::Right => {
            handle_volume_adjust(app, entries, selected, 2);
            SpeakerListAction::Handled
        }
        KeyCode::Char(' ') => {
            if selected >= entries.len() {
                return SpeakerListAction::Handled;
            }
            if let ListEntry::SpeakerRow(speaker_id) = &entries[selected] {
                let original_group = app
                    .system
                    .speaker_by_id(speaker_id)
                    .and_then(|s| s.group())
                    .map(|g| g.id.clone());

                app.navigation.speakers_state.pick_up = Some(PickUpState {
                    speaker_id: speaker_id.clone(),
                    original_group_id: original_group,
                    drop_index: selected,
                });
            }
            SpeakerListAction::Handled
        }
        _ => SpeakerListAction::Handled,
    }
}

fn handle_volume_adjust(app: &mut App, entries: &[ListEntry], selected: usize, delta: i8) {
    if selected >= entries.len() {
        return;
    }
    match &entries[selected] {
        ListEntry::GroupHeader(group_id) => {
            if let Some(group) = app.system.group_by_id(group_id) {
                if let Err(e) = group.set_relative_volume(delta as i16) {
                    app.status_message = Some(format!("error: {e}"));
                }
            }
        }
        ListEntry::SpeakerRow(speaker_id) => {
            if let Some(speaker) = app.system.speaker_by_id(speaker_id) {
                if let Err(e) = speaker.set_relative_volume(delta) {
                    app.status_message = Some(format!("error: {e}"));
                }
            }
        }
        _ => {}
    }
}

fn handle_pick_up_key(app: &mut App, key: KeyEvent, entries: &[ListEntry]) -> SpeakerListAction {
    let pick_up = match app.navigation.speakers_state.pick_up.clone() {
        Some(p) => p,
        None => return SpeakerListAction::Handled,
    };

    let drop_index = pick_up.drop_index.min(entries.len().saturating_sub(1));

    match key.code {
        KeyCode::Up => {
            if let Some(prev) = prev_selectable(entries, drop_index) {
                if let Some(ref mut pu) = app.navigation.speakers_state.pick_up {
                    pu.drop_index = prev;
                }
            }
            SpeakerListAction::Handled
        }
        KeyCode::Down => {
            if let Some(next) = next_selectable(entries, drop_index) {
                if let Some(ref mut pu) = app.navigation.speakers_state.pick_up {
                    pu.drop_index = next;
                }
            }
            SpeakerListAction::Handled
        }
        KeyCode::Char(' ') => {
            let target_group = group_for_entry(entries, drop_index);
            let same_group = pick_up.original_group_id.as_ref() == target_group.as_ref();

            if same_group {
                app.navigation.speakers_state.pick_up = None;
                return SpeakerListAction::Handled;
            }

            if let Some(speaker) = app.system.speaker_by_id(&pick_up.speaker_id) {
                match target_group {
                    Some(target_gid) => {
                        if let Some(group) = app.system.group_by_id(&target_gid) {
                            match group.add_speaker(&speaker) {
                                Ok(()) => {
                                    let group_name = group
                                        .coordinator()
                                        .map(|c| c.name.clone())
                                        .unwrap_or_else(|| "group".to_string());
                                    app.status_message =
                                        Some(format!("{} moved to {}", speaker.name, group_name));
                                }
                                Err(e) => {
                                    app.status_message = Some(format!("error: {e}"));
                                }
                            }
                        }
                    }
                    None => match speaker.leave_group() {
                        Ok(_) => {
                            app.status_message = Some(format!("{} ungrouped", speaker.name));
                        }
                        Err(e) => {
                            app.status_message = Some(format!("error: {e}"));
                        }
                    },
                }
            }

            app.navigation.speakers_state.pick_up = None;
            SpeakerListAction::Handled
        }
        KeyCode::Esc => {
            app.navigation.speakers_state.pick_up = None;
            SpeakerListAction::Handled
        }
        _ => SpeakerListAction::Handled,
    }
}
