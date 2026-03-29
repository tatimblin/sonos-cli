---
status: complete
priority: p1
issue_id: "023"
tags: [code-review, quality, tui, watch-lifecycle]
dependencies: []
---

# eprintln! Corrupts TUI Display in Raw Mode

## Problem Statement

The plan's `app.watch()` method calls `eprintln!("watch failed for property: {e}")` on error. In a TUI using crossterm raw mode, writing to stderr goes directly to the terminal's underlying file descriptor, producing garbage characters overlaid on the TUI. crossterm's alternate screen captures stdout but not stderr.

Additionally, at 4fps rendering, a persistent watch failure produces 4 log lines per second per failed property — a wall of identical messages after the TUI exits.

## Findings

- **architecture-strategist (Critical):** "Writing to stderr in raw mode corrupts the TUI. Replace eprintln! with tracing::warn! or app.status_message."
- **julik-frontend-races-reviewer:** "eprintln! during render writes directly to file descriptor 2, producing visual garbage."
- **pattern-recognition-specialist:** "Error message format inconsistency — TUI debug output has no established pattern in the codebase."

## Proposed Solutions

### Option 1: Silent fallback to get() (Recommended)

Match the current behavior — `setup_watches()` at line 131 of `event.rs` silently falls back when `watch()` fails (`if let Ok(status) = coordinator.current_track.watch()`). No logging needed for a fallback path.

```rust
Err(_) => prop.get(),
```

- **Pros:** Matches existing behavior, zero risk, simplest
- **Cons:** Failures are invisible
- **Effort:** Small (remove 1 line per method)
- **Risk:** None

### Option 2: Store in app.status_message

Display the error in the TUI's own status bar:

```rust
Err(e) => {
    app.set_status_message(format!("error: watch failed: {e}"));
    prop.get()
}
```

- **Pros:** Visible to user, uses existing status_message infrastructure
- **Cons:** Requires &mut App or RefCell for status_message; may spam on repeated failures
- **Effort:** Medium
- **Risk:** Low

## Acceptance Criteria

- [ ] No direct stderr writes during TUI raw mode operation
- [ ] Watch failures degrade gracefully without visual corruption

## Sources

- Plan: `docs/plans/2026-03-28-refactor-watch-api-migration-plan.md` section 1
- Current silent fallback: `src/tui/event.rs:131` (if let Ok pattern)
