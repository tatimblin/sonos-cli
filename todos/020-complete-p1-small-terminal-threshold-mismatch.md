---
status: complete
priority: p1
issue_id: "020"
tags: [code-review, architecture, tui, watch-lifecycle]
dependencies: []
---

# Small-Terminal Guard Threshold Mismatch

## Problem Statement

The plan proposes a small-terminal guard in the event loop that checks `size.width >= 80 && size.height >= 24`, but `ui::render()` at `src/tui/ui.rs:19` early-returns at `height < 4 || width < 20`. These are different thresholds. Terminals between 20x4 and 79x23 hit a dead zone where the event loop skips the handle swap but `ui::render()` proceeds to render widgets. Widgets push handles via `app.watch()` but handles are never swapped out, causing unbounded accumulation.

## Findings

- **architecture-strategist:** "The plan's 80x24 guard does not match ui.rs's 20x4 check. This is a logic error that would cause handles to accumulate unboundedly."
- **julik-frontend-races-reviewer:** "Terminals between 20x4 and 79x23 where the event loop takes the else branch but ui::render() does not early-return. Handles grow without bound. After an hour, ~432,000 handles."
- **code-simplicity-reviewer:** "The small-terminal guard is unnecessary — it is a consequence of the double-buffer complexity."

## Proposed Solutions

### Option 1: Match thresholds exactly (Recommended)

Use the same predicate as `ui::render()`: `size.width >= 20 && size.height >= 4`.

- **Pros:** Simple, eliminates the dead zone
- **Cons:** Still duplicates threshold logic across two files
- **Effort:** Small
- **Risk:** Low

### Option 2: Always swap, let grace periods handle small terminals

Remove the guard entirely. Always `swap_watch_handles()` before `draw()`. When `ui::render()` early-returns, no new handles are pushed, old handles drop, grace periods start. When terminal grows back, `Resize` event triggers render which re-acquires handles within 50ms.

- **Pros:** Simplest, no duplication, no dead zone
- **Cons:** Subscriptions may drop during small terminal periods (50ms grace period)
- **Effort:** Small
- **Risk:** Low — grace period covers quick resizes

### Option 3: Have ui::render() communicate whether widgets ran

Return a bool from `ui::render()` indicating if a full render occurred. Event loop uses this signal instead of duplicating the size check.

- **Pros:** Eliminates threshold drift permanently
- **Cons:** Requires changing render function signature
- **Effort:** Medium
- **Risk:** Low

## Acceptance Criteria

- [ ] No dead zone where handles accumulate without being swapped
- [ ] Thresholds in event loop and ui::render() are either identical or derived from a single source
- [ ] Terminal resize from small to large restores subscriptions correctly

## Sources

- Plan: `docs/plans/2026-03-28-refactor-watch-api-migration-plan.md` section 2.1
- UI render: `src/tui/ui.rs:19` — actual early-return threshold
