---
status: complete
priority: p3
issue_id: "027"
tags: [code-review, architecture, tui, watch-lifecycle, simplification]
dependencies: []
---

# Consider Simpler clear() Before draw() Instead of Double-Buffer

## Problem Statement

The double-buffer pattern adds conceptual overhead (two vecs, swap semantics, ordering contracts) to solve a race where `draw()` takes longer than 50ms. In practice, ratatui renders text widgets to an in-memory buffer and diffs against the previous frame — this is microseconds of CPU work, not 50ms. The 50ms grace period is 1000x longer than a typical text-mode render.

## Findings

- **code-simplicity-reviewer:** "The double-buffer pattern solves a theoretical problem. A simple clear() -> draw() sequence works. The 50ms grace period is 100x longer than a text-mode render cycle."
- **architecture-strategist:** "The double-buffer is well-designed but adds implicit ordering contracts. The sequence must be exactly: swap -> render -> drop."
- **performance-oracle:** Confirmed that `clear() -> draw()` works because the SDK ref-count transitions happen within microseconds, well within the 50ms grace period.

## Proposed Solutions

### Option 1: Simple clear() before draw()

```rust
if app.dirty {
    app.watch_handles.get_mut().clear();  // drop old, start grace periods
    terminal.draw(|frame| ui::render(frame, app))?;  // widgets push new, cancel grace
    app.dirty = false;
}
```

- **Pros:** 3 lines, no double-buffer concept, no ordering contract, no small-terminal guard needed
- **Cons:** Theoretically vulnerable if draw() exceeds 50ms (highly unlikely for text TUI)
- **Effort:** Small
- **Risk:** Very low

### Option 2: Keep double-buffer (current plan)

- **Pros:** Handles the theoretical >50ms render case
- **Cons:** More complex, requires ordering documentation, creates small-terminal dead zone
- **Effort:** Medium (already designed)
- **Risk:** Low but introduces complexity

## Decision Notes

This is a design choice between theoretical safety and practical simplicity. The double-buffer is more robust but the simpler approach is almost certainly sufficient for this text-mode TUI. Worth discussing with the user.

## Sources

- Plan: `docs/plans/2026-03-28-refactor-watch-api-migration-plan.md` sections 2, 2.1
