---
date: 2026-03-01
status: active
version: 1.0
---

# Product Requirements Document — sonos-cli

## Vision

**sonos-cli** is the best way to control Sonos from a computer. It gives any Sonos user — not just developers — fast, keyboard-driven control over their speakers through both a beautiful interactive TUI and a full set of one-off CLI commands. The terminal should feel like a first-class Sonos client.

---

## Problem Statement

The official Sonos app requires picking up your phone, navigating a GUI, and context-switching away from your work. For anyone at a computer — developers, remote workers, power-users — this friction adds up dozens of times a day.

There is no good terminal-native Sonos client today. Existing unofficial tools are partial, unmaintained, or require technical setup beyond most users. sonos-cli fills that gap: a polished, complete, easy-to-install terminal client that works for anyone.

---

## Target Users

**Primary:** Any Sonos owner who uses a terminal regularly. This is broader than "developers" — it includes writers, designers, sysadmins, students, and anyone who works at a computer and has Sonos in their home.

**Their context:**
- Multiple speakers across their home (living room, kitchen, office, bedroom)
- Use the Sonos app daily but find it slow to reach for
- Comfortable in a terminal but not necessarily a programmer
- Want control without leaving their keyboard

**Success for them looks like:** Opening the TUI once to get oriented, then using CLI commands from muscle memory for the rest of the day. Or keeping the TUI open in a tmux pane as a persistent dashboard.

---

## Core Principles

1. **Complete over partial.** If the Sonos SDK supports it, sonos-cli supports it. No arbitrary gaps.
2. **Fast over feature-rich.** Common operations (play, pause, volume) should be instant. Discovery is cached.
3. **Delightful TUI.** The interactive mode should feel alive — reactive updates, smooth animations, real album art. Something worth showing off.
4. **Predictable CLI.** Commands follow [clig.dev](https://clig.dev) conventions. Every flag, every exit code, every error message is consistent and human-readable.
5. **Welcoming to newcomers.** First run should work with zero configuration. Discovery is automatic. Help text is clear.

---

## Feature Requirements

### CLI Mode

One-off commands executed and exited. Designed for scripts, keyboard shortcuts, and quick adjustments.

**Guiding rules (from `docs/references/cli-guidelines.md`):**
- Flat subcommands: `sonos <verb>`
- Flags over positional args: `--group "Kitchen"` not `sonos play Kitchen`
- `--group` wins over `--speaker` if both given
- Errors to stderr; clean stdout for scripting
- Exit codes: 0 = success, 1 = runtime error, 2 = usage error

**Full command surface (v1):**

| Category | Commands |
|----------|----------|
| Discovery | `discover`, `speakers`, `groups`, `status` |
| Playback | `play`, `pause`, `stop`, `next`, `prev`, `seek`, `mode` |
| Volume & EQ | `volume`, `mute`, `unmute`, `bass`, `treble`, `loudness` |
| Queue | `queue`, `queue add`, `queue clear` |
| Grouping | `join`, `leave` |
| Sleep timer | `sleep <duration>`, `sleep cancel` |

**Targeting:** Every command accepts `--speaker NAME` and/or `--group NAME`. Default target is the configured default group (falls back to first discovered group).

**Discovery cache:** Results cached to `~/.config/sonos/cache.json` with 24h TTL. Auto-rediscovers on miss. `sonos discover` for manual refresh.

---

### TUI Mode

Launched when `sonos` is run with no arguments. Full-screen terminal interface built with `ratatui` + `crossterm`.

#### Design Goals

- **Reactive by default.** Every value on screen reflects the current state. Volume changes from the Sonos app appear instantly. Playing a track from Spotify shows up immediately. The TUI is always live.
- **Groups-first.** The home screen shows groups, not individual speakers. Sonos groups are the natural unit of playback.
- **Keyboard-only.** No mouse. Every action has a key. The key legend at the bottom updates for every screen.
- **Fun and alive.** Progress bars tick in real-time. Album art renders in the terminal. Transitions animate. This should feel different from a dry command-line tool.

#### Reactivity Model

The TUI uses `system.iter()` from `sonos-sdk` as its state update source. On each event loop tick:

1. Poll `try_iter()` (non-blocking) to drain any pending change events
2. For each `ChangeEvent`, update the relevant component state
3. Re-render only changed components

Properties are watched selectively — only the handles visible on the current screen are watched. Navigating to a new screen watches new handles and unwatches stale ones. This keeps subscriptions lean.

#### Screen Map

```
Launch → Startup / Discovery screen
  └── Home screen
        ├── Groups tab (default)
        │     └── Group view (Enter)
        │           ├── Now Playing tab (default)
        │           ├── Speakers tab
        │           └── Queue tab
        └── Speakers tab
              └── Speaker Detail (Enter)
```

**Home — Groups tab:** Responsive grid of group cards. Each card: group name, playback state, current track + artist, volume bar, animated progress bar, speaker count. Selected card has bold border. Mini-player at bottom tracks focused card.

**Home — Speakers tab:** All speakers organized by group. System-wide group management: create, dissolve, move speakers.

**Group — Now Playing:** Album art hero (left), track metadata + group volume (right), playback controls + progress bar.

**Group — Speakers:** Per-speaker EQ (volume, bass, treble, loudness, mute). Add/remove members. Same visual component as Home > Speakers, different scope.

**Group — Queue:** Track list with 1×1 album art blocks. Jump to track, remove, scrollable.

**Speaker Detail:** ASCII art device rendering, product info (model, IP, firmware), audio controls.

#### Visual Design

**Album art rendering** — detected at startup, user-configurable:
- Sixel/Kitty graphics (iTerm2, Kitty, WezTerm)
- Half-block pixel art (▀▄, truecolor, broadly compatible)
- ASCII art (universal fallback)

**Themes** — four built-in, set in `~/.config/sonos/config.toml`:
- `dark` (default)
- `light`
- `neon` (cyberpunk)
- `sonos` (black/white/orange)

**Motion:**
- Progress bars tick every second
- Volume sliders animate to new values
- Marquee scrolling for long track titles
- Pulsing `●` on the active group card
- Reactive state updates propagate instantly via SDK watch channels

---

### Configuration

File: `~/.config/sonos/config.toml`

```toml
default_group = "Living Room"   # default target when no --group / --speaker given
cache_ttl_hours = 24            # discovery cache TTL
theme = "dark"                  # TUI color theme
```

Environment variable overrides: `SONOS_DEFAULT_GROUP`, `SONOS_CONFIG_DIR`, `NO_COLOR`.

---

### Onboarding

First run should work with zero configuration:

1. `sonos` — runs SSDP discovery automatically, shows startup screen with discovered speakers
2. User sees the TUI with their actual speakers
3. Can start controlling immediately

No config file required. No manual IP entry. No API keys.

---

## Non-Functional Requirements

| Requirement | Target |
|-------------|--------|
| CLI command latency (cache hit) | < 500ms from invocation to output |
| TUI startup time | < 4s (includes 3s SSDP discovery on first run) |
| TUI frame rate | 10fps minimum, 30fps target |
| State update latency | < 1s from SDK event to screen update |
| Platforms | macOS, Linux (primary); Windows (best effort) |
| Terminal compatibility | Works in iTerm2, Kitty, WezTerm, Alacritty, Terminal.app, tmux |

---

## Out of Scope — v1

| Feature | Reason |
|---------|--------|
| Music library browsing / search | SDK doesn't expose content directory yet |
| Alarm CRUD | SDK only exposes snooze + query |
| `--json` output flag | YAGNI; easy to add in v2 |
| `sonos tui --group NAME` deep-link | Nice-to-have, defer |
| Mouse support in TUI | Keyboard-only is intentional |
| Windows native support | Crossterm works, but untested — best effort |

---

## References

- Technical architecture: `docs/brainstorm/2026-03-01-sonos-cli-architecture-brainstorm.md`
- TUI design: `docs/brainstorm/2026-02-26-sonos-tui-brainstorm.md`
- SDK API: `docs/references/sonos-sdk.md`
- CLI conventions: `docs/references/cli-guidelines.md`
- Roadmap: `docs/product/roadmap.md`
