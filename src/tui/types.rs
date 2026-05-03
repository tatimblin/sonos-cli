//! Shared types used across TUI layers (widgets, screens, handlers).

use ratatui_image::protocol::StatefulProtocol;
use sonos_sdk::{GroupId, PlaybackState, SonosSystem, SpeakerId};

/// A single row in the flat speaker list. Navigation and rendering dispatch on this.
#[derive(Clone, Debug, PartialEq)]
pub enum ListEntry {
    GroupHeader(GroupId),
    SpeakerRow(SpeakerId),
    /// Inline drop target shown during pickup mode — "Add to group" or "Already in group".
    AddToGroupRow(GroupId),
    /// "Create new group" action row at the bottom during pickup mode.
    CreateNewGroupRow,
}

/// State for a speaker being moved between groups.
#[derive(Clone, Debug)]
pub struct PickUpState {
    pub speaker_id: SpeakerId,
    pub original_group_id: Option<GroupId>,
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
///
/// When `pick_up` is `Some`, each group gets an `AddToGroupRow` after its
/// speakers, and a `CreateNewGroupRow` appears at the end.
pub fn build_list_entries(system: &SonosSystem, pick_up: Option<&PickUpState>) -> Vec<ListEntry> {
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
        if pick_up.is_some() {
            entries.push(ListEntry::AddToGroupRow(group.id.clone()));
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
        if pick_up.is_some() {
            entries.push(ListEntry::AddToGroupRow(group.id.clone()));
        }
    }

    if pick_up.is_some() {
        entries.push(ListEntry::CreateNewGroupRow);
    }

    entries
}

/// Determine which group a list entry at `index` belongs to.
pub fn group_for_entry(entries: &[ListEntry], index: usize) -> Option<GroupId> {
    if index >= entries.len() {
        return None;
    }
    if let ListEntry::AddToGroupRow(gid) = &entries[index] {
        return Some(gid.clone());
    }
    if matches!(&entries[index], ListEntry::CreateNewGroupRow) {
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
    pub track_album: Option<String>,
    pub album_art_protocol: Option<StatefulProtocol>,
    pub playback_state: Option<PlaybackState>,
    pub progress: f64,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: u16,
    pub is_wide: bool,
    /// True when the selected entry is a group header (volume bar shows accent).
    /// False for speaker rows (volume bar shows dimmed).
    pub group_volume_active: bool,
}

/// Pre-computed render data for the speaker list widget.
pub struct SpeakerListData {
    pub entries: Vec<ListEntry>,
    pub entry_data: Vec<EntryRenderData>,
    pub selected_index: usize,
    /// ID of the picked-up speaker (for reverse-highlight matching).
    pub picked_up_speaker_id: Option<SpeakerId>,
}

/// Per-entry display data, pre-resolved by the screen layer.
pub struct EntryRenderData {
    pub name: String,
    pub model_name: Option<String>,
    pub speaker_volume: Option<u16>,
    pub group_volume: Option<u16>,
    pub playback_state: Option<PlaybackState>,
    pub track_info: Option<String>,
    /// True when this entry is the last member of its group block (renders `└` connector).
    pub is_last_in_group: bool,
    /// True for `AddToGroupRow` where the speaker already belongs (dimmed, not selectable).
    pub is_home_group: bool,
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
}
