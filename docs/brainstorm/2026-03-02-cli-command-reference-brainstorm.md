---
date: 2026-03-02
topic: cli-command-reference
roadmap: Milestones 2–5 (CLI — Discovery, Playback, Volume/EQ/Grouping, Queue)
---

# CLI Command Reference Documentation

## What We're Building

A dedicated `docs/references/cli-commands.md` that serves as the authoritative human-readable reference for every planned `sonos` CLI command. Unlike the table in `docs/goals.md` (which is a brief summary), this document gives developers and users the full picture: exact syntax, all accepted flags, example invocations, expected output, and error behavior per command.

The document covers all **planned** commands — everything in `docs/goals.md`'s CLI Command Reference — not just the commands currently wired in `src/cli/mod.rs`. It is aspirational-but-final: it describes v1 scope, so nothing in it will become out-of-date as implementation catches up.

## Why This Approach

### Location: `docs/references/cli-commands.md`

Follows the existing `docs/references/` convention where `cli-guidelines.md` already lives. Keeps reference material separate from goals/brainstorm/product docs. Consistent with how the project organizes authoritative documentation.

### Format: Detailed per-command entries (not a table)

The goals.md table is too brief to use as a build reference — it omits flag details, example output, and error cases. A full-detail format matches how tools like `gh` or `kubectl` document their CLI surface.

Each command entry will include:
- **Syntax line** — copy-pasteable, showing all arguments and optional flags
- **Description** — one-sentence plain-language summary
- **Arguments/Flags** — name, type, default, whether required
- **Example invocations** — 1–3 realistic bash examples
- **Success output** — what stdout shows on success
- **Error cases** — common failures with the error format from `cli-guidelines.md`

### Scope: All planned commands

Documenting planned commands (not just wired ones) makes this the build reference for Milestones 2–5. It avoids maintaining two separate documents as implementation progresses.

## Key Decisions

- **File:** `docs/references/cli-commands.md`
- **Scope:** All v1 CLI commands (27 total, across 6 categories)
- **Format:** Markdown with H3 per command, code blocks for syntax and examples
- **clig.dev alignment:** Every entry reflects the rules in `cli-guidelines.md` — flags over positional args, `--group` wins over `--speaker`, EQ commands are speaker-only, errors use the `error: <msg>\n<hint>` format
- **Output examples:** Use the same tone as `cli-guidelines.md` examples (brief, lowercase confirmation)
- **Does not duplicate:** `cli-guidelines.md` covers design rules; this doc covers per-command facts

## Commands to Document (27 total)

### Discovery & System (4)
`discover`, `speakers`, `groups`, `status`

### Playback (7)
`play`, `pause`, `stop`, `next`, `prev`, `seek <HH:MM:SS>`, `mode <...>`

### Volume & EQ (6)
`volume <0-100>`, `mute`, `unmute`, `bass <-10..10>`, `treble <-10..10>`, `loudness <on|off>`

### Queue (3)
`queue`, `queue add <URI>`, `queue clear`

### Grouping (2)
`join`, `leave`

### Sleep Timer (2)
`sleep <DURATION>`, `sleep cancel`

### Global Flags (covered in intro)
`--speaker NAME`, `--group NAME`, `--quiet / -q`, `--verbose`, `--no-input`, `--help / -h`, `--version / -v`

## Open Questions

None — scope is fully defined.

## Next Steps

→ Create `docs/references/cli-commands.md` with full per-command documentation
