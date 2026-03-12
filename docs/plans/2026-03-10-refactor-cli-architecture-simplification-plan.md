---
title: "refactor: Simplify CLI architecture — remove Action/executor, improve SDK interface"
type: refactor
status: completed
date: 2026-03-10
origin: docs/brainstorms/2026-03-10-cli-architecture-simplification-brainstorm.md
milestone: "Milestone 2: CLI — Discovery & System Commands"
---

# Simplify CLI Architecture

## Overview

Remove the Action enum and executor indirection from sonos-cli. Make the SDK the only shared layer — CLI commands call SDK methods directly from clap handlers. Tighten the SDK's public API so consumers never see discovery internals.

(see brainstorm: docs/brainstorms/2026-03-10-cli-architecture-simplification-brainstorm.md)

## Problem Statement

The CLI wraps every SDK call in a 4-file pipeline: `Commands → into_action() → Action → execute()`. The Action enum maps 1:1 to SDK methods with zero cross-cutting logic. The executor is entirely stubbed — no real SDK calls. This adds indirection without value. The SDK already has a clean DOM-like API (`system.get_speaker_by_name("Kitchen")?.play()`) that the CLI should use directly.

Additionally, the SDK exposes `sonos_discovery::Device` and `from_discovered_devices()` publicly, forcing consumers to manage discovery internals they shouldn't see.

## Proposed Solution

### Phase 1: SDK Interface Changes

Tighten the SDK public API and add missing ergonomics.

#### 1.1 Remove public discovery internals

- [x] Make `from_discovered_devices()` `pub(crate)` in `sonos-sdk/src/system.rs:112`
- [x] Remove `pub use sonos_discovery;` from `sonos-sdk/src/lib.rs:93`

#### 1.2 Add `get_group_by_name()`

- [x] Add `pub fn get_group_by_name(&self, name: &str) -> Option<Group>` to `SonosSystem`
- [x] Call `self.ensure_topology()` first (same as `groups()`)
- [x] Match groups by coordinator speaker name (groups don't have independent names in Sonos)

#### 1.3 Add test-support feature

- [x] Add `[features]` section to `sonos-sdk/Cargo.toml`: `test-support = []`
- [x] Add `#[cfg(feature = "test-support")] pub fn with_speakers(names: &[&str]) -> Self` to `SonosSystem`
- [x] Constructor creates `StateManager` without event manager, builds speakers from names with synthetic IPs
- [x] Make `_event_manager` field `Option<Arc<SonosEventManager>>` to support test mode (no socket binding)

**Key technical detail:** `SonosEventManager::new()` binds a network socket. The test constructor must skip this. Making the field `Option<_>` is the cleanest approach — the event manager is only used to keep subscriptions alive; test code doesn't need it.

### Phase 2: CLI Simplification

Remove the indirection layer and call SDK directly.

#### 2.1 Delete dead code

- [x] Delete `src/actions.rs`
- [x] Delete `src/executor.rs`
- [x] Remove `mod actions;` and `mod executor;` from `src/main.rs`

#### 2.2 Refactor `cli/mod.rs`

- [x] Add `pub fn run(&self, system: &SonosSystem, config: &Config) -> Result<String, CliError>` to `Commands`
- [x] Each variant resolves its target and calls SDK methods directly
- [x] Move target resolution into helpers: `resolve_speaker()` returns `&Speaker`, `resolve_group()` returns `&Group`
- [x] Remove `into_action()` method and `Action`/`Target` imports

Example of the new pattern:

```rust
// src/cli/mod.rs
impl Commands {
    pub fn run(&self, system: &SonosSystem, config: &Config) -> Result<String, CliError> {
        match self {
            Commands::Play { speaker, group } => {
                let spk = resolve_speaker(system, config, speaker.as_deref(), group.as_deref())?;
                spk.play()?;
                Ok(format!("Playing on {}", spk.name))
            }
            Commands::Speakers => {
                let speakers = system.speakers();
                // format speaker list for stdout
                Ok(format_speakers(&speakers))
            }
            // ...
        }
    }
}
```

#### 2.3 Update `main.rs`

- [x] Replace `cmd.into_action()` + `executor::execute(action, &system, &config)` with `cmd.run(&system, &config)`
- [x] Remove unused imports

#### 2.4 Update tests

- [x] Add `sonos-sdk = { ..., features = ["test-support"] }` to CLI `[dev-dependencies]`
- [x] Rewrite CLI tests to use `SonosSystem::with_speakers(&["Kitchen", "Bedroom"])`
- [x] Test `Commands::run()` directly instead of `into_action()` + `execute()`
- [x] Keep `errors.rs` tests unchanged (independent of this refactor)

### Phase 3: Documentation

- [x] Update CLAUDE.md Rule 1: replace "Action dispatch only" with direct SDK call pattern
- [x] Update CLAUDE.md Module Structure: remove `actions.rs`, `executor.rs`
- [x] Update CLAUDE.md Project Overview: remove "both modes go through the same Action enum"
- [x] Update `docs/product/roadmap.md`: mark executor/Action items as superseded

## Technical Considerations

**Event manager in tests:** The `SonosSystem` struct currently requires `_event_manager: Arc<SonosEventManager>`. The test-support constructor needs this to be `Option<_>` to avoid binding a network socket. This is a one-field struct change. The event manager is prefixed `_` already — it's only held to keep the subscription thread alive.

**Group name semantics:** Sonos groups don't have independent names. `get_group_by_name("Living Room")` matches the coordinator speaker's name. This matches how the CLI's `--group` flag is documented.

**Speaker re-export check:** `Speaker::from_device()` takes `&Device`. Since `Device` becomes internal, verify this method isn't used outside the SDK. (Research confirmed: only called from `SonosSystem` internals.)

## Acceptance Criteria

### SDK
- [x] `SonosSystem::new()` is the only public constructor
- [x] `sonos_discovery` is not re-exported
- [x] `get_group_by_name()` works with `ensure_topology()`
- [x] `with_speakers()` creates an in-memory system with no network calls
- [x] SDK compiles and all 40+ existing tests pass

### CLI
- [x] `src/actions.rs` and `src/executor.rs` do not exist
- [x] `Commands::run()` calls SDK methods directly
- [x] All CLI tests pass using `SonosSystem::with_speakers()`
- [x] `CLAUDE.md` reflects the new architecture
- [x] `cargo test` passes with 0 failures

## Dependencies & Risks

| Risk | Mitigation |
|------|------------|
| `Option<EventManager>` changes struct layout | Field is `_`-prefixed, only held alive — no callers access it |
| Existing SDK tests use `from_discovered_devices()` | Stays `pub(crate)` — internal tests unaffected |
| CLI tests can't construct real speakers | `with_speakers()` provides test doubles |
| Group name resolution may not find matches | `ensure_topology()` already handles lazy fetch; fall back to empty |

## Sources & References

- **Origin brainstorm:** [docs/brainstorms/2026-03-10-cli-architecture-simplification-brainstorm.md](../brainstorms/2026-03-10-cli-architecture-simplification-brainstorm.md) — Key decisions: delete Action/executor, SDK is the shared layer, `new()` is the only constructor
- CLI architecture: `src/main.rs`, `src/actions.rs`, `src/executor.rs`, `src/cli/mod.rs`
- SDK public API: `sonos-sdk/src/lib.rs`, `sonos-sdk/src/system.rs`
- Roadmap: `docs/product/roadmap.md` (Milestones 1-2)
- CLAUDE.md: Rules 1 (Action dispatch), Module Structure, Project Overview
