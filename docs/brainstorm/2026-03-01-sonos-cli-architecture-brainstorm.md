---
date: 2026-03-01
topic: sonos-cli-architecture
---

# Sonos CLI Architecture

## What We're Building

A single Rust binary (`sonos`) that controls Sonos speakers via `sonos-sdk`. It has two modes:

1. **One-off commands** — subcommands like `sonos play`, `sonos volume 50 --group "Kitchen"`. Instant after the first run (discovery is cached).
2. **Interactive TUI** — launched when no arguments are given (or via `sonos tui`). Implements the screen design from `2026-02-26-sonos-tui-brainstorm.md` using `ratatui`.

Both modes dispatch through the same `Action` enum, so logic never diverges.

---

## Why This Approach

**Direct SDK calls in both places** was the simpler option but would cause the CLI and TUI to drift — a volume change in one place wouldn't automatically reflect the same validation and optimistic-update logic in the other.

The **shared Action dispatch layer** adds a small upfront cost (defining the `Action` enum) in exchange for a single place to maintain SDK-call logic. Because the SDK is sync and operations are simple (no complex workflows), the executor stays thin.

---

## Key Decisions

- **Default behavior**: `sonos` with no args launches the TUI. One-off commands are explicit subcommands.
- **Command style**: Flat subcommands following [clig.dev](https://clig.dev/#the-basics) — flags over positional args, full `--flag` names, standard conventions.
- **Targeting**: `--speaker NAME` or `--group NAME` flags on every command. If both are given, `--group` wins. If neither, targets the default group (first/last active, configurable).
- **Discovery**: SSDP results cached to `~/.config/sonos/cache.json`. On cache miss or unknown speaker, auto-rediscovers once. If still not found, fail with a clear error. `sonos discover` refreshes manually.
- **Output**: Plain human-readable text. No JSON for now (easy to add later with `--json`).
- **Architecture**: Single Cargo package (`sonos`). Shared `Action` enum + `executor` module. CLI and TUI both emit `Action` values; executor resolves them against `SonosSystem`.

---

## Module Structure

```
src/
  main.rs           ← arg parse; no args → TUI, else → CLI dispatch
  actions.rs        ← Action enum covering all SDK operations
  executor.rs       ← execute(Action, &SonosSystem) → Result
  cache.rs          ← read/write ~/.config/sonos/cache.json
  config.rs         ← read ~/.config/sonos/config.toml
  cli/
    mod.rs          ← clap top-level Commands enum → Action
  tui/
    app.rs          ← App state; keypress → Action → executor
    screens/        ← Home, Group, Speaker screens (ratatui components)

Cargo.toml dependencies (approximate):
  sonos-sdk  = { path = "../sonos-sdk/sonos-sdk" }
  clap       = { version = "4", features = ["derive"] }
  ratatui    = "0.28"
  crossterm  = "0.28"
  serde      = { version = "1", features = ["derive"] }
  serde_json = "1"
  dirs       = "5"
  anyhow     = "1"
```

---

## Command Reference (v1 scope)

### Discovery & System
| Command | Description |
|---------|-------------|
| `sonos discover` | Run SSDP, refresh cache |
| `sonos speakers` | List all speakers with state and volume |
| `sonos groups` | List all groups with state and volume |

### Playback
All accept `[--speaker NAME \| --group NAME]`. Defaults to active group.

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
| `sonos mute` / `sonos unmute` | Speaker or group |
| `sonos bass <-10..10> --speaker NAME` | Speaker only |
| `sonos treble <-10..10> --speaker NAME` | Speaker only |
| `sonos loudness <on\|off> --speaker NAME` | Speaker only |

### Status
| Command | Description |
|---------|-------------|
| `sonos status` | Current track, playback state, volume |
| `sonos queue` | Show current queue |

### Grouping
| Command | Description |
|---------|-------------|
| `sonos join --speaker NAME --group NAME` | Add speaker to group |
| `sonos leave --speaker NAME` | Remove speaker from group |

### Sleep Timer
| Command | Description |
|---------|-------------|
| `sonos sleep <DURATION>` | Set sleep timer (e.g. `30m`, `1h`) |
| `sonos sleep cancel` | Cancel sleep timer |

### Queue Management
| Command | Description |
|---------|-------------|
| `sonos queue add <URI>` | Add URI to queue |
| `sonos queue clear` | Clear entire queue |

---

## Targeting Rules

```
--group wins over --speaker if both are given.
Neither given → default group (configurable; falls back to first discovered group).
group volume = GroupVolumeHandle on Group
speaker volume = VolumeHandle on Speaker
```

---

## Discovery Flow

```
1. Load cache (~/.config/sonos/cache.json)
2. Find target speaker/group in cache
3. If not found → run SSDP discovery, update cache
4. If still not found → exit with error:
   "Speaker 'X' not found. Run `sonos discover` to refresh."
```

---

## Resolved Questions

- **Error output**: Errors and warnings go to stderr only (clig.dev standard).
- **Cache TTL**: Cache auto-expires after 24h (configurable). Next command after expiry triggers rediscovery before executing.
- **`sonos tui` deep-link**: TUI always opens at the Home screen. `--group NAME` flag is a future nice-to-have.

## Open Questions

- **Config format**: TOML at `~/.config/sonos/config.toml`? Values to decide in planning: default group, cache TTL duration, TUI theme.

---

## Next Steps

→ `/workflows:plan` for implementation details and file-by-file task breakdown.
