# Project Goals

This document captures the vision, design decisions, and v1 scope for `sonos-cli`. It is synthesized from both brainstorm sessions and is the authoritative reference for **what** we're building.

For **how** to build it, see `CLAUDE.md` and `docs/references/`.

---

## What We're Building

A single Rust binary named `sonos` that controls Sonos speakers from the terminal. It has two modes:

**Interactive TUI (default)**
Running `sonos` with no arguments launches a full-screen terminal interface built with `ratatui`. It feels alive — real-time progress bars, reactive state updates, album art, animated transitions. Equal parts dashboard and controller.

**One-off commands**
Running `sonos <subcommand>` executes a single action and exits. Instant after the first run (discovery is cached). Designed for scripts, keyboard shortcuts, and quick adjustments without opening the TUI.

Both modes dispatch through the same `Action` enum, so they stay in sync automatically.

---

## Design Philosophy

**Groups-first.** Sonos groups are the natural unit of playback. A "Living Room" group might be a Beam + two surrounds playing as one. Single speakers appear as single-member groups. The TUI home screen and CLI defaults both operate on groups, not individual speakers.

**Keyboard-only TUI.** No mouse support. Pure terminal experience — fast, predictable, works over SSH.

**Flat CLI commands.** `sonos play`, not `sonos playback play`. One level of subcommands. Simple and fast to type.

**clig.dev compliance.** Every command follows the [CLI guidelines](references/cli-guidelines.md): flags over positional args, errors to stderr, meaningful exit codes, no surprises.

**Sync SDK.** The underlying `sonos-sdk` is synchronous — no `async`/`await`. The TUI event loop uses non-blocking `try_iter()` on the change iterator each frame tick.

---

## TUI: Screen Architecture

```
sonos (launch)
  └── Startup / Discovery screen
        └── Home screen
              ├── [Groups tab]    ← default
              │     └── Group view (Enter on a card)
              │           ├── [Now Playing tab]   ← default
              │           ├── [Speakers tab]
              │           └── [Queue tab]
              └── [Speakers tab]
                    └── Speaker Detail screen (Enter on a speaker)
```

Navigation model:
- `←→` switches tabs at any level
- `↑↓` navigates within the current view
- `Enter` drills in (open group, open speaker)
- `Esc` goes back one level
- Media keys (play/pause/next/prev) work **globally** regardless of active screen

### Home — Groups Tab (landing screen)

Responsive grid of group cards. The selected card has a bold border and `●`. Cards show: group name, playback state icon, current track + artist, volume bar, progress bar, speaker count.

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  ♪  S O N O S                                          [▸Groups]      Speakers  │
│─────────────────────────────────────────────────────────────────────────────────│
│                                                                                 │
│  ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓  ┌─────────────────────────────────┐      │
│  ┃ ● Living Room           ▶ Playing┃  │   Kitchen              ⏸ Paused │      │
│  ┃ Bohemian Rhapsody — Queen        ┃  │ Hotel California — Eagles        │      │
│  ┃ ██████████████████░░░░░░ 80%     ┃  │ ██████████████░░░░░░░░░░ 50%     │      │
│  ┃ ━━━━━━━━━━━╺────────── 2:31/5:55┃  │ ━━━━━━━━━━━━━━━━━╺── 4:12/6:30  │      │
│  ┃ 🔊 Beam + 2 surrounds           ┃  │ 🔊 Sonos One                     │      │
│  ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛  └─────────────────────────────────┘      │
│                                                                                 │
│─────────────────────────────────────────────────────────────────────────────────│
│ ▓▓▓ Living Room  ▶ Bohemian Rhapsody — Queen   ━━━━╺──── 2:31/5:55   🔊 80%  │
│─────────────────────────────────────────────────────────────────────────────────│
│ ←→ Tabs   ↑↓ Select   ⏎ Open group   ? Help   ⎋ Quit                          │
└─────────────────────────────────────────────────────────────────────────────────┘
```

The **mini-player** at the bottom tracks the focused group card. It shows: group name, current track, progress, volume. It is only visible on the Home screen — it disappears when you enter a group view.

### Home — Speakers Tab

System-wide speaker management. Speakers organized by group with headers. Ungrouped speakers listed under "NOT IN A GROUP". Actions: `n` create new group, `Enter` move to group, `d` ungroup.

### Group View — Now Playing Tab

Album art hero on the left, track metadata on the right. Group volume (`↑↓`). Playback controls + progress bar centered below.

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  ♪  S O N O S  ›  Living Room                 [▸Now Playing]  Speakers   Queue  │
│─────────────────────────────────────────────────────────────────────────────────│
│    ┌──────────────────────┐                                                     │
│    │                      │     Bohemian Rhapsody                               │
│    │      A L B U M       │     Queen                                           │
│    │       A R T          │     A Night at the Opera (1975)                     │
│    │   (sixel/halfblock/  │                                                     │
│    │      ascii art)      │     🔊  ██████████████████░░░░░░  80%              │
│    └──────────────────────┘     🔊×3  Beam + One SL × 2                        │
│                                                                                 │
│                          ⏮     ▶     ⏭                                         │
│              ━━━━━━━━━━━━━━━━━━━╺──────────────────────                         │
│              2:31                                 5:55                           │
│─────────────────────────────────────────────────────────────────────────────────│
│ ←→ Tabs   ↑↓ Volume   ⏮ Prev   ␣ Pause   ⏭ Next   ⎋ Back to overview        │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Group View — Speakers Tab

Per-speaker EQ (volume, bass, treble, loudness, mute) for each member of the group. Available speakers listed below with `○` — Enter to add, `d` to remove.

### Group View — Queue Tab

Track list with 1×1 pixel album art block per track. Track count + total duration at top. `Enter` to jump to a track, `d` to remove, scrollable.

### Startup / Discovery Screen

Shown briefly on launch. Centered logo with spinner. Speakers appear one by one as discovered. Press Enter to skip ahead, Esc to quit.

### Speaker Detail Screen

Accessible from any Speakers tab. ASCII art device rendering + product info (model, IP, firmware). Audio controls: volume, bass, treble, loudness, mute.

---

## TUI: Visual Design

**Album art rendering** — user-configurable, terminal capability detected at startup:
- **Sixel/Kitty graphics** — Full-color images (Kitty, iTerm2, WezTerm)
- **Half-block pixel art** — Unicode `▀▄` with truecolor. Broadly compatible.
- **ASCII art** — Braille/character-based. Universal fallback.

Album art appears at three sizes:
| Location | Size | Purpose |
|----------|------|---------|
| Now Playing tab | ~20×20 chars | Hero display |
| Mini-player | 3×3 chars | Thumbnail accent |
| Queue track list | 1×1 char | Colored block per track |

**Theming** — four built-in themes, selectable in config:
- `dark` (default) — dark background, light text
- `light` — light background, dark text
- `neon` — cyberpunk accent colors
- `sonos` — Sonos brand colors (black/white/orange)

**Motion** — the UI feels alive:
- Progress bars tick in real-time every second
- Volume sliders animate to new values
- Marquee scrolling for long track titles
- Pulsing `●` indicator on the active group card
- Reactive updates via SDK watch channels — changes from the Sonos app or physical buttons appear instantly

**Breadcrumb navigation** — path always shown: `SONOS › Living Room › Beam`

**Context-sensitive key legend** — bottom bar updates per screen:

| Screen | Legend |
|--------|--------|
| Home > Groups | `←→ Tabs  ↑↓ Select  ⏎ Open group  ? Help  ⎋ Quit` |
| Home > Speakers | `←→ Tabs  ↑↓ Navigate  ⏎ Open speaker  n New group  d Ungroup  ⎋ Quit` |
| Group > Now Playing | `←→ Tabs  ↑↓ Volume  ⏮ Prev  ␣ Pause  ⏭ Next  ⎋ Back` |
| Group > Speakers | `←→ Tabs / Adjust  ↑↓ Navigate  ⏎ Open speaker  ⎋ Back` |
| Group > Queue | `←→ Tabs  ↑↓ Select  ⏎ Play track  d Remove  ⎋ Back` |
| Speaker Detail | `↑↓ Navigate  ←→ Adjust  ⎋ Back to speakers` |

---

## CLI: Command Reference

All one-off commands are flat subcommands. `sonos` with no args = TUI.

### Targeting

Every command that acts on a speaker or group accepts:
```
--speaker NAME    target a speaker by friendly name
--group NAME      target a group by name (wins over --speaker if both given)
```
If neither is given, the default group is used.

### Discovery & System

| Command | Description |
|---------|-------------|
| `sonos speakers` | List all speakers with state and volume |
| `sonos groups` | List all groups with state and volume |
| `sonos status` | Current track, playback state, volume for the default/targeted group |

### Playback

All accept `[--speaker NAME \| --group NAME]`.

| Command | Description |
|---------|-------------|
| `sonos play` | Resume playback |
| `sonos pause` | Pause playback |
| `sonos stop` | Stop playback |
| `sonos next` | Skip to next track |
| `sonos prev` | Previous track |
| `sonos seek <HH:MM:SS>` | Seek to position |
| `sonos mode <normal\|repeat\|repeat-one\|shuffle\|shuffle-no-repeat>` | Set play mode |

### Volume & EQ

| Command | Scope |
|---------|-------|
| `sonos volume <0-100>` | Speaker or group |
| `sonos mute` | Speaker or group |
| `sonos unmute` | Speaker or group |
| `sonos bass <-10..10> --speaker NAME` | Speaker only |
| `sonos treble <-10..10> --speaker NAME` | Speaker only |
| `sonos loudness <on\|off> --speaker NAME` | Speaker only |

### Queue

| Command | Description |
|---------|-------------|
| `sonos queue` | Show current queue |
| `sonos queue add <URI>` | Add URI to queue |
| `sonos queue clear` | Clear entire queue |

### Grouping

| Command | Description |
|---------|-------------|
| `sonos join --speaker NAME --group NAME` | Add speaker to group |
| `sonos leave --speaker NAME` | Remove speaker from group (standalone) |

### Sleep Timer

| Command | Description |
|---------|-------------|
| `sonos sleep <DURATION>` | Set sleep timer (e.g. `30m`, `1h`) |
| `sonos sleep cancel` | Cancel active sleep timer |

---

## Architecture Decisions

| Decision | What was chosen | Why |
|----------|----------------|-----|
| **Shared dispatch** | `Action` enum + `executor.rs` shared by CLI and TUI | Prevents logic drift; single place to maintain SDK calls |
| **Default mode** | No args = TUI | TUI is the primary experience; commands are the power-user shortcut |
| **Command structure** | Flat subcommands | Easy to type; no cognitive overhead of domain grouping |
| **Targeting** | `--group` / `--speaker` flags; group wins if both | clig.dev: flags over positional args |
| **Discovery** | Cached to `~/.config/sonos/cache.json`; 24h TTL; auto-rediscover on miss | 3s SSDP scan on every command would be terrible UX |
| **Output** | Plain text, no JSON in v1 | YAGNI; `--json` easy to add later |
| **SDK threading** | Sync; `try_iter()` in TUI loop | SDK has no async surface; non-blocking polling keeps TUI responsive |

---

## v1 Scope

**In scope:**
- Full TUI as described in this document
- All one-off commands in the CLI reference above
- Discovery cache with TTL
- Four color themes
- Three album art rendering modes (Sixel/Kitty, half-block, ASCII)
- Config file (`~/.config/sonos/config.toml`)

**Explicitly out of scope for v1:**
- Music library browsing / search (SDK doesn't expose content directory yet)
- `--json` output flag
- `sonos tui --group NAME` deep-link
- Alarm management (only snooze + query currently available in SDK)
- Mouse support in TUI

---

## Open Questions (config defaults)

These will be finalized during implementation:

- What is the fallback `default_group` when nothing is configured? First discovered group by name alphabetically?
- Should `cache_ttl_hours = 24` be the shipped default, or shorter (e.g., 6h)?
- What config values should the TUI theme picker expose vs. what's config-file only?
