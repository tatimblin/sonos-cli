//! Shared speaker list widget — renders grouped speakers with volume, playback state,
//! and pick-up/drop regrouping. Used by both Home > Speakers and GroupView > Speakers tabs.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use sonos_sdk::{GroupId, PlaybackState, SonosSystem, SpeakerId};

use crate::tui::app::{App, Screen, SpeakerListScreenState};
use crate::tui::hooks::RenderContext;
use crate::tui::widgets::volume_bar;

// ============================================================================
// Types
// ============================================================================

/// Controls which speakers appear in the list.
#[derive(Clone, Debug)]
pub enum SpeakerListMode {
    /// Show all groups with nested speakers (Home > Speakers tab).
    FullList,
    /// Show only this group's members + "Add Speaker" row (GroupView > Speakers tab).
    GroupScoped { group_id: GroupId },
}

/// A single row in the flat list. Navigation and rendering dispatch on this.
#[derive(Clone, Debug, PartialEq)]
pub enum ListEntry {
    GroupHeader(GroupId),
    SpeakerRow(SpeakerId),
    AddSpeaker,
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
    NavigateToGroup(GroupId),
    NavigateToSpeaker(SpeakerId),
    FocusTabBar,
}

// ============================================================================
// List building
// ============================================================================

/// Build the flat list of entries from the current system state.
pub fn build_list_entries(
    system: &SonosSystem,
    mode: &SpeakerListMode,
    pick_up: &Option<PickUpState>,
) -> Vec<ListEntry> {
    match mode {
        SpeakerListMode::FullList => build_full_list(system),
        SpeakerListMode::GroupScoped { group_id } => {
            if pick_up.is_some() {
                // When picking up in scoped mode, expand to full list
                build_full_list(system)
            } else {
                build_scoped_list(system, group_id)
            }
        }
    }
}

fn build_full_list(system: &SonosSystem) -> Vec<ListEntry> {
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

fn build_scoped_list(system: &SonosSystem, group_id: &GroupId) -> Vec<ListEntry> {
    let mut entries = Vec::new();

    if let Some(group) = system.group_by_id(group_id) {
        for member in group.members() {
            entries.push(ListEntry::SpeakerRow(member.id.clone()));
        }
    }

    entries.push(ListEntry::AddSpeaker);
    entries
}

/// Determine which group a list entry at `index` belongs to.
fn group_for_entry(entries: &[ListEntry], index: usize) -> Option<GroupId> {
    // Walk backwards from index to find the nearest GroupHeader
    for i in (0..=index).rev() {
        match &entries[i] {
            ListEntry::GroupHeader(gid) => return Some(gid.clone()),
            ListEntry::UngroupedHeader => return None,
            _ => continue,
        }
    }
    None
}

// ============================================================================
// Rendering
// ============================================================================

/// Render the speaker list widget.
pub fn render(
    frame: &mut Frame,
    area: Rect,
    ctx: &mut RenderContext,
    mode: &SpeakerListMode,
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

    let entries = build_list_entries(&ctx.app.system, mode, &state.pick_up);

    if entries.is_empty() {
        let paragraph = Paragraph::new("No speakers in group")
            .alignment(ratatui::layout::Alignment::Center)
            .style(ctx.app.theme.muted);
        frame.render_widget(paragraph, area);
        return;
    }

    // Watch volumes for all speakers and group properties for all groups in the list.
    // Collect into parallel vecs indexed by entry position.
    let mut speaker_volumes: Vec<Option<u16>> = Vec::new();
    let mut group_volumes: Vec<Option<u16>> = Vec::new();
    let mut group_playback_states: Vec<Option<PlaybackState>> = Vec::new();
    let mut group_track_info: Vec<Option<String>> = Vec::new();

    for entry in &entries {
        match entry {
            ListEntry::SpeakerRow(speaker_id) => {
                let vol = ctx
                    .app
                    .system
                    .speaker_by_id(speaker_id)
                    .and_then(|s| ctx.hooks.use_watch(&s.volume))
                    .map(|v| v.value() as u16);
                speaker_volumes.push(vol);
                group_volumes.push(None);
                group_playback_states.push(None);
                group_track_info.push(None);
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

                speaker_volumes.push(None);
                group_volumes.push(gvol);
                group_playback_states.push(pb);
                group_track_info.push(track);
            }
            _ => {
                speaker_volumes.push(None);
                group_volumes.push(None);
                group_playback_states.push(None);
                group_track_info.push(None);
            }
        }
    }

    let selected_index = state.selected_index.min(entries.len().saturating_sub(1));
    let is_pick_up = state.pick_up.is_some();
    let pick_up_speaker_id = state.pick_up.as_ref().map(|p| p.speaker_id.clone());
    let drop_index = state.pick_up.as_ref().map(|p| p.drop_index);

    // Build lines
    let mut lines: Vec<Line> = Vec::new();

    for (i, entry) in entries.iter().enumerate() {
        let is_selected = if is_pick_up {
            drop_index == Some(i)
        } else {
            i == selected_index
        };
        let is_picked_up_row = pick_up_speaker_id
            .as_ref()
            .is_some_and(|pid| matches!(entry, ListEntry::SpeakerRow(sid) if sid == pid));

        match entry {
            ListEntry::GroupHeader(group_id) => {
                let group_name = ctx
                    .app
                    .system
                    .group_by_id(group_id)
                    .and_then(|g| g.coordinator())
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| "Unknown Group".to_string());

                // Play state icon
                let (icon, icon_style) = match &group_playback_states[i] {
                    Some(PlaybackState::Playing) => ("\u{25b6} ", ctx.app.theme.playing_icon),
                    Some(PlaybackState::Paused) => ("\u{23f8} ", ctx.app.theme.paused_icon),
                    _ => ("\u{25a0} ", ctx.app.theme.stopped_icon),
                };

                let track_info = group_track_info[i]
                    .as_deref()
                    .unwrap_or("");

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
                    spans.push(Span::styled(track_info.to_string(), ctx.app.theme.track_info));
                }

                // Volume
                if let Some(vol) = group_volumes[i] {
                    if is_selected {
                        spans.push(Span::raw("  "));
                        let vol_line = volume_bar::render_volume_bar(
                            vol,
                            16.min(area.width.saturating_sub(50)),
                            ctx.app.theme.volume_filled,
                            ctx.app.theme.volume_empty,
                        );
                        spans.extend(vol_line.spans);
                    } else {
                        spans.push(Span::raw("  "));
                        spans.push(Span::styled(
                            format!("{vol}%"),
                            ctx.app.theme.muted,
                        ));
                    }
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

                let name_style = if is_picked_up_row {
                    ctx.app.theme.muted // dimmed when picked up
                } else if is_selected {
                    ctx.app.theme.speaker_cursor
                } else {
                    ctx.app.theme.speaker_name
                };

                let mut spans = vec![
                    Span::styled(cursor.to_string(), name_style),
                    Span::styled(speaker_name, name_style),
                ];

                // Volume
                if let Some(vol) = speaker_volumes[i] {
                    if is_selected && !is_picked_up_row {
                        spans.push(Span::raw("  "));
                        let vol_line = volume_bar::render_volume_bar(
                            vol,
                            16.min(area.width.saturating_sub(50)),
                            ctx.app.theme.volume_filled,
                            ctx.app.theme.volume_empty,
                        );
                        spans.extend(vol_line.spans);
                    } else {
                        spans.push(Span::raw("  "));
                        spans.push(Span::styled(
                            format!("{vol}%"),
                            ctx.app.theme.muted,
                        ));
                    }
                }

                lines.push(Line::from(spans));
            }
            ListEntry::AddSpeaker => {
                let style = if is_selected {
                    ctx.app.theme.speaker_cursor
                } else {
                    ctx.app.theme.muted
                };
                lines.push(Line::from(vec![Span::styled(
                    "    + Add speaker...",
                    style,
                )]));
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

    // Status message for pick-up mode
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

    // Render status message if present (non-pick-up)
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

// ============================================================================
// Key handling
// ============================================================================

/// Handle a key event for the speaker list. Returns an action for the caller.
pub fn handle_key(app: &mut App, key: KeyEvent, mode: &SpeakerListMode) -> SpeakerListAction {
    let pick_up = get_pick_up_state(app);
    let entries = build_list_entries(&app.system, mode, &pick_up);

    if entries.is_empty() {
        return SpeakerListAction::Handled;
    }

    if pick_up.is_some() {
        return handle_pick_up_key(app, key, mode, &entries);
    }

    handle_normal_key(app, key, mode, &entries)
}

/// Get a reference to the speaker list state from the current screen.
fn get_speakers_state(app: &App) -> Option<&SpeakerListScreenState> {
    match app.navigation.current() {
        Screen::Home { speakers_state, .. } => Some(speakers_state),
        Screen::GroupView { speakers_state, .. } => Some(speakers_state),
        _ => None,
    }
}

fn get_selected_index(app: &App) -> usize {
    get_speakers_state(app)
        .map(|s| s.selected_index)
        .unwrap_or(0)
}

fn set_selected_index(app: &mut App, index: usize) {
    match app.navigation.current_mut() {
        Screen::Home {
            ref mut speakers_state,
            ..
        } => speakers_state.selected_index = index,
        Screen::GroupView {
            ref mut speakers_state,
            ..
        } => speakers_state.selected_index = index,
        _ => {}
    }
}

fn get_pick_up_state(app: &App) -> Option<PickUpState> {
    get_speakers_state(app).and_then(|s| s.pick_up.clone())
}

fn set_pick_up_state(app: &mut App, pick_up: Option<PickUpState>) {
    match app.navigation.current_mut() {
        Screen::Home {
            ref mut speakers_state,
            ..
        } => speakers_state.pick_up = pick_up,
        Screen::GroupView {
            ref mut speakers_state,
            ..
        } => speakers_state.pick_up = pick_up,
        _ => {}
    }
}

fn next_selectable(entries: &[ListEntry], from: usize) -> Option<usize> {
    ((from + 1)..entries.len()).find(|&i| entries[i].is_selectable())
}

fn prev_selectable(entries: &[ListEntry], from: usize) -> Option<usize> {
    (0..from).rev().find(|&i| entries[i].is_selectable())
}

fn handle_normal_key(
    app: &mut App,
    key: KeyEvent,
    mode: &SpeakerListMode,
    entries: &[ListEntry],
) -> SpeakerListAction {
    let selected = get_selected_index(app);

    match key.code {
        KeyCode::Up => {
            if let Some(prev) = prev_selectable(entries, selected) {
                set_selected_index(app, prev);
            } else {
                return SpeakerListAction::FocusTabBar;
            }
            SpeakerListAction::Handled
        }
        KeyCode::Down => {
            if let Some(next) = next_selectable(entries, selected) {
                set_selected_index(app, next);
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
        KeyCode::Enter => {
            if selected >= entries.len() {
                return SpeakerListAction::Handled;
            }
            match &entries[selected] {
                ListEntry::GroupHeader(group_id) => {
                    SpeakerListAction::NavigateToGroup(group_id.clone())
                }
                ListEntry::SpeakerRow(speaker_id) => {
                    SpeakerListAction::NavigateToSpeaker(speaker_id.clone())
                }
                ListEntry::AddSpeaker => {
                    enter_add_speaker_mode(app, mode, entries);
                    SpeakerListAction::Handled
                }
                _ => SpeakerListAction::Handled,
            }
        }
        KeyCode::Char(' ') => {
            if selected >= entries.len() {
                return SpeakerListAction::Handled;
            }
            match &entries[selected] {
                ListEntry::SpeakerRow(speaker_id) => {
                    let original_group = app
                        .system
                        .speaker_by_id(speaker_id)
                        .and_then(|s| s.group())
                        .map(|g| g.id.clone());

                    set_pick_up_state(
                        app,
                        Some(PickUpState {
                            speaker_id: speaker_id.clone(),
                            original_group_id: original_group,
                            drop_index: selected,
                        }),
                    );
                    SpeakerListAction::Handled
                }
                ListEntry::AddSpeaker => {
                    enter_add_speaker_mode(app, mode, entries);
                    SpeakerListAction::Handled
                }
                _ => SpeakerListAction::Handled,
            }
        }
        _ => SpeakerListAction::Handled,
    }
}

fn handle_volume_adjust(app: &mut App, entries: &[ListEntry], selected: usize, delta: i16) {
    if selected >= entries.len() {
        return;
    }
    match &entries[selected] {
        ListEntry::GroupHeader(group_id) => {
            if let Some(group) = app.system.group_by_id(group_id) {
                if let Err(e) = group.set_relative_volume(delta) {
                    app.status_message = Some(format!("error: {e}"));
                }
            }
        }
        ListEntry::SpeakerRow(speaker_id) => {
            if let Some(speaker) = app.system.speaker_by_id(speaker_id) {
                if let Err(e) = speaker.set_relative_volume(delta as i8) {
                    app.status_message = Some(format!("error: {e}"));
                }
            }
        }
        _ => {}
    }
}

fn enter_add_speaker_mode(app: &mut App, mode: &SpeakerListMode, _entries: &[ListEntry]) {
    // In GroupScoped mode, "Add Speaker" expands the list to show all speakers.
    // We enter pick-up mode with the first non-member speaker pre-selected.
    if let SpeakerListMode::GroupScoped { group_id } = mode {
        let full_entries = build_full_list(&app.system);
        let first_outside = full_entries
            .iter()
            .enumerate()
            .find(|(_, e)| {
                if let ListEntry::SpeakerRow(sid) = e {
                    app.system
                        .speaker_by_id(sid)
                        .and_then(|s| s.group())
                        .map(|g| g.id != *group_id)
                        .unwrap_or(true)
                } else {
                    false
                }
            })
            .map(|(i, _)| i)
            .unwrap_or(0);

        if let Some(ListEntry::SpeakerRow(speaker_id)) = full_entries.get(first_outside) {
            set_pick_up_state(
                app,
                Some(PickUpState {
                    speaker_id: speaker_id.clone(),
                    original_group_id: None,
                    drop_index: first_outside,
                }),
            );
        }
    }
}

fn handle_pick_up_key(
    app: &mut App,
    key: KeyEvent,
    _mode: &SpeakerListMode,
    entries: &[ListEntry],
) -> SpeakerListAction {
    let pick_up = match get_pick_up_state(app) {
        Some(p) => p,
        None => return SpeakerListAction::Handled,
    };

    match key.code {
        KeyCode::Up => {
            if let Some(prev) = prev_selectable(entries, pick_up.drop_index) {
                let mut updated = pick_up;
                updated.drop_index = prev;
                set_pick_up_state(app, Some(updated));
            }
            SpeakerListAction::Handled
        }
        KeyCode::Down => {
            if let Some(next) = next_selectable(entries, pick_up.drop_index) {
                let mut updated = pick_up;
                updated.drop_index = next;
                set_pick_up_state(app, Some(updated));
            }
            SpeakerListAction::Handled
        }
        KeyCode::Char(' ') => {
            // Drop the speaker
            let target_group = group_for_entry(entries, pick_up.drop_index);
            let same_group = pick_up.original_group_id.as_ref() == target_group.as_ref();

            if same_group {
                set_pick_up_state(app, None);
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
                                    app.status_message = Some(format!(
                                        "{} moved to {}",
                                        speaker.name, group_name
                                    ));
                                }
                                Err(e) => {
                                    app.status_message = Some(format!("error: {e}"));
                                }
                            }
                        }
                    }
                    None => {
                        match speaker.leave_group() {
                            Ok(_) => {
                                app.status_message =
                                    Some(format!("{} ungrouped", speaker.name));
                            }
                            Err(e) => {
                                app.status_message = Some(format!("error: {e}"));
                            }
                        }
                    }
                }
            }

            set_pick_up_state(app, None);
            SpeakerListAction::Handled
        }
        KeyCode::Esc => {
            set_pick_up_state(app, None);
            SpeakerListAction::Handled
        }
        _ => SpeakerListAction::Handled,
    }
}
