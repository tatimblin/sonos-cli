# TUI v1 Simplification

**Date:** 2026-04-30
**Status:** decided
**Supersedes:** `2026-02-26-sonos-tui-brainstorm.md` (TUI screen architecture), relevant sections of `docs/goals.md`

---

## What We're Building

A radically simplified TUI with two views and a persistent Spotify-style player bar. The current goals.md describes a multi-screen, multi-tab architecture (Groups tab, Now Playing tab, Queue tab, Speaker Detail, Startup animation) that is too ambitious for v1. This strips it down to one useful screen that does the two things people actually want: see all their speakers and control playback.

### Two views, one player bar

**1. Main View — Speakers by Group (tab: `Speakers`)**

A single scrollable list. Groups are section headers (2 lines each). Speakers are nested under their group. Ungrouped speakers appear under a separator.

```
♪ S O N O S                                                    [▸Speakers]  Settings
─────────────────────────────────────────────────────────────────────────────────────
▶ Living Room                                                                   70%
  Bohemian Rhapsody — Queen
  ├ Beam                                                                        60%
  ├ One SL (Left)                              ■■■■■■■····                      60%  ◀ cursor
  └ One SL (Right)                                                              60%

⏸ Kitchen                                                                       40%
  Hotel California — Eagles
  └ Sonos One                                                                   40%

■ Bedroom                                                                        0%
  Nothing playing
  └ Sonos One                                                                   25%
─────────────────────────────────────────────────────────────────────────────────────
┌────┐ Bohemian Rhapsody          ⏮  ▶  ⏭                       Living Room
│░░░░│ Queen            2:31 ━━━━━━━━━━━╺──────────── 5:55    ■■■■■■■··· 70%
└────┘
─────────────────────────────────────────────────────────────────────────────────────
 ↑↓ Navigate  ←→ Volume  ␣ Pick up/Drop  ? Help  ⎋ Quit
```

Key behaviors:
- **Group headers** (2 lines): Line 1 = play/pause/stop icon + group name + group volume number. Line 2 = current track — artist (or "Nothing playing").
- **Speaker rows** (1 line): tree connector + speaker name + volume number.
- **Selected row only** gets the volume bar visualization. All others show just the number.
- **Volume adjustment**: Left/Right arrows on any row (group header adjusts group volume, speaker row adjusts speaker volume).
- **Regrouping**: Space to pick up a speaker, the view transforms into drop zone mode (see below), navigate to target group, Space to drop.
- **Bottom bar** follows cursor — always shows the group that contains the currently focused row.

### Pick-Up / Drop Zone Mode

When a speaker is picked up (Space), the entire list transforms. Speaker rows collapse and are replaced by dashed-border drop zones under each group header. The focused drop zone gets a **solid border in accent color** to clearly indicate the active target. An "Add new group" option appears at the bottom as a headerless drop zone.

```
  Picked up: One SL (Left)                              ␣ drop  Esc cancel

  ▶ Living Room                                                          70%
    Bohemian Rhapsody — Queen
    ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓  ◀
    ┃          Drop here — Beam + One SL (Right)                          ┃
    ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛

  ⏸ Kitchen                                                               40%
    Hotel California — Eagles
    ┎┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┒
    ┆          Drop here — Sonos One                                      ┆
    ┖┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┚

    ┎┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┒
    ┆          Add new group                                              ┆
    ┖┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┚
```

Key behaviors:
- **Status line** at top shows the picked-up speaker name + source group
- **Group headers stay** with their 2-line layout (name, track info)
- **Speaker rows collapse** into a dashed-border drop zone showing remaining members
- **Active drop zone** (cursor) uses a **solid accent-color border** (━/┃) to distinguish from inactive dashed zones
- **"Add new group"** appears at the bottom as a drop zone with no group header — drops the speaker into a new standalone group
- **↑↓** navigates between drop zones, **Space** drops, **Esc** cancels and restores the normal view
- **Last speaker edge case**: if the picked-up speaker was the only member of a group, that group stays visible with an empty drop zone ("Drop here (empty)"). Dropping back into the same group is a no-op — this lets the user cancel by dropping back instead of pressing Esc

**2. Settings View (tab: `Settings`)**

Minimal config screen accessible via tab bar (Left/Right arrows on tab bar).

```
♪ S O N O S                         Speakers  [▸Settings]
──────────────────────────────────────────────────────────
  Settings

  Theme:          [ dark ▼ ]
  Default group:  [ Living Room ▼ ]
  Album art:      [ on ▼ ]
```

Three settings only:
- **Theme**: dark, light, neon, sonos
- **Default group**: dropdown of discovered groups
- **Album art**: on/off toggle (for terminal compatibility)

### Spotify-Style Bottom Player Bar

Persistent bar at the bottom of the screen (both views). Shows the currently focused group's playback info. Responsive layout adapts to terminal width.

**Wide layout** (terminal ≥ 100 cols) — 3 columns like flexbox, 3 lines:

```
┌────┐ Bohemian Rhapsody          ⏮  ▶  ⏭                       Living Room
│░░░░│ Queen            2:31 ━━━━━━━━━━━╺──────────── 5:55    ■■■■■■■··· 70%
└────┘
```

**Narrow layout** (terminal < 100 cols) — 50/50 split left/right, progress full-width below:

```
┌────┐ Bohemian Rhapsody             Living Room
│░░░░│ Queen                      ■■■■■■■··· 70%
└────┘
         ⏮  ▶  ⏭   2:31 ━━━━━━━━━━━━━━━━━━━━━━━╺──────────────── 5:55
```

In narrow mode: left half = album art + track/artist, right half = group name + volume (space-between). Controls move down to join the progress bar on a full-width row underneath.

Layout regions (3-column flexbox model):
- **Left**: 3×3 album art + track title (line 1) and artist (line 2), with **ticker scrolling** when text exceeds available width
- **Center**: Playback controls ⏮ ▶ ⏭ (line 1) and progress bar with split times (line 2)
- **Right**: Group name (line 1) and volume bar (line 2)

Tracking: Follows cursor — when cursor moves into a different group, bar updates to that group's playback.

Controls:
- `p` or media key: play/pause toggle
- `n` or media key: next track
- `b` or media key: previous track

---

## Why This Approach

The original design tried to replicate the Sonos mobile app in a terminal — full Now Playing screens, per-group drill-in with 3 tabs, queue management, speaker detail with EQ controls. That's months of work for features most terminal users don't need.

This design optimizes for the two primary use cases:
1. **Glance at what's playing** — group headers show playback state at a glance, bottom bar shows detail for the focused group
2. **Regroup speakers** — the existing pick-up/drop interaction is the killer TUI feature

Everything else (queue management, EQ, seek, play mode) stays in the CLI commands where it already works.

---

## Key Decisions

| Decision | Choice | Why |
|----------|--------|-----|
| Screen count | 2 views (Speakers + Settings) | Everything else is YAGNI for v1 |
| Group header | 2 lines: name+icon+vol / track+artist | Scannable without being noisy |
| Volume bar | Cursor-only expansion | Keep non-selected rows compact |
| Bottom bar | Spotify-style with album art, controls, progress | Single control surface, always visible |
| Bottom bar tracking | Follows cursor group | Simple mental model — what you see is what you control |
| Album art | Bottom bar thumbnail only (3×3) | Cut hero (20×20) and queue (1×1) sizes |
| Navigation | Tab bar at top (Speakers / Settings) | Familiar pattern from existing implementation |
| Drop zone mode | Dashed borders replace speaker rows; solid accent on active zone; "Add new group" at bottom | Clear visual affordance for regrouping without extra screens |
| Settings | Theme + default group + album art toggle | Only things a user would actually change |

---

## What's Cut (vs. current goals.md)

| Feature | Status | Reason |
|---------|--------|--------|
| Now Playing full-screen tab | Cut | Bottom bar is sufficient |
| Queue tab / TUI queue management | Cut | Use `sonos queue` CLI commands |
| Speaker Detail screen | Cut | Volume is on the list; EQ uses CLI |
| Startup/Discovery animation | Cut | Just show the main view when ready |
| Group card grid layout | Cut | Replaced by flat list with group headers |
| Breadcrumb navigation | Cut | Only one level of navigation now |
| Hero album art (20×20) | Cut | Only keep 3×3 thumbnail in bottom bar |
| Queue track thumbnails (1×1) | Cut | No queue view |
| Marquee scrolling in list | Cut | Ticker scrolling only in bottom bar for long track/artist text |
| Pulsing indicator animation | Cut | Play/pause icon is enough |
| Mini-player on home screen | Cut | Replaced by persistent bottom bar |

### What's Kept

- 4 color themes (dark, light, neon, sonos)
- Speaker regrouping via pick-up/drop (already implemented)
- Per-speaker and per-group volume control
- Playback controls (play/pause, next, prev) in bottom bar
- Progress bar with real-time animation
- Album art thumbnail (3×3 in bottom bar)
- Ticker scrolling for long track/artist text in bottom bar
- Key legend bar
- SDK property watching + hooks system

---

## Existing Code Reuse

Most of the simplified TUI can be built from existing widgets:

| Component | Existing code | Changes needed |
|-----------|--------------|----------------|
| Speaker list | `widgets/speaker_list.rs` | Enhance group headers to 2 lines with track info; add drop zone mode (collapse speakers into dashed-border zones, solid accent on active) |
| Volume bar | `widgets/volume_bar.rs` | Already works, no changes |
| Progress bar | `widgets/progress_bar.rs` | Already works, use in bottom bar |
| Album art | `widgets/album_art.rs` + `image_loader.rs` | Already supports 3×3, use in bottom bar only |
| Theme | `theme.rs` | Already has 4 themes, no changes |
| Hooks | `hooks.rs` | Already has use_state, use_watch, use_animation |
| App state | `app.rs` | Simplify Screen enum: remove GroupView, SpeakerDetail |
| Event loop | `event.rs` | Simplify — fewer screen states to handle |

Code to remove:
- `screens/home_groups.rs` (group card grid)
- `screens/now_playing.rs` (full now playing screen)
- `widgets/group_card.rs` (individual group cards)
- GroupView and SpeakerDetail screen variants from `app.rs`
- Group-level handlers from `handlers/group.rs` (replace with drop zone + bottom bar handlers)

---

## Open Questions

*None — all questions resolved during brainstorming.*
