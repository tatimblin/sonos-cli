---
title: "docs: Add CLI command reference"
type: docs
status: completed
date: 2026-03-02
origin: docs/brainstorm/2026-03-02-cli-command-reference-brainstorm.md
---

# docs: Add CLI command reference

## Overview

Create `docs/references/cli-commands.md` — the authoritative per-command reference for every planned `sonos` CLI command. This document fills the gap between the brief command table in `docs/goals.md` and the design-rules reference in `docs/references/cli-guidelines.md`. It serves as the build reference for Milestones 2–5 and the end-user reference once the binary is complete.

Implements: Milestones 2–5 documentation (Discovery & System, Playback, Volume/EQ/Grouping, Queue).

## Problem Statement / Motivation

`docs/goals.md` has a one-line table per command — not enough to build from or to hand to a user. Engineers implementing Milestones 2–5 need exact flag names, accepted argument ranges, expected output formats, and error behavior for each command. Users need the same to use the tool. This plan produces that document.

## Proposed Solution

A single Markdown file at `docs/references/cli-commands.md` following the same style as `docs/references/cli-guidelines.md` (H1/H2/H3 headings, tables for flags, `bash` code blocks for examples, prose before each block, no YAML frontmatter).

Each command gets a dedicated `###` section with:
- **Syntax** — copy-pasteable usage line
- **Description** — one sentence
- **Arguments & Flags** — table of name / type / required / default / description
- **Examples** — 1–3 realistic `bash` blocks
- **Output** — what stdout prints on success
- **Errors** — 1–2 common failures in the `error: <msg>\n<hint>` format from `cli-guidelines.md`

## Technical Considerations

### Exact argument types (from `src/actions.rs` and `src/cli/mod.rs`)

| Command | Positional arg | Type | Range |
|---|---|---|---|
| `volume` | `<LEVEL>` | `u8` | 0–100 |
| `seek` | `<POSITION>` | `String` | `HH:MM:SS` (single-digit hour OK: `0:02:30`) |
| `mode` | `<MODE>` | enum string | `normal`, `repeat`, `repeat-one`, `shuffle`, `shuffle-no-repeat` |
| `bass` | `<LEVEL>` | `i8` | -10–10 |
| `treble` | `<LEVEL>` | `i8` | -10–10 |
| `loudness` | `<STATE>` | bool string | `on`, `off` |
| `join` | (none) | — | requires both `--speaker` and `--group` |
| `leave` | (none) | — | requires `--speaker` |
| `sleep` | `<DURATION>` | `String` | e.g. `30m`, `1h`, `90m` |
| `queue add` | `<URI>` | `String` | Sonos URI |

### Targeting rules (from `cli-guidelines.md`)

- `--group NAME` wins over `--speaker NAME` when both are provided
- Neither provided → default group from `config.default_group`
- EQ commands (`bass`, `treble`, `loudness`) are **speaker-only** — `--group` not accepted
- `join` and `leave` always require explicit `--speaker`

### Global flags (Milestone 10 scope, document now)

| Flag | Short | Description |
|---|---|---|
| `--help` | `-h` | Show help and exit |
| `--version` | `-v` | Print version and exit |
| `--quiet` | `-q` | Suppress non-error stdout output |
| `--verbose` | — | Show raw SDK errors and debug output |
| `--no-input` | — | Disable all interactive prompts (for scripts) |

### `PlayMode` enum mapping (from `actions.rs`)

The `actions::PlayMode` variants map to these CLI values:

| CLI value | `actions::PlayMode` |
|---|---|
| `normal` | `Normal` |
| `repeat` | `RepeatAll` |
| `repeat-one` | `RepeatOne` |
| `shuffle` | `Shuffle` |
| `shuffle-no-repeat` | `ShuffleNoRepeat` |

(`ShuffleRepeatOne` variant exists in code but is not exposed in the CLI per `docs/goals.md`.)

### Reference doc style conventions (from `docs/references/cli-guidelines.md`)

- No YAML frontmatter
- H1 for document title, H2 for category sections, H3 for individual commands
- Prose paragraph before each table or code block
- `bash` language tag on all shell examples
- `rust` language tag for Rust snippets
- Tables use `---` alignment separators

## Commands to Document (27 total)

### Discovery & System (4)

| Command | Syntax | Notes |
|---|---|---|
| `discover` | `sonos discover` | No flags. Runs SSDP, writes `~/.config/sonos/cache.json`. Shows progress spinner. |
| `speakers` | `sonos speakers` | No flags. Lists all cached speakers with state and volume. |
| `groups` | `sonos groups` | No flags. Lists all cached groups with state and volume. |
| `status` | `sonos status [--speaker NAME \| --group NAME]` | Reports current track, playback state, and volume for the targeted group. |

### Playback (7)

All accept `[--speaker NAME | --group NAME]`.

| Command | Syntax |
|---|---|
| `play` | `sonos play [--speaker NAME \| --group NAME]` |
| `pause` | `sonos pause [--speaker NAME \| --group NAME]` |
| `stop` | `sonos stop [--speaker NAME \| --group NAME]` |
| `next` | `sonos next [--speaker NAME \| --group NAME]` |
| `prev` | `sonos prev [--speaker NAME \| --group NAME]` |
| `seek` | `sonos seek <HH:MM:SS> [--speaker NAME \| --group NAME]` |
| `mode` | `sonos mode <normal\|repeat\|repeat-one\|shuffle\|shuffle-no-repeat> [--speaker NAME \| --group NAME]` |

### Volume & EQ (6)

| Command | Syntax | Scope |
|---|---|---|
| `volume` | `sonos volume <0-100> [--speaker NAME \| --group NAME]` | Speaker or group |
| `mute` | `sonos mute [--speaker NAME \| --group NAME]` | Speaker or group |
| `unmute` | `sonos unmute [--speaker NAME \| --group NAME]` | Speaker or group |
| `bass` | `sonos bass <-10..10> --speaker NAME` | Speaker only |
| `treble` | `sonos treble <-10..10> --speaker NAME` | Speaker only |
| `loudness` | `sonos loudness <on\|off> --speaker NAME` | Speaker only |

### Queue (3)

| Command | Syntax |
|---|---|
| `queue` | `sonos queue [--speaker NAME \| --group NAME]` |
| `queue add` | `sonos queue add <URI> [--speaker NAME \| --group NAME]` |
| `queue clear` | `sonos queue clear [--speaker NAME \| --group NAME]` |

Note: `queue` is a subcommand with optional sub-subcommands (`add`, `clear`). No subcommand defaults to showing the queue.

### Grouping (2)

| Command | Syntax |
|---|---|
| `join` | `sonos join --speaker NAME --group NAME` |
| `leave` | `sonos leave --speaker NAME` |

### Sleep Timer (2)

| Command | Syntax |
|---|---|
| `sleep` | `sonos sleep <DURATION>` |
| `sleep cancel` | `sonos sleep cancel` |

`sleep` is a subcommand; `cancel` is a sub-subcommand. Duration format: `30m`, `1h`, `90m`.

## Acceptance Criteria

- [x] `docs/references/cli-commands.md` created at the correct path
- [x] Document follows the style of `docs/references/cli-guidelines.md` (no frontmatter, H1/H2/H3, tables with prose, bash code blocks)
- [x] All 27 commands documented (4 discovery, 7 playback, 6 volume/EQ, 3 queue, 2 grouping, 2 sleep, plus global flags intro)
- [x] Every command entry includes: syntax line, one-sentence description, flags table, at least one example, success output, at least one error case
- [x] EQ commands correctly note speaker-only scope (no `--group` flag)
- [x] `join` and `leave` correctly note that `--speaker` is required
- [x] `queue` sub-subcommand structure is clearly explained
- [x] `sleep cancel` syntax is clearly documented
- [x] Global flags section documents all 5 flags (`--help`, `--version`, `--quiet`, `--verbose`, `--no-input`)
- [x] Targeting section explains `--group` wins over `--speaker`, and default group fallback
- [x] Argument ranges match code: `volume` 0–100 `u8`, `bass`/`treble` -10–10 `i8`, `seek` `HH:MM:SS`, `loudness` `on|off`
- [x] PlayMode CLI values match the mapping table above
- [x] Error format follows `cli-guidelines.md`: `error: <msg>\n<hint>`
- [x] Output examples are brief and lowercase (matching `cli-guidelines.md` tone)
- [x] No contradiction with `docs/goals.md` or `docs/references/cli-guidelines.md`

## Dependencies & Risks

- No code changes required — this is pure documentation
- Content is derived from `docs/goals.md`, `docs/references/cli-guidelines.md`, `src/actions.rs`, and `src/cli/mod.rs`
- Low risk: document is aspirational (describes planned commands), so it won't go stale as Milestones 2–5 are implemented

## Sources & References

### Origin

- **Brainstorm document:** [docs/brainstorm/2026-03-02-cli-command-reference-brainstorm.md](../brainstorm/2026-03-02-cli-command-reference-brainstorm.md) — Key decisions carried forward: (1) file lives at `docs/references/cli-commands.md`; (2) scope is all planned commands, not just wired ones; (3) format matches existing reference doc style

### Internal References

- Goals & command table: [`docs/goals.md` — CLI Command Reference section](../goals.md)
- CLI design rules: [`docs/references/cli-guidelines.md`](../references/cli-guidelines.md)
- All Action variants: [`src/actions.rs`](../../src/actions.rs)
- Wired CLI commands: [`src/cli/mod.rs`](../../src/cli/mod.rs)
- Milestone scope: [`docs/product/roadmap.md`](../product/roadmap.md)
