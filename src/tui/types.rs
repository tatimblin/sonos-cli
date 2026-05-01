//! Shared types used across TUI layers (widgets, screens, handlers).

use ratatui_image::protocol::StatefulProtocol;
use sonos_sdk::{GroupId, PlaybackState, SonosSystem, SpeakerId};

/// A single row in the flat speaker list. Navigation and rendering dispatch on this.
#[derive(Clone, Debug, PartialEq)]
pub enum ListEntry {
    GroupHeader(GroupId),
    SpeakerRow(SpeakerId),
}

/// State for a speaker being moved between groups.
#[derive(Clone, Debug)]
pub struct PickUpState {
    pub speaker_id: SpeakerId,
    pub speaker_name: String,
    pub original_group_id: Option<GroupId>,
    pub active_zone_index: usize,
}

/// The kind of drop zone target.
#[derive(Clone, Debug, PartialEq)]
pub enum DropZoneKind {
    /// An existing group to drop into.
    ExistingGroup(GroupId),
    /// Create a new standalone group (leave current group).
    NewGroup,
}

/// A single drop zone in the pick-up view.
#[derive(Clone, Debug)]
pub struct DropZone {
    pub kind: DropZoneKind,
    /// Group name (coordinator name) for display. "Add new group" for NewGroup.
    pub group_name: String,
    /// Names of remaining members (excluding the picked-up speaker).
    pub remaining_members: Vec<String>,
    /// Inner height of the zone (equals original member count).
    pub inner_height: usize,
}

/// Pre-computed render data for drop zone mode.
pub struct DropZoneData {
    pub zones: Vec<DropZone>,
    pub active_zone_index: usize,
    pub speaker_name: String,
    pub status_message: Option<String>,
}

/// Screen data enum — normal speaker list or pick-up drop zone view.
pub enum SpeakerScreenData {
    Normal(SpeakerListData),
    PickUp(DropZoneData),
}

/// Action returned from speaker list key handling so callers can respond.
pub enum SpeakerListAction {
    Handled,
    FocusTabBar,
}

/// Build the flat list of entries from the current system state.
///
/// All groups get headers — including standalone speakers (Sonos treats every
/// speaker as belonging to a group). Multi-member groups are listed first,
/// then standalone groups.
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

    // Standalone groups — each gets a GroupHeader + single SpeakerRow
    for group in &groups {
        if !group.is_standalone() {
            continue;
        }
        entries.push(ListEntry::GroupHeader(group.id.clone()));
        if let Some(coord) = group.coordinator() {
            entries.push(ListEntry::SpeakerRow(coord.id.clone()));
        }
    }

    entries
}

/// Determine which group a list entry at `index` belongs to.
pub fn group_for_entry(entries: &[ListEntry], index: usize) -> Option<GroupId> {
    if index >= entries.len() {
        return None;
    }
    for i in (0..=index).rev() {
        if let ListEntry::GroupHeader(gid) = &entries[i] {
            return Some(gid.clone());
        }
    }
    None
}

/// Pre-computed render data for the bottom player bar widget.
pub struct BottomBarData {
    pub group_name: String,
    pub track_title: Option<String>,
    pub track_artist: Option<String>,
    pub album_art_protocol: Option<StatefulProtocol>,
    pub playback_state: Option<PlaybackState>,
    pub progress: f64,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: u16,
    pub is_wide: bool,
}

/// Pre-computed render data for the speaker list widget (normal mode).
pub struct SpeakerListData {
    pub entries: Vec<ListEntry>,
    pub entry_data: Vec<EntryRenderData>,
    pub selected_index: usize,
    pub status_message: Option<String>,
}

/// Per-entry display data, pre-resolved by the screen layer.
pub struct EntryRenderData {
    pub name: String,
    pub speaker_volume: Option<u16>,
    pub group_volume: Option<u16>,
    pub playback_state: Option<PlaybackState>,
    pub track_info: Option<String>,
    /// True when this speaker row is the last member of its group (renders `└` connector).
    pub is_last_in_group: bool,
}

/// Action returned from settings key handling so callers can respond.
pub enum SettingsAction {
    Handled,
    FocusTabBar,
}

/// A single row in the settings form.
pub struct SettingsItem {
    pub label: &'static str,
    pub value: String,
    pub options: Vec<String>,
}

/// Pre-computed render data for the settings widget.
pub struct SettingsData {
    pub items: Vec<SettingsItem>,
    pub selected_row: usize,
    pub dropdown_open: bool,
    pub dropdown_index: usize,
    pub status_message: Option<String>,
}
