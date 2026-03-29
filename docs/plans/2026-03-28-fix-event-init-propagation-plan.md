---
title: "fix: Move EventInitFn to StateManager to fix TUI watch() propagation"
type: fix
status: completed
date: 2026-03-28
origin: docs/brainstorms/2026-03-28-fix-event-init-propagation-brainstorm.md
---

# fix: Move EventInitFn to StateManager to fix TUI watch() propagation

## Overview

The TUI shows all speakers as "not playing" with no volume because every `watch()` call falls back to `CacheOnly` mode. The root cause: speakers obtained via `Group::coordinator()` and `Group::members()` are created with `event_init: None`, so the lazy event manager initialization never triggers.

The fix moves `EventInitFn` from per-speaker `SpeakerContext` to `StateManager` as the single source of truth. Since the init closure is identical for ALL speakers (it's a system-level concern), centralizing it eliminates all propagation gaps at once.

(see brainstorm: `docs/brainstorms/2026-03-28-fix-event-init-propagation-brainstorm.md`)

## Problem Statement

Tracing instrumentation revealed the chain:

1. TUI renders → calls `app.watch(&coordinator.playback_state)`
2. `coordinator` comes from `group.coordinator()` → `Speaker::new()` → `event_init: None`
3. `PropertyHandle::watch()` checks `self.context.event_init` → finds `None`
4. Falls back to `CacheOnly` mode → no UPnP subscriptions → no events → stale UI

Six propagation gaps exist in the current code:

| Gap | Location | What happens |
|-----|----------|-------------|
| `Group::coordinator()` | `group.rs:148` | `Speaker::new()` → `event_init: None` |
| `Group::members()` | `group.rs:172` | `Speaker::new()` → `event_init: None` |
| `Group::speaker()` | `group.rs` | `Speaker::new()` → `event_init: None` |
| `Speaker::group()` chain | `speaker.rs` | Returns Group with no init fn knowledge |
| `try_rediscover()` | `system.rs:391` | Passes `None` for `event_init` |
| `GroupPropertyHandle::watch()` | `handles.rs:873` | No init fn trigger at all |

## Proposed Solution

Move `EventInitFn` storage from `SpeakerContext` to `StateManager`. This means:

- **Zero changes to Group, Speaker, or SpeakerContext constructors** — no threading through factories
- **Any speaker from any code path gets access automatically** — via the shared `Arc<StateManager>`
- **Single source of truth** — remove the redundant `event_init` field from `SpeakerContext`
- **GroupPropertyHandle::watch() gets init logic for free** — it already has `state_manager`

### Crate Boundary: Type Definition

**Critical constraint:** `EventInitFn` is currently defined in `sonos-sdk` and references `SdkError`. But `StateManager` lives in `sonos-state`, which cannot depend on `sonos-sdk` (circular dependency).

**Solution:** Define a new type alias in `sonos-state` using a generic error:

```rust
// sonos-state/src/state.rs
pub type EventInitFn = Arc<dyn Fn() -> Result<(), Box<dyn std::error::Error + Send + Sync>> + Send + Sync>;
```

The closure in `system.rs` already returns `Result<(), SdkError>`, and `SdkError` implements `std::error::Error`, so it converts automatically via `Box<dyn Error>`.

### Arc Reference Cycle

Storing the init fn on `StateManager` creates a reference cycle: `StateManager` → `EventInitFn` → `Arc<StateManager>`. This is acceptable because:

- `StateManager` is process-lifetime (never dropped until exit)
- The `OnceLock` pattern means the fn is called at most once, then the event manager takes over
- No memory leak concern in practice

## Technical Approach

### Step 1: Add EventInitFn storage to StateManager

**File:** `sonos-state/src/state.rs`

- [x] Define `EventInitFn` type alias (with `Box<dyn Error>` return)
- [x] Add `event_init: OnceLock<EventInitFn>` field to `StateManager`
- [x] Initialize to empty `OnceLock::new()` in `StateManager::new()`
- [x] Add `set_event_init(&self, f: EventInitFn)` setter method
- [x] Add `event_init(&self) -> Option<&EventInitFn>` getter method
- [x] Re-export `EventInitFn` from crate root

### Step 2: Update PropertyHandle::watch() to use StateManager

**File:** `sonos-sdk/src/property/handles.rs`

- [x] Remove `event_init` field from `SpeakerContext` (line 40)
- [x] Remove `SpeakerContext::with_event_init()` constructor (lines 61-75)
- [x] Update `PropertyHandle::watch()` (line 355): resolve init fn from `self.context.state_manager.event_init()` instead of `self.context.event_init`
- [x] Map the `Box<dyn Error>` to `SdkError` at the call site
- [x] Add lazy init trigger to `GroupPropertyHandle::watch()` (line 873) — same pattern as speaker watch

### Step 3: Simplify Speaker constructors

**File:** `sonos-sdk/src/speaker.rs`

- [x] Remove `Speaker::new_with_event_init()` method (lines 229-263)
- [x] Update `Speaker::new()` to use `SpeakerContext::new()` directly (no init fn parameter)

### Step 4: Update SonosSystem to set init fn on StateManager

**File:** `sonos-sdk/src/system.rs`

- [x] In `from_devices_inner()`: after creating `init_fn`, call `state_manager.set_event_init(init_fn.clone())` (after line 194)
- [x] Adapt the `init_fn` closure to return `Box<dyn Error>` instead of `SdkError`
- [x] Simplify `build_speakers_with_init()` — remove `event_init` parameter entirely
- [x] Fix `try_rediscover()` (line 391) — no longer needs to pass `None`; speakers automatically get access via StateManager
- [x] Remove `EventInitFn` type alias from `handles.rs` (now lives in `sonos-state`)
- [x] Update the `use` import in `system.rs` to use `sonos_state::EventInitFn`

### Step 5: Update old EventInitFn references

**File:** `sonos-sdk/src/property/handles.rs`

- [x] Remove or update the `EventInitFn` type alias (line 26) — it now lives in `sonos-state`
- [x] Update any `use` imports that reference the old location

### Step 6: Verify and test

- [x] `cargo check` across all SDK crates (no compile errors)
- [x] `cargo test --lib` on SDK (all 464 unit tests pass)
- [x] `cargo clippy` on SDK (no warnings)
- [x] `cargo fmt --check` on SDK
- [x] `cargo check` on CLI
- [x] `cargo test` on CLI (all 37 tests pass)
- [x] Manual TUI test: run `sonos -vvv`, verify log shows:
  - "Lazy-initializing event manager (first watch() call)"
  - "Event worker started"
  - `watch()` resolves to `Events` mode (not `CacheOnly`)
  - TUI displays live playback state, volume, and track info

## Acceptance Criteria

- [x] `EventInitFn` is defined in `sonos-state`, not `sonos-sdk`
- [x] `SpeakerContext` has no `event_init` field
- [x] `Speaker::new_with_event_init()` no longer exists
- [x] `build_speakers_with_init()` has no `event_init` parameter
- [x] `PropertyHandle::watch()` resolves init fn from `StateManager`
- [x] `GroupPropertyHandle::watch()` triggers lazy init (same as speaker watch)
- [x] `try_rediscover()` does not pass `None` for event_init
- [x] TUI `watch()` calls resolve to `Events` mode
- [x] All existing tests pass without modification

## Scope

**SDK files changed:**

| File | Changes |
|------|---------|
| `sonos-state/src/state.rs` | Add `EventInitFn` type, `OnceLock` field, setter/getter |
| `sonos-state/src/lib.rs` | Re-export `EventInitFn` |
| `sonos-sdk/src/property/handles.rs` | Remove `event_init` from `SpeakerContext`, add init trigger to `GroupPropertyHandle::watch()`, remove old type alias |
| `sonos-sdk/src/speaker.rs` | Remove `new_with_event_init()`, simplify `new()` |
| `sonos-sdk/src/system.rs` | Set init fn on StateManager, simplify `build_speakers_with_init()`, fix `try_rediscover()` |

**CLI files changed:** None — TUI automatically benefits from SDK fix.

## Sources

- **Origin brainstorm:** [docs/brainstorms/2026-03-28-fix-event-init-propagation-brainstorm.md](docs/brainstorms/2026-03-28-fix-event-init-propagation-brainstorm.md) — Key decisions: move EventInitFn to StateManager exclusively, remove per-speaker storage, single source of truth
- **Tracing evidence:** `~/.local/share/sonos/sonos.log` — every watch() call shows "No event_init closure available" and "falling back to cache-only mode"
- **Related fix:** `docs/plans/2026-03-28-fix-event-router-registration-race-plan.md` — event router race condition (separate issue)
