# CLI Logging System

**Date:** 2026-03-28
**Status:** Draft
**Relates to:** Cross-cutting (supports all milestones)

## What We're Building

A permanent logging system for sonos-cli that surfaces both CLI and SDK trace events. The primary use case is agent-readable debugging — an agent running the TUI needs to read structured log output from a file while the TUI owns the terminal.

### The Problem

The SDK is instrumented with `tracing` (warn, info, debug events across system.rs, state.rs, handles.rs), but no subscriber is wired up — all events are silently dropped. The CLI relies on `eprintln!` with a `--verbose` flag, which is insufficient for diagnosing SDK-level issues and unusable by agents watching TUI sessions.

### Target Behavior

- **Always:** Write logs to `~/.local/share/sonos/sonos.log`, truncated on each launch (fresh per session).
- **CLI commands only:** Also write logs to stderr (TUI cannot use stderr — ratatui owns the terminal).
- **Verbosity:** Controlled by `-v` / `-vv` / `-vvv` flag. Single global level applied to both CLI and SDK events.

| Flag | Level | What you see |
|------|-------|-------------|
| (none) | warn | Warnings and errors only |
| `-v` | info | + informational events (discovery, cache hits, connections) |
| `-vv` | debug | + debug details (SOAP requests, state diffs, UPnP parsing) |
| `-vvv` | trace | Everything (raw payloads, event loop ticks) |

**Agent workflow:** An agent launches `sonos -vv` (TUI) in one terminal, tails `~/.local/share/sonos/sonos.log` in another, and reads structured log output to diagnose issues.

## Why This Approach

### tracing + tracing-subscriber

The SDK already emits `tracing` events. Wiring up `tracing-subscriber` in the CLI is the natural fit:

1. **SDK events surface automatically** — no SDK changes needed, no compatibility layer.
2. **Layered output** — file layer (always) + stderr layer (CLI only) via `tracing-subscriber`'s `Layer` trait.
3. **Rust ecosystem standard** — familiar to contributors, good tooling, structured logging possible later.
4. **Minimal new code** — subscriber setup in `main.rs`, add `tracing` macros in CLI handlers as needed.

### Alternatives considered

- **`env_logger` + `log`:** Loses tracing spans and structured context. SDK uses `tracing` natively, so this downgrades its events through the compatibility layer.
- **Custom `eprintln!` logging:** Can't capture SDK events at all. Reinvents subscriber/layer concepts.

## Key Decisions

1. **tracing + tracing-subscriber** for the logging framework.
2. **File output always, stderr only for CLI commands** (not TUI).
3. **Single global verbosity level** via `-v` / `-vv` / `-vvv` flags. No per-target filtering for now.
4. **Fresh log per session** — truncate the log file on each launch for clean agent reads.
5. **Log file path:** `~/.local/share/sonos/sonos.log` (follows XDG data dir convention via `dirs` crate, which is already a dependency).

## Open Questions

None — all questions resolved during brainstorming.
