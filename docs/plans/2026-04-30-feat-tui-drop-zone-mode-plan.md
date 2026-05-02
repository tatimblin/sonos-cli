---
title: "feat: Drop zone regrouping mode"
type: feat
status: active
date: 2026-04-30
origin: docs/brainstorms/2026-04-30-tui-v1-simplification-brainstorm.md
parallel-group: tui-v1-simplification
---

# Drop Zone Regrouping Mode

## Overview

Redesign the pick-up/drop interaction for speaker regrouping. When a speaker is picked up (Space), the list transforms: speaker rows collapse into dashed-border drop zones under each group header. The active drop zone uses a solid accent-color border. An "Add new group" option appears at the bottom. This replaces the current inline-reorder visual approach.

(see brainstorm: `docs/brainstorms/2026-04-30-tui-v1-simplification-brainstorm.md` — "Pick-Up / Drop Zone Mode" section)

## Problem Statement / Motivation

The current pick-up mode visually reorders the picked-up speaker within the flat list, which makes it unclear where the speaker will end up. The user has to infer the target group from surrounding list entries. The drop zone design makes regrouping targets explicit: each group becomes a labeled drop zone showing its current members, with clear visual borders distinguishing the active target.

## Proposed Solution

Modify the speaker list widget and handler to render a completely different visual when in pick-up mode. The three-layer architecture is preserved: the handler manages pick-up state, the screen assembles drop zone data, and the widget renders the transformed layout.

## Technical Approach

### Target Layout (Pick-Up Mode)

```
  Picked up: One SL (Left)                              ␣ drop  Esc cancel

  ▶ Living Room                                                          70%
    Bohemian Rhapsody — Queen
    ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓  ◀
    ┃ ╲ ╲ ╲ ╲ ╲ ╲ ╲  Drop here — Beam + One SL (Right)  ╲ ╲ ╲ ╲ ╲ ╲ ╲  ┃
    ┃ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲  ┃
    ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛

  ⏸ Kitchen                                                               40%
    Hotel California — Eagles
    ┎┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┒
    ┆ ╲ ╲ ╲ ╲ ╲  Drop here — Sonos One  ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲  ┆
    ┖┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┚

    ┎┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┒
    ┆ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲  Add new group  ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲ ╲  ┆
    ┖┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┚
```

Note: The Living Room drop zone is taller (4 inner lines) because it has 2 remaining members (Beam + One SL (Right)) — the zone height is proportional to the number of speakers that were in that section of the normal list. Kitchen has 1 member → 1 inner line. "Add new group" always gets 1 inner line. The diagonal stripe pattern (`╲ ╲ ╲`) fills all inner lines, with the label text centered on the first line.

### Phase 1: Extend Data Structures

**New types in `types.rs`:**

```rust
pub struct DropZoneData {
    pub zones: Vec<DropZone>,
    pub picked_speaker_name: String,
    pub active_zone_index: usize,
}

pub struct DropZone {
    pub kind: DropZoneKind,
    pub group_name: Option<String>,          // None for "Add new group"
    pub playback_state: Option<PlaybackState>,
    pub track_info: Option<String>,
    pub group_volume: Option<u16>,
    pub remaining_members: Vec<String>,      // speaker names still in group (excluding picked)
    pub original_member_count: usize,        // total speakers in group before pick-up (determines zone height)
}

pub enum DropZoneKind {
    ExistingGroup(GroupId),
    NewGroup,
}
```

**Update `PickUpState` in `types.rs`:**

```rust
pub struct PickUpState {
    pub speaker_id: SpeakerId,
    pub speaker_name: String,        // new: for the status line display
    pub original_group_id: Option<GroupId>,
    pub active_zone_index: usize,    // renamed from drop_index: index into DropZoneData.zones
}
```

### Phase 2: Screen Data Assembly for Drop Zones

**`screens/speakers.rs` — when `pick_up` is `Some`:**

Build `DropZoneData` instead of normal `SpeakerListData`:

1. For each group in `system.groups()`:
   - Subscribe to coordinator's `playback_state`, `current_track`, `group.volume`
   - Collect remaining member names (excluding the picked-up speaker)
   - Create `DropZone { kind: ExistingGroup(group_id), ... }`
2. Append `DropZone { kind: NewGroup, ... }` at the end
3. Set `active_zone_index` from `pick_up.active_zone_index`
4. Return `DropZoneData` alongside (or instead of) `SpeakerListData`

The screen can return an enum:

```rust
pub enum SpeakerScreenData {
    Normal(SpeakerListData),
    PickUp(DropZoneData),
}
```

### Phase 3: Widget Rendering — Drop Zone Mode

**`widgets/speaker_list.rs` — new rendering branch:**

When receiving `DropZoneData`, render the transformed layout:

**Status line** (top, 1 line):
`  Picked up: {speaker_name}` left-aligned + `␣ drop  Esc cancel` right-aligned (muted style)

**For each drop zone:**

1. **Group header** (2 lines, same as normal mode): icon + name + volume / track info
   - Skipped for `NewGroup` zone
2. **Drop zone box** (proportional height with diagonal stripe fill):
   - **Height**: The inner height of each zone equals `original_member_count` — the number of speaker rows that section occupied in the normal list. This makes the drop zone "fill the space" where those speakers were, giving a spatial feel to the transformation. "Add new group" always has 1 inner line.
   - **Active zone** (cursor): solid accent-color border using `┏━┓┃┗━┛` characters + accent style
   - **Inactive zone**: dashed border using `┎┄┒┆┖┄┚` characters + muted style
   - **Stripe fill**: All inner lines are filled with a repeating diagonal stripe pattern `╲ ╲ ╲ ╲ ╲` in muted style. The label text is centered on the first inner line, cutting through the stripes.
   - **Label text**:
     - Existing group: `Drop here — {remaining_members joined with " + "}`
     - Existing group (empty after pick-up): `Drop here (empty)`
     - New group: `Add new group`
3. **Cursor indicator**: `◀` at the right of the active zone's border

**Spacing**: blank line between drop zones.

**Stripe pattern detail:**

The stripe fill uses `╲` (U+2572, Box Drawing Light Diagonal Upper Left to Lower Right) alternating with spaces: `╲ ╲ ╲ ╲`. This creates a hatched/construction-zone feel that clearly signals "this is a drop target, not content." The stripe characters use `theme.muted` style so they don't compete with the label text.

For the label line (first inner line), the stripe pattern fills the background and the label text overwrites the center portion:

```rust
fn stripe_line(width: usize) -> String {
    "╲ ".repeat(width / 2 + 1)[..width].to_string()
}

fn label_over_stripes(label: &str, width: usize) -> String {
    let stripes = stripe_line(width);
    let pad = (width.saturating_sub(label.len())) / 2;
    let mut line = stripes;
    line.replace_range(pad..pad + label.len(), label);
    line
}
```

For non-label inner lines (rows 2+ in taller zones), render pure stripes:

```rust
// Inner line with stripes only
let stripe = format!("    ┃{}┃", stripe_line(inner_width));  // active
let stripe = format!("    ┆{}┆", stripe_line(inner_width));  // inactive
```

**Border rendering detail:**

```rust
// Active zone (solid, accent color)
let top    = format!("    ┏{}┓", "━".repeat(inner_width));
let label  = format!("    ┃{}┃", label_over_stripes(&content, inner_width));
let stripe = format!("    ┃{}┃", stripe_line(inner_width));  // repeated for remaining inner lines
let bottom = format!("    ┗{}┛", "━".repeat(inner_width));

// Inactive zone (dashed, muted)
let top    = format!("    ┎{}┒", "┄".repeat(inner_width));
let label  = format!("    ┆{}┆", label_over_stripes(&content, inner_width));
let stripe = format!("    ┆{}┆", stripe_line(inner_width));  // repeated for remaining inner lines
let bottom = format!("    ┖{}┚", "┄".repeat(inner_width));
```

### Phase 4: Handler Updates

**`handlers/speaker_list.rs` — pick-up mode:**

Replace the current `build_display_order`-based navigation:

- **Up/Down**: Move `active_zone_index` between zones (0..zones.len()-1). Clamp to bounds on every press (topology may have changed between frames).
- **Space**: Execute drop based on `zones[active_zone_index].kind`:
  - `ExistingGroup(group_id)`: if different from `original_group_id`, call `group.add_speaker(&speaker)`. If `system.group_by_id(&group_id)` returns `None` (group dissolved during pick-up), show error and cancel.
  - `ExistingGroup(group_id)`: if same as `original_group_id`, no-op (cancel equivalent)
  - `NewGroup`: call `speaker.leave_group()` to make it a standalone group. If speaker is already standalone, treat as no-op.
  - Show status message on success/error
  - Clear pick-up state
  - Set `selected_index` to the moved speaker's new position in the rebuilt list (find by `speaker_id`), or clamp to list end if not found
- **Esc**: Clear pick-up state, restore `selected_index` to original value, restore normal view
- **All other keys** (`p`, `n`, `b`, Left, Right, etc.): swallowed — return `Handled` without action

**Edge case — last speaker in group:**

When the picked-up speaker is the only member of a group, that group still appears as a drop zone with `remaining_members: vec![]` and content "Drop here (empty)". Dropping back is a no-op. This gives the user a visual "undo" without needing Esc.

**Edge case — coordinator pick-up:**

Picking up a coordinator is allowed. When dropped into a different group, use `group.add_speaker(&speaker)` — the Sonos system handles coordinator delegation to the next member automatically. The group header during pick-up mode may show the old coordinator name for the duration of the pick-up; this is acceptable since the mode is transient.

**Initial `active_zone_index`:** Set to the index of the speaker's current group in the zone list. This makes Space-Space a safe no-op cancel (default gesture does nothing).

### Phase 5: Scrolling in Drop Zone Mode

Each drop zone consumes ~5 visual lines (2 header + 3 box) plus blank separators. With 5+ groups, the total exceeds a standard terminal.

**Approach:** Track a `scroll_offset` in visual lines. On each render, compute the visual position of the active zone. If outside the viewport, adjust `scroll_offset` to center the active zone. The widget clips zones that fall outside the viewport.

This is independent of the speaker list's normal-mode scrolling (from the Enhanced Speaker List plan) — pick-up mode has its own scroll state.

### Phase 6: Remove Old Pick-Up Rendering

Delete `build_display_order()` from `types.rs` — it's replaced by the drop zone approach. The function currently reorders flat list indices to simulate the speaker moving; the new design doesn't need this.

## Design Decisions (from SpecFlow analysis)

| Question | Decision | Rationale |
|----------|----------|-----------|
| Can coordinators be picked up? | **Yes.** Use `group.add_speaker()` for the drop — Sonos handles coordinator delegation automatically. | Restricting to non-coordinators would confuse users who don't understand the coordinator concept. |
| How are standalone speakers shown in drop zone mode? | **Each standalone group gets its own drop zone** via `system.groups()`. If the picked-up speaker IS the only member, that zone shows "Drop here (empty)". | Consistent treatment of all groups. The brainstorm's "visual undo" pattern relies on the original group always being visible. |
| Suppress "Add new group" when speaker is already standalone? | **No — always show it.** Treat it as a no-op if the speaker is already standalone. | Visual consistency. Two no-op targets (own zone + add new group) is mildly redundant but predictable. |
| Static or dynamic zone list during pick-up? | **Dynamic** — rebuild from `system.groups()` each frame. Clamp `active_zone_index` to `zones.len()-1`. | Static snapshots would diverge from reality. Clamping handles edge cases simply. |
| What if target group dissolves between entering pick-up and dropping? | **Show error status message and cancel the drop.** | Better than a silent no-op. The user sees "error: group not found" and can retry. |
| Cursor position after successful drop? | **Find the moved speaker in the rebuilt list by ID, set `selected_index` there.** | Maintains user context — they see the speaker they just moved. |
| Cursor position after cancel (Esc)? | **Restore original `selected_index`.** | The list didn't change, so the original position is valid. |
| Bottom bar during pick-up mode? | **Freeze on the speaker's original group.** | The active zone already has a visible group header; updating the bar would be confusing. |
| Key blocking during pick-up? | **Only Up/Down/Space/Esc active. All other keys swallowed.** | Prevents accidental volume changes or playback mutations during the modal regrouping operation. |
| Drop zone content text truncation? | **Truncate with `…` when member names exceed inner box width.** | Long member lists (5+ speakers) would overflow the box. |

## Acceptance Criteria

- [ ] Space on a speaker row enters drop zone mode with the transformed layout
- [ ] Space on a group header does nothing (only speaker rows can be picked up)
- [ ] Group headers stay visible in drop zone mode with 2-line layout (icon+name+vol / track)
- [ ] Speaker rows collapse into drop zones with height proportional to original member count
- [ ] Drop zone inner area filled with diagonal stripe pattern (`╲ ╲ ╲`) in muted style
- [ ] Label text centered on first stripe line, cutting through the pattern
- [ ] Active drop zone has solid accent-color border (`┏━┓┃┗━┛`)
- [ ] Inactive drop zones have dashed muted border (`┎┄┒┆┖┄┚`)
- [ ] "Add new group" zone appears at the bottom
- [ ] Up/Down navigates between drop zones; all other keys swallowed
- [ ] Space drops the speaker into the active zone (calls SDK `add_speaker` or `leave_group`)
- [ ] Esc cancels and restores normal view with original cursor position
- [ ] Dropping into the same group is a no-op
- [ ] Coordinators can be picked up and moved (SDK handles delegation)
- [ ] Last-speaker-in-group edge case: group stays visible with "Drop here (empty)"
- [ ] Already-standalone speaker dropped on "Add new group" treated as no-op
- [ ] Status message shown on successful drop or error
- [ ] Cursor tracks the moved speaker after successful drop
- [ ] Drop zone view scrolls to keep active zone visible
- [ ] `active_zone_index` clamped to bounds on each frame (handles topology changes)
- [ ] `build_display_order()` removed
- [ ] Three-layer architecture maintained

## Dependencies & Risks

**Dependencies:**
- Should be developed after or in coordination with the **Enhanced Speaker List** plan, since both modify `widgets/speaker_list.rs`. However, the drop zone rendering is a completely separate branch within the widget (triggered by `PickUpState` being `Some`), so merge conflicts should be limited to `use` statements and the main render function's top-level dispatch.
- If the Enhanced Speaker List plan removes `UngroupedHeader` (treating all groups including standalone uniformly), this plan benefits — `system.groups()` naturally includes standalones.

**Risks:**
- **Box drawing character support**: The dashed border characters (`┎┄┒┆┖┄┚`) are part of Unicode Box Drawing (U+250x–U+257F) and are well-supported, but some monospace fonts may render them at inconsistent widths. Test across iTerm2, Kitty, and Terminal.app.
- **SDK call latency**: `add_speaker()` and `leave_group()` are synchronous SOAP calls (~200–500ms). The TUI freezes during the call. Acceptable for v1 — a "Moving…" status message before the call would require flushing a frame mid-handler, violating the three-layer architecture.
- **Vertical space**: Each drop zone takes ~5 lines. Scrolling (Phase 5) mitigates this, but very short terminals (< 12 rows) may need the drop zone mode suppressed entirely.

## Sources & References

- **Origin brainstorm:** [docs/brainstorms/2026-04-30-tui-v1-simplification-brainstorm.md](docs/brainstorms/2026-04-30-tui-v1-simplification-brainstorm.md) — drop zone visual design, border styles, "Add new group", last-speaker edge case
- **Current pick-up code:** `src/tui/handlers/speaker_list.rs` — existing pick-up/drop logic to replace
- **Current types:** `src/tui/types.rs` — `PickUpState`, `build_display_order()` (to remove)
- **SDK methods:** `group.add_speaker(&speaker)`, `speaker.leave_group()` — already used by current handler
