---
title: "feat: TUI Home Screen with live group cards and speaker management"
type: feat
status: completed
date: 2026-03-23
origin: docs/brainstorms/2026-02-26-sonos-tui-brainstorm.md
milestone: "Milestone 7: TUI — Home Screen"
---

# feat: TUI Home Screen with Live Group Cards and Speaker Management

## Overview

Implement the TUI Home Screen — the landing experience when a user runs `sonos` with no arguments. The Home Screen has two tabs: **Groups** (default) showing a responsive grid of live-updating group cards with a mini-player, and **Speakers** showing all speakers organized by group with management actions (create group, move, ungroup).

This builds on the M6 TUI foundation (event loop, navigation stack, theme, breadcrumb/legend) and replaces the four placeholder render stubs in `src/tui/ui.rs` with real, data-driven screens.

**Implements:** Roadmap Milestone 7 — TUI Home Screen
**Origin brainstorm:** [docs/brainstorms/2026-02-26-sonos-tui-brainstorm.md](../brainstorms/2026-02-26-sonos-tui-brainstorm.md) — groups-first overview, card design, mini-player behavior, speakers-by-group layout, tabbed navigation model

## Problem Statement / Motivation

After M6, the TUI launches and navigates between screen/tab stubs but shows no real data. Users can't see or control their Sonos system. The Home Screen is the first thing users see — it must feel alive with real-time playback data, be scannable at a glance, and provide group/speaker management.

## Proposed Solution

Replace the placeholder stubs with four interconnected components:

1. **Group Card Grid** — Responsive 2-column (or 1-column on narrow terminals) grid of cards showing each group's name, playback state, track info, volume bar, animated progress bar, and speaker count
2. **Mini-Player** — 2-line bar above the key legend tracking the focused group's now-playing info
3. **Speakers List** — Speakers organized under group headers with cursor navigation and management actions
4. **Property Watch Lifecycle** — Watch/unwatch SDK properties when entering/leaving the Groups tab for real-time updates

### Key Design Decisions

These decisions are carried forward from the brainstorm and SpecFlow analysis:

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Selection state location | Per-tab state structs inside `Screen::Home` variant | Preserves selection when switching tabs (see brainstorm: navigation model) |
| Grid navigation | Flat index with `row = idx/cols, col = idx%cols` | Simple, unit-testable, standard grid behavior |
| Progress interpolation storage | `HashMap<GroupId, ProgressState>` on `App` | Hybrid state (SDK + wall-clock) needs frame-level access |
| Initial data seeding | `fetch()` after `watch()` if `WatchStatus.current` is `None` | SDK `.get()` returns `None` until populated; paused speakers may not send events for a long time |
| Poll interval | 50ms always (changed from 250ms) | Simpler than adaptive; overhead is one poll syscall + conditional draw per 50ms |
| Animation dirty flag | Set `dirty = true` each tick if any group is `Playing` | Drives progress bar interpolation without SDK events |
| Mini-player height | 2 lines (no album art — M8 scope) | Line 1: group name + track. Line 2: progress + volume |
| Mini-player visibility | Both Home tabs (Groups and Speakers) | Per brainstorm mock-ups (lines 156-158, 200-207) |
| Long track titles | Truncate with `…` | Marquee scroll deferred to M10 polish |
| Empty track state | "Nothing playing" with `--:--/--:--` | Per brainstorm mock-up (line 148: "Nothing playing") |
| `d` on coordinator | Error message if group has members; no-op if standalone | Prevents confusing `InvalidOperation` SDK error |
| Speaker count format | `"{coordinator_model} + {N} speakers"` or `"{model}"` if standalone | SDK has no surround role info; keep it simple |
| Group picker for move-to-group | Centered modal overlay with bordered list | Esc to cancel, Enter to confirm, up/down to navigate |
| `n` on Speakers tab | Selected speaker becomes coordinator of new standalone group | Single-action, no multi-step picker |

## Technical Approach

### Architecture

```
src/tui/
  app.rs          ← Add per-tab state structs, ProgressState map, WatchRegistry
  event.rs        ← Process ChangeEvents by property_key, animation tick logic
  theme.rs        ← Expand with card/bar/mini-player/speaker styles
  ui.rs           ← Keep as dispatch hub (match Screen → render fn)
  screens/
    mod.rs        ← Re-exports
    home_groups.rs    ← Groups tab: card grid + mini-player rendering
    home_speakers.rs  ← Speakers tab: speaker list rendering
  widgets/
    mod.rs        ← Re-exports
    group_card.rs     ← Single group card widget
    progress_bar.rs   ← Track progress bar with interpolation
    volume_bar.rs     ← Volume gauge bar
    mini_player.rs    ← Mini-player bar widget
    modal.rs          ← Reusable modal overlay (for group picker)
```

### Implementation Phases

#### Phase 1: State & Theme Expansion

Expand `App` and `Theme` to support M7 data needs. No rendering changes yet.

**`src/tui/app.rs`:**
- Add `HomeGroupsState { selected_index: usize, scroll_offset: usize }` and `HomeSpeakersState { selected_index: usize, scroll_offset: usize }`
- Change `Screen::Home { tab: HomeTab }` to `Screen::Home { tab: HomeTab, groups_state: HomeGroupsState, speakers_state: HomeSpeakersState }`
- Add `progress_states: HashMap<GroupId, ProgressState>` to `App`
- `ProgressState { last_position_ms: u64, last_duration_ms: u64, wall_clock_at_last_update: Instant, is_playing: bool }`
- Add `watch_registry: HashSet<(SpeakerId, &'static str)>` to `App` for tracking active watches

**`src/tui/theme.rs`:**
- Add styles: `card_border`, `card_border_selected`, `card_title`, `track_info`, `playing_icon`, `paused_icon`, `stopped_icon`, `volume_filled`, `volume_empty`, `progress_filled`, `progress_empty`, `progress_cursor`, `mini_player_border`, `mini_player_title`, `group_header`, `speaker_cursor`, `speaker_name`, `accent`
- Implement all four themes: `dark()`, `light()`, `neon()`, `sonos()`

**Exit criteria:** `cargo build` succeeds. Existing tests pass. No rendering changes.

#### Phase 2: Widget Library

Build reusable render functions for the visual primitives. Unit-testable with `ratatui::buffer::Buffer`.

**`src/tui/widgets/group_card.rs`:**
- Render a single group card within a given `Rect`
- Inputs: group name, playback state, track title/artist, volume (0-100), progress (0.0-1.0), elapsed/duration strings, speaker count text, selected flag, theme
- Selected: `BorderType::Double` + bold title + `●` prefix
- Unselected: `BorderType::Plain` + dimmed border
- Playback icons: `▶` Playing, `⏸` Paused, `■` Stopped

**`src/tui/widgets/volume_bar.rs`:**
- Render `████████░░░░ 80%` pattern within a given width
- Inputs: level (0-100), width, filled/empty styles from theme

**`src/tui/widgets/progress_bar.rs`:**
- Render `━━━━━━━╺──── 2:31/5:55` pattern
- Inputs: progress ratio (0.0-1.0), elapsed string, duration string, width, theme styles
- `format_time(ms: u64) -> String` helper: `"M:SS"` or `"H:MM:SS"` for tracks > 1 hour

**`src/tui/widgets/mini_player.rs`:**
- 2-line widget: Line 1 = group name + `▶`/`⏸`/`■` + track title — artist. Line 2 = progress bar + volume
- Inputs: group name, playback state, track info, progress, volume, theme
- Bordered with `mini_player_border` style

**`src/tui/widgets/modal.rs`:**
- Centered overlay with bordered list of string items
- Inputs: title, items, selected_index, width/height constraints
- Used for the group picker on Speakers tab

**Exit criteria:** Each widget renders correctly in isolation. Buffer-based unit tests verify output for key states (playing, paused, stopped, empty track, selected/unselected, zero volume, full volume).

#### Phase 3: Groups Tab Rendering

Wire the widgets into the Groups tab screen. Replace the `render_home` stub for `HomeTab::Groups`.

**`src/tui/screens/home_groups.rs`:**
- Query `app.system.groups()` for group list
- Determine column count: `if area.width >= 100 { 2 } else { 1 }`
- Layout cards in rows using `Layout::horizontal` with equal `Constraint::Ratio` per column
- For each group:
  - Get coordinator via `group.coordinator()`
  - Read cached properties: `current_track.get()`, `playback_state.get()`, `position.get()`, `group.volume.get()`
  - Compute interpolated position from `app.progress_states` if playing
  - Build speaker count string from `group.members()`
  - Render `group_card` widget with `selected = (index == state.selected_index)`
- Handle empty state: centered "No groups found" message
- Handle coordinator `None`: show group name with "Unavailable" body
- Scrolling: if rows exceed content height, offset by `scroll_offset` rows

**Layout structure:**
```
┌─────────────────────────────────────────────┐
│ Breadcrumb + tabs                    (1 line) │
│─────────────────────────────────────────────│
│ Card grid (scrollable)           (remaining) │
│                                              │
│─────────────────────────────────────────────│
│ Mini-player                        (2 lines) │
│─────────────────────────────────────────────│
│ Key legend                         (1 line)  │
└─────────────────────────────────────────────┘
```

Update `src/tui/ui.rs` layout from 3 regions (header, content, legend) to 4 regions (header, content, mini-player, legend). Mini-player region is only allocated when `Screen::Home`.

**Exit criteria:** Groups tab renders real group data from the SDK. Cards display all fields. 2-column/1-column responsive layout works. Selected card visually distinct. Empty state handled.

#### Phase 4: Key Handling & Grid Navigation

Wire arrow keys, Enter, and tab switching for the Groups and Speakers tabs.

**`src/tui/event.rs` — `handle_home_key` updates:**
- Groups tab:
  - `Up/Down/Left/Right` — update `groups_state.selected_index` using grid navigation logic
  - Grid nav: `cols = if width >= 100 { 2 } else { 1 }`, `total = groups.len()`. Clamp index to `0..total-1`
  - `Enter` — push `Screen::GroupView { group_id, tab: NowPlaying }` for the selected group
  - Auto-scroll: adjust `scroll_offset` to keep `selected_index` visible
- Speakers tab:
  - `Up/Down` — navigate speaker list (skip group headers)
  - `n` — create new group: `app.system.create_group(&speaker, &[])`, handle errors
  - `d` — ungroup: check if coordinator with members → show error; else `speaker.leave_group()`
  - `Enter` — open group picker modal for move-to-group
- Tab switching:
  - `Left/Right` at tab level toggles `HomeTab::Groups` / `HomeTab::Speakers` (already partially implemented)

**Grid navigation helper** (unit-testable):
```rust
fn grid_navigate(current: usize, total: usize, cols: usize, direction: Direction) -> usize
```

**Exit criteria:** Arrow keys navigate cards smoothly. Enter drills into group view. Tab switching preserves per-tab selection. Grid navigation is unit tested for edge cases (1 group, odd count, 1 column, empty).

#### Phase 5: Property Watch Lifecycle

Establish watches on entering Groups tab, unwatch on leaving. Process `ChangeEvent`s in the event loop.

**Watch setup** (called on Groups tab enter):
- For each group in `system.groups()`:
  - Get coordinator speaker
  - Call `speaker.current_track.watch()`, `speaker.playback_state.watch()`, `speaker.position.watch()`, `group.volume.watch()`
  - If `WatchStatus.current` is `None`, call `fetch()` to seed the cache
  - Record watches in `app.watch_registry`
  - Initialize `ProgressState` from fetched position and playback state

**Watch teardown** (called on Groups tab leave — tab switch or drill-in):
- For each entry in `app.watch_registry`:
  - Call `unwatch()` on the corresponding property handle
  - Clear the registry
  - Clear `app.progress_states`

**Event processing** in `event.rs` event loop:
```rust
for event in change_iter.try_iter() {
    match event.property_key {
        "current_track" | "playback_state" | "position" | "group_volume" => {
            // Update progress_states if position or playback_state changed
            app.dirty = true;
        }
        "group_membership" => {
            // Topology changed — teardown all watches and re-establish
            app.dirty = true;
        }
        _ => {}
    }
}
```

**Exit criteria:** Properties are watched on Groups tab entry. Changes from the Sonos app (volume knob, track skip) appear on screen within 1 second. Watches are cleaned up on tab switch and quit.

#### Phase 6: Progress Bar Animation

Implement client-side position interpolation for smooth progress bars.

**`src/tui/event.rs` — animation tick:**
- After processing all events in each loop iteration:
  - Check if any `ProgressState` has `is_playing == true`
  - If so, set `app.dirty = true` (forces re-render on next frame)
  - During render, each card computes interpolated position:
    ```rust
    let elapsed_since_update = now.duration_since(state.wall_clock_at_last_update);
    let interpolated_ms = state.last_position_ms + elapsed_since_update.as_millis() as u64;
    let clamped_ms = interpolated_ms.min(state.last_duration_ms);
    ```

**Reset interpolation** on SDK events:
- `"position"` event: update `last_position_ms`, `last_duration_ms`, `wall_clock_at_last_update` from `position.get()`
- `"playback_state"` event: update `is_playing` from `playback_state.get()`. If changed to `Paused`/`Stopped`, freeze `last_position_ms` at current interpolated value

**Change poll interval:**
- `event.rs` line 26: change `Duration::from_millis(250)` to `Duration::from_millis(50)`

**Exit criteria:** Progress bars animate smoothly for playing groups. Bars freeze when paused. Position resets correctly on SDK position events. No visible jitter or drift.

#### Phase 7: Speakers Tab Rendering

Replace the Speakers tab stub with the real speaker list.

**`src/tui/screens/home_speakers.rs`:**
- Build speaker list organized by group:
  - For each group in `system.groups()` where `!group.is_standalone()`:
    - Render group name as section header with `group_header` style
    - List members with `speaker.name` + `speaker.model_name` + volume bar
    - Mark coordinator with `(coordinator)` suffix
  - "NOT IN A GROUP" section for standalone groups:
    - List each standalone group's coordinator speaker
- `▸` cursor on selected speaker
- Volume bars using the `volume_bar` widget (fetch each speaker's volume via `.get()`)
- Empty state: "No speakers found" centered message

**Group picker modal** (for Enter = move to group):
- Render `modal` widget with list of non-standalone group names
- `Up/Down` to select target group, `Enter` to confirm, `Esc` to cancel
- On confirm: `target_group.add_speaker(&speaker)` — handle errors with inline message
- Store modal state as `Option<ModalState>` in `HomeSpeakersState`

**Exit criteria:** Speakers tab shows all speakers grouped correctly. Cursor navigation works. `n`/`d`/Enter actions modify topology correctly. Error messages appear for invalid operations.

#### Phase 8: Mini-Player

Render the mini-player bar for both Home tabs.

**Integration in `src/tui/ui.rs`:**
- When rendering `Screen::Home`, allocate 2 lines for mini-player between content and legend
- Determine focused group:
  - Groups tab: group at `groups_state.selected_index`
  - Speakers tab: group of the speaker at `speakers_state.selected_index`
- Read focused group's coordinator properties via `.get()`
- Render `mini_player` widget with group name, playback state, track info, interpolated progress, volume
- If no groups exist, hide mini-player (content area gets the full space)

**Exit criteria:** Mini-player shows the focused group's now-playing info. Updates when arrow-keying between cards/speakers. Displays correct data for playing, paused, and stopped states.

## System-Wide Impact

- **Event loop change:** Poll interval drops from 250ms to 50ms. All screens (including future M8/M9) will run at this cadence.
- **Layout change:** `ui.rs` main layout adds a conditional mini-player region. Future screens that don't need the mini-player (GroupView, SpeakerDetail) continue with the 3-region layout.
- **Theme expansion:** All future screens will use the expanded theme styles. The four built-in themes must be maintained going forward.
- **Watch pattern:** The watch/unwatch lifecycle pattern established here becomes the template for GroupView (M8) and SpeakerDetail (M9).
- **Module structure:** Creating `screens/` and `widgets/` directories sets the organization pattern for all future TUI work.

## Acceptance Criteria

### Functional Requirements

- [ ] **Groups tab renders live group cards** with name, playback state icon, track/artist, volume bar, progress bar, speaker count
- [ ] **Responsive grid**: 2 columns on terminals >= 100 chars, 1 column on narrow
- [ ] **Selected card** has double border, bold title, `●` indicator
- [ ] **Arrow key navigation** moves between cards in the grid
- [ ] **Enter** drills into GroupView for the selected group
- [ ] **Mini-player** shows focused group's now-playing info on both Home tabs
- [ ] **Mini-player updates** when navigating between cards/speakers
- [ ] **Progress bars animate** smoothly for playing groups (client-side interpolation)
- [ ] **Progress resets** on authoritative SDK position events
- [ ] **Progress freezes** when playback state changes to Paused/Stopped
- [ ] **Live updates** from SDK change events: volume, track change, play/pause appear within 1 second
- [ ] **Property watches** established on Groups tab entry, cleaned up on leave
- [ ] **Speakers tab** shows speakers organized by group with headers
- [ ] **"NOT IN A GROUP"** section shows standalone speakers
- [ ] **`n`** creates a new standalone group from selected speaker
- [ ] **`d`** ungroups selected speaker (error message for coordinators with members)
- [ ] **Enter** opens group picker modal, confirms move-to-group
- [ ] **Tab switching** preserves per-tab selection state
- [ ] **Empty states** handled: no groups shows message, no speakers shows message, stopped groups show "Nothing playing"
- [ ] **Coordinator unavailable** handled: card shows group name with "Unavailable"
- [ ] **Scrolling** works when groups/speakers exceed viewport height

### Non-Functional Requirements

- [ ] **50ms poll interval** does not cause noticeable CPU usage on idle
- [ ] **No `fetch()` calls during render** — render functions only use `.get()` cache reads
- [ ] **No panics** from `None` coordinator, empty groups, or missing property data
- [ ] **Theme consistency** — all widgets use `app.theme` styles, no hardcoded colors

## Dependencies & Risks

| Dependency | Risk | Mitigation |
|------------|------|------------|
| SDK `watch()` reliability | UPnP subscriptions may fail on some networks | Fallback to periodic `fetch()` if `WatchMode::CacheOnly` |
| SDK `group_membership` events | Topology changes must trigger re-watch | Teardown and re-establish all watches on topology event |
| `ratatui` Gauge/block rendering | Custom progress bars may need precise character-level control | Use raw `Span`/`Line` rendering instead of `Gauge` widget if needed |
| Terminal width detection | Some terminals report incorrect width | Fallback to 1-column layout on any width < 50 |

## Sources & References

### Origin

- **Brainstorm document:** [docs/brainstorms/2026-02-26-sonos-tui-brainstorm.md](../brainstorms/2026-02-26-sonos-tui-brainstorm.md) — groups-first overview (section 1), card design (section 3), mini-player behavior (section 2), speakers-by-group layout (section 1b), album art deferred (section 4), key legend (section "Key Legend Behavior")

### Internal References

- TUI foundation plan: `docs/plans/2026-03-22-feat-tui-foundation-plan.md` — event loop, navigation, theme, rendering patterns
- SDK API reference: `docs/references/sonos-sdk.md` — property handles, watch/unwatch, group/speaker methods
- Roadmap milestone 7: `docs/product/roadmap.md` (lines 370-545) — checklist items this plan implements
- Current TUI code: `src/tui/app.rs`, `src/tui/event.rs`, `src/tui/ui.rs`, `src/tui/theme.rs`
