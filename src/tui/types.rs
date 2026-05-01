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
    pub original_group_id: Option<GroupId>,
    pub drop_index: usize,
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

/// Determine which group a list entry at `index` belongs to.
pub fn group_for_entry(entries: &[ListEntry], index: usize) -> Option<GroupId> {
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

/// Pre-computed render data for the speaker list widget.
pub struct SpeakerListData {
    pub entries: Vec<ListEntry>,
    pub entry_data: Vec<EntryRenderData>,
    pub selected_index: usize,
    pub pick_up: Option<PickUpState>,
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

/// Build display order for pick-up mode: the picked-up speaker is removed from its
/// original position and inserted at the drop position, so it visually moves through
/// the list with other entries shifting to fill the gap.
pub fn build_display_order(entries: &[ListEntry], pick_up: &Option<PickUpState>) -> Vec<usize> {
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
