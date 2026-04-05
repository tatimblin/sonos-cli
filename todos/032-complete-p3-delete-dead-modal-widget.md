---
status: complete
priority: p3
issue_id: "032"
tags: [code-review, simplification, tui, dead-code]
dependencies: []
---

# Delete Dead Modal Widget

## Problem Statement

`src/tui/widgets/modal.rs` (68 lines) has `#[allow(dead_code)]` on every public item. No callers exist. The comment says "Retained for future milestones" but this is a YAGNI violation — git history preserves the code if ever needed, and future requirements will likely differ.

The associated theme fields (`modal_border`, `modal_title`, `modal_selected`) also have `#[allow(dead_code)]`.

## Findings

- **code-simplicity-reviewer:** Textbook YAGNI. 68 lines of untested, unmaintained code that must be kept compiling across refactors.

## Proposed Solutions

### Solution A: Delete modal.rs and theme fields (Recommended)
Remove `modal.rs`, its registration in `widgets/mod.rs`, and the three modal theme fields from all four theme variants.

- **Effort:** Small
- **Risk:** None — no callers

## Technical Details

- **Affected files:** `src/tui/widgets/modal.rs` (delete), `src/tui/widgets/mod.rs`, `src/tui/theme.rs`

## Acceptance Criteria

- [ ] `modal.rs` deleted
- [ ] Modal theme fields removed
- [ ] `cargo build` passes with no `dead_code` warnings for modal items

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-04-05 | Created from PR #45 code review | |

## Resources

- PR: #45
