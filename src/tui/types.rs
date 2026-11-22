//! Shared types used across TUI layers (widgets, screens, handlers).

use ratatui_image::protocol::StatefulProtocol;
use sonos_sdk::{Group, GroupId, PlaybackState, SonosSystem, SpeakerId};

use crate::config::Config;

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

/// Returned from screen key handlers so the caller can respond.
pub enum KeyOutcome {
    Handled,
    /// Up from the first row: move focus to the tab bar.
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

/// The group targeted by the entry at `index` — what the bottom bar shows and
/// playback keys control. Falls back to a speaker row's own group when no
/// header resolves.
pub fn group_for_selection(
    system: &SonosSystem,
    entries: &[ListEntry],
    index: usize,
) -> Option<Group> {
    group_for_entry(entries, index)
        .and_then(|gid| system.group_by_id(&gid))
        .or_else(|| match entries.get(index) {
            Some(ListEntry::SpeakerRow(sid)) => system.speaker_by_id(sid).and_then(|s| s.group()),
            _ => None,
        })
}

/// The speaker list cursor, tracked by entry identity rather than position.
///
/// The list is rebuilt from live topology on every frame and key press, so a
/// bare index would silently land on a different entry when a speaker joins
/// or leaves a group elsewhere. The selected entry is stored instead and
/// resolved against the current entries with [`Self::index_in`].
#[derive(Clone, Debug, Default)]
pub struct ListSelection {
    entry: Option<ListEntry>,
    /// Members of a selected group, used to find its header again when the
    /// group's ID changes (Sonos group IDs follow the coordinator).
    group_members: Vec<SpeakerId>,
    /// Index at selection time — the fallback when the entry disappears.
    index: usize,
}

impl ListSelection {
    /// Select the entry at `index` in `entries`.
    pub fn select(&mut self, entries: &[ListEntry], index: usize) {
        self.entry = entries.get(index).cloned();
        self.group_members = match self.entry {
            Some(ListEntry::GroupHeader(_)) => entries[index + 1..]
                .iter()
                .map_while(|e| match e {
                    ListEntry::SpeakerRow(sid) => Some(sid.clone()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };
        self.index = index;
    }

    /// Select `entry` by identity, keeping the current index as the fallback.
    pub fn select_entry(&mut self, entry: ListEntry) {
        self.entry = Some(entry);
        self.group_members.clear();
    }

    /// Resolve the selected entry's index in the current `entries`.
    ///
    /// When the entry is gone: a group header falls back to the header of the
    /// group now holding one of its members, an "Add to group" row to its
    /// group's header (pick-up ended) or the nearest remaining action row
    /// (group dissolved mid pick-up), and anything else to the nearest index.
    pub fn index_in(&self, entries: &[ListEntry]) -> usize {
        let nearest = self.index.min(entries.len().saturating_sub(1));
        let Some(entry) = &self.entry else {
            return nearest;
        };
        if let Some(i) = entries.iter().position(|e| e == entry) {
            return i;
        }

        match entry {
            ListEntry::GroupHeader(_) => self
                .group_members
                .iter()
                .find_map(|sid| {
                    let row = entries
                        .iter()
                        .position(|e| matches!(e, ListEntry::SpeakerRow(s) if s == sid))?;
                    entries[..row]
                        .iter()
                        .rposition(|e| matches!(e, ListEntry::GroupHeader(_)))
                })
                .unwrap_or(nearest),
            ListEntry::AddToGroupRow(gid) => entries
                .iter()
                .position(|e| matches!(e, ListEntry::GroupHeader(g) if g == gid))
                .or_else(|| {
                    entries
                        .iter()
                        .enumerate()
                        .filter(|(_, e)| {
                            matches!(
                                e,
                                ListEntry::AddToGroupRow(_) | ListEntry::CreateNewGroupRow
                            )
                        })
                        .min_by_key(|(i, _)| i.abs_diff(self.index))
                        .map(|(i, _)| i)
                })
                .unwrap_or(nearest),
            _ => nearest,
        }
    }
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
    /// Display data for each entry, index-aligned with `entries`.
    pub rows: Vec<RowData>,
    pub selected_index: usize,
    /// ID of the picked-up speaker (for reverse-highlight matching).
    pub picked_up_speaker_id: Option<SpeakerId>,
}

/// Per-entry display data, pre-resolved by the screen layer. One variant per
/// [`ListEntry`] variant.
pub enum RowData {
    GroupHeader(GroupHeaderData),
    Speaker(SpeakerRowData),
    /// Drop target during pick-up. `is_home` marks the picked-up speaker's own
    /// group, shown dimmed as "Already in group".
    AddToGroup {
        is_home: bool,
    },
    CreateNewGroup,
}

pub struct GroupHeaderData {
    /// The coordinator's name.
    pub name: String,
    pub volume: Option<u16>,
    pub is_muted: bool,
    pub playback_state: Option<PlaybackState>,
    pub track_info: Option<String>,
}

pub struct SpeakerRowData {
    pub name: String,
    pub model_name: Option<String>,
    pub volume: Option<u16>,
    pub is_muted: bool,
    /// True when this is the last member of its group block (renders `└` connector).
    pub is_last_in_group: bool,
}

/// Available theme names.
const THEME_OPTIONS: &[&str] = &["default", "bw", "minimal", "dance_party"];

/// Available album art mode options.
const ALBUM_ART_OPTIONS: &[&str] = &["image", "halfblock", "off"];

/// Display label for "no default group" (auto-detect).
pub const AUTO_GROUP_LABEL: &str = "(auto)";

/// A row in the settings form, in display order. `SettingsScreenState::selected_row`
/// indexes [`SettingsRow::ALL`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsRow {
    Theme,
    DefaultGroup,
    AlbumArt,
}

impl SettingsRow {
    pub const ALL: [SettingsRow; 3] = [Self::Theme, Self::DefaultGroup, Self::AlbumArt];

    pub fn at(index: usize) -> Option<Self> {
        Self::ALL.get(index).copied()
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Theme => "Theme",
            Self::DefaultGroup => "Default group",
            Self::AlbumArt => "Album art",
        }
    }

    /// Dropdown options. Default group lists live coordinators after "(auto)".
    pub fn options(self, system: &SonosSystem) -> Vec<String> {
        match self {
            Self::Theme => THEME_OPTIONS.iter().map(|s| s.to_string()).collect(),
            Self::DefaultGroup => std::iter::once(AUTO_GROUP_LABEL.to_string())
                .chain(
                    system
                        .groups()
                        .iter()
                        .filter_map(|g| g.coordinator())
                        .map(|c| c.name.clone()),
                )
                .collect(),
            Self::AlbumArt => ALBUM_ART_OPTIONS.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// The configured value, as it appears in [`Self::options`].
    pub fn current_value(self, config: &Config) -> String {
        match self {
            Self::Theme => config.theme.clone(),
            Self::DefaultGroup => config
                .default_group
                .clone()
                .unwrap_or_else(|| AUTO_GROUP_LABEL.to_string()),
            Self::AlbumArt => config.album_art_mode.to_string(),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn header(id: &str) -> ListEntry {
        ListEntry::GroupHeader(GroupId::new(id))
    }

    fn row(id: &str) -> ListEntry {
        ListEntry::SpeakerRow(SpeakerId::new(id))
    }

    fn add_row(id: &str) -> ListEntry {
        ListEntry::AddToGroupRow(GroupId::new(id))
    }

    fn selected<'a>(entries: &'a [ListEntry], selection: &ListSelection) -> &'a ListEntry {
        &entries[selection.index_in(entries)]
    }

    /// Kitchen, Office and Bedroom, all standalone.
    fn standalone() -> Vec<ListEntry> {
        vec![
            header("K:1"),
            row("K"),
            header("O:1"),
            row("O"),
            header("B:1"),
            row("B"),
        ]
    }

    #[test]
    fn selection_follows_a_speaker_that_joins_another_group() {
        let before = standalone();
        let mut selection = ListSelection::default();
        selection.select(&before, 3);
        assert_eq!(selected(&before, &selection), &row("O"));

        // Office joins Kitchen elsewhere: its row moves up, and index 3 now
        // holds Bedroom's header.
        let after = vec![header("K:1"), row("K"), row("O"), header("B:1"), row("B")];
        assert_eq!(selected(&after, &selection), &row("O"));
    }

    #[test]
    fn selection_follows_a_group_header_that_moves() {
        let before = standalone();
        let mut selection = ListSelection::default();
        selection.select(&before, 4);

        // A speaker above disappears, shifting Bedroom up.
        let after = vec![header("K:1"), row("K"), header("B:1"), row("B")];
        assert_eq!(selected(&after, &selection), &header("B:1"));
    }

    /// Sonos group IDs follow the coordinator, so a group can come back under
    /// a new ID after a member leaves. The cursor stays with its members.
    #[test]
    fn selection_falls_back_to_the_header_of_a_renamed_group() {
        let before = vec![header("B:1"), row("B"), header("K:1"), row("K"), row("O")];
        let mut selection = ListSelection::default();
        selection.select(&before, 2);

        // Kitchen leaves: the group lives on as Office's.
        let after = vec![
            header("B:1"),
            row("B"),
            header("O:2"),
            row("O"),
            header("K:3"),
            row("K"),
        ];
        assert_eq!(
            selected(&after, &selection),
            &header("K:3"),
            "first remaining member wins"
        );

        let after = vec![header("B:1"), row("B"), header("O:2"), row("O")];
        assert_eq!(selected(&after, &selection), &header("O:2"));
    }

    #[test]
    fn selection_falls_back_to_the_nearest_index_when_a_speaker_vanishes() {
        let before = standalone();
        let mut selection = ListSelection::default();
        selection.select(&before, 5);

        let after = vec![header("K:1"), row("K"), header("O:1"), row("O")];
        assert_eq!(selection.index_in(&after), 3);
        assert_eq!(selection.index_in(&[]), 0);
    }

    #[test]
    fn add_row_falls_back_to_the_nearest_action_row_or_its_header() {
        let pick_up = vec![
            header("K:1"),
            row("K"),
            add_row("K:1"),
            header("O:1"),
            row("O"),
            add_row("O:1"),
            ListEntry::CreateNewGroupRow,
        ];
        let mut selection = ListSelection::default();
        selection.select(&pick_up, 5);

        // Office's group vanishes mid pick-up: land on an action row.
        let dissolved = vec![
            header("K:1"),
            row("K"),
            add_row("K:1"),
            ListEntry::CreateNewGroupRow,
        ];
        assert!(matches!(
            selected(&dissolved, &selection),
            ListEntry::AddToGroupRow(_) | ListEntry::CreateNewGroupRow
        ));

        // Pick-up ends: action rows are gone, the group's header remains.
        let normal = vec![header("K:1"), row("K"), header("O:1"), row("O")];
        assert_eq!(selected(&normal, &selection), &header("O:1"));
    }

    #[test]
    fn select_entry_pins_identity_and_keeps_the_fallback_index() {
        let entries = standalone();
        let mut selection = ListSelection::default();
        selection.select(&entries, 2);

        selection.select_entry(row("B"));
        assert_eq!(selection.index_in(&entries), 5);

        selection.select_entry(row("gone"));
        assert_eq!(selection.index_in(&entries), 2);
    }

    #[test]
    fn group_for_selection_resolves_headers_rows_and_headerless_rows() {
        let system = SonosSystem::with_groups(&["Kitchen", "Office"]);
        let entries = build_list_entries(&system, None);
        for (i, entry) in entries.iter().enumerate() {
            let group = group_for_selection(&system, &entries, i).expect("every entry has a group");
            match entry {
                ListEntry::GroupHeader(gid) => assert_eq!(&group.id, gid),
                ListEntry::SpeakerRow(sid) => {
                    assert_eq!(
                        Some(group.id),
                        system
                            .speaker_by_id(sid)
                            .and_then(|s| s.group())
                            .map(|g| g.id)
                    )
                }
                _ => unreachable!("no action rows outside pick-up"),
            }
        }

        // A speaker row with no header above it falls back to its own group.
        let headerless = vec![entries[1].clone()];
        let ListEntry::GroupHeader(gid) = &entries[0] else {
            panic!("entry 0 should be a header");
        };
        assert_eq!(
            group_for_selection(&system, &headerless, 0).map(|g| g.id),
            Some(gid.clone())
        );
        assert!(group_for_selection(&system, &[ListEntry::CreateNewGroupRow], 0).is_none());
    }

    #[test]
    fn settings_rows_offer_their_current_value() {
        let system = SonosSystem::with_groups(&["Kitchen", "Office"]);
        let config = Config::default();
        for row in SettingsRow::ALL {
            let value = row.current_value(&config);
            assert!(row.options(&system).contains(&value), "{row:?}: {value}");
        }

        let groups = SettingsRow::DefaultGroup.options(&system);
        assert_eq!(groups[0], AUTO_GROUP_LABEL);
        assert_eq!(groups.len(), 3, "auto + one per coordinator");
    }
}
