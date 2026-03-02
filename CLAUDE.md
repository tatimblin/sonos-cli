# CLAUDE.md

Steering document for Claude Code working on `sonos-cli`. Read this first.

## Project Overview

`sonos` is a single Rust binary that controls Sonos speakers via `sonos-sdk`.

**Two modes:**
- **Default (no args):** launches an interactive TUI built with `ratatui`
- **One-off commands:** flat subcommands like `sonos play`, `sonos volume 50 --group "Kitchen"`

Both modes go through the same `Action` enum + `executor` module — never call the SDK directly from `main.rs`, `cli/`, or `tui/`.

## Technology Stack

| Concern | Crate |
|---------|-------|
| CLI parsing | `clap` v4 (derive feature) |
| TUI | `ratatui` + `crossterm` |
| Sonos SDK | `sonos-sdk` (path = `"../sonos-sdk/sonos-sdk"`) |
| Config / cache | `serde` + `serde_json`, `dirs` |
| Error handling | `anyhow` |

**Language:** Rust, edition 2021. The SDK is **sync** — no `async`/`await` anywhere.

## Module Structure

```
src/
  main.rs           ← arg parse; no args → TUI, else → CLI dispatch
  actions.rs        ← Action enum covering all SDK operations
  executor.rs       ← execute(Action, &SonosSystem) → Result<(), anyhow::Error>
  cache.rs          ← read/write ~/.config/sonos/cache.json (24h TTL)
  config.rs         ← read ~/.config/sonos/config.toml
  cli/
    mod.rs          ← clap Commands enum → maps args to Action values
  tui/
    app.rs          ← App state; crossterm event loop; keypress → Action → executor
    screens/        ← ratatui components: Home, Group, Speaker screens
```

## Non-Negotiable Architectural Rules

1. **Action dispatch only.** Both `cli/` and `tui/` emit `Action` values. `executor.rs` is the only place SDK methods are called. No exceptions.
2. **Errors to stderr.** `stdout` is for command output only. Use `eprintln!` / `anyhow` for all errors.
3. **Discovery is cached.** Never run SSDP on every command. Load `~/.config/sonos/cache.json`; rediscover only on miss or TTL expiry. `sonos discover` refreshes manually.
4. **`--group` wins over `--speaker`.** If both flags are given, target the group. Default to the configured/first group when neither is given.

## CLI Conventions

Follow `docs/references/cli-guidelines.md` for all command design decisions:
- Flat subcommands: `sonos <verb>`, never `sonos <domain> <verb>`
- Flags over positional args: `--speaker "Kitchen"` not `sonos play Kitchen`
- Exit code 0 = success, 1 = runtime error, 2 = usage error
- Error format: `error: <description>\nRun 'sonos discover' to refresh.`

## Reference Documentation

| Document | What it covers |
|----------|---------------|
| `docs/goals.md` | Project vision, TUI screen designs, CLI command reference, v1 scope |
| `docs/references/sonos-sdk.md` | Complete SDK API — every type, method, and example |
| `docs/references/cli-guidelines.md` | clig.dev rules applied to this project |

## Brainstorm Sources

| File | What was decided |
|------|-----------------|
| `docs/brainstorm/2026-02-26-sonos-tui-brainstorm.md` | TUI screen architecture, navigation model, album art, theming, keyboard layout |
| `docs/brainstorm/2026-03-01-sonos-cli-architecture-brainstorm.md` | CLI architecture, Action dispatch pattern, discovery cache, command reference |

## Product Direction

`docs/product/` holds long-term product management initiatives. Check there for upcoming feature work.
