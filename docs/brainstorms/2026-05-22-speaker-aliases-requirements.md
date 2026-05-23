# Speaker & Group Aliases

**Date:** 2026-05-22
**Status:** Ready for planning
**Scope:** Two PRs — alias management, then shell completions

---

## Problem

Speaker and group names can be long (`Master Bedroom`, `Living Room Surround`). Every CLI invocation requires typing the full name with correct casing and quoting. This creates friction for power users who interact with the same speakers repeatedly.

## Solution

### PR 1: Alias Management

User-defined short names that map to speaker or group names. Stored in config, resolved transparently during speaker/group lookup.

**Commands:**

| Invocation | Behavior |
|------------|----------|
| `sonos config alias` | List all aliases |
| `sonos config alias [SPEAKER/GROUP NAME] [ALIAS]` | Set alias for a speaker or group |
| `sonos config alias [SPEAKER/GROUP NAME]` | Clear alias for that speaker/group |

**Examples:**
```
$ sonos config alias "Master Bedroom" bed
Alias set: bed → Master Bedroom

$ sonos config alias
bed → Master Bedroom
kit → Kitchen

$ sonos pause -s bed
# Resolves 'bed' → 'Master Bedroom', pauses that speaker

$ sonos config alias "Master Bedroom"
Alias cleared: bed → Master Bedroom
```

**Storage:** New `[aliases]` table in `~/.config/sonos/config.toml`:
```toml
default_group = "Living Room"
theme = "default"

[aliases]
"Master Bedroom" = "bed"
"Kitchen" = "kit"
```

**Resolution order:**
1. Check if the value matches a defined alias → resolve to the full speaker/group name
2. If no alias match, treat as a literal speaker/group name (existing behavior)

This applies to both `--speaker` and `--group` flags. Aliases work for speakers and groups interchangeably — the flag determines how the resolved name is looked up.

**`config` as a CLI exception:** The flat-subcommand rule (`sonos <verb>`) is relaxed for `config` since it's a meta-concern (managing CLI configuration), not a speaker action. This is a deliberate exception, not precedent for nesting speaker commands.

**Constraints:**
- Alias values must be non-empty and not contain whitespace
- A speaker/group can have at most one alias
- An alias string must be unique across all aliases
- Setting a new alias for a speaker that already has one replaces the old alias

### PR 2: Shell Completions

Generate completion scripts for bash, zsh, and fish that include speaker names, group names, and defined aliases as candidates.

**Command:**
```
sonos completions [SHELL]
# Outputs completion script to stdout for sourcing
```

**Completion sources:**
- All known speaker names (from discovery cache)
- All known group names
- All defined aliases

**Fuzzy matching:** Leverages each shell's native fuzzy/approximate matching (e.g., zsh's `zstyle ':completion:*' matcher-list`). The CLI generates the candidate list; the shell handles the matching UX.

## Non-Goals

- No interactive fuzzy picker at runtime (considered, deferred)
- No alias namespacing or prefix syntax (`@alias`)
- No aliases for command verbs — only speaker/group targets
- No JSON output for alias commands in v1

## Success Criteria

- `sonos pause -s bed` works identically to `sonos pause --speaker "Master Bedroom"` when alias is configured
- Aliases persist across sessions (stored in config file)
- Tab completion in supported shells shows speakers, groups, and aliases
- Existing behavior unchanged when no aliases are configured
