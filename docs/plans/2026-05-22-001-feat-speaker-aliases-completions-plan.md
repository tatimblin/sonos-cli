---
title: "feat: Add speaker/group aliases and dynamic shell completions"
status: active
origin: docs/brainstorms/2026-05-22-speaker-aliases-requirements.md
created: 2026-05-22
depth: standard
---

# feat: Add speaker/group aliases and dynamic shell completions

## Summary

Speaker/group names are long and tedious to type. This adds user-defined aliases (short names mapping to speaker/group names) and dynamic shell completions that surface speakers, groups, and aliases as Tab candidates. Delivered as two stacked PRs: aliases first, completions on top.

---

## Problem Frame

Every CLI invocation requiring a speaker or group target demands the full name with correct casing and quoting (`--speaker "Master Bedroom"`). Power users who repeatedly target the same speakers need a shorthand. Additionally, shell Tab completion currently only covers subcommands and flags — not speaker/group names, which are runtime-discovered values.

---

## Scope Boundaries

**In scope:**
- Alias CRUD via `sonos config alias` (list/set/clear)
- Alias resolution in all speaker/group resolution paths
- `-s` / `-g` short flags for `--speaker` / `--group`
- Dynamic shell completions for speaker names, group names, and aliases
- Storage in existing config.toml

**Out of scope (see origin: `docs/brainstorms/2026-05-22-speaker-aliases-requirements.md`):**
- Interactive fuzzy picker at runtime
- Alias namespacing or prefix syntax
- Aliases for command verbs
- JSON output for alias commands

### Deferred to Follow-Up Work

- None identified

---

## Key Technical Decisions

1. **`config` as a CLI nesting exception.** The flat-subcommand rule is relaxed for `sonos config` since it manages CLI configuration rather than controlling speakers. This is a deliberate, bounded exception (see origin).

2. **Alias-first resolution.** When `-s bed` is passed, check aliases before treating the value as a literal name. This allows aliases to shadow real names intentionally and keeps the fast path (alias hit) first.

3. **Dynamic completions via `CompleteEnv` (clap_complete `unstable-dynamic`).** The binary is re-invoked on each Tab press with `COMPLETE=$SHELL` set. `CompleteEnv` intercepts before normal parsing, runs candidate functions, prints matches, and exits. This is the standard pattern for CLIs with runtime-discovered values (docker, kubectl, gh all use this architecture). No separate `sonos completions [SHELL]` subcommand needed — user setup is `source <(COMPLETE=bash sonos)`. This diverges from the origin doc's proposed `sonos completions [SHELL]` interface; research confirmed dynamic completions are required for runtime-discovered values (speaker names from cache), which static script generation cannot provide.

4. **Config command bypasses discovery.** `sonos config alias` doesn't need speaker discovery, so it's dispatched in `main.rs` before `SonosSystem::new()` — avoiding a 2-3s SSDP penalty for config-only operations.

---

## Patterns to Follow

- `Queue` sub-subcommand pattern in `src/cli/commands.rs` (existing nesting with `QueueAction`)
- `Config::load()` / `Config::save()` pattern in `src/config.rs`
- Resolution logic in `src/cli/resolve.rs` (central choke point for all speaker/group lookups)
- Early-return dispatch pattern (will be new, but mirrors how the TUI branch in `main.rs` avoids unnecessary discovery)

---

## Implementation Units

### U1. Add `-s` / `-g` short flags to GlobalFlags

**Goal:** Enable `sonos pause -s "Master Bedroom"` as shorthand for `--speaker`.

**Requirements:** Prerequisite for alias ergonomics — aliases are short strings, so the flag should be short too.

**Dependencies:** None

**Files:**
- `src/cli/mod.rs`

**Approach:** Add `short = 's'` to the `speaker` arg attribute and `short = 'g'` to the `group` arg attribute in `GlobalFlags`.

**Patterns to follow:** Existing `short` usage on `--quiet` (`short`).

**Test scenarios:**
- Parse `sonos pause -s Kitchen` — `global.speaker` equals `"Kitchen"`
- Parse `sonos volume 50 -g "Living Room"` — `global.group` equals `"Living Room"`
- `-s` and `--speaker` are interchangeable
- `-g` and `--group` are interchangeable

**Verification:** `cargo build` succeeds; `sonos pause -s Kitchen` parses correctly.

---

### U2. Add aliases field and helper methods to Config

**Goal:** Store and resolve aliases in the config struct.

**Requirements:** Alias storage, resolution, set, and clear (see origin constraints).

**Dependencies:** None

**Files:**
- `src/config.rs`

**Approach:**
- Add `aliases: HashMap<String, String>` to `Config` (key = speaker/group name, value = alias)
- Serde: `#[serde(default, skip_serializing_if = "HashMap::is_empty")]`
- `resolve_alias(&self, input: &str) -> &str` — scan values for match, return key; else return input
- `set_alias(&mut self, name: &str, alias: &str)` — insert/replace
- `clear_alias(&mut self, name: &str) -> Option<String>` — remove and return old alias

**Patterns to follow:** Existing `Config::save()` serialization with `toml::to_string_pretty`.

**Test scenarios:**
- `resolve_alias("bed")` returns `"Master Bedroom"` when alias `"Master Bedroom" = "bed"` exists
- `resolve_alias("Kitchen")` returns `"Kitchen"` when no alias matches (passthrough)
- `set_alias("Master Bedroom", "bed")` followed by `resolve_alias("bed")` returns `"Master Bedroom"`
- `set_alias` on a name that already has an alias replaces the old one
- `clear_alias("Master Bedroom")` removes the alias and returns the old value
- `clear_alias` on a name with no alias returns `None`
- Aliases round-trip through `toml::to_string_pretty` / `toml::from_str`
- Empty `aliases` map is not serialized (skip_serializing_if works)

**Verification:** `cargo test` passes; config serialization includes `[aliases]` only when non-empty.

---

### U3. Add `Config` subcommand and `ConfigAction::Alias` enum

**Goal:** Define the CLI parsing structure for `sonos config alias`.

**Requirements:** Three-arity command: 0 args = list, 1 arg = clear, 2 args = set.

**Dependencies:** None

**Files:**
- `src/cli/commands.rs`

**Approach:** Add `Config` variant to `Commands` with `#[command(subcommand)] action: Option<ConfigAction>`. Add `ConfigAction::Alias { name: Option<String>, alias: Option<String> }`. Bare `sonos config` (no subcommand) prints help via clap default.

**Patterns to follow:** `Queue` / `QueueAction` pattern in same file.

**Test scenarios:**
- `sonos config alias` parses as `Config { action: Some(Alias { name: None, alias: None }) }`
- `sonos config alias "Master Bedroom" bed` parses with both fields populated
- `sonos config alias "Master Bedroom"` parses with name only
- `sonos config` alone shows help (no error)

**Verification:** `cargo build` succeeds; clap generates correct help text for `sonos config --help` and `sonos config alias --help`.

---

### U4. Wire alias resolution into resolve.rs

**Goal:** All speaker/group lookups transparently resolve aliases before SDK lookup.

**Requirements:** `sonos pause -s bed` works identically to `sonos pause --speaker "Master Bedroom"`.

**Dependencies:** U2

**Files:**
- `src/cli/resolve.rs`

**Approach:** In `resolve_speaker`, `resolve_group`, and `require_speaker_only`, call `config.resolve_alias()` on the flag value before passing to `system.speaker()` / `system.group()`. The resolved name flows through existing error handling unchanged.

**Test scenarios:**
- `resolve_speaker` with `global.speaker = "bed"` and alias configured resolves to the correct speaker
- `resolve_speaker` with `global.group = "lr"` and alias configured resolves to the correct group coordinator
- `resolve_speaker` with an alias that doesn't match any real speaker returns `SpeakerNotFound` with the resolved (full) name
- `require_speaker_only` resolves aliases before speaker lookup
- No alias configured — behavior unchanged (passthrough)

**Verification:** Existing resolve tests still pass; new alias-aware tests pass.

---

### U5. Add config command handler with early dispatch

**Goal:** Execute alias list/set/clear operations without requiring speaker discovery.

**Requirements:** Full CRUD for aliases with validation (see origin constraints).

**Dependencies:** U2, U3

**Files:**
- `src/cli/run.rs`
- `src/main.rs`

**Approach:**
- In `main.rs`, match `Commands::Config` before `SonosSystem::new()` and dispatch to a config handler. Make `config` mutable.
- Handler logic:
  - `Alias { name: None, .. }` → list all aliases (format: `alias → name`, one per line)
  - `Alias { name: Some(n), alias: None }` → clear alias for `n`, save config
  - `Alias { name: Some(n), alias: Some(a) }` → validate, set alias, save config
- Validation: alias non-empty, no whitespace, unique across all aliases → `CliError::Validation`
- Output format matches the origin doc examples

**Patterns to follow:** Existing `run_command` match arms; early-return pattern for TUI vs CLI in `main.rs`.

**Test scenarios:**
- Set alias: `sonos config alias "Master Bedroom" bed` → prints "Alias set: bed → Master Bedroom", config file updated
- List aliases: shows all configured aliases in `alias → name` format
- List when empty: prints "No aliases configured"
- Clear alias: removes from config, prints confirmation with the old alias
- Clear non-existent: returns validation error
- Validation: empty alias string → error
- Validation: alias with whitespace → error
- Validation: alias already used by another name → error
- Setting new alias on name that already has one → replaces old, prints new

**Verification:** Manual end-to-end: set → list → use with `-s` → clear → list shows empty.

---

### U6. Dynamic shell completions with `CompleteEnv`

**Goal:** Tab completion surfaces speaker names, group names, and aliases in bash/zsh/fish.

**Requirements:** Tab completion in supported shells shows speakers, groups, and aliases (see origin success criteria).

**Dependencies:** U2 (for alias loading in candidate functions)

**Files:**
- `Cargo.toml`
- `src/main.rs`
- `src/cli/mod.rs` (or new `src/cli/complete.rs`)

**Approach:**
- Add `features = ["unstable-dynamic"]` to `clap_complete` in Cargo.toml; make it non-optional
- Add `CompleteEnv::with_factory(Cli::command).complete()` at the very top of `main()` — this intercepts Tab-press re-invocations and exits before normal CLI flow
- Attach `ArgValueCandidates::new(speaker_candidates)` to `--speaker` and `ArgValueCandidates::new(group_candidates)` to `--group` in `GlobalFlags`
- Implement `speaker_candidates()`: load config for aliases, attempt `SonosSystem::new()` for speaker names, return `Vec<CompletionCandidate>` (aliases include help text showing the full name)
- Implement `group_candidates()`: same pattern but with group names
- Performance: `SonosSystem::new()` reads from cache (~instant file read); gracefully returns empty vec on failure

**Patterns to follow:** clap_complete engine API (`CompletionCandidate::new().help()`).

**Test scenarios:**
- `COMPLETE=bash sonos play --speaker ""` invocation returns speaker names and aliases
- `COMPLETE=bash sonos play --speaker "b"` returns only candidates starting with "b"
- Aliases appear with help text showing the full name
- `SonosSystem::new()` failure gracefully returns aliases-only (no crash)
- Static completion generation (`src/generate.rs`) still works unchanged

**Verification:** `source <(COMPLETE=bash sonos)` then `sonos play -s <Tab>` shows candidates. Existing `cargo run --features generate --example sonos-generate` still produces static completions.

---

## PR Structure

| PR | Branch | Base | Units |
|----|--------|------|-------|
| PR 1: Speaker/group aliases | `feat/speaker-aliases` | `main` | U1, U2, U3, U4, U5 |
| PR 2: Dynamic shell completions | `feat/shell-completions` | `feat/speaker-aliases` | U6 |

---

## System-Wide Impact

- **Config file format:** New `[aliases]` table in config.toml. Backward-compatible — missing table deserializes to empty HashMap via `serde(default)`.
- **`main.rs` control flow:** Config commands dispatch before discovery. Completions intercept before parsing. Both are early-exit paths that don't affect the existing TUI/CLI flow.
- **Dependency change:** `clap_complete` moves from optional to always-on (small binary size increase, ~50KB).

---

## Risks and Mitigations

| Risk | Mitigation |
|------|-----------|
| `CompleteEnv` API is marked `unstable-dynamic` | API has been stable in behavior since clap_complete 4.4; widely used by production CLIs. Pin to clap_complete 4.x. |
| Tab-press invokes `SonosSystem::new()` which might trigger slow SSDP | SDK cache (24h TTL) makes subsequent calls instant. First-ever Tab press may take ~3s — acceptable for one-time cost. |
| Alias shadowing real speaker names | Documented as intentional behavior. Alias-first resolution is the user's choice. |
