---
status: complete
priority: p2
issue_id: "025"
tags: [code-review, performance, tui, watch-lifecycle]
dependencies: []
---

# Use std::mem::take Instead of split_off(0) for Vec Swap

## Problem Statement

The plan uses `split_off(0)` to extract handles from the vec. `split_off(0)` allocates a new empty Vec (capacity 0) and gives the old vec's buffer to the return value. This means the new `watch_handles` vec starts with zero capacity every frame, causing a cascade of `realloc` calls (1 -> 2 -> 4 -> 8 -> 16 -> 32) as widgets push handles.

## Findings

- **architecture-strategist:** "split_off(0) vs std::mem::take — mem::take swaps in a zero-alloc empty Vec. More idiomatic."
- **performance-oracle:** "split_off(0) creates a new empty vec every frame, causing realloc cascade. Replace with mem::take + reserve."
- **julik-frontend-races-reviewer:** "split_off(0) vs std::mem::take — pedantic but worth a one-line change since it runs every frame."

## Proposed Solutions

### Option 1: std::mem::take with reserve (Recommended)

```rust
fn swap_watch_handles(&self) -> Vec<Box<dyn Any>> {
    let mut handles = self.watch_handles.borrow_mut();
    let capacity = handles.len();
    let old = std::mem::take(&mut *handles);
    handles.reserve(capacity);
    old
}
```

- **Pros:** Pre-allocates to previous frame's size, eliminates realloc cascade
- **Cons:** None
- **Effort:** Small
- **Risk:** None

## Acceptance Criteria

- [ ] swap_watch_handles uses std::mem::take, not split_off(0)
- [ ] New vec is pre-allocated to previous frame's capacity

## Sources

- Plan: `docs/plans/2026-03-28-refactor-watch-api-migration-plan.md` section 1
