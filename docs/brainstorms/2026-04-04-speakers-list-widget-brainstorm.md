# Speakers List Widget Brainstorm

**Date:** 2026-04-04
**Status:** Ready for planning
**Roadmap:** Milestone 8 (Group View — Speakers tab) + Home Speakers tab rework

## What We're Building

A shared `SpeakerList` widget used by both **Home > Speakers** tab and **GroupView > Speakers** tab. Speakers are displayed nested under group headers. The widget supports volume control, navigation into speaker/group views, and a pick-up/drop interaction for regrouping speakers between Sonos groups.

### Two Modes

| Mode | Context | Default View | Pick-up View |
|------|---------|-------------|--------------|
| `FullList` | Home > Speakers tab | All groups with nested speakers | Same (already showing everything) |
| `GroupScoped { group_id }` | GroupView > Speakers tab | Only that group's members + "Add Speaker" row | Expands to full group list (so you can see where to drop) |

### Display Layout

**Group header line:**
```
▶ Living Room    Shape of You · Ed Sheeran    ████████░░ 65    (highlighted)
  Kitchen Group  Blinding Lights · The Weeknd                72    (not highlighted)
```
- Play state icon (▶ playing, ⏸ paused, ■ stopped)
- Group name
- Current track — song name · artist name
- Volume bar (only visible when row is highlighted/selected)
- Volume percent (always visible)

**Speaker line (indented under group):**
```
    Kitchen                                    ██████░░░░ 48    (highlighted)
    Bedroom                                                52    (not highlighted)
```
- Speaker name
- Volume bar (only visible when row is highlighted/selected)
- Volume percent (always visible)

**"Not in a group" section** at the bottom for standalone speakers.

**"Add Speaker" row** (GroupScoped mode only, at end of group members):
```
    + Add speaker...
```

### Keyboard Interactions

| Key | On Group Header | On Speaker Line | On "Add Speaker" Row |
|-----|----------------|-----------------|---------------------|
| **Up/Down** | Navigate list | Navigate list | Navigate list |
| **Left/Right** | Adjust group volume | Adjust speaker volume | — |
| **Enter** | Navigate to GroupView | Navigate to SpeakerDetail | Enter pick-up mode (show full list) |
| **Space** | — | Pick up speaker (enter move mode) | Enter pick-up mode (show full list) |
| **Esc** | — | Cancel pick-up (if in move mode) | Cancel pick-up |

### Pick-Up / Drop Interaction (Move Mode)

1. **Space on a speaker** → speaker is "picked up" (visually highlighted/detached)
2. In `GroupScoped` mode, the view expands to show all groups (so you have somewhere to drop)
3. **Up/Down** moves the picked-up speaker indicator through the full list
4. **Space again** → drops the speaker:
   - Dropped anywhere in a group's section (on header or between members) → `speaker.join_group(&group)` or `group.add_speaker(&speaker)`
   - Dropped below all groups / in "Not in a group" section → `speaker.leave_group()` (becomes standalone)
   - Dropped in its current group → no-op
5. **Esc** → cancels the pick-up, returns speaker to original position

### SDK Calls

| Action | SDK Method |
|--------|-----------|
| Adjust speaker volume | `speaker.set_relative_volume(delta)` |
| Adjust group volume | `group.set_relative_volume(delta)` |
| Move speaker to group | `group.add_speaker(&speaker)` |
| Ungroup speaker | `speaker.leave_group()` |
| Read speaker volume | `use_watch(speaker.volume())` |
| Read group volume | `use_watch_group(group.volume())` (or watch coordinator) |
| Read playback state | `use_watch_group(group.playback_state())` |
| Read current track | `use_watch_group(group.current_track())` |

## Why This Approach

**Single shared widget with mode enum** was chosen over separate implementations because:
- User explicitly wants both views to "function the same, maybe even be reused"
- One place to fix bugs, one set of key handlers, consistent behavior guaranteed
- The mode branching is minimal: `GroupScoped` just filters the initial list and adds the "Add Speaker" row
- Pick-up mode in `GroupScoped` naturally expands to the full list, converging with `FullList` behavior

## Key Decisions

1. **Shared widget, not separate implementations** — single `SpeakerList` widget with `FullList` and `GroupScoped` modes
2. **Pick-up/drop for regrouping** — Space picks up, arrows move, Space drops. This is the primary mechanism for moving speakers between groups
3. **Context-aware volume** — Left/Right on group header adjusts group volume; on speaker line adjusts speaker volume
4. **Enter is context-aware** — group header → GroupView screen; speaker line → SpeakerDetail screen
5. **Drop outside groups = ungroup** — dropping a speaker below all groups or in "Not in a group" section makes it standalone via `speaker.leave_group()`
6. **GroupScoped expands on pick-up** — when a speaker is picked up in GroupScoped mode, the view expands to show all groups so the user can see where to drop it
7. **"Add Speaker" row** — GroupScoped mode shows an "Add Speaker" row at the end that enters pick-up mode showing the full list of available speakers

8. **Volume bar visibility** — volume percent is always visible on every row; the volume bar graphic only renders when the row is highlighted/selected

## Open Questions

None — all questions resolved during brainstorm.
