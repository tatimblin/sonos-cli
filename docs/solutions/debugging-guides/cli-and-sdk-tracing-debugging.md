---
title: "Debugging sonos-cli and sonos-sdk with tracing"
date: 2026-03-28
category: debugging-guides
tags:
  - tracing
  - logging
  - debugging
  - sdk-integration
  - tui
severity: medium
component:
  - CLI
  - TUI
  - sonos-sdk
symptom: |
  SDK had 73 tracing call sites but no subscriber wired up — all events were silently dropped.
  CLI had only eprintln! with a --verbose bool flag, insufficient for debugging SDK-level issues.
  Agents monitoring TUI sessions had no visibility into internal state.
root_cause: |
  sonos-cli did not initialize a tracing subscriber despite sonos-sdk heavily instrumenting
  its discovery, playback, and state management paths with tracing macros.
solution_type: feature-implementation
---

# Debugging sonos-cli and sonos-sdk with tracing

## Problem

The sonos-sdk instruments its internals with `tracing` (73 call sites across `system.rs`, `state.rs`, `handles.rs`, and more), but without a subscriber initialized in the consuming binary, all events are silently dropped. The CLI only had `eprintln!` with a `--verbose` bool flag — no structured logging, no file output, and no way for agents to monitor TUI sessions.

## Solution

A `tracing-subscriber` is wired up in `src/logging.rs` with two layers:

- **File layer** (always active): writes to the platform-specific log file, truncated per session
- **Stderr layer** (CLI commands only): disabled in TUI mode since ratatui owns the terminal

The subscriber is initialized in `main()` **before** any SDK calls, so discovery and cache events are captured from the start.

## Debugging Recipes

### CLI commands: increasing verbosity

```bash
sonos play -v          # info: discovery, cache hits, connections
sonos play -vv         # debug: SOAP requests, state diffs, UPnP parsing
sonos play -vvv        # trace: raw payloads, event loop ticks
```

Both stderr and the log file show traces in CLI mode.

### TUI sessions: tail the log file

The TUI owns the terminal, so traces go to a log file only. Use two terminals:

```bash
# Terminal 1: run TUI with trace logging
sonos -vvv

# Terminal 2: tail the log file
# macOS:
tail -f ~/Library/Application\ Support/sonos/sonos.log
# Linux:
tail -f ~/.local/share/sonos/sonos.log
```

### Isolating SDK internals with RUST_LOG

Use `RUST_LOG` for per-target filtering when you need to isolate specific crate output:

```bash
RUST_LOG=sonos_sdk=trace sonos play          # only SDK trace events
RUST_LOG=sonos_sdk::system=debug sonos play  # just the system module
RUST_LOG=sonos_state=debug sonos play        # state manager events
RUST_LOG=ureq=debug sonos play               # HTTP/SOAP traffic
```

**Precedence:** The `-v` flag takes precedence over `RUST_LOG`. To use `RUST_LOG`, omit `-v`.

## Verbosity Levels

| Flag | Level | What you see |
|------|-------|-------------|
| (none) | warn | Warnings and errors only |
| `-v` | info | + discovery, cache hits, connections |
| `-vv` | debug | + SOAP requests, state diffs, UPnP parsing |
| `-vvv` | trace | Everything (raw payloads, event loop ticks) |

## Log File

| Platform | Path |
|----------|------|
| macOS | `~/Library/Application Support/sonos/sonos.log` |
| Linux | `~/.local/share/sonos/sonos.log` |

- Truncated on each launch (fresh per session)
- Includes timestamps, level, and target module
- Gracefully degrades if file can't be created (warns and continues)

## Example Output

From a TUI session at `-vvv`:

```
2026-03-29T05:56:41.167374Z  INFO sonos_state::state: StateManager created (sync-first mode)
2026-03-29T05:56:41.167713Z DEBUG sonos_state::state: Added speaker RINCON_7828CAFB9D9C01400 at IP 192.168.4.47
2026-03-29T05:56:41.169427Z  WARN sonos_sdk::system: duplicate speaker name "Basement", keeping last discovered
2026-03-29T05:56:41.184111Z DEBUG ureq::unit: sending request POST http://192.168.4.48:1400/ZoneGroupTopology/Control
2026-03-29T05:56:41.204117Z DEBUG sonos_sdk::system: Fetched zone group topology on-demand (2 groups)
```

## Common Gotchas

- **`sonos -vvv` produces no stderr output** — Correct. TUI mode routes all logs to the file. Check the file path above.
- **Log file is empty after restart** — The file is truncated per session. Copy it before restarting if you need previous logs.
- **`RUST_LOG` has no effect** — The `-v` flag takes precedence. Remove `-v` to use `RUST_LOG`.
- **Library crates must not initialize subscribers** — That's the binary's job. sonos-sdk instruments with `tracing::info!()` etc., but sonos-cli calls `init_logging()`.

## Prevention: "Missing Logs" Checklist

When debugging "why aren't my tracing events showing up":

1. Is a `tracing-subscriber` initialized in `main()`?
2. Is it initialized **before** the code that emits events?
3. Does the filter level match? (e.g., `debug!` won't show at `warn` level)
4. Does the target name in `RUST_LOG` match the actual crate name? (underscores, not hyphens)
5. Is the output going where you expect? (stderr for CLI, file for TUI)

## Key Files

| File | Role |
|------|------|
| `src/logging.rs` | `init_logging(verbosity, is_tui)` — subscriber setup |
| `src/main.rs` | Calls `init_logging()` before TUI/CLI branch |
| `src/cli/mod.rs` | `-v` / `--verbose` flag definition (`ArgAction::Count`) |

## Related Documents

- [CLI logging brainstorm](../../brainstorms/2026-03-28-cli-logging-brainstorm.md) — design decisions
- [CLI logging plan](../../plans/2026-03-28-feat-cli-logging-system-plan.md) — implementation plan
- [Discovery failure diagnostics](../../brainstorms/2026-03-22-discovery-failure-diagnostics-brainstorm.md) — related: platform-specific hints
- [CLI commands reference](../../references/cli-commands.md) — `--verbose` flag documentation
