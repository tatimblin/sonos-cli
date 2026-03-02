---
title: "docs: Add project steering documentation"
type: docs
status: completed
date: 2026-03-01
origin: docs/brainstorm/2026-03-01-sonos-cli-architecture-brainstorm.md
---

# docs: Add Project Steering Documentation

## Overview

Create foundational documentation that gives Claude (and any future contributor) everything needed to build `sonos-cli` correctly — without having to re-derive architecture decisions, re-read the SDK, or guess at CLI conventions.

Five documents + one empty directory:
1. `CLAUDE.md` — root-level AI steering file
2. `docs/goals.md` — project vision and key design decisions from brainstorming
3. `docs/references/sonos-sdk.md` — SDK reference (partially drafted by research agent; needs editing + move from `docs/sonos-sdk.md`)
4. `docs/references/cli-guidelines.md` — CLI design principles (clig.dev) adapted for this project
5. `docs/product/` — empty directory, placeholder for future product management initiatives

**Directory structure:**
```
docs/
  brainstorm/         ← existing brainstorm docs
  plans/              ← this plan and future plans
  references/         ← technical reference material
    sonos-sdk.md
    cli-guidelines.md
  product/            ← long-term PM initiatives (empty for now)
  goals.md
```

---

## Problem Statement / Motivation

The project is a clean slate with no source code. Before implementing anything, there must be clear documentation that:

- Tells Claude what the project is and how it should be built
- Captures architectural decisions already made in brainstorming so they aren't re-litigated
- Documents the SDK API so commands can be wired up without re-spelunking the codebase
- Records the CLI design philosophy so every command is consistent

Without this steering layer, each implementation session risks inconsistency, drift from the brainstorm decisions, or re-inventing conventions already chosen.

---

## Proposed Solution

Four markdown files covering goals, SDK API, CLI guidelines, and a `CLAUDE.md` that ties them together. All files are written for an AI audience — concise, precise, and actionable.

---

## Acceptance Criteria

- [x] `CLAUDE.md` exists at repo root, referencing all other docs with updated paths
- [x] `docs/goals.md` captures the full project vision from both brainstorms with no contradictions
- [x] `docs/references/sonos-sdk.md` covers the complete public API with code examples; `docs/sonos-sdk.md` (old location) is removed
- [x] `docs/references/cli-guidelines.md` distills clig.dev into rules that apply specifically to this project
- [x] `docs/product/` directory exists (empty; `.gitkeep` if needed)
- [x] All internal cross-references between files are valid relative links
- [x] No implementation code exists in the docs (steering only)

---

## File Specifications

### 1. `CLAUDE.md` (root)

**Purpose:** Primary steering document. Claude reads this first on every session.

**Contents:**

```
# CLAUDE.md

## Project Overview
Single binary `sonos` that controls Sonos speakers.
Two modes: default TUI (ratatui), and one-off subcommands.

## Technology Stack
- Language: Rust (edition 2021)
- CLI parsing: clap v4 (derive feature)
- TUI: ratatui + crossterm
- SDK: sonos-sdk (path = "../sonos-sdk/sonos-sdk")
- Config/cache: serde + serde_json, dirs crate
- Error handling: anyhow

## Module Structure
src/main.rs           — entry point
src/actions.rs        — Action enum (shared by CLI + TUI)
src/executor.rs       — execute(Action, &SonosSystem) → Result
src/cache.rs          — ~/.config/sonos/cache.json
src/config.rs         — ~/.config/sonos/config.toml
src/cli/mod.rs        — clap command definitions → Action
src/tui/app.rs        — TUI App state + event loop
src/tui/screens/      — ratatui screen components

## Key Architectural Rules
1. Both CLI and TUI dispatch ONLY through the Action enum + executor.
   Never call SDK methods directly from main.rs, cli/, or tui/.
2. Errors go to stderr. stdout is for command output only.
3. Discovery is cached. Never run SSDP on every command.
4. --group wins over --speaker if both are given.

## Reference Docs
- docs/goals.md                    — project vision and design decisions
- docs/references/sonos-sdk.md     — complete SDK API reference
- docs/references/cli-guidelines.md — CLI design rules (clig.dev)

## Product Docs
- docs/product/  — long-term product management initiatives (empty; add as needed)

## Brainstorm Sources
- docs/brainstorm/2026-02-26-sonos-tui-brainstorm.md  — TUI design
- docs/brainstorm/2026-03-01-sonos-cli-architecture-brainstorm.md — CLI architecture
```

---

### 2. `docs/goals.md`

**Purpose:** Project vision, design decisions, and scope for v1. Synthesized from both brainstorms.

**Section outline:**

1. **What We're Building** — Single binary, two modes. The "why" of each.
2. **Design Philosophy** — Groups-first model, keyboard-only TUI, flat CLI commands, clig.dev compliance.
3. **TUI: Screen Architecture** — Summary of the screen hierarchy (Home → Groups/Speakers → Group view → Now Playing/Speakers/Queue). Include the ASCII mockups from the TUI brainstorm for the Home screen and Group > Now Playing screen.
4. **CLI: Command Reference** — Full command table from the architecture brainstorm. One-off commands, flat subcommands, targeting rules.
5. **Architecture Decisions** — Action enum pattern, discovery cache, sync SDK, no JSON output in v1.
6. **v1 Scope** — What's in, what's explicitly out (music library browsing, JSON output, TUI deep-link).
7. **Open Questions** — Config values (default group, cache TTL, TUI theme). These are planning-phase decisions.

**Key content to carry forward from brainstorms:**

From `2026-02-26-sonos-tui-brainstorm.md`:
- Groups-first navigation model
- Two-level tabbed navigation (Home tabs: Groups/Speakers; Group tabs: Now Playing/Speakers/Queue)
- Mini-player follows focus on overview, hides in group view
- Album art rendering modes (Sixel/Kitty, half-block, ASCII) — user-configurable
- Theming (Dark, Light, Neon, Sonos-branded)
- Global media keys regardless of active screen
- Context-sensitive key legend at bottom
- Breadcrumb navigation path

From `2026-03-01-sonos-cli-architecture-brainstorm.md`:
- `sonos` (no args) = TUI; subcommands = one-off commands
- Flat subcommands, flags over positional args
- `--speaker` / `--group` targeting, `--group` wins if both
- Discovery cache with 24h TTL, auto-rediscover on miss
- Plain text output (no JSON in v1)
- Shared Action + executor architecture

---

### 3. `docs/references/sonos-sdk.md`

**Purpose:** Complete public API reference for `sonos-sdk`. Enables wiring up SDK calls without reading the source.

**Status:** A 1069-line draft was written by a research agent to `docs/sonos-sdk.md`. It needs to be moved to `docs/references/sonos-sdk.md` and reviewed for:
- Accuracy against `../sonos-sdk/sonos-sdk/src/`
- Appropriate level of detail (remove internal implementation details)
- Good code examples for each major operation
- Clear organization matching how the CLI will use it

**Required sections (confirm all are present in the existing file):**

1. **Quick Start** — `SonosSystem::new()` → get speakers → call an action
2. **Initialization** — `new()` vs `from_discovered_devices()`
3. **Discovery** — `sonos_discovery::get()`, `get_with_timeout()`, `get_iter()`, `Device` struct
4. **SonosSystem** — All methods: `speakers()`, `groups()`, `get_speaker_by_name()`, `get_group_by_id()`, `get_group_for_speaker()`, `create_group()`, `iter()`
5. **Speaker API** — Playback, volume/EQ, seek, play mode, queue, sleep timer, grouping methods. Parameter types and return types.
6. **Group API** — Membership, volume, dissolve. `GroupChangeResult`.
7. **Property Handles** — `.get()`, `.fetch()`, `.watch()`, `.unwatch()`. Optimistic update model.
8. **Change Events** — `ChangeIterator`, `try_iter()`, `recv_timeout()`. Use with TUI event loop.
9. **Error Types** — `SdkError`, `ApiError`, `StateError` variants.
10. **Key Types** — `SpeakerId`, `GroupId`, `PlaybackState`, `CurrentTrack`, `Position`, `PlayMode`.

**Code examples must include:**
- Discovering and building a `SonosSystem`
- Finding a speaker by name and playing it
- Setting volume on a group
- Watching for property changes in a non-blocking TUI loop
- Handling `SdkError`

---

### 4. `docs/references/cli-guidelines.md`

**Purpose:** CLI design rules this project follows, derived from [clig.dev](https://clig.dev). Applied specifically to `sonos-cli`.

**Section outline:**

1. **Philosophy** — Human-first, composable, consistent. Why this matters for a media control tool.
2. **Output Rules** (applied to this project)
   - Success output → stdout, brief and human-readable
   - Errors and warnings → stderr only
   - No decoration on piped output (detect TTY)
   - Color: use intentionally, disable when `NO_COLOR` is set or not in a TTY
   - Progress: show spinner/indicator during discovery (>1s operation)
3. **Arguments & Flags** (applied to this project)
   - Always prefer `--flag` over positional args
   - Full flag names required; single-letter shortcuts only for very common flags
   - Standard flag names: `--help`, `--version`, `--quiet`
   - Our targeting convention: `--speaker NAME`, `--group NAME`; `--group` wins if both given
   - No flags for secrets/passwords
4. **Subcommands** (applied to this project)
   - Flat structure: `sonos <verb>` not `sonos <domain> <verb>`
   - Consistent verb tense (imperative: `play`, `pause`, `seek`, not `playing`, `pausing`)
   - No abbreviation: `sonos pl` is not valid; `sonos play` is required
5. **Exit Codes**
   - 0 = success
   - 1 = general error (speaker not found, command failed)
   - 2 = usage error (bad arguments, unknown flag)
6. **Configuration Precedence** (for this project)
   - Flags override everything
   - Environment variables (e.g., `SONOS_DEFAULT_GROUP`) override config file
   - Config file: `~/.config/sonos/config.toml`
   - Discovery cache: `~/.config/sonos/cache.json`
7. **Interactivity**
   - Only prompt in interactive TTY
   - `--no-input` flag disables all prompts
   - TUI mode only launches in a TTY; error gracefully if not a TTY
8. **Error Messages**
   - Errors go to stderr, one line
   - Format: `error: <human-readable description>`
   - Include what to do next: `Run 'sonos discover' to refresh the speaker list.`
   - Debug info: `--verbose` flag for stack traces / raw SDK errors

---

## Sources & References

### Origin Brainstorms
- **Architecture:** [docs/brainstorm/2026-03-01-sonos-cli-architecture-brainstorm.md](../brainstorm/2026-03-01-sonos-cli-architecture-brainstorm.md) — Key decisions carried forward: Action dispatch architecture, flat subcommands, clig.dev compliance, discovery cache with 24h TTL
- **TUI Design:** [docs/brainstorm/2026-02-26-sonos-tui-brainstorm.md](../brainstorm/2026-02-26-sonos-tui-brainstorm.md) — Key decisions carried forward: groups-first model, tabbed navigation, album art rendering, theming, global media keys

### External References
- CLI guidelines: https://clig.dev
- ratatui: https://ratatui.rs
- clap v4 derive: https://docs.rs/clap/latest/clap/_derive/

### Internal References
- SDK source: `../sonos-sdk/sonos-sdk/src/`
- Existing partial SDK doc: `docs/sonos-sdk.md` (written by research agent; move to `docs/references/sonos-sdk.md` and review)
