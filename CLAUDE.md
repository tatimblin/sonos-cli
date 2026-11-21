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
| Sonos SDK | `sonos-sdk` `0.9.0` from crates.io (`test-support` feature in dev-dependencies) |
| Config | `serde` + `toml`, `dirs` |
| Errors | `thiserror` (`CliError`) on the CLI path; `anyhow` in the TUI and `Config::save` |
| Logging | `tracing` + `tracing-subscriber` |
| Completions / man pages | `clap_complete` (dynamic); `clap_mangen` behind the `generate` feature |

**Language:** Rust, edition 2021. The SDK is **sync** — no `async`/`await` anywhere.

## SDK Development

`Cargo.toml` depends on the published `sonos-sdk`. For local development, a **gitignored** `.cargo/config.toml` in the repo root patches it (and `sonos-sdk-state`) to the checkout at `../sonos-sdk` via `[patch.crates-io]`; CI and releases use crates.io. With the patch active, cargo rewrites `Cargo.lock` to drop the registry `source`/`checksum` lines for the SDK crates — never commit that churn (`git checkout -- Cargo.lock` before committing).

If a feature or fix requires SDK changes, make them in `../sonos-sdk` — don't work around SDK limitations in `sonos-cli`. Using it here then needs an SDK release and a version bump. The SDK API reference is at `docs/references/sonos-sdk.md`.

## Module Structure

```
src/
  main.rs           ← parse args, load config, init logging; no args → TUI, else → run_command()
  lib.rs            ← library crate (used by main.rs, tests/, and the generate example)
  config.rs         ← Config load/save (platform config dir or $SONOS_CONFIG_DIR); refuses to save over a file that failed to parse
  errors.rs         ← CliError (thiserror) with recovery_hint() and exit_code()
  diagnostics.rs    ← platform discovery hints, unverified-output banner, open-settings prompt
  liveness.rs       ← did a read command actually reach a speaker? (exit 3, --offline/--require-live)
  logging.rs        ← tracing setup; appends sonos.log in the data dir, rotates to sonos.log.1 past 1 MiB
  generate.rs       ← `sonos-generate` example: completions + man page (`--features generate`)
  cli/
    mod.rs          ← Cli + GlobalFlags (clap), re-exports
    commands.rs     ← Commands enum and sub-actions (parsing only; value ranges enforced by clap)
    run.rs          ← run_command(): one arm per command, calls SDK methods directly
    resolve.rs      ← --speaker/--group/alias resolution, Target, require_speaker_only
    parse.rs        ← seek time / sleep duration parsing
    format.rs       ← output formatting for read commands
    complete.rs     ← dynamic shell-completion candidates
    config_cmd.rs   ← `sonos config` (runs before discovery)
  tui/
    mod.rs          ← run(): image-protocol detection, App::new, event loop
    app.rs          ← App (App::with_system for tests), Navigation, Tab (Speakers, Settings)
    event.rs        ← event loop (ratatui::init/restore), Ctrl-C, SDK change events
    keymap.rs       ← key binding tables: single source for dispatch and the legend
    types.rs        ← shared types across layers (ListEntry, RowData, PickUpState, SettingsRow)
    helpers.rs      ← shared data helpers (track_summary)
    hooks.rs        ← reactive state: use_watch, use_watch_group_or_fetch, use_animation, use_state
    theme.rs        ← Theme + Glyphs, built-in themes
    image_loader.rs ← background album-art fetch + cache
    ui.rs           ← header + legend + screen dispatch (no screen logic)
    screens/        ← data assembly: hooks → widget data structs → widget calls
    widgets/        ← render-only components: take data + theme, output to frame
    handlers/       ← key handling + SDK mutations
tests/              ← integration tests (liveness)
```

## Non-Negotiable Architectural Rules

1. **Direct SDK calls.** CLI command handlers and TUI event handlers call SDK methods directly. The SDK (`sonos-sdk`) is the shared API layer — no intermediate Action enum or executor dispatch.
2. **Errors to stderr.** `stdout` is for command output only. CLI handlers return `Result<_, CliError>`; `main` prints `error: {e}` plus `recovery_hint()` to stderr and exits with `exit_code()`. The TUI propagates `anyhow::Result`.
3. **Discovery is cached by the SDK.** `SonosSystem::new()` handles caching transparently — `cache.json` in the platform cache dir (`~/.cache/sonos/` on Linux, `~/Library/Caches/sonos/` on macOS, or `$SONOS_CACHE_DIR`) with 24h TTL, falls back to SSDP on miss or expiry. `system.speaker(name)` re-runs SSDP on a miss, at most once per 30 s.
4. **`--group` wins over `--speaker`.** If both flags are given, target the group. With neither, use the configured default group, then the largest group's coordinator. Speaker-only commands (`bass`, `treble`, `loudness`) reject `--group` as a usage error.

## CLI Conventions

Follow `docs/references/cli-guidelines.md` for all command design decisions:
- Flat subcommands: `sonos <verb>`, never `sonos <domain> <verb>`
- Flags over positional args: `--speaker "Kitchen"` not `sonos play Kitchen`
- Exit code 0 = success, 1 = runtime error, 2 = usage error, 3 = output printed but no speaker responded (see `--offline` / `--require-live`)
- Error format: `error: <description>` then, when one exists, a recovery line from `CliError::recovery_hint()` (a platform-specific discovery `hint:` for not-found/discovery errors)

## Contributing Workflow

Follow `docs/references/contributing-agent-guide.md` for the complete workflow when making changes:
- Always work on a feature branch, never commit directly to `main`
- Commit messages and PR titles must use conventional commit format (`feat:`, `fix:`, etc.)
- Verify CI passes before merging: `gh pr checks`
- Never manually bump versions — release-plz handles versioning and changelog automatically

## Reference Documentation

| Document | What it covers |
|----------|---------------|
| `docs/goals.md` | Project vision, TUI screen designs, CLI command reference, v1 scope |
| `docs/references/sonos-sdk.md` | Complete SDK API — every type, method, and example |
| `docs/references/cli-guidelines.md` | clig.dev rules applied to this project |
| `docs/references/cli-commands.md` | Per-command reference — syntax, flags, examples, output, errors for every command |
| `docs/references/contributing-agent-guide.md` | Step-by-step workflow for AI agents: branching, commits, PRs, CI validation, versioning |
| `docs/references/tui-architecture.md` | TUI three-layer architecture, hooks system, how to add screens/widgets/handlers |

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
