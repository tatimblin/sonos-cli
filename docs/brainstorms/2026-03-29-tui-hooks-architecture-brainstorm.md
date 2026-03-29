# TUI Hooks Architecture Brainstorm

**Date:** 2026-03-29
**Status:** Draft
**Trigger:** Progress bar shows 0:00 on launch instead of actual position; broader desire to co-locate widget state

## What We're Building

A general-purpose hooks system for the TUI that moves widget state from global (`App`) to local (co-located with the widget that uses it). Modeled after React hooks but adapted for Rust's ownership model and ratatui's immediate-mode rendering.

**Three hooks** form the initial API surface:

| Hook | Purpose | Example |
|------|---------|---------|
| `use_state<V>(key, default)` | Persistent local state across renders | Progress interpolation state |
| `use_watch(property_handle)` | Subscribe to SDK property, return current value | Position, playback state, volume |
| `use_animation(key, active)` | Request periodic re-renders when active | Progress bar ticking while playing |

## Why This Approach

**Problem:** Progress state lives globally in `App.progress_states` — a `HashMap<GroupId, ProgressState>`. The event loop's `handle_change_event` manually updates this map when SDK events arrive. The rendering code in `home_groups.rs` reads from it. This separation means:

1. **Cold start bug:** No `ProgressState` exists until the first SDK position event arrives (~200-500ms), so the bar shows `0:00`
2. **Scattered logic:** Position interpolation state is in `App`, subscription is in screen render, event handling is in the event loop — three files for one widget's concern
3. **No reuse:** If another screen needs progress (e.g., GroupView), it must replicate the same global state + event handler wiring

**Solution:** A hooks system that co-locates subscription, state, and animation with the widget. The progress bar widget owns its own state, subscribes to the properties it needs, and requests animation ticks — all in one place.

## Key Decisions

### 1. State Identity: Typed Composite Keys

Keys combine the value type (`TypeId`) with a user-provided hashable key. This prevents collisions between different state types while keeping keys explicit.

```rust
// Internally keyed by (TypeId::of::<V>(), hash_of(key))
hooks.use_state::<ProgressState>((group_id, "progress"), ProgressState::default);
```

### 2. Ownership: RenderContext

`RenderContext` wraps `&App` (read) and `&mut Hooks` (write), passed to all render functions. This separates immutable app data from mutable hook state, satisfying Rust's borrow checker.

```rust
struct RenderContext<'a> {
    app: &'a App,
    hooks: &'a mut Hooks,
}
```

Replaces the current pattern where `&App` is passed to render functions and `app.watch()` handles subscriptions.

### 3. Hook Surface: Three Primitives

- **`use_state<V>(key, default)`** — Returns `&mut V` from persistent storage. Created on first call, persisted across renders. Replaces `App.progress_states`.
- **`use_watch(property_handle)`** — Subscribes to an SDK property, returns `Option<V>`. Replaces `app.watch()`.
- **`use_animation(key, active)`** — When `active=true`, marks the app dirty on a timer. Replaces the `has_active_animation()` check in the event loop.

### 4. No Explicit Event Bus

Widgets don't need to emit typed events. Instead:
- `use_watch` handles SDK event subscriptions transparently
- `use_animation` handles timer-based re-renders
- `use_state` mutations are visible on the next render

The hooks system internally manages dirty-marking — no `AppEvent` enum needed.

### 5. Event Flow: WatchHandle Reads Directly

`use_watch` stores a persistent SDK `WatchHandle` internally. Each frame, it reads `handle.value()` to get the latest value — the SDK updates the handle's value via its internal UPnP subscription mechanism. No event draining needed for state updates.

The event loop still drains `system.iter().try_iter()`, but **only for dirty-marking** (so the TUI knows to re-render). It no longer interprets events or updates state — that responsibility moves to `use_watch` + `use_state` in the render phase.

```
Frame N:
  use_watch(&speaker.position)
    → stored WatchHandle internally
    → returns handle.value()

Between frames:
  SDK receives UPnP event
    → updates WatchHandle's internal value
    → sends ChangeEvent to system.iter()
  Event loop drains system.iter() → marks dirty

Frame N+1:
  use_watch(&speaker.position)
    → reuses stored WatchHandle
    → returns handle.value() (now updated)
```

### 6. General-Purpose Design

The hooks system is not SDK-specific. `use_state` and `use_animation` work for any widget state. `use_watch` is the only SDK-aware hook.

### 7. Frame Lifecycle

`Hooks` requires explicit frame boundaries so mark-and-sweep works:

- **Before render:** `hooks.begin_frame()` — resets access tracking
- **During render:** widgets call `use_state`, `use_watch`, `use_animation` — each marks its key as accessed
- **After render:** `hooks.end_frame()` — evicts unaccessed state and drops unaccessed watch handles

## How It Relates to the Original Bug

The cold start issue (progress showing 0:00) will be fixed in the SDK — `watch()` will return accurate initial values without needing a `fetch()` workaround in the CLI. This hooks architecture doesn't solve that bug directly, but it does improve the situation:

- **`use_state` persists immediately** — `ProgressState` is created on the first render frame, not deferred until the first SDK event. Once the SDK delivers accurate initial values through `watch()`, the hooks architecture is ready to use them from frame one.

## Before/After

**Before** (home_groups.rs + app.rs + event.rs):
```rust
// Screen (home_groups.rs): subscribe + read global state
let position = app.watch(&coordinator.position);
let (progress, elapsed_ms, duration_ms) =
    if let Some(ps) = app.progress_states.get(&group.id) {
        (ps.interpolated_position_ms(), ...)
    } else if let Some(pos) = position.as_ref() {
        (pos.progress(), ...)
    } else {
        (0.0, 0, 0)  // BUG: cold start shows 0:00
    };

// Event loop (event.rs): handle SDK events
fn handle_change_event(app: &mut App, event: &ChangeEvent) {
    match event.property_key {
        "position" => { /* update App.progress_states */ }
        "playback_state" => { /* update App.progress_states */ }
        _ => {}
    }
}
```

**After** (all in the widget/screen render):
```rust
// Everything co-located in the render function
let playback = ctx.hooks.use_watch(&coordinator.playback_state);
let position = ctx.hooks.use_watch(&coordinator.position);
let is_playing = playback.map_or(false, |p| p == PlaybackState::Playing);

let progress = ctx.hooks.use_state::<ProgressState>(
    (group.id.clone(), "progress"),
    ProgressState::default,
);

if let Some(pos) = &position {
    progress.update(pos.position_ms, pos.duration_ms, is_playing);
}

ctx.hooks.use_animation((group.id.clone(), "tick"), is_playing);

let elapsed = progress.interpolated_position_ms();
let duration = progress.last_duration_ms;
```

## Resolved Questions

1. **Watch handle lifecycle:** `use_watch` keeps handles persistent — alive as long as the widget calls it each frame. No clear-and-reacquire cycle. Simpler, fewer UPnP subscription toggles. Handles drop naturally when a widget stops calling `use_watch` for a given key (detected by mark-and-sweep).

2. **Animation granularity:** All `use_animation` calls consolidate into one global timer. If any widget is animating, one global tick marks dirty. Matches current behavior, simple, and widgets don't need independent rates.

3. **State cleanup:** Mark-and-sweep. Track which keys were accessed each frame. After render, evict any state not accessed. Automatic cleanup, mirrors React's unmount behavior. This also handles watch handle cleanup — unaccessed watches get dropped.
