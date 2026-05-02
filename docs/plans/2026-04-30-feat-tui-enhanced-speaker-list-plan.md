---
title: "feat: Enhanced speaker list with 2-line group headers"
type: feat
status: active
date: 2026-04-30
origin: docs/brainstorms/2026-04-30-tui-v1-simplification-brainstorm.md
parallel-group: tui-v1-simplification
---

# Enhanced Speaker List with 2-Line Group Headers

## Overview

Redesign the speaker list rendering to show richer group headers (2 lines: name+icon+volume / track+artist), tree-connector speaker rows, and cursor-only volume bar expansion. The list should feel like a scannable dashboard — you can glance at every group's playback state without drilling into anything.

(see brainstorm: `docs/brainstorms/2026-04-30-tui-v1-simplification-brainstorm.md` — "Main View — Speakers by Group" section)

## Problem Statement / Motivation

The current speaker list has 1-line group headers showing only the group name, playback icon, and volume. Track info isn't visible without a separate Now Playing screen (which was cut). Users need to see what's playing in each group at a glance. The brainstorm specifies a denser, more informative list layout that makes the speakers view self-contained.

## Proposed Solution

Enhance the existing `widgets/speaker_list.rs` and `screens/speakers.rs` to render the new layout. The three-layer architecture stays the same — the screen assembles richer data, the widget renders it in the new format.

## Technical Approach

### Target Layout

```
▶ Living Room                                                            70%
  Bohemian Rhapsody — Queen
  ├ Beam                                                                 60%
  ├ One SL (Left)                              ■■■■■■■····              60%  ◀ cursor
  └ One SL (Right)                                                       60%

⏸ Kitchen                                                                40%
  Hotel California — Eagles
  └ Sonos One                                                            40%

■ Bedroom                                                                 0%
  Nothing playing
  └ Sonos One                                                            25%
```

### Phase 1: Enrich Data Structures

**Update `EntryRenderData` in `types.rs`:**

```rust
pub struct EntryRenderData {
    pub name: String,
    // Speaker-level data
    pub speaker_volume: Option<u8>,
    // Group-level data
    pub group_volume: Option<u16>,
    pub playback_state: Option<PlaybackState>,
    pub track_info: Option<String>,      // "Title — Artist" or "Nothing playing"
    // New fields for enhanced layout
    pub is_last_in_group: bool,          // for └ vs ├ tree connector
    pub member_count: usize,             // speakers in this group (for group headers)
}
```

**Update `build_list_entries()` in `types.rs`:**

- Track each speaker's position within its group to set `is_last_in_group`
- Count members per group for `member_count`

### Phase 2: Update Screen Data Assembly

**`screens/speakers.rs`:**

For each `GroupHeader` entry, subscribe to the coordinator's `current_track` in addition to the existing `playback_state` and `group.volume` watches:

```rust
ListEntry::GroupHeader(group_id) => {
    // Existing watches
    let group_vol = ctx.hooks.use_watch_group(group.volume.clone());
    let playback = ctx.hooks.use_watch(coordinator.playback_state.clone());
    // New watch
    let current_track = ctx.hooks.use_watch(coordinator.current_track.clone());
    
    let track_info = current_track
        .map(|ct| track_summary(&ct))
        .unwrap_or_else(|| "Nothing playing".to_string());
    // ...
}
```

For each `SpeakerRow`, the screen already fetches `speaker.volume`. Add the `is_last_in_group` flag from the enriched `build_list_entries()`.

### Phase 3: Redesign Widget Rendering

**`widgets/speaker_list.rs` — group header rendering (2 lines):**

Line 1: `{playback_icon} {group_name}` left-aligned + `{volume}%` right-aligned
- Playback icon: `▶` playing, `⏸` paused, `■` stopped (using theme icon styles)
- Volume number always shown (no bar on group headers)

Line 2: `  {track_info}` (indented 2 spaces, muted style)
- Track info from `track_summary()`: "Title — Artist" or "Nothing playing"

**`widgets/speaker_list.rs` — speaker row rendering (1 line):**

`  {tree_connector} {speaker_name}` left-aligned + volume display right-aligned
- Tree connector: `├` for non-last members, `└` for last member (using `is_last_in_group`)
- Volume display: 
  - **Selected row (cursor)**: full volume bar via `volume_bar::render_volume_bar()` + `N%`
  - **Non-selected rows**: just `N%` number, right-aligned

**Cursor indicator:**

The selected row gets a `◀` indicator at the far right (after volume). This replaces the current `▸` prefix cursor, avoiding horizontal shift of all content.

**Spacing:**

A blank line between groups for visual separation.

### Phase 4: Volume Adjustment Updates

**`handlers/speaker_list.rs`:**

Left/Right arrow behavior depends on what's selected:
- **Group header selected**: adjust group volume via `group.set_relative_volume(+/-2)`
- **Speaker row selected**: adjust speaker volume via `speaker.set_relative_volume(+/-2)`

This is how it currently works — no change needed in the handler logic, only in which entries are selectable. Group headers must now be selectable (they currently are).

### Phase 5: Remove Dead Code

After this plan is implemented, the following can be cleaned up:

- Remove `card_border`, `card_border_selected`, `card_title` dead-code fields from `theme.rs` (these were for the cut group card grid)
- Any remnants of group card rendering logic

### Phase 6: Scrolling

The 2-line group headers and blank separators roughly double the visual line count. A system with 4 groups averaging 3 speakers each consumes ~24 visual lines — exceeding most terminals' content area. Scrolling is required, not optional.

**Approach: edge-triggered scroll with visual-line tracking.**

The widget must map `selected_index` (an entry index) to a visual line position, accounting for:
- Group headers = 2 visual lines
- Speaker rows = 1 visual line
- Blank separators between groups = 1 visual line (rendering artifact, not a list entry)

Add a `scroll_offset: usize` to `SpeakerListScreenState` (in visual lines, not entry indices). On each render:
1. Compute `visual_line` for the selected entry by summing line heights of all entries before it
2. If `visual_line < scroll_offset`, set `scroll_offset = visual_line` (scroll up)
3. If `visual_line + entry_height > scroll_offset + viewport_height`, set `scroll_offset = visual_line + entry_height - viewport_height` (scroll down)
4. When selected entry is a group header, ensure both lines are visible (use `entry_height = 2`)

The widget skips rendering entries whose visual lines fall entirely outside `scroll_offset..scroll_offset+viewport_height`.

## Design Decisions (from SpecFlow analysis)

These resolve ambiguities identified during spec analysis:

| Question | Decision | Rationale |
|----------|----------|-----------|
| Standalone groups: group headers or "NOT IN A GROUP"? | **All groups get headers, including standalone.** Remove the `UngroupedHeader` / `is_standalone()` filtering from `build_list_entries()`. | The brainstorm mockup shows "Bedroom" as a single-speaker group with a full 2-line header. Sonos treats every speaker as belonging to a group. |
| Group header: one selectable entry or two? | **One selectable entry spanning 2 visual lines.** Down from a group header moves to the first speaker row. | Keeps the data model simple — `ListEntry::GroupHeader` stays a single variant. Navigation arithmetic uses visual-line mapping for scroll, not for selection. |
| Blank separator lines: list entries or rendering artifacts? | **Rendering artifacts.** The widget inserts blank lines between groups without adding entries to the list. | Keeps `build_list_entries()` clean and avoids new `ListEntry` variants. |
| Volume bar on selected group headers? | **No. Group headers always show just the volume number.** The "selected row shows full volume bar" criterion applies only to speaker rows. | Keeps headers compact and visually distinct from speaker rows. Resolves the Phase 3 / acceptance criteria conflict. |
| Track separator character? | **Change from middle dot (`·`) to em dash (`—`).** Update `helpers::track_summary()`. | Matches the brainstorm mockup. |
| Cursor indicator on selected group header? | **`◀` appears on line 1** (name+volume line), right-aligned after volume percentage. Line 2 (track info) stays in muted style. | Line 1 is the "control" line where volume adjustment happens. |
| Mute state display? | **Defer to follow-up.** Show volume numbers without mute indication for v1. | Adds SDK subscription complexity (GroupMuteHandle, MuteHandle) and visual design work. Better as a separate small plan. |
| Transitioning playback state icon? | **Use stopped icon (`■`) for `PlaybackState::Transitioning`.** | Transitioning is brief; dedicated icon adds visual noise for little value. |
| Volume percentage alignment? | **Fixed 4-char width: `" 0%"`, `"60%"`, `"100%"`.** Right-aligned at terminal edge. | Prevents column jitter when volume changes between 1-digit and 2-digit values. |
| Tree connector theme style? | **Use `theme.muted`.** | Connectors are structural, not content. Muted style keeps them subtle. |
| `member_count` field in `EntryRenderData`? | **Remove it.** No rendering consumer in this plan. | Avoids dead-on-arrival fields. Can add later if needed. |
| `selected_index` after topology change? | **Clamp to `entries.len() - 1`.** Accept that cursor may jump to a different entry. | Simple, matches current behavior. Preserving by speaker ID adds complexity for a rare edge case. |

## Acceptance Criteria

- [ ] Group headers render as 2 lines: name+icon+volume / track+artist
- [ ] Track info updates reactively when SDK fires `current_track` change events
- [ ] Speaker rows use tree connectors: `├` for non-last, `└` for last member
- [ ] Selected **speaker row** shows full volume bar; all other rows show volume number only
- [ ] Group headers always show volume number only (no bar), even when selected
- [ ] Cursor indicator `◀` appears at the far right of the selected row (line 1 for group headers)
- [ ] Blank line between groups for visual separation (rendering artifact, not list entry)
- [ ] All groups rendered with headers, including standalone single-speaker groups (no "NOT IN A GROUP" section)
- [ ] Left/Right volume adjustment works on both group headers and speaker rows
- [ ] Up/Down navigation skips blank lines (group headers and speaker rows are selectable)
- [ ] List scrolls to keep the selected entry visible when content exceeds viewport
- [ ] Three-layer architecture maintained: data in screen, rendering in widget, input in handler
- [ ] `"Nothing playing"` shown for groups with no active track
- [ ] Track separator changed from `·` to `—` in `helpers::track_summary()`
- [ ] Volume percentages right-aligned with fixed 4-char width

## Dependencies & Risks

**Dependencies:**
- None on other parallel plans. The bottom bar reads `group_for_entry()` independently — it doesn't depend on the visual layout of the speaker list.
- The drop zone plan modifies `widgets/speaker_list.rs` for pick-up mode, but the normal-mode rendering is independent.
- Removing `UngroupedHeader` affects the drop zone plan — it should also iterate all groups rather than separating standalones. Coordinate on the `build_list_entries()` change.

**Risks:**
- **Wide group names + long track titles**: Truncate with `…` when content exceeds available width. Line 1: truncate group name first, preserve volume percentage (4 chars). Line 2: truncate track info, full width minus 2-char indent.
- **Tree connector rendering**: Unicode tree characters (`├`, `└`) are standard Box Drawing (U+251C, U+2514) and well-supported.
- **Scroll implementation complexity**: The visual-line-to-entry mapping adds a calculation layer. Keep it in the widget (rendering concern), not the handler.

## Sources & References

- **Origin brainstorm:** [docs/brainstorms/2026-04-30-tui-v1-simplification-brainstorm.md](docs/brainstorms/2026-04-30-tui-v1-simplification-brainstorm.md) — main view layout, group header design, cursor-only volume expansion, tree connectors
- **Current widget:** `src/tui/widgets/speaker_list.rs` — existing 1-line group header rendering to enhance
- **Current screen:** `src/tui/screens/speakers.rs` — data assembly with hook subscriptions
- **Types:** `src/tui/types.rs` — `EntryRenderData`, `build_list_entries()`
- **Helpers:** `src/tui/helpers.rs` — `track_summary()` for "Title — Artist" formatting
