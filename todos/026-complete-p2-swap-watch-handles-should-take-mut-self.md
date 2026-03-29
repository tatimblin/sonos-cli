---
status: complete
priority: p2
issue_id: "026"
tags: [code-review, architecture, tui, watch-lifecycle]
dependencies: []
---

# swap_watch_handles() Should Take &mut self in Event Loop Context

## Problem Statement

The plan's `swap_watch_handles()` takes `&self` and uses `RefCell::borrow_mut()`, but it is called from the event loop which already has `&mut App`. Using `RefCell` when you already have exclusive access via `&mut` is a mixed-borrow-model smell. The `RefCell` should be reserved exclusively for the render path where `&App` is genuinely the constraint.

## Findings

- **pattern-recognition-specialist (Medium):** "swap_watch_handles uses &self for interior mutability while the event loop already has exclusive access via &mut app. Mixed borrow model."
- **architecture-strategist:** "The event loop has &mut app — use that directly. Reserve RefCell for only the render path."

## Proposed Solutions

### Option 1: Provide both &mut self and &self access methods (Recommended)

```rust
impl App {
    /// Called from event loop (has &mut self)
    pub fn swap_watch_handles(&mut self) -> Vec<Box<dyn Any>> {
        std::mem::take(self.watch_handles.get_mut())
    }

    /// Called from widgets during render (only has &self)
    pub fn watch<P>(&self, prop: &PropertyHandle<P>) -> Option<P> {
        // Uses RefCell::borrow_mut() — only place RefCell is needed
    }
}
```

- **Pros:** Uses compile-time borrow checking where possible, RefCell only where needed
- **Cons:** Slightly more nuanced API
- **Effort:** Small
- **Risk:** None

## Acceptance Criteria

- [ ] swap_watch_handles takes &mut self, accesses vec directly via get_mut()
- [ ] RefCell only used inside watch()/watch_group() where &self is the constraint

## Sources

- Plan: `docs/plans/2026-03-28-refactor-watch-api-migration-plan.md` section 1
