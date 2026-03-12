# CLAUDE.md

Steering document for Claude Code working on `sonos-cli`. Read this first.

## Project Overview

`sonos` is a single Rust binary that controls Sonos speakers via `sonos-sdk`.

**Two modes:**
- **Default (no args):** launches an interactive TUI built with `ratatui`
- **One-off commands:** flat subcommands like `sonos play`, `sonos volume 50 --group "Kitchen"`

SDK methods are called directly from CLI command handlers and TUI event handlers. The SDK is the shared API layer — no intermediate dispatch layer.

## Technology Stack

| Concern | Crate |
|---------|-------|
| CLI parsing | `clap` v4 (derive feature) |
| TUI | `ratatui` + `crossterm` |
| Sonos SDK | `sonos-sdk` (path = `"../sonos-sdk/sonos-sdk"`) |
| Config / cache | `serde` + `serde_json`, `dirs` |
| Error handling | `anyhow` |

**Language:** Rust, edition 2021. The SDK is **sync** — no `async`/`await` anywhere.

## SDK Development

The `sonos-sdk` source lives at `../sonos-sdk` (repo root) with the crate at `../sonos-sdk/sonos-sdk`. If a feature or fix requires SDK changes, make them there directly — don't work around SDK limitations in `sonos-cli`. The SDK API reference is at `docs/references/sonos-sdk.md`.

## Module Structure

```
src/
  main.rs           ← arg parse; no args → TUI, else → cmd.run()
  config.rs         ← read ~/.config/sonos/config.toml
  errors.rs         ← CliError with recovery hints and exit codes
  cli/
    mod.rs          ← clap Commands enum + run() → calls SDK methods directly
  tui/
    app.rs          ← App state; crossterm event loop; keypress → SDK calls
    screens/        ← ratatui components: Home, Group, Speaker screens
```

## Non-Negotiable Architectural Rules

1. **Direct SDK calls.** CLI command handlers and TUI event handlers call SDK methods directly. The SDK (`sonos-sdk`) is the shared API layer — no intermediate Action enum or executor dispatch.
2. **Errors to stderr.** `stdout` is for command output only. Use `eprintln!` / `anyhow` for all errors.
3. **Discovery is cached by the SDK.** `SonosSystem::new()` handles caching transparently — loads from `~/.cache/sonos/cache.json` with 24h TTL, falls back to SSDP on miss or expiry. Auto-rediscovers once per session when a speaker isn't found.
4. **`--group` wins over `--speaker`.** If both flags are given, target the group. Default to the configured/first group when neither is given.

## CLI Conventions

Follow `docs/references/cli-guidelines.md` for all command design decisions:
- Flat subcommands: `sonos <verb>`, never `sonos <domain> <verb>`
- Flags over positional args: `--speaker "Kitchen"` not `sonos play Kitchen`
- Exit code 0 = success, 1 = runtime error, 2 = usage error
- Error format: `error: <description>\nCheck that your speakers are on the same network, then retry.`

## Reference Documentation

| Document | What it covers |
|----------|---------------|
| `docs/goals.md` | Project vision, TUI screen designs, CLI command reference, v1 scope |
| `docs/references/sonos-sdk.md` | Complete SDK API — every type, method, and example |
| `docs/references/cli-guidelines.md` | clig.dev rules applied to this project |
| `docs/references/cli-commands.md` | Per-command reference — syntax, flags, examples, output, errors for all 27 commands |

## Brainstorm Sources

| File | What was decided |
|------|-----------------|
| `docs/brainstorm/2026-02-26-sonos-tui-brainstorm.md` | TUI screen architecture, navigation model, album art, theming, keyboard layout |
| `docs/brainstorm/2026-03-01-sonos-cli-architecture-brainstorm.md` | CLI architecture, discovery cache, command reference (Action dispatch superseded) |
| `docs/brainstorms/2026-03-10-cli-architecture-simplification-brainstorm.md` | Removed Action/executor, SDK is the shared layer, direct SDK calls from handlers |

## Product Direction

`docs/product/` holds the long-term product roadmap and PRD.

| Document | What it covers |
|----------|---------------|
| `docs/product/roadmap.md` | v1 milestone breakdown — every task, SDK method, and exit criteria |
| `docs/product/prd.md` | Product requirements — vision, target users, feature surface, non-functional requirements |

**When creating a new plan (`docs/plans/`):**
1. Read `docs/product/roadmap.md` first.
2. Identify which roadmap milestone(s) and checklist items the plan addresses.
3. Reference the milestone in the plan's frontmatter or overview (e.g., "Implements Milestone 3: CLI — Playback Commands").
4. After completing work, check off the corresponding `- [ ]` items in the roadmap so it stays current.

The roadmap is the source of truth for what needs to be built. Plans should be driven by roadmap priorities — don't start work that isn't traceable back to a milestone.
