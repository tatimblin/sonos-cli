---
title: "feat: Shared SpeakerList widget with pick-up/drop regrouping"
type: feat
status: completed
date: 2026-04-04
origin: docs/brainstorms/2026-04-04-speakers-list-widget-brainstorm.md
---

# Shared SpeakerList Widget

## Overview

Build a shared `SpeakerList` widget that renders an interactive, grouped speaker list in both the **Home > Speakers** tab and **GroupView > Speakers** tab. The widget supports nested group/speaker display, context-aware volume control, navigation into GroupView and SpeakerDetail screens, and a pick-up/drop interaction for regrouping speakers between Sonos groups.

This replaces the current `home_speakers.rs` screen with a reusable widget, and fills the GroupView > Speakers stub (Milestone 8).

## Problem Statement / Motivation

The current Home > Speakers tab (`home_speakers.rs`) has a flat speaker list with group headers as non-selectable labels. Group headers can't be interacted with (no volume control, no Enter-to-navigate). Regrouping is done via a modal picker, which is indirect. The GroupView > Speakers tab is unimplemented (stub since Milestone 8).

The user wants both views to share the same widget, with group headers as first-class interactive rows, and a spatial pick-up/drop metaphor for regrouping that feels more intuitive than a modal.

## Proposed Solution

A single widget module at `src/tui/widgets/speaker_list.rs` with:
- A `SpeakerListMode` enum (`FullList` vs `GroupScoped { group_id }`)
- A `ListEntry` tagged enum for the flat list (`GroupHeader`, `SpeakerRow`, `AddSpeaker`, `UngroupedHeader`)
- A render function and a key handler function, both parameterized by mode
- A `PickUpState` for the pick-up/drop state machine
- State managed via the hooks system (`use_state`) so it doesn't require `Screen` enum changes

(See brainstorm: `docs/brainstorms/2026-04-04-speakers-list-widget-brainstorm.md`)

## Technical Approach

### Architecture

#### Data Model: `ListEntry` enum

The current `speakers_in_display_order()` returns `Vec<SpeakerId>` — group headers are visual-only. The new model makes every row selectable:

```rust
// src/tui/widgets/speaker_list.rs

#[derive(Clone, Debug, PartialEq)]
pub enum ListEntry {
    GroupHeader(GroupId),
    SpeakerRow(SpeakerId),
    AddSpeaker,           // GroupScoped mode only
    UngroupedHeader,      // "Not in a group" divider — not selectable
}
```

A shared `build_list_entries()` function produces `Vec<ListEntry>` from the current `SonosSystem` state, parameterized by mode:
- `FullList`: all groups (multi-member first) with nested speakers, then `UngroupedHeader` + standalone speakers
- `GroupScoped { group_id }`: only that group's members + `AddSpeaker` sentinel
- `GroupScoped` during pick-up: expands to full list (same as `FullList`) so the user can see drop targets

Navigation skips `UngroupedHeader` entries (non-selectable divider).

#### State: Hooks-based

Use `hooks.use_state("speaker_list", ...)` to store widget state, avoiding changes to the `Screen` enum:

```rust
pub struct SpeakerListState {
    pub selected_index: usize,
    pub pick_up: Option<PickUpState>,
}

pub struct PickUpState {
    pub speaker_id: SpeakerId,
    pub original_group_id: Option<GroupId>,  // None if standalone
    pub drop_index: usize,                   // cursor position for drop target
}
```

The hooks mark-and-sweep automatically cleans up state when the screen changes.

#### Render Function

Signature follows the established pattern:

```rust
pub fn render(
    frame: &mut Frame,
    area: Rect,
    ctx: &mut RenderContext,
    mode: &SpeakerListMode,
)
```

For each `ListEntry`:
- **`GroupHeader`**: play state icon + group name + track info + volume percent (+ volume bar if highlighted)
- **`SpeakerRow`**: indented speaker name + volume percent (+ volume bar if highlighted)
- **`AddSpeaker`**: indented `"+ Add speaker..."` text
- **`UngroupedHeader`**: `"NOT IN A GROUP"` styled header (dimmed, not selectable)

Volume bar visibility rule: bar renders only when `entry_index == selected_index` (see brainstorm decision #8).

During pick-up mode:
- The picked-up speaker's original row is dimmed/ghosted
- The cursor position shows a highlighted indicator of where the speaker will land
- Status line: `"Moving {name} — Space to drop, Esc to cancel"`

#### Key Handler Function

```rust
pub fn handle_key(
    app: &mut App,
    key: KeyEvent,
    mode: &SpeakerListMode,
) -> SpeakerListAction
```

Returns a `SpeakerListAction` enum so callers can react:

```rust
pub enum SpeakerListAction {
    Handled,                           // key consumed, no navigation needed
    NavigateToGroup(GroupId),          // Enter on group header
    NavigateToSpeaker(SpeakerId),     // Enter on speaker row
    FocusTabBar,                      // Up past top of list
}
```

Key bindings (normal mode):
- **Up/Down**: navigate `selected_index`, skip `UngroupedHeader`, clamp to bounds
- **Left/Right**: adjust volume — group volume on `GroupHeader`, speaker volume on `SpeakerRow`
- **Enter**: on `GroupHeader` → return `NavigateToGroup`; on `SpeakerRow` → return `NavigateToSpeaker`; on `AddSpeaker` → enter pick-up mode (expanded)
- **Space**: on `SpeakerRow` → enter pick-up mode
- **Up past top**: return `FocusTabBar` so the caller can set `tab_focused = true`

Key bindings (pick-up mode):
- **Up/Down**: move `drop_index` through the expanded full list
- **Space**: drop — determine target group from `drop_index` position, call SDK
- **Esc**: cancel pick-up, restore original state

#### GroupView `tab_focused` Addition

The GroupView screen currently lacks `tab_focused`, so Left/Right always switches tabs. To free Left/Right for volume control on the Speakers tab, add `tab_focused: bool` to `Screen::GroupView`:

```rust
Screen::GroupView {
    group_id: GroupId,
    tab: GroupTab,
    tab_focused: bool,  // NEW
}
```

When `tab_focused`, Left/Right switches tabs. When not focused (content area), Left/Right goes to the tab-specific handler (volume for Speakers tab). Up past the top of the list sets `tab_focused = true`. Any Down/Enter/Space sets it back to `false`.

This matches the existing Home screen pattern (`app.rs:109`).

#### SDK Calls

| User Action | Condition | SDK Call |
|-------------|-----------|----------|
| Left on speaker row | Normal mode | `speaker.set_relative_volume(-2)` |
| Right on speaker row | Normal mode | `speaker.set_relative_volume(2)` |
| Left on group header | Normal mode | `group.set_relative_volume(-2)` |
| Right on group header | Normal mode | `group.set_relative_volume(2)` |
| Space drop in group section | Pick-up mode, different group | `group.add_speaker(&speaker)` |
| Space drop outside groups | Pick-up mode | `speaker.leave_group()` |
| Space drop in same group | Pick-up mode | No-op |

Volume step of `+/- 2` matches the existing Now Playing handler (`handlers/group.rs:71`).

### Implementation Phases

#### Phase 1: Data Model and Basic Rendering

Build the `ListEntry` enum, `build_list_entries()`, and render function with navigation.

**Tasks:**
- [x] Create `src/tui/widgets/speaker_list.rs` with `ListEntry`, `SpeakerListMode`, `SpeakerListState` types
- [x] Implement `build_list_entries(system, mode, pick_up)` → `Vec<ListEntry>`
- [x] Implement `render()` with hooks: `use_watch` for each speaker's volume, group's playback state, current track
- [x] Group header line: play state icon (reuse `theme.playing_icon/paused_icon/stopped_icon`), group name, track `title · artist`, volume percent
- [x] Speaker line: indented name, volume percent
- [x] Volume bar conditional on `selected_index == entry_index` (decision #8)
- [x] `UngroupedHeader` as non-selectable styled divider
- [x] Register in `src/tui/widgets/mod.rs`

**Files:**
- `src/tui/widgets/speaker_list.rs` (new)
- `src/tui/widgets/mod.rs` (add module)

**Exit criteria:** Widget renders a grouped speaker list with live volume data and play state icons.

#### Phase 2: Navigation and Volume Control

Wire up key handling for normal mode (no pick-up yet).

**Tasks:**
- [x] Implement `handle_key()` returning `SpeakerListAction`
- [x] Up/Down navigation: skip `UngroupedHeader`, clamp to list bounds
- [x] Left/Right volume: detect `GroupHeader` vs `SpeakerRow` at `selected_index`, call appropriate SDK method
- [x] Enter on `GroupHeader` → return `NavigateToGroup(group_id)`
- [x] Enter on `SpeakerRow` → return `NavigateToSpeaker(speaker_id)`
- [x] Up past top → return `FocusTabBar`
- [x] Add `tab_focused: bool` to `Screen::GroupView`
- [x] Update `handlers/group.rs` to use `tab_focused` pattern: Left/Right switches tabs only when `tab_focused`, otherwise delegates to tab-specific handler
- [x] Update all `Screen::GroupView` construction sites to include `tab_focused: false`

**Files:**
- `src/tui/widgets/speaker_list.rs` (key handler)
- `src/tui/app.rs` (`Screen::GroupView` + `tab_focused`)
- `src/tui/handlers/group.rs` (tab_focused pattern, speakers tab delegation)
- `src/tui/handlers/home.rs` (replace home speakers handler with shared widget)
- `src/tui/ui.rs` (wire render for GroupView > Speakers)

**Exit criteria:** Both Home > Speakers and GroupView > Speakers navigate the list, adjust volume, and Enter navigates to GroupView/SpeakerDetail.

#### Phase 3: Integration — Replace Home Speakers

Replace the current `home_speakers.rs` with the shared widget.

**Tasks:**
- [x] Update `src/tui/screens/home_speakers.rs` to delegate to `speaker_list::render()` with `SpeakerListMode::FullList`
- [x] Update `src/tui/handlers/home.rs` to delegate to `speaker_list::handle_key()` with `FullList` mode
- [x] Handle `SpeakerListAction` returns: push `Screen::GroupView` or `Screen::SpeakerDetail`
- [x] Handle `FocusTabBar` → set `tab_focused = true` on Home screen
- [x] Remove old `speakers_in_display_order()` function (replaced by `build_list_entries()`)
- [x] Remove old modal-based group picker logic (replaced by pick-up/drop in Phase 4)

**Files:**
- `src/tui/screens/home_speakers.rs` (delegate to widget)
- `src/tui/handlers/home.rs` (delegate to widget)

**Exit criteria:** Home > Speakers uses the shared widget. Old modal picker removed. Navigation and volume work.

#### Phase 4: Pick-Up/Drop Regrouping

The pick-up/drop state machine for moving speakers between groups.

**Tasks:**
- [x] Implement `PickUpState` in the widget state
- [x] Space on `SpeakerRow` → enter pick-up mode: store `speaker_id` and `original_group_id`
- [x] In `GroupScoped` mode: expand list entries to full system (same as `FullList`) on pick-up
- [x] Pick-up mode rendering: dim the original row, highlight the drop cursor, show status message
- [x] Pick-up Up/Down: move `drop_index` through entries (skip `UngroupedHeader`)
- [x] Space to drop: determine target from `drop_index` position in `ListEntry` vec
  - If `drop_index` is in a `GroupHeader` or `SpeakerRow` section for group X → `group_x.add_speaker(&speaker)`
  - If `drop_index` is in the ungrouped section (below `UngroupedHeader`) → `speaker.leave_group()`
  - If target group == original group → no-op
- [x] Esc to cancel: clear `PickUpState`, restore view
- [x] Error handling: on SDK error, show `app.status_message`, exit pick-up mode
- [x] `AddSpeaker` row: Enter or Space → enter pick-up mode with expanded list, but no speaker pre-selected. User navigates to a speaker and presses Space to select, then it auto-adds to the scoped group via `group.add_speaker(&speaker)`

**Files:**
- `src/tui/widgets/speaker_list.rs` (pick-up state machine, rendering, key handling)

**Exit criteria:** Users can pick up a speaker, move it to another group, drop it. GroupScoped mode expands on pick-up. Esc cancels. Errors show status messages.

#### Phase 5: Wire GroupView > Speakers Tab

Connect the shared widget to the GroupView screen.

**Tasks:**
- [x] Update `src/tui/ui.rs` `render_group_view()` to call `speaker_list::render()` with `GroupScoped` mode for `GroupTab::Speakers`
- [x] Update `src/tui/handlers/group.rs` to call `speaker_list::handle_key()` with `GroupScoped` mode
- [x] Handle `SpeakerListAction::NavigateToSpeaker` → push `Screen::SpeakerDetail`
- [x] Handle `SpeakerListAction::NavigateToGroup` → push new `Screen::GroupView` (if user enters a different group header during pick-up expansion — this should be blocked during pick-up mode)
- [x] Update key legend in `ui.rs` for Group > Speakers: `"↑↓ Navigate  ←→ Volume  ⏎ Open  ␣ Move  ⎋ Back"`

**Files:**
- `src/tui/ui.rs` (render dispatch + key legend)
- `src/tui/handlers/group.rs` (speakers tab handler)

**Exit criteria:** GroupView > Speakers tab is fully functional with the shared widget. Pick-up/drop works in GroupScoped mode with list expansion.

### Edge Cases and Mitigations

| Edge Case | Mitigation |
|-----------|-----------|
| **Picking up a group coordinator** | Allow it. `group.add_speaker()` works for any speaker. If the SDK returns an error, show it and exit pick-up mode. |
| **Last speaker leaves a group** | Group dissolves naturally. List rebuilds on next frame (SDK events trigger `app.dirty`). |
| **GroupScoped group no longer exists** | Check `system.group_by_id()` at render time. If `None`, show "Group not found" and pop navigation on next key press. |
| **`selected_index` out of bounds after topology change** | Clamp to `entries.len() - 1` at render time and in handler. Preserve selection by matching `SpeakerId` when possible. |
| **SDK call blocks during pick-up** | Accept for v1 (typically < 200ms). Show status message on completion. |
| **Volume at 0 or 100** | SDK clamps internally. `set_relative_volume(-2)` at 0 is a no-op, not an error. |
| **Left/Right conflict with tab switching** | `tab_focused` pattern: Left/Right switches tabs only when tab bar is focused. Content area uses Left/Right for volume. |

## Acceptance Criteria

### Functional Requirements

- [x] Shared `SpeakerList` widget renders in both Home > Speakers and GroupView > Speakers tabs
- [x] Group header rows show: play state icon, group name, current track (title/artist), volume percent
- [x] Speaker rows show: speaker name, volume percent
- [x] Volume bar only visible on the highlighted/selected row
- [x] Up/Down navigates the flat list, skipping non-selectable dividers
- [x] Left/Right adjusts volume (group volume on header, speaker volume on speaker row)
- [x] Enter on group header pushes GroupView screen
- [x] Enter on speaker row pushes SpeakerDetail screen
- [x] Space picks up a speaker, Up/Down moves, Space drops, Esc cancels
- [x] Drop in a group section → `group.add_speaker()`; drop outside → `speaker.leave_group()`
- [x] GroupScoped mode shows only group members + "Add Speaker" row by default
- [x] GroupScoped mode expands to full list on pick-up or "Add Speaker" activation
- [x] GroupView gains `tab_focused` pattern matching Home screen behavior
- [x] Status messages shown on SDK errors

### Quality Gates

- [x] `cargo build` succeeds with no warnings
- [x] `cargo clippy` passes
- [x] Old `home_speakers.rs` modal picker code removed (replaced by pick-up/drop)
- [x] All `Screen::GroupView` construction sites updated for `tab_focused` field

## Dependencies & Risks

**Dependencies:**
- `SpeakerDetail` screen (Milestone 9) doesn't exist yet — Enter on speaker row will push the screen, but it's currently a stub. This is fine; the navigation is wired up for when it's built.
- SDK `group.add_speaker()` and `speaker.leave_group()` are tested and working (used by CLI commands already).

**Risks:**
- **Borrow checker complexity**: The render function needs `&mut Hooks` for `use_watch`/`use_state` while also reading `&App`. The existing `RenderContext` pattern handles this, but the number of `use_watch` calls per frame (one per speaker + one per group) may be significant. Profile if rendering feels slow.
- **`tab_focused` addition to GroupView**: Touches every `Screen::GroupView` construction site. Grep for all occurrences to ensure none are missed.

## Sources & References

### Origin

- **Brainstorm document:** [docs/brainstorms/2026-04-04-speakers-list-widget-brainstorm.md](docs/brainstorms/2026-04-04-speakers-list-widget-brainstorm.md) — Key decisions carried forward: shared widget with mode enum, pick-up/drop for regrouping, context-aware volume, volume bar visibility rule.

### Internal References

- Existing speakers screen: `src/tui/screens/home_speakers.rs`
- Existing speakers handler: `src/tui/handlers/home.rs:127-321`
- GroupView handler stub: `src/tui/handlers/group.rs:45`
- Hooks system: `src/tui/hooks.rs` (use_watch, use_state, RenderContext)
- Volume bar widget: `src/tui/widgets/volume_bar.rs`
- Group card widget: `src/tui/widgets/group_card.rs` (GroupCardData pattern)
- Theme styles: `src/tui/theme.rs:37-40` (group_header, speaker_cursor, speaker_name)
- Screen enum: `src/tui/app.rs:106-121`
- Render dispatch: `src/tui/ui.rs:226-252`
- SDK API: `docs/references/sonos-sdk.md`

### Roadmap

- Implements **Milestone 8**: Group View — Speakers Tab
- Enhances **Milestone 7**: Home Screen — Speakers Tab (rework with shared widget)
