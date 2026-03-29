---
title: "refactor: Migrate TUI and docs to WatchHandle RAII API"
type: refactor
status: completed
date: 2026-03-28
deepened: 2026-03-28
roadmap: Milestone 7 (Home Screen) — updates existing watch lifecycle code
---

# refactor: Migrate TUI and docs to WatchHandle RAII API

## Enhancement Summary

**Deepened on:** 2026-03-28
**Sections enhanced:** 5 (WatchCache, widgets, event loop, docs, lifecycle)
**Research agents used:** architecture-strategist, performance-oracle, code-simplicity-reviewer, pattern-recognition-specialist, julik-frontend-races-reviewer, best-practices-researcher (React patterns), Context7 (ratatui, React useSyncExternalStore)

### Key Improvements

1. **Simple clear-before-draw** — event loop clears old handles, `draw()` re-acquires them. The 50ms grace period is 1000x longer than a text-mode render, so subscriptions are always reacquired in time. No double-buffer, no small-terminal guard.
2. **Non-blocking render path** — `app.watch()` never calls `fetch()`. Cold cache returns `None`; the SDK subscription delivers data within ~50-200ms, triggering a re-render. Blocking SOAP calls stay out of `terminal.draw()`.
3. **Progress state bootstrapping** — `handle_change_event` uses `entry().or_insert_with()` to self-heal after topology changes

---

## Overview

The `sonos-sdk` watch API changed from explicit `watch()`/`unwatch()` with a
`WatchStatus<P>` return type to an RAII-based `WatchHandle<P>`. This plan
updates the TUI code and all documentation to match the new API.

Alongside the API migration, we're making an architectural improvement: **move
watch calls from centralized lifecycle functions into the widgets that need the
data**. Instead of `setup_watches()` / `teardown_watches()` managing
subscriptions globally, each widget calls `app.watch(&property)` inline where
it reads the value. A handle pool on `App` holds the handles between renders
to keep subscriptions alive.

This follows the same pattern as React's `useSyncExternalStore` hook — each
component declares its own subscriptions at render time, the framework manages
subscription lifecycle automatically, and stale subscriptions are cleaned up
when components unmount (or in our case, when screens transition).

**What changed in the SDK:**

| Before | After |
|--------|-------|
| `watch()` returns `WatchStatus<P>` | `watch()` returns `WatchHandle<P>` |
| Value via `status.current: Option<P>` | Value via `handle.value() -> Option<&P>` |
| `unwatch()` method on handle | Dropping `WatchHandle` starts 50ms grace period |
| `WatchStatus` is a plain struct | `WatchHandle` is `#[must_use]`, not `Clone`, implements `Deref<Target = Option<P>>` |

## Proposed Solution

### 1. Handle storage on App (`src/tui/app.rs`)

Replace `watch_registry: HashSet<(SpeakerId, &'static str)>` with a bare
`RefCell<Vec<Box<dyn Any>>>` field. No wrapper struct needed — a type alias
and two methods on `App` are sufficient.

```rust
// src/tui/app.rs
use std::any::Any;
use std::cell::RefCell;

pub struct App {
    // Remove: pub watch_registry: HashSet<(SpeakerId, &'static str)>,

    /// Current frame's watch handles. Widgets push into this during render.
    /// Cleared before each render cycle; widgets repopulate via app.watch().
    watch_handles: RefCell<Vec<Box<dyn Any>>>,

    // ... everything else unchanged
}
```

**Why `RefCell`:** `terminal.draw()` takes `&App` (immutable borrow), but
`app.watch()` must mutate the handle vec. `RefCell` provides interior
mutability. This is safe because the TUI is single-threaded — `borrow_mut()`
is only called from widget render code, which runs sequentially within a
single `draw()` call. No concurrent borrows are possible.

**Why `Box<dyn Any>`:** Watch handles are generic (`WatchHandle<Volume>`,
`WatchHandle<PlaybackState>`, etc.). We need a heterogeneous collection to
hold them all. `Box<dyn Any>` is the standard Rust idiom for "type-erased
drop bag" — we don't need to downcast, just hold the handles alive until drop.

Add `watch()` and `watch_group()` helper methods on `App`:

```rust
use sonos_sdk::{PropertyHandle, GroupPropertyHandle, WatchHandle};
use sonos_state::property::SonosProperty;

impl App {
    /// Watch a speaker property — returns current value, keeps subscription alive.
    ///
    /// Call this in widget rendering code wherever you need a property value.
    /// The returned WatchHandle is stored internally and kept alive until the
    /// next render cycle.
    ///
    /// Returns `None` on cold cache (first watch before any event arrives).
    /// The SDK subscription will deliver data within ~50-200ms, setting
    /// `app.dirty` via `handle_change_event`, which triggers a re-render
    /// with the populated value. Widgets already handle `None` gracefully.
    ///
    /// Note: borrows `watch_handles` mutably via RefCell. Do not hold a
    /// separate borrow on `watch_handles` when calling this.
    pub fn watch<P>(&self, prop: &PropertyHandle<P>) -> Option<P>
    where
        P: SonosProperty + Clone + 'static,
    {
        match prop.watch() {
            Ok(wh) => {
                let val = wh.value().cloned();
                self.watch_handles.borrow_mut().push(Box::new(wh));
                val
            }
            Err(_) => prop.get(),
        }
    }

    /// Watch a group property — returns current value, keeps subscription alive.
    pub fn watch_group<P>(&self, prop: &GroupPropertyHandle<P>) -> Option<P>
    where
        P: SonosProperty + Clone + 'static,
    {
        match prop.watch() {
            Ok(wh) => {
                let val = wh.value().cloned();
                self.watch_handles.borrow_mut().push(Box::new(wh));
                val
            }
            Err(_) => prop.get(),
        }
    }

    /// Drop all watch handles — called by event loop before render.
    /// Starts grace periods; widgets re-acquire during draw(), cancelling them.
    fn clear_watch_handles(&mut self) {
        self.watch_handles.get_mut().clear();
    }
}
```

### Research Insights — Handle Storage

**No `fetch()` in render path:** The old `setup_watches()` called `fetch()` on
cold cache, but it ran *outside* the render loop. Putting `fetch()` inside
`terminal.draw()` would block the event loop for 5-20ms per property — with 5
groups x 4 properties, that's 100-400ms of frozen UI. Instead, `app.watch()`
returns `None` on cold cache. The SDK subscription delivers data within ~50-200ms,
triggering a re-render. One blank frame is imperceptible. If the SDK's cold-cache
latency ever becomes a UX problem, that's best solved in the SDK (e.g., initial
event delivery), not by blocking the TUI render path.

**Silent fallback on watch failure:** Matches the current `setup_watches()`
behavior at `event.rs:131` which uses `if let Ok(status) = ...` — silently
falls back. Writing to stderr via `eprintln!` during crossterm raw mode corrupts
the TUI display, so we avoid it.

**`clear_watch_handles` takes `&mut self`:** The event loop has `&mut App`, so
we use `get_mut()` to access the vec directly — no `RefCell` overhead. `RefCell`
is only needed inside `watch()`/`watch_group()` where `&self` is the constraint
(called from widgets during `terminal.draw()` which borrows `&App`).

**No wrapper struct:** The simplicity reviewer correctly identified that
`WatchCache` adds a layer of indirection for two methods. A bare field with
methods on `App` is clearer.

---

### 2. Clear-before-draw (`src/tui/event.rs`)

Clear old handles before `draw()`. Grace periods start, but `draw()` immediately
re-acquires handles via `app.watch()`, cancelling the grace periods. The 50ms
grace period is ~1000x longer than a text-mode ratatui render cycle (which writes
to an in-memory buffer and diffs — microseconds of CPU work). No double-buffer
or small-terminal guard needed.

```rust
// In run_event_loop_inner:
if app.dirty {
    app.clear_watch_handles();                          // old handles drop → grace periods start
    terminal.draw(|frame| ui::render(frame, app))?;     // widgets call app.watch() → grace periods cancelled
    app.dirty = false;
}
```

**Why this is safe:** `clear()` and `draw()` are sequential in the same function.
The grace period thread sleeps 50ms before checking its cancellation flag. The
`draw()` call completes in microseconds (text widgets, cell-level diffing),
re-acquiring handles for the same `(ip, service)` pairs, which increments the
SDK's ref count and sets the cancellation flag. The grace period thread wakes
after 50ms, finds the flag set, and exits without unsubscribing.

**Small terminal:** When `ui::render()` early-returns (terminal < 20×4), no
widgets run, no `app.watch()` calls happen, and grace periods expire after 50ms.
This is fine — when the terminal grows back, the `Resize` event sets `dirty`,
the next render re-acquires all subscriptions. The brief subscription gap during
resize is imperceptible.

---

### 3. Widget changes — inline watch calls (`src/tui/screens/`)

**`home_groups.rs`** — replace `get()` with `app.watch()`:

```rust
// Before (lines 63-66):
let playback_state = coordinator.playback_state.get();
let current_track = coordinator.current_track.get();
let position = coordinator.position.get();
let group_volume = group.volume.get();

// After:
let playback_state = app.watch(&coordinator.playback_state);
let current_track = app.watch(&coordinator.current_track);
let position = app.watch(&coordinator.position);
let group_volume = app.watch_group(&group.volume);
```

**`home_speakers.rs`** — replace `get()` with `app.watch()`:

```rust
// Line 89 — member volume in grouped speakers:
let volume = app.watch(&member.volume).map(|v| v.value() as u16).unwrap_or(0);

// Line 138 — standalone speaker volume:
let volume = app.watch(&speaker.volume).map(|v| v.value() as u16).unwrap_or(0);
```

### Research Insights — Widget Ergonomics

**Developer experience:** The call site reads naturally — `app.watch(&speaker.volume)`
is declarative ("I need this value, keep it live"). The widget doesn't know or care
about subscription lifecycle, handle storage, or grace periods. This matches the
React hooks principle: "describe what you need, let the framework manage how."

**No behavioral change at widget level:** Widgets still receive `Option<P>`.
The only difference is the source — `watch()` + cache instead of `get()` from
cold cache. Widgets don't need conditional logic for the migration.

---

### 4. Event loop changes (`src/tui/event.rs`)

**Delete `setup_watches()`** — no longer needed. Widgets self-subscribe during render.

**Delete `teardown_watches()`** — handled by `clear_watch_handles()` + not re-subscribing.

**Delete `setup_watches_if_groups_tab()`** — same reason.

**Remove tab-transition watch lifecycle** — the `was_groups_tab` / `is_groups_tab`
check block in the key handler becomes unnecessary.

**Replace render block** with clear-before-draw (section 2 above).

**Simplify `group_membership` handler:**

```rust
"group_membership" => {
    // Topology changed — next render will re-watch new groups automatically.
    // Just clear progress states and mark dirty.
    app.progress_states.clear();
}
```

**Progress state bootstrapping:** The old `setup_watches()` eagerly initialized
`ProgressState` entries for each group. The new design relies on
`handle_change_event` — but it currently only *updates* existing entries via
`get_mut()`, never *inserts* new ones. Fix: change the `position` and
`playback_state` match arms to use `entry().or_insert_with()`:

```rust
"position" => {
    if let Some(speaker) = app.system.speaker_by_id(&event.speaker_id) {
        if let Some(pos) = speaker.position.get() {
            if let Some(group) = speaker.group() {
                let ps = app.progress_states
                    .entry(group.id.clone())
                    .or_insert_with(|| ProgressState::new(0, 0, false));
                ps.last_position_ms = pos.position_ms;
                ps.last_duration_ms = pos.duration_ms;
                ps.wall_clock_at_last_update = Instant::now();
            }
        }
    }
}
"playback_state" => {
    if let Some(speaker) = app.system.speaker_by_id(&event.speaker_id) {
        if let Some(state) = speaker.playback_state.get() {
            if let Some(group) = speaker.group() {
                let ps = app.progress_states
                    .entry(group.id.clone())
                    .or_insert_with(|| ProgressState::new(0, 0, false));
                let now_playing = state.is_playing();
                if ps.is_playing && !now_playing {
                    ps.last_position_ms = ps.interpolated_position_ms();
                    ps.wall_clock_at_last_update = Instant::now();
                }
                ps.is_playing = now_playing;
                if now_playing {
                    ps.wall_clock_at_last_update = Instant::now();
                }
            }
        }
    }
}
```

This self-heals after topology changes: the first `position` or `playback_state`
event for a new group creates its `ProgressState` entry automatically. No eager
initialization needed.

### Research Insights — Event Loop

**Batching already correct:** The TUI event loop already implements batching
via `try_iter()` — it drains all pending SDK events before marking dirty and
rendering once. This is equivalent to React 18's automatic batching where
multiple `setState` calls in the same event handler produce a single re-render.
No changes needed here.

**Single-threaded prevents tearing:** Because the TUI runs on one thread,
there's no risk of "subscription tearing" (reading partially-updated state).
The `clear → render` sequence is atomic from the subscription's perspective.

---

### 5. SDK docs (`docs/references/sonos-sdk.md`)

**Section 5.1 — PropertyHandle<P>:**
- Remove `pub fn unwatch(&self)` from method list
- Change `watch()` return type: `WatchStatus<P>` → `WatchHandle<P>`

**Section 5.2 — GroupPropertyHandle<P>:**
- Same changes as 5.1 — remove `unwatch()`, update `watch()` return type

**Section 5.3 — Method semantics table:**
- Remove `unwatch()` row
- Update `watch()` description: "Returns a `WatchHandle`; hold it to keep the subscription alive. Dropping starts a 50ms grace period."

**Section 5.4 — Rename to "WatchHandle<P>":**
- Replace `WatchStatus<P>` struct definition with:

```rust
#[must_use]
pub struct WatchHandle<P> {
    // Internal: value, mode, cleanup guard
}

impl<P> WatchHandle<P> {
    pub fn value(&self) -> Option<&P>
    pub fn has_value(&self) -> bool
    pub fn mode(&self) -> WatchMode
    pub fn has_realtime_events(&self) -> bool
}

impl<P> Deref for WatchHandle<P> {
    type Target = Option<P>;
}
```

- Add: "Not `Clone` — each handle is one subscription hold. Dropping starts a 50ms grace period before the UPnP subscription is torn down. Re-calling `watch()` within the grace period cancels it and reuses the existing subscription."

**Section 8 — Change-event iteration example:**
- Update to hold watch handles: `let _vol = speaker.volume.watch()?;`
- Remove any `unwatch()` calls

**Complete example (end of doc):**
- Update to use `_vol` binding pattern and `vol.value()` accessors

### 6. Product docs

**`docs/product/roadmap.md`:**
- Line 522: `unwatch() all of the above` → `Watch handles dropped automatically when leaving tab`
- Line 588: `Unwatch when leaving` → `Watch handles dropped when leaving`
- Line 655: `Unwatch on leaving` → `Watch handles dropped on leaving`

**`docs/product/prd.md`:**
- Line 96: `unwatches stale ones` → `drops stale watch handles`

## How it works — lifecycle walkthrough

1. **Render frame N**: `clear_watch_handles()` drops old handles → grace periods
   start. `draw()` runs immediately — widgets call `app.watch(&prop)` → new
   `WatchHandle` acquired, grace periods cancelled, subscription reused.

2. **First render (cold cache)**: Widget calls `app.watch(&prop)` → `WatchHandle`
   acquired but `value()` returns `None` (no event received yet). Widget renders
   with `None` (shows placeholder / defaults). The SDK subscription delivers data
   within ~50-200ms → `handle_change_event` sets `app.dirty` → next render has
   the populated value.

3. **Idle between frames**: Handles persist in `watch_handles` vec →
   subscriptions alive → SDK events flow into `ChangeIterator` →
   `handle_change_event` sets `app.dirty`.

4. **Screen transition away from Groups**: Next render clears handles, but
   widgets for the new screen don't call `app.watch()` on group properties →
   grace periods start with no new handle to cancel → subscriptions expire
   after 50ms.

5. **Navigate back to Groups**: Widgets call `app.watch()` again → new
   subscriptions created. First frame may show `None` for ~50-200ms until
   SDK events arrive. No explicit setup/teardown needed.

6. **Small terminal** (below 20×4): `clear_watch_handles()` runs but `ui::render()`
   early-returns — no widgets call `app.watch()`, so grace periods expire and
   subscriptions drop. When terminal grows back, `Resize` event sets `dirty`,
   next render re-acquires all subscriptions. Brief gap is imperceptible.

7. **Topology change** (`group_membership` event): Progress states cleared,
   dirty flag set. Next render re-subscribes to the new topology automatically.
   First `position`/`playback_state` events for new groups self-heal
   `ProgressState` entries via `entry().or_insert_with()`.

## Acceptance Criteria

- [x] `watch_handles: RefCell<Vec<Box<dyn Any>>>` field added to `App`
- [x] `App::watch()` and `App::watch_group()` helper methods implemented (no `fetch()`, no `eprintln!`)
- [x] `App::watch()` returns `None` on cold cache (widgets handle gracefully)
- [x] `App::watch()` silently falls back to `get()` on watch failure
- [x] `App::clear_watch_handles(&mut self)` uses `get_mut().clear()` (no RefCell overhead)
- [x] `watch_registry` field removed from `App`
- [x] `setup_watches()`, `teardown_watches()`, `setup_watches_if_groups_tab()` deleted from `event.rs`
- [x] Tab-transition watch lifecycle logic removed from key handler
- [x] `clear_watch_handles()` called before `terminal.draw()` in event loop
- [x] `handle_change_event` uses `entry().or_insert_with()` for `position` and `playback_state`
- [x] `home_groups.rs`: 4 `get()` calls replaced with `app.watch()` / `app.watch_group()`
- [x] `home_speakers.rs`: 2 `get()` calls replaced with `app.watch()`
- [x] No references to `.unwatch()` remain in `src/`
- [x] No references to `WatchStatus` or `status.current` remain in `src/`
- [x] `docs/references/sonos-sdk.md` documents `WatchHandle<P>` (not `WatchStatus<P>`)
- [x] `docs/references/sonos-sdk.md` has no `unwatch()` references
- [x] `docs/product/roadmap.md` updated (3 lines)
- [x] `docs/product/prd.md` updated (1 line)
- [x] `cargo check` passes
- [x] `cargo clippy` passes

## Sources

- SDK source: `../sonos-sdk/sonos-sdk/src/property/handles.rs` — `WatchHandle<P>` definition (lines 118-188)
- SDK event manager: `../sonos-sdk/sonos-event-manager/src/manager.rs` — `release_watch()` grace period (lines 271-333), `GRACE_PERIOD` 50ms constant (line 27)
- TUI event loop: `src/tui/event.rs` — current `setup_watches()` / `teardown_watches()`
- TUI app state: `src/tui/app.rs` — current `watch_registry` field
- TUI render: `src/tui/ui.rs` — small-terminal early return (lines 19-21)
- Widget code: `src/tui/screens/home_groups.rs` (lines 63-66), `home_speakers.rs` (lines 89, 138)
- SDK docs: `docs/references/sonos-sdk.md` — sections 5.1–5.4, 8, complete example
- React `useSyncExternalStore`: subscription + getSnapshot pattern parallel
- ratatui: immediate-mode rendering with cell-level diffing (double buffer at terminal level)
