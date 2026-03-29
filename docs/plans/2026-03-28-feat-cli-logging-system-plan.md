---
title: "feat: Add tracing-based logging system"
type: feat
status: active
date: 2026-03-28
origin: docs/brainstorms/2026-03-28-cli-logging-brainstorm.md
---

# feat: Add tracing-based logging system

## Overview

Wire up `tracing` + `tracing-subscriber` in sonos-cli so that the 73 existing SDK trace events surface automatically, and the CLI has a permanent logging system for debugging — especially for agents monitoring TUI sessions via log file.

## Problem Statement

The SDK is instrumented with `tracing` (73 call sites across 5 sub-crates), but no subscriber is initialized — all events are silently dropped. The CLI relies on `eprintln!` with a `--verbose` bool flag in only 2 places. This is insufficient for diagnosing SDK-level issues and unusable by agents watching TUI sessions (see brainstorm: `docs/brainstorms/2026-03-28-cli-logging-brainstorm.md`).

## Proposed Solution

Add a `src/logging.rs` module that initializes a `tracing-subscriber` with two layers:

- **File layer** (always active) — writes to `~/.local/share/sonos/sonos.log`, truncated per session. Works for both TUI and CLI modes.
- **Stderr layer** (CLI commands only) — writes to stderr. Disabled in TUI mode because ratatui owns the terminal.

Verbosity controlled by `-v` / `-vv` / `-vvv` flag (replaces the current `--verbose` bool). Single global level.

## Technical Approach

### Subscriber initialization location

Initialize in `main()` after `Cli::parse()` but **before** `SonosSystem::new()` and `tui::run()`. This is critical because `SonosSystem::new()` triggers SDK discovery traces — if the subscriber isn't set yet, those events are lost.

Determine whether to include the stderr layer by checking the same condition used for the TUI/CLI branch: `cli.command.is_none() && std::io::stdout().is_terminal()`. When true (TUI mode), omit the stderr layer.

```
main() flow:
  1. Cli::parse()
  2. Config::load()
  3. init_logging(verbosity, is_tui_mode)   ← NEW
  4. match cli.command { None => tui::run(), Some(cmd) => run_command() }
```

The TUI module does **not** need to receive `GlobalFlags` — the subscriber is fully initialized before `tui::run()` is called.

### Verbosity mapping

| Flag | Count | Level | What surfaces |
|------|-------|-------|--------------|
| (none) | 0 | warn | Warnings and errors only |
| `-v` / `--verbose` | 1 | info | + discovery, cache hits, connections |
| `-vv` | 2 | debug | + SOAP requests, state diffs, UPnP parsing |
| `-vvv` | 3+ | trace | Everything (raw payloads, event loop ticks) |

**`RUST_LOG` fallback:** If no `-v` flag is specified (count = 0), check for `RUST_LOG` env var. If set, use it (enables per-target filtering for power users). Otherwise default to `warn`. The `-v` flag always takes precedence over `RUST_LOG`.

### Log format

- **File layer:** Full format with timestamps and target module. ANSI disabled explicitly (`.with_ansi(false)`). Example: `2026-03-28T10:15:30.123Z  INFO sonos_sdk::system: speaker 'Kitchen' not found, running auto-rediscovery...`
- **Stderr layer:** Compact format without timestamps (ephemeral output). Example: `INFO sonos_sdk::system: auto-rediscovery triggered for 'Kitchen'`

### Log file management

- **Path:** `dirs::data_local_dir()` / `sonos` / `sonos.log` (XDG `~/.local/share/sonos/sonos.log` on Linux, `~/Library/Application Support/sonos/sonos.log` on macOS). The `dirs` crate is already a dependency.
- **Directory creation:** Call `std::fs::create_dir_all()` for the log directory before file creation.
- **Truncation:** Open with `File::create()` (truncates existing file) — fresh per session.
- **Graceful degradation:** If the log file cannot be created (permissions, read-only FS), print a warning to stderr and continue without file logging. The tool must never fail to launch because of a log file issue.
- **Writer:** Use raw `File` (unbuffered) to avoid losing log lines on panic. No `BufWriter`.
- **No ANSI:** `.with_ansi(false)` on the file layer.

### Flag migration

Change `GlobalFlags.verbose` from `bool` to `u8` with `clap::ArgAction::Count`:

```rust
// src/cli/mod.rs
/// Increase log verbosity (-v info, -vv debug, -vvv trace)
#[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count, global = true)]
pub verbose: u8,
```

`--verbose` remains valid as a long form (equivalent to `-v`, sets count to 1). This is backward-compatible — existing `--verbose` users get `info` level instead of the previous ad-hoc debug output.

### Existing code migration

**Two `eprintln!("debug: ...")` calls in `main.rs` (lines 47-49, 69-71):**
Replace with `tracing::debug!("{e:?}")`. These now surface only when verbosity >= 2 (`-vv`) and the stderr layer is active (CLI mode). This eliminates the manual `if verbose` check.

**16 test fixtures using `verbose: false`:**
Change to `verbose: 0`. Mechanical change — the plan must include this to avoid compilation failures. Located in `src/cli/resolve.rs` and `src/diagnostics.rs`.

## Files Changed

| File | Change |
|------|--------|
| `Cargo.toml` | Add `tracing = "0.1"` and `tracing-subscriber = { version = "0.3", features = ["env-filter"] }` |
| `src/lib.rs` | Add `pub mod logging;` |
| `src/logging.rs` | **New.** `init_logging(verbosity: u8, is_tui: bool)` — creates file + optional stderr layers, sets global subscriber |
| `src/main.rs` | Call `logging::init_logging()` after parse, before branch. Remove manual `if verbose` checks, replace with `tracing::debug!` |
| `src/cli/mod.rs` | Change `verbose: bool` → `verbose: u8` with `ArgAction::Count`, update help text |
| `src/cli/resolve.rs` | Update test fixtures: `verbose: false` → `verbose: 0` |
| `src/diagnostics.rs` | Update test fixtures: `verbose: false` → `verbose: 0` |

## Scope boundaries

**In scope:**
- Subscriber wiring (file + stderr layers)
- `-v` / `-vv` / `-vvv` flag
- `RUST_LOG` fallback
- Migration of existing `verbose` bool to count
- Migration of 2 manual `eprintln!("debug: ...")` calls

**Out of scope (follow-up):**
- Adding `tracing` instrumentation to CLI command handlers (the 73 SDK call sites surface immediately; CLI-side instrumentation is a separate effort)
- Log file rotation or size limits (acceptable for v1 — users opting into `-vvv` understand the verbosity)
- `sonos --log-path` helper command (nice-to-have, not blocking)
- JSON log format option

## Acceptance Criteria

- [ ] `sonos play -vv` prints SDK debug traces to stderr and writes them to the log file
- [ ] `sonos -vv` (TUI) writes SDK debug traces to the log file only — stderr is clean, TUI renders normally
- [ ] `sonos play` (no `-v`) only shows warnings/errors (default `warn` level)
- [ ] `RUST_LOG=sonos_sdk=trace sonos play` surfaces SDK trace output when no `-v` flag is given
- [ ] `-v` flag takes precedence over `RUST_LOG` when both are present
- [ ] Log file is created at `~/.local/share/sonos/sonos.log` (or platform equivalent)
- [ ] Log file is truncated on each launch (fresh per session)
- [ ] If log directory/file cannot be created, the tool prints a warning and continues normally
- [ ] `--verbose` long flag still works (backward-compatible, equivalent to `-v`)
- [x] All existing tests pass after `verbose: bool` → `verbose: u8` migration
- [x] `cargo clippy -- -D warnings` passes
- [x] `cargo fmt --check` passes

## Sources

- **Origin brainstorm:** [docs/brainstorms/2026-03-28-cli-logging-brainstorm.md](docs/brainstorms/2026-03-28-cli-logging-brainstorm.md) — key decisions: tracing framework, file + stderr layers, single global level, fresh per session
- SDK tracing example pattern: `../sonos-sdk/sonos-sdk/examples/smart_dashboard.rs:24-31`
- Current `--verbose` flag: `src/cli/mod.rs:40-41`
- Manual debug output: `src/main.rs:47-49, 69-71`
