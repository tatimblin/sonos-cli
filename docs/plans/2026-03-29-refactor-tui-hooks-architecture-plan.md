---
title: "refactor: TUI hooks architecture for co-located widget state"
type: refactor
status: completed
date: 2026-03-29
origin: docs/brainstorms/2026-03-29-tui-hooks-architecture-brainstorm.md
roadmap: Milestone 7 (Home Screen) — architectural improvement to widget state management
---

# refactor: TUI hooks architecture for co-located widget state

## Overview

Replace the global `App.progress_states` and `App.watch_handles` pattern with a general-purpose hooks system that co-locates state, subscriptions, and animation with the widgets that use them. Three hook primitives — `use_state`, `use_watch`, `use_animation` — move widget concerns from scattered global state into the render functions where they belong.

This is an architectural refactor of the TUI rendering pipeline. No user-visible behavior changes. The progress bar bug (showing 0:00 on cold start) is **not** fixed here — that will be addressed in the SDK.

## Problem Statement

Progress bar state is scattered across three files and two lifecycle phases:

1. **`app.rs`** — `ProgressState` struct + `HashMap<GroupId, ProgressState>` on `App`
2. **`event.rs`** — `handle_change_event()` creates/updates `ProgressState` when SDK events arrive
3. **`home_groups.rs`** — reads `app.progress_states`, falls back to raw SDK position

This separation means:
- No `ProgressState` exists until the first SDK event (~200-500ms after launch)
- Adding progress to another screen (GroupView) requires duplicating event handler wiring
- The event loop interprets SDK events and manages widget-specific state — responsibilities that belong in the widget

(See brainstorm: `docs/brainstorms/2026-03-29-tui-hooks-architecture-brainstorm.md`)

## Proposed Solution

A `Hooks` struct with three primitives, threaded through render functions via `RenderContext`:

```rust
// src/tui/hooks.rs

pub struct Hooks { /* internal state */ }

impl Hooks {
    /// Persistent local state across renders. Returns &mut V.
    /// Call AFTER use_watch and use_animation to avoid borrow conflicts.
    pub fn use_state<V: 'static>(
        &mut self,
        key: &str,
        default: impl FnOnce() -> V,
    ) -> &mut V;

    /// Subscribe to SDK property, return current value (owned clone).
    /// Persistent WatchHandle stored internally — reused across frames.
    pub fn use_watch<P: SonosProperty + Clone + 'static>(
        &mut self,
        prop: &PropertyHandle<P>,
    ) -> Option<P>;

    /// Request periodic re-renders when active=true.
    /// Consolidates into one global animation timer.
    pub fn use_animation(&mut self, key: &str, active: bool);

    // Frame lifecycle
    pub fn begin_frame(&mut self);
    pub fn end_frame(&mut self);
    pub fn has_active_animations(&self) -> bool;
}
```

Threaded via `RenderContext`:

```rust
// src/tui/hooks.rs

pub struct RenderContext<'a> {
    pub app: &'a App,
    pub hooks: &'a mut Hooks,
}
```

## Technical Approach

### Architecture

#### Key Design Decisions (from brainstorm)

| Decision | Choice | Rationale |
|----------|--------|-----------|
| State identity | `(TypeId::of::<V>(), key: String)` | Prevents type collisions, simple string keys |
| Ownership | `RenderContext { &App, &mut Hooks }` | Separates read (app) from write (hooks) |
| Watch handles | Persistent, not clear-and-reacquire | Fewer UPnP subscription toggles |
| Animation timer | Global consolidated, 250ms | Matches current behavior, simple |
| State cleanup | Mark-and-sweep after each frame | Automatic, mirrors React unmount |
| Event flow | WatchHandle reads directly, event loop only dirty-marks | No event interpretation in event loop |

#### Borrow Checker Strategy

The critical Rust constraint: `use_state` returns `&mut V` which borrows `&mut self` on `Hooks`. While this borrow is held, no other hook methods can be called.

**Solution: Calling convention.** Widgets call hooks in this order:

1. `use_watch` — returns owned `Option<V>` (cloned from handle), borrow released immediately
2. `use_animation` — takes `&mut self` briefly, borrow released
3. `use_state` — returns `&mut V`, must be called last or in a scoped block

```rust
// This compiles because use_watch returns owned values
let playback = ctx.hooks.use_watch(&coordinator.playback_state); // borrow released
let position = ctx.hooks.use_watch(&coordinator.position);       // borrow released
let is_playing = playback.map_or(false, |p| p == PlaybackState::Playing);

ctx.hooks.use_animation(&format!("{}:tick", group.id), is_playing); // borrow released

// use_state last — holds &mut borrow until `progress` goes out of scope
let progress = ctx.hooks.use_state::<ProgressState>(
    &format!("{}:progress", group.id),
    ProgressState::default,
);
if let Some(pos) = &position {
    progress.update(pos.position_ms, pos.duration_ms, is_playing);
}
let elapsed = progress.interpolated_position_ms();
let duration = progress.last_duration_ms;
// progress dropped here, &mut borrow released
```

#### Hooks Internal Storage

```rust
// src/tui/hooks.rs (internal)

use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};

#[derive(Hash, Eq, PartialEq, Clone)]
struct HookKey {
    type_id: TypeId,
    name: String,
}

pub struct Hooks {
    // use_state storage
    states: HashMap<HookKey, Box<dyn Any>>,

    // use_watch storage — type-erased WatchHandle<P>
    watches: HashMap<String, Box<dyn Any>>,

    // use_animation — active animation keys
    animations: HashSet<String>,

    // Mark-and-sweep tracking
    accessed_states: HashSet<HookKey>,
    accessed_watches: HashSet<String>,
    accessed_animations: HashSet<String>,
}
```

#### use_watch Key Derivation

`use_watch` keys are auto-derived from the `PropertyHandle`'s identity — `(speaker_id, property_key)`. Two widgets watching the same property **share one handle** (the second call reuses the stored handle and reads its current value). This is more efficient and matches the SDK's ref-counting model.

```rust
fn watch_key<P: SonosProperty>(prop: &PropertyHandle<P>) -> String {
    format!("{}:{}", prop.speaker_id(), P::KEY)
}
```

#### use_watch Failure Behavior

If `prop.watch()` returns `Err`, fall back to `prop.get()` (matching current `app.watch()` behavior). No `WatchHandle` is stored. The key is still marked as accessed to prevent eviction of any previous successful watch.

#### Event Loop Integration

The event loop changes from 4 phases to a cleaner model:

```
Phase 1: Render when dirty
    hooks.begin_frame()                    // reset access tracking
    terminal.draw(|frame| {
        let ctx = RenderContext { app: &app, hooks: &mut hooks };
        ui::render(frame, ctx)
    })
    hooks.end_frame()                      // evict unaccessed state + handles
    app.dirty = false

Phase 2: Poll terminal events (50ms)
    Key/resize → handle_key(app, key), dirty = true

Phase 3: Drain SDK events (dirty-marking only)
    for _ in change_iter.try_iter() {
        app.dirty = true;                  // no handle_change_event — just mark dirty
    }

Phase 4: Animation tick
    if hooks.has_active_animations() {     // replaces has_active_animation(app)
        if last_animation >= 250ms {
            app.dirty = true;
        }
    }
```

#### Navigation and Cleanup

When the user navigates (e.g., Home → GroupView), the old screen's widgets stop rendering. Their hooks are not called on the next frame. `end_frame()` mark-and-sweep evicts the unaccessed state and drops watch handles. When navigating back, hooks are recreated from defaults. The brief resubscription latency (~50-200ms) is acceptable.

### Implementation Phases

#### Phase 1: Build `Hooks` module

Create `src/tui/hooks.rs` with the core types:

- [x]`Hooks` struct with internal storage maps — `src/tui/hooks.rs`
- [x]`HookKey` type for state identity — `src/tui/hooks.rs`
- [x]`RenderContext` struct — `src/tui/hooks.rs`
- [x]`use_state<V>(&mut self, key, default) -> &mut V` — `src/tui/hooks.rs`
- [x]`begin_frame()` / `end_frame()` with mark-and-sweep — `src/tui/hooks.rs`
- [x]Unit tests for `use_state`: create, persist across frames, evict on unaccessed frame — `src/tui/hooks.rs`

**Success criteria:** `use_state` creates, persists, and evicts state correctly in unit tests. No ratatui dependency.

#### Phase 2: Add `use_watch`

- [x]`use_watch<P>(&mut self, prop) -> Option<P>` with persistent WatchHandle storage — `src/tui/hooks.rs`
- [x]Watch key auto-derivation from `PropertyHandle` identity — `src/tui/hooks.rs`
- [x]Fallback to `prop.get()` on watch failure — `src/tui/hooks.rs`
- [x]Mark-and-sweep for watch handles (drop unaccessed handles in `end_frame`) — `src/tui/hooks.rs`
- [x]`use_watch_group<P>(&mut self, prop) -> Option<P>` for `GroupPropertyHandle` — `src/tui/hooks.rs`

**Success criteria:** `use_watch` creates persistent handles, reuses them across frames, and drops them when unaccessed.

#### Phase 3: Add `use_animation`

- [x]`use_animation(&mut self, key, active)` — `src/tui/hooks.rs`
- [x]`has_active_animations(&self) -> bool` for event loop — `src/tui/hooks.rs`
- [x]Mark-and-sweep for animation keys — `src/tui/hooks.rs`

**Success criteria:** `has_active_animations()` returns true when any widget registers an active animation, false when none do.

#### Phase 4: Integrate with event loop and render pipeline

- [x]Add `hooks: Hooks` as standalone variable in `run_event_loop_inner` — `src/tui/event.rs`
- [x]Add `begin_frame()` / `end_frame()` calls around `terminal.draw()` — `src/tui/event.rs`
- [x]Change `terminal.draw()` closure to create `RenderContext` from `&app` + `&mut hooks` — `src/tui/event.rs`
- [x]Replace `has_active_animation(app)` with `hooks.has_active_animations()` — `src/tui/event.rs`
- [x]Simplify SDK event drain: remove `handle_change_event()`, keep only dirty-marking — `src/tui/event.rs`
- [x]Update `ui::render()` signature to take `RenderContext` instead of `&App` — `src/tui/ui.rs`
- [x]Update header/footer render functions — `src/tui/ui.rs`
- [x]Declare `hooks` module in `src/tui/mod.rs`

**Success criteria:** TUI compiles with new event loop structure. Old `app.watch()` and new `ctx.hooks.use_watch()` may coexist temporarily.

#### Phase 5: Migrate screens and widgets

- [x]Migrate `home_groups.rs` — replace `app.watch()` with `ctx.hooks.use_watch()`, replace `app.progress_states` with `ctx.hooks.use_state()`, add `ctx.hooks.use_animation()` — `src/tui/screens/home_groups.rs`
- [x]Migrate `home_speakers.rs` — replace `app.watch()` with `ctx.hooks.use_watch()` — `src/tui/screens/home_speakers.rs`
- [x]Update `group_card::render_group_card()` if signature changes — `src/tui/widgets/group_card.rs`
- [x]Update `modal::render_modal()` if signature changes — `src/tui/widgets/modal.rs`
- [x]Update key handlers to work with new App (no progress_states) — `src/tui/handlers/home.rs`
- [x]Update `GroupView` / `SpeakerDetail` stubs if affected — `src/tui/handlers/group.rs`

**Success criteria:** All screens render correctly using hooks. No `app.watch()` calls remain.

#### Phase 6: Cleanup

- [x]Remove `App.progress_states: HashMap<GroupId, ProgressState>` — `src/tui/app.rs`
- [x]Remove `App.watch_handles: RefCell<Vec<Box<dyn Any>>>` — `src/tui/app.rs`
- [x]Remove `App::watch()` and `App::watch_group()` methods — `src/tui/app.rs`
- [x]Remove `App::clear_watch_handles()` — `src/tui/app.rs`
- [x]Remove `handle_change_event()` function — `src/tui/event.rs`
- [x]Remove `has_active_animation()` function — `src/tui/event.rs`
- [x]Move `ProgressState` from `app.rs` to `hooks.rs` or a dedicated module — `src/tui/app.rs`
- [x]Verify `cargo clippy` passes, no dead code warnings

**Success criteria:** No global widget state remains on `App`. All widget-specific state lives in `Hooks` via hooks.

## System-Wide Impact

### Interaction Graph

`use_watch(prop)` → `prop.watch()` → SDK `EventManager::acquire_watch()` → UPnP SUBSCRIBE → speaker callback → `StateManager` updates property → `WatchHandle.value()` returns new value on next frame.

`use_animation(key, true)` → `Hooks.animations.insert(key)` → `has_active_animations()` returns true in event loop → `app.dirty = true` every 250ms → triggers re-render → `use_state` returns `ProgressState` → `interpolated_position_ms()` advances.

### Error Propagation

- `use_watch` failures (SDK subscription errors) silently fall back to `prop.get()`. No user-visible error. Logged via `tracing::warn!`.
- Render panics are terminal (ratatui panic hook restores terminal). No recovery needed for hooks state.

### State Lifecycle Risks

- **Mark-and-sweep false eviction:** If a widget conditionally skips `use_watch` (e.g., coordinator is `None`), the handle drops. When the coordinator returns, a new subscription is established with ~50-200ms latency. This is acceptable — same behavior as current screen transitions.
- **Topology changes:** When `group_membership` fires, the old group's widgets stop rendering. Mark-and-sweep evicts their state. New group widgets create fresh state. No stale data risk.

### API Surface Parity

All render functions change signature from `(&Frame, Rect, &App, &ScreenState)` to `(&Frame, Rect, &mut RenderContext, &ScreenState)`. This is a mechanical change across 6 functions.

## Acceptance Criteria

- [x]`Hooks` struct implements `use_state`, `use_watch`, `use_animation` with mark-and-sweep cleanup
- [x]`RenderContext` wraps `&App` + `&mut Hooks` and is passed to all render functions
- [x]Event loop uses `begin_frame()` / `end_frame()` lifecycle
- [x]Event loop drains SDK events for dirty-marking only — no `handle_change_event()`
- [x]Progress bar works identically to current behavior (same animation, same interpolation)
- [x]Watch handles are persistent (not cleared each frame)
- [x]`App` no longer has `progress_states`, `watch_handles`, `watch()`, or `watch_group()` methods
- [x]`cargo clippy` and `cargo test` pass
- [x]TUI renders correctly after navigation transitions (push/pop/tab switch)

## Files Changed

| File | Change |
|------|--------|
| `src/tui/hooks.rs` | **New** — Hooks, RenderContext, HookKey, all three hooks |
| `src/tui/mod.rs` | Add `pub mod hooks;` |
| `src/tui/app.rs` | Remove progress_states, watch_handles, watch methods. Move ProgressState out. |
| `src/tui/event.rs` | New event loop structure, remove handle_change_event, remove has_active_animation |
| `src/tui/ui.rs` | Update render() and helpers to take RenderContext |
| `src/tui/screens/home_groups.rs` | Migrate to hooks |
| `src/tui/screens/home_speakers.rs` | Migrate to hooks |
| `src/tui/handlers/home.rs` | Remove progress_states references if any |
| `src/tui/handlers/group.rs` | Update if affected by signature changes |
| `src/tui/widgets/group_card.rs` | Signature update if needed |
| `src/tui/widgets/modal.rs` | Signature update if needed |

## Sources & References

### Origin

- **Brainstorm document:** [docs/brainstorms/2026-03-29-tui-hooks-architecture-brainstorm.md](docs/brainstorms/2026-03-29-tui-hooks-architecture-brainstorm.md) — Key decisions carried forward: typed composite keys for state identity, RenderContext ownership model, persistent watch handles with mark-and-sweep, WatchHandle-reads-directly event flow, global consolidated animation timer.

### Internal References

- Previous watch migration plan: `docs/plans/2026-03-28-refactor-watch-api-migration-plan.md`
- Current App state: `src/tui/app.rs:17-34`
- Current event loop: `src/tui/event.rs:29-96`
- Current watch pattern: `src/tui/app.rs:62-103`
- ProgressState: `src/tui/app.rs:107-145`
- SDK WatchHandle: `../sonos-sdk/sonos-sdk/src/property/handles.rs:117-160`
- SDK PropertyHandle: `../sonos-sdk/sonos-sdk/src/property/handles.rs:269-443`
- WatchHandle grace period docs: `../sonos-sdk/docs/WATCH_GRACE_PERIOD_DEMO.md`
