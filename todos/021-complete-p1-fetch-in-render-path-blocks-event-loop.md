---
status: complete
priority: p1
issue_id: "021"
tags: [code-review, performance, tui, watch-lifecycle]
dependencies: []
---

# fetch() in Render Path Blocks Event Loop

## Problem Statement

The plan's `app.watch()` method calls `prop.fetch()` (synchronous SOAP HTTP call, 5-20ms per property) when `wh.value()` returns `None`. This runs inside `terminal.draw()`, blocking the single-threaded event loop. With 5 groups x 4 properties, that is up to 20 sequential SOAP calls (100-400ms) during a single `draw()` call. The terminal appears frozen during this time.

Worse, if a speaker is unreachable, `fetch()` blocks for the SOAP timeout (2-5 seconds). One offline speaker turns the first render into a multi-second hang.

## Findings

- **performance-oracle (P0):** "Move fetch() out of the render path. 5 groups = 20 SOAP calls = 200-800ms blocking. Unacceptable."
- **architecture-strategist (Critical):** "Render functions should be side-effect-free and non-blocking. The multiplicative blocking effect scales with group count."
- **code-simplicity-reviewer:** "The current setup_watches() does the same fetches, but outside the render loop — that is a more appropriate place."
- **julik-frontend-races-reviewer:** "fetch() may fire more than once per navigation — if the SDK subscription hasn't delivered its first event by the second frame, wh.value() returns None again."
- **pattern-recognition-specialist:** "fetch() in render path is an anti-pattern for immediate-mode TUI rendering."

## Proposed Solutions

### Option 1: Accept None, show placeholder data (Recommended)

Remove `fetch()` from `app.watch()`. Return `None` when value is cold. Widgets already handle `None` gracefully (show empty strings / 0 volume). The watch subscription delivers data within 50-200ms, triggering a re-render via `dirty = true`.

```rust
pub fn watch<P>(&self, prop: &PropertyHandle<P>) -> Option<P> {
    match prop.watch() {
        Ok(wh) => {
            let val = wh.value().cloned();
            self.watch_handles.borrow_mut().push(Box::new(wh));
            val
        }
        Err(_) => prop.get(),
    }
}
```

- **Pros:** Zero blocking in render path, simplest code, matches React model (show what you have, update when data arrives)
- **Cons:** One blank frame (~50-250ms) on first navigation to Groups tab
- **Effort:** Small (remove 3 lines from each method)
- **Risk:** Low — blank frame is imperceptible

### Option 2: Pre-fetch outside draw(), before render

Move cold-cache fetching into the event loop before `terminal.draw()`:

```rust
if app.dirty {
    prefetch_cold_caches(app);  // blocking but outside draw()
    let _old_handles = app.swap_watch_handles();
    terminal.draw(|frame| ui::render(frame, app))?;
}
```

- **Pros:** Preserves current behavior (no blank frame), keeps render non-blocking
- **Cons:** Still blocks the event loop for the same duration, just outside draw()
- **Effort:** Medium
- **Risk:** Low

### Option 3: Cap to N fetches per frame

Add a per-frame budget (e.g., max 2 fetches per render cycle). Remaining cold properties fetch on subsequent frames.

- **Pros:** Bounds worst-case latency to ~40ms regardless of group count
- **Cons:** More complex, multiple frames to populate all data
- **Effort:** Medium
- **Risk:** Low

## Acceptance Criteria

- [ ] No synchronous SOAP calls inside `terminal.draw()` closure
- [ ] First render completes in <50ms regardless of group count
- [ ] All property values populated within 500ms of tab navigation
- [ ] Unreachable speakers do not freeze the UI

## Sources

- Plan: `docs/plans/2026-03-28-refactor-watch-api-migration-plan.md` section 1
- Current fetch location: `src/tui/event.rs:131-165` (setup_watches, outside render)
