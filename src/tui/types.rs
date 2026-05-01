//! Shared types used across TUI layers (widgets, screens, handlers).

use sonos_sdk::{GroupId, PlaybackState, SonosSystem, SpeakerId};

/// A single row in the flat speaker list. Navigation and rendering dispatch on this.
#[derive(Clone, Debug, PartialEq)]
pub enum ListEntry {
    GroupHeader(GroupId),
    SpeakerRow(SpeakerId),
    UngroupedHeader,
}

impl ListEntry {
    pub fn is_selectable(&self) -> bool {
        !matches!(self, ListEntry::UngroupedHeader)
    }
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
}
