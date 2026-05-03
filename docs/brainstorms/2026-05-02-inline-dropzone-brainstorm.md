# Inline Dropzone — Zero Layout Shift Pickup Mode

**Date:** 2026-05-02
**Status:** Decided
**Milestone:** TUI Polish

## What We're Building

Replace the current dropzone view (which swaps the entire speaker list for bordered boxes) with an inline pickup mode that preserves the normal speaker list layout. When a speaker is picked up, each group gains a `└─ Add to group` row in the blank-line slot below its last member. Navigation moves between these "Add to group" rows. The result is zero layout shift — the list stays exactly where it was.

## Why This Approach

The current pickup mode is jarring. It replaces group headers, tree connectors, speaker rows, and dividers with a completely different bordered-box layout. Users lose spatial context of where groups are. The inline approach keeps everything in place and uses the blank line the layout already reserves between groups.

## Key Decisions

### 1. "Add to group" rows use tree connectors
Each group's add row renders as `└─ Add to group`, visually extending the tree structure. This makes it feel like a natural part of the group, not an alien UI element.

### 2. Picked-up speaker stays visible with reverse/inverted highlight
The picked-up speaker's row remains in its original position with a reverse (inverted fg/bg) style — impossible to miss, clearly communicating "this is what you're carrying." Tree connectors within the group stay unchanged. True zero layout shift.

### 3. Home group shows dimmed "Already in group"
The group the speaker currently belongs to shows `└─ Already in group` in dimmed/muted text. This row is visible but not selectable (navigation skips it). Makes it clear the speaker is already there without hiding the row.

### 4. Navigation targets are "Add to group" rows + "Create new group"
Up/Down moves between selectable add rows and the "Create new group" entry at the bottom. Group headers and speaker rows are visible but not navigable in pickup mode.

### 5. "Create new group" row at the bottom
A `► Create new group` row appears after the last group divider. Dropping here calls `speaker.leave_group()`.

### 6. Status bar stays
The `Picked up: {name}` status line and `␣ drop  Esc cancel` hints remain at the top, same as current behavior.

## Visual Design

```
Picked up: Kitchen                           ␣ drop  Esc cancel

  ⏸ Bathroom ::::: :::::::::::::::::::::::::::::::::::::::::::  -
    ├─ Bathroom • Sonos Connect:Amp                           35%
    ├─ ██ Kitchen • Sonos Connect:Amp ██████████████████████  27%   ← reverse highlight
    ├─ Bedroom • Sonos Connect:Amp                            34%
    └─ Already in group                                             ← dimmed, not selectable

    +────────────────────────────────────────────────────────────+

  ■ Basement ::::: Unknown ::::::::::::::::::::::::::::::::::::  -
    ├─ Unknown
 ❯  └─ Add to group                                                ← accent, selected

    +────────────────────────────────────────────────────────────+

  ▶ Office / Roam ::::: Used To — Drake :::::::::::::::::::::::  -
    ├─ Office / Roam • Sonos Roam 2                            23%
    └─ Add to group

    +────────────────────────────────────────────────────────────+

  ▶ Living Room ::::: Once Upon a Time | S2 E9 :::::::::::::::  -
    ├─ Living Room • Sonos Amp                                 17%
    └─ Add to group

    ► Create new group
```

## Open Questions

None — all design questions resolved during brainstorm.
