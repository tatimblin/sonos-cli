# Sonos TUI Brainstorm

**Date:** 2026-02-26
**Status:** Draft
**Author:** Tristan Timblin + Claude

## What We're Building

A terminal-based Sonos controller and dashboard built on top of the sonos-sdk. It should feel alive, interactive, and beautiful — inspired by the rich interactive examples on [ratatui.rs](https://ratatui.rs/). Equal parts dashboard (always seeing what's playing) and controller (actively managing playback, volume, groups).

### Core Concept: Tabbed Views + Persistent Mini-Player

The TUI has two navigation levels, both tabbed. The **home screen** has Groups and Speakers tabs. Pressing Enter on a group opens a **group view** with Now Playing, Speakers, and Queue tabs. A **persistent mini-player** on the home screen shows the highlighted group's now-playing info with 3×3 pixel album art.

## Key Decisions

### 1. Navigation Model: Groups-First Overview + Detail

The home screen lists **groups** (not individual speakers). Sonos groups are the natural unit of playback — a "Living Room" group might contain a soundbar + two surrounds. Single speakers appear as single-member groups.

- **Overview screen**: Grid of medium-density group cards
- **Detail screen**: Drill into a group for full controls
- **Navigation**: Arrow keys to select, Enter to drill in, Esc to go back

### 2. Screen Architecture

```
┌──────────────────────────────────────────┐
│                                          │
│   CONTENT AREA                           │
│   (Overview grid OR Group detail)        │
│                                          │
│──────────────────────────────────────────│
│ Living Room  ▶ Bohemian Rhap..  2:31  80%│  ← MINI-PLAYER
│              ━━━━━╺──────   ⏮ ⏯ ⏭      │    (tracks focused group)
└──────────────────────────────────────────┘
```

**Mini-player behavior:** Visible on the overview screen only. Shows the currently highlighted group (group name displayed for clarity). Updates as you arrow between group cards. Hides when you enter the tabbed group view (Enter), since the group view has its own full controls.

**Two levels, both tabbed:**

**Home Tabs** (←→ to switch):
- **Groups** — Grid of group cards showing state at a glance (the overview)
- **Speakers** — System-wide speaker management, create/delete groups, see all speakers

**Group View Tabs** (Enter on a group card, ←→ to switch):
- **Now Playing** — Album art hero, track info, group volume, playback controls
- **Speakers** — This group's speakers with per-speaker EQ, add/remove members
- **Queue** — Track list with play, remove, reorder

The **Speakers tab** is a reusable component at both levels — same visual pattern, different scope:

| Context | Scope | Actions |
|---------|-------|---------|
| Home > Speakers | All speakers on the network | Create groups, assign speakers to groups, see group assignments |
| Group > Speakers | This group's members + available speakers | Per-speaker EQ (vol/bass/treble/loudness), add/remove from group |

### 3. Group Card Design (Overview)

Medium density — enough info to scan quickly without overwhelming:

```
┌───────────────────────────────┐
│ Living Room            ▶ Playing│
│ Bohemian Rhapsody - Queen     │
│ Vol: ████████░░ 80%           │
│ ━━━━━━━╺──── 2:31/5:55      │
└───────────────────────────────┘
```

Cards show: group name, playback state icon, track + artist, volume bar, progress bar with timestamps. Cards arranged in a responsive grid that adapts to terminal width.

### 4. Album Art Rendering

User-configurable rendering mode via settings:

- **Sixel/Kitty graphics** — Full-color images for modern terminals (Kitty, iTerm2, WezTerm)
- **Half-block pixel art** — Unicode half-blocks (▀▄) with truecolor. Works broadly, looks retro-cool.
- **ASCII art** — Braille/character-based conversion. Universal compatibility.

The TUI detects terminal capabilities on startup and suggests the best mode, but users can override in settings.

**Album art appears at multiple sizes:**

| Location | Size | Purpose |
|----------|------|---------|
| Now Playing tab | Large (~20×20 chars) | Hero display, full detail |
| Mini-player | 3×3 chars | Tiny pixel thumbnail, visual accent |
| Queue track list | 1×1 char | Single colored block per track, visual texture |

The smaller sizes are intentionally pixelated — they add color and visual interest without needing detail.

### 5. Controls & Input

- **Media keys** — Keyboard media keys (play/pause, next, prev) work globally for the focused group, regardless of which screen or tab is active
- **Key legend** — Always visible at the bottom of the screen, updates based on the current screen/tab context
- **Visual icons** — ⏮ ⏯ ⏭ displayed above the progress bar in the Now Playing tab as visual indicators
- **←→** — Switch tabs when no setting is focused; adjust setting value (volume, bass, treble) when a slider is focused. Focus determines behavior — tab-level focus switches tabs, setting-level focus adjusts values.
- **↑↓** — Navigate between groups on overview; navigate within tabs (settings, tracks, speakers)
- **Enter** — Open a group from overview; context-sensitive action within tabs (play track, toggle membership)
- **Esc** — Go back from group view to overview; quit from overview

### 6. Motion & Aliveness

The UI should feel alive through multiple layers of motion:

- **Track progress bar** — Smooth real-time ticking every second with elapsed/remaining time
- **State transitions** — Animations when values change (volume slider glides to new value, play/pause icon transitions)
- **Ambient motion** — Scrolling marquee for long track titles, pulsing now-playing indicator on the active group card, breathing/subtle color shifts
- **Real-time updates** — All values update reactively via sonos-state watch channels. Volume changes from the physical speaker or Sonos app appear instantly.

### 7. Theming

User-configurable color themes. Ship with multiple built-in options:

- Dark (default)
- Light
- Neon/cyberpunk
- Sonos-branded (black/white/orange)

Theme selection in settings. Themes define: background, foreground, accent color, progress bar colors, card borders, etc.

## Detailed Screen Designs

### Screen 1: Group Overview (Home)

The landing screen. Home has two tabs: Groups and Speakers. ←→ switches tabs.

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  ♪  S O N O S                                          [▸Groups]      Speakers  │
│─────────────────────────────────────────────────────────────────────────────────│
│                                                                                 │
│  ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓  ┌─────────────────────────────────┐      │
│  ┃ ● Living Room           ▶ Playing┃  │   Kitchen              ⏸ Paused │      │
│  ┃                                  ┃  │                                  │      │
│  ┃ Bohemian Rhapsody — Queen        ┃  │ Hotel California — Eagles        │      │
│  ┃                                  ┃  │                                  │      │
│  ┃ ██████████████████░░░░░░ 80%     ┃  │ ██████████████░░░░░░░░░░ 50%     │      │
│  ┃ ━━━━━━━━━━━╺────────── 2:31/5:55┃  │ ━━━━━━━━━━━━━━━━━╺── 4:12/6:30  │      │
│  ┃ 🔊 Beam + 2 surrounds           ┃  │ 🔊 Sonos One                     │      │
│  ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛  └─────────────────────────────────┘      │
│                                                                                 │
│  ┌─────────────────────────────────┐  ┌─────────────────────────────────┐      │
│  │   Bedroom               ■ Stopped│  │   Office               ▶ Playing│      │
│  │                                  │  │                                  │      │
│  │ Nothing playing                  │  │ Lo-Fi Beats — ChilledCow         │      │
│  │                                  │  │                                  │      │
│  │ ██████░░░░░░░░░░░░░░░░░░ 25%     │  │ ████████████████████░░░░ 65%     │      │
│  │ ──────────────────────── --/--   │  │ ━╺──────────────────── 0:42/3:15│      │
│  │ 🔊 Sonos One                     │  │ 🔊 Sonos Move                    │      │
│  └─────────────────────────────────┘  └─────────────────────────────────┘      │
│                                                                                 │
│─────────────────────────────────────────────────────────────────────────────────│
│ ▓▓▓ Living Room  ▶ Bohemian Rhapsody — Queen   ━━━━╺──── 2:31/5:55   🔊 80%  │
│ ▓▓▓                                                                            │
│ ▓▓▓                                                                            │
│─────────────────────────────────────────────────────────────────────────────────│
│ ←→ Tabs   ↑↓ Select   ⏎ Open group   ? Help   ⎋ Quit                          │
└─────────────────────────────────────────────────────────────────────────────────┘
```

**Design notes:**
- The selected group card has a **bold/double border** (┏━┓) and a `●` indicator
- Unselected cards have thin borders (┌─┐), dimmed
- The mini-player shows the **focused group** — 3×3 pixel album art on the left, group name + track info
- The tiny album art is just a splash of color — intentionally pixelated, adds visual warmth
- As you arrow between cards, the mini-player updates to match the highlighted group
- Speaker count shown at bottom of each card (e.g., "Beam + 2 surrounds")
- Progress bars animate in real-time for playing groups
- Cards reflow into 1 column on narrow terminals

---

### Screen 1b: Home > Speakers Tab (←→ from Groups)

System-wide speaker management. Same visual structure as the group-level Speakers tab but scoped to the entire network.

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  ♪  S O N O S                                           Groups     [▸Speakers]  │
│─────────────────────────────────────────────────────────────────────────────────│
│                                                                                 │
│   Living Room ──────────────────────────────────────────────────                │
│   ▸ Beam (coordinator)           ██████████████████░░░░░░ 80%                  │
│     One SL — Left surround       ██████████████████░░░░░░ 80%                  │
│     One SL — Right surround      ██████████████████░░░░░░ 80%                  │
│                                                                                 │
│   Kitchen ──────────────────────────────────────────────────────                │
│     Sonos One                    ██████████████░░░░░░░░░░ 50%                  │
│                                                                                 │
│   Bedroom ──────────────────────────────────────────────────────                │
│     Sonos One                    ██████░░░░░░░░░░░░░░░░░░ 25%                  │
│                                                                                 │
│   NOT IN A GROUP ───────────────────────────────────────────────                │
│     Office — Sonos Move          ████████████████████░░░░ 65%                  │
│                                                                                 │
│                                                                                 │
│                                                                                 │
│─────────────────────────────────────────────────────────────────────────────────│
│ ▓▓▓ Living Room  ▶ Bohemian Rhapsody — Queen   ━━━━╺──── 2:31/5:55   🔊 80%  │
│ ▓▓▓                                                                            │
│ ▓▓▓                                                                            │
│─────────────────────────────────────────────────────────────────────────────────│
│ ←→ Tabs   ↑↓ Navigate   n New group   ⏎ Move to group   d Ungroup   ⎋ Quit    │
└─────────────────────────────────────────────────────────────────────────────────┘
```

**Design notes:**
- Speakers organized by group, with group headers
- Ungrouped speakers listed under "NOT IN A GROUP"
- `▸` marks the focused speaker
- **n** creates a new group from the selected speaker(s)
- **Enter** moves the focused speaker into a different group (could open a small picker)
- **d** removes the focused speaker from its group (ungroups it)
- Same volume bars as the group-level Speakers tab — reused component
- Mini-player still visible at bottom (shows the focused group based on which speaker is highlighted)

---

### Screen 2: Group View — Tabbed Interface (press ⏎ on a group card)

When you open a group, you enter a **tabbed view** with three tabs. ←→ switches between tabs. The mini-player hides since the group view has its own controls.

---

#### Tab 1: Now Playing

The album art hero view. Group-level playback and volume controls.

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  ♪  S O N O S  ›  Living Room                 [▸Now Playing]  Speakers   Queue  │
│─────────────────────────────────────────────────────────────────────────────────│
│                                                                                 │
│                                                                                 │
│    ┌──────────────────────┐                                                     │
│    │                      │     Bohemian Rhapsody                               │
│    │                      │     Queen                                           │
│    │      A L B U M       │     A Night at the Opera (1975)                     │
│    │                      │                                                     │
│    │       A R T          │                                                     │
│    │                      │     🔊  ██████████████████░░░░░░  80%              │
│    │      (rendered       │                                                     │
│    │       via sixel,     │     🔊×3  Beam + One SL × 2                         │
│    │       halfblock,     │                                                     │
│    │       or ascii)      │                                                     │
│    │                      │                                                     │
│    └──────────────────────┘                                                     │
│                                                                                 │
│                          ⏮     ▶     ⏭                                         │
│              ━━━━━━━━━━━━━━━━━━━╺──────────────────────                         │
│              2:31                                 5:55                           │
│                                                                                 │
│─────────────────────────────────────────────────────────────────────────────────│
│ ←→ Tabs   ↑↓ Volume   ⏮ Prev   ␣ Pause   ⏭ Next   ⎋ Back to overview        │
└─────────────────────────────────────────────────────────────────────────────────┘
```

**Design notes:**
- Tabs right-aligned on the same line as the breadcrumb — active tab is highlighted with `[▸ ]`
- Album art large on the left, track metadata on the right
- Group volume (↑↓ to adjust)
- Speaker count shown as info line
- Playback icons + progress bar centered below
- ←→ switches to Speakers or Queue tab
- Clean and focused on the music

---

#### Tab 2: Speakers

Per-speaker settings and group membership management.

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  ♪  S O N O S  ›  Living Room                  Now Playing  [▸Speakers]   Queue  │
│─────────────────────────────────────────────────────────────────────────────────│
│                                                                                 │
│   IN GROUP                                                                      │
│                                                                                 │
│ ▸ Beam (coordinator)                                                            │
│     Volume   ██████████████████░░░░░░  80%                                     │
│     Bass     ███████████████░░░░░░░░░  +2                                      │
│     Treble   ████████████████░░░░░░░░  +4                                      │
│     Loudness [ON]          Mute [OFF]                                          │
│                                                                                 │
│   One SL — Left surround                                                        │
│     Volume   ██████████████████░░░░░░  80%                                     │
│     Bass     ██████████░░░░░░░░░░░░░░   0                                      │
│     Treble   ██████████░░░░░░░░░░░░░░   0                                      │
│     Loudness [OFF]         Mute [OFF]                                          │
│                                                                                 │
│   One SL — Right surround                                                       │
│     Volume   ██████████████████░░░░░░  80%                                     │
│     Bass     ██████████░░░░░░░░░░░░░░   0                                      │
│     Treble   ██████████░░░░░░░░░░░░░░   0                                      │
│     Loudness [OFF]         Mute [OFF]                                          │
│                                                                                 │
│   AVAILABLE ─────────────────────────────────────                               │
│   ○ Kitchen — Sonos One                                                         │
│   ○ Bedroom — Sonos One                                                         │
│   ○ Office — Sonos Move                                                         │
│                                                                                 │
│─────────────────────────────────────────────────────────────────────────────────│
│ ←→ Tabs / Adjust   ↑↓ Navigate   ⏎ Toggle membership   ⎋ Back to overview     │
└─────────────────────────────────────────────────────────────────────────────────┘
```

**Design notes:**
- Each speaker in the group shown with full EQ controls (volume, bass, treble, loudness, mute)
- `▸` marks the focused speaker — ↑↓ moves between speakers and their settings
- ←→ adjusts the focused setting when a slider is highlighted; switches tabs when no setting is focused
- Available (ungrouped) speakers listed below with `○` — Enter to add to group
- Grouped speakers can be removed with a key (e.g., `d`)
- Scrollable for groups with many speakers

---

#### Tab 3: Queue

Track queue management.

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  ♪  S O N O S  ›  Living Room                  Now Playing   Speakers  [▸Queue]  │
│─────────────────────────────────────────────────────────────────────────────────│
│                                                                                 │
│   6 tracks · 38:59 total                                                        │
│                                                                                 │
│   ▶ 1  ▓  Bohemian Rhapsody          Queen                  5:55               │
│     2  ▓  We Will Rock You            Queen                  2:02               │
│     3  ▓  Hotel California            Eagles                 6:30               │
│     4  ▓  Stairway to Heaven          Led Zeppelin           8:02               │
│     5  ▓  Comfortably Numb            Pink Floyd             6:22               │
│     6  ▓  Free Bird                   Lynyrd Skynyrd        10:08               │
│                                                                                 │
│                                                                                 │
│                                                                                 │
│                                                                                 │
│                                                                                 │
│                                                                                 │
│                                                                                 │
│                                                                                 │
│                                                                                 │
│                                                                                 │
│                                                                                 │
│                                                                                 │
│─────────────────────────────────────────────────────────────────────────────────│
│ ←→ Tabs   ↑↓ Select   ⏎ Play track   d Remove   ⎋ Back to overview            │
└─────────────────────────────────────────────────────────────────────────────────┘
```

**Design notes:**
- Currently playing track marked with `▶`
- Track count + total duration at top
- Each track has a **1×1 pixel** album art block (▓) — just a colored square for visual texture
- Four-column layout: number, art pixel, title + artist, duration
- ↑↓ to navigate, Enter to jump to a track, `d` to remove
- Scrollable for long queues
- Full terminal width gives generous space for track info
- Media keys still work for playback control regardless of tab

---

### Startup / Discovery Screen

Shown briefly on launch while discovering speakers.

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                                                                                 │
│                                                                                 │
│                                                                                 │
│                           ♪  S O N O S                                          │
│                                                                                 │
│                      Discovering speakers...                                    │
│                                                                                 │
│                      ◐  Scanning network                                        │
│                                                                                 │
│                      Found:                                                     │
│                        ✓ Living Room (Beam)                                     │
│                        ✓ Kitchen (Sonos One)                                    │
│                        ✓ Bedroom (Sonos One)                                    │
│                        ● Office (Sonos Move)                                    │
│                                                                                 │
│                                                                                 │
│                                                                                 │
│                                                                                 │
│                                                                                 │
│─────────────────────────────────────────────────────────────────────────────────│
│ Discovering speakers on your network... Press ⏎ to continue, ⎋ to quit         │
└─────────────────────────────────────────────────────────────────────────────────┘
```

**Design notes:**
- Centered logo with a spinning indicator
- Speakers appear one by one as they're discovered
- The `●` indicates a speaker still being identified
- Press Enter to skip ahead once enough speakers are found
- Transitions to the overview once discovery completes

---

### Screen 3: Speaker Detail (press ⏎ on a speaker)

Accessible from any Speakers tab (home or group level). Full individual speaker information and controls. Features an ASCII art rendering of the physical device.

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  ♪  S O N O S  ›  Living Room  ›  Beam                                         │
│─────────────────────────────────────────────────────────────────────────────────│
│                                                                                 │
│                                                                                 │
│    ┌─────────────────────────────┐     Sonos Beam (Gen 2)                       │
│    │     ___________________     │     Model: S14                               │
│    │    /                   \    │     IP: 192.168.1.100                         │
│    │   |   ─ ─ ─ ─ ─ ─ ─   |   │     Serial: XX-XX-XX-XX-XX                   │
│    │   |                     |   │     Firmware: 16.3                            │
│    │    \___________________/    │                                               │
│    │                             │     Group: Living Room (coordinator)          │
│    └─────────────────────────────┘                                              │
│                                                                                 │
│─────────────────────────────────────────────────────────────────────────────────│
│                                                                                 │
│   Audio Settings                                                                │
│                                                                                 │
│ ▸ Volume     ██████████████████░░░░░░  80%                                     │
│   Bass       ███████████████░░░░░░░░░  +2                                      │
│   Treble     ████████████████░░░░░░░░  +4                                      │
│   Loudness   [ON]                                                              │
│   Mute       [OFF]                                                             │
│                                                                                 │
│                                                                                 │
│                                                                                 │
│─────────────────────────────────────────────────────────────────────────────────│
│ ↑↓ Navigate   ←→ Adjust   ⎋ Back to speakers                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

**Design notes:**
- **ASCII art device** — A rough rendering of the physical Sonos product (Beam, One, Move, etc.). Could have a few variants for different form factors. Purely decorative but adds personality.
- Product info: model name, IP address, serial number, firmware version
- Group membership shown with coordinator status
- **Audio controls** — Volume, bass, treble, loudness, mute. ↑↓ to navigate, ←→ to adjust.
- `▸` marks the focused setting
- Breadcrumb shows the full path: `SONOS › Living Room › Beam`
- Esc goes back to the Speakers tab
- No mini-player here — deep in settings mode

---

### Key Legend Behavior

The key legend at the bottom is **context-sensitive** — it changes based on the current screen/state:

| Screen / Tab | Legend |
|-------------|--------|
| Home > Groups | `←→ Tabs  ↑↓ Select  ⏎ Open group  ? Help  ⎋ Quit` |
| Home > Speakers | `←→ Tabs  ↑↓ Navigate  ⏎ Open speaker  n New group  d Ungroup  ⎋ Quit` |
| Group > Now Playing | `←→ Tabs  ↑↓ Volume  ⏮ Prev  ␣ Pause  ⏭ Next  ⎋ Back` |
| Group > Speakers | `←→ Tabs / Adjust  ↑↓ Navigate  ⏎ Open speaker  ⎋ Back` |
| Group > Queue | `←→ Tabs  ↑↓ Select  ⏎ Play track  d Remove  ⎋ Back` |
| Speaker Detail | `↑↓ Navigate  ←→ Adjust  ⎋ Back to speakers` |

Media keys (play/pause, next, prev) work **globally** regardless of which screen is active.

## Why This Approach

**Groups-first** matches how Sonos actually works — playback is per-group, not per-speaker. This avoids confusing users about why multiple speakers play the same thing.

**Consistent tabbed pattern** at both levels (home and group) makes navigation predictable. ←→ always switches tabs, ↑↓ always navigates within, Enter always drills in, Esc always goes back.

**Reusable Speakers component** at home level (system-wide management) and group level (per-speaker EQ) reduces cognitive overhead — same visual pattern, different scope.

**User-configurable everything** (art rendering mode, themes) means the TUI works well across terminal environments and personal preferences.

## Data Available from SDK

The sonos-sdk provides everything needed:

| Feature | SDK Source | Update Method |
|---------|-----------|---------------|
| Track info (title, artist, album, art URI) | `CurrentTrack` property | Reactive watch |
| Playback state (playing/paused/stopped) | `PlaybackState` property | Reactive watch |
| Track position & progress | `Position` property | Reactive watch |
| Volume (0-100) | `Volume` property | Reactive watch |
| Mute | `Mute` property | Reactive watch |
| Bass (-10 to +10) | `Bass` property | Reactive watch |
| Treble (-10 to +10) | `Treble` property | Reactive watch |
| Loudness | `Loudness` property | Reactive watch |
| Group membership | `GroupMembership` property | Reactive watch |
| System topology | `Topology` property | Reactive watch |
| Group volume | `GroupVolume` property | Reactive watch |
| Playback control | `Play/Pause/Stop/Next/Prev` operations | Direct API call |
| Volume control | `SetVolume/SetRelativeVolume` operations | Direct API call |
| EQ control | `SetBass/SetTreble/SetLoudness` operations | Direct API call |
| Group management | `AddMember/RemoveMember` operations | Direct API call |
| Device discovery | `sonos_discovery::get()` | One-time scan |

## Resolved Questions

1. **Separate repo** — The TUI will live in its own repository, depending on published sonos-sdk crates. Clean separation between the SDK and its consumers.
2. **Tabbed group view** — Queue and speakers are tabs alongside Now Playing, not overlays or sidebars. ←→ switches tabs. Clean and discoverable.
3. **Keyboard only** — No mouse support. Pure keyboard experience for consistency and a true terminal purist feel.
4. **Mini-player follows focus** — On the overview, the mini-player shows whichever group is highlighted. Group name is displayed for clarity. Hides when inside the tabbed group view (which has its own controls).

## Open Questions

1. **Search/browse** — Should users be able to browse music libraries or search? The SDK has queue management but not music library browsing yet. Likely a v2 feature.
2. **Config file format** — TOML? YAML? Where should it live? (`~/.config/sonos-tui/config.toml`?)
3. **Startup behavior** — Auto-discover speakers on launch? Remember last-used speakers? Show a discovery/loading screen?
