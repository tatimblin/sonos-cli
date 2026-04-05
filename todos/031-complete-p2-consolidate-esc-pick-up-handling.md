---
status: complete
priority: p2
issue_id: "031"
tags: [code-review, architecture, tui, speaker-list, duplication]
dependencies: ["030"]
---

# Consolidate Esc Pick-Up Cancellation Handling

## Problem Statement

Pick-up cancellation via Esc is handled in two places: `event.rs` (lines 136-154) and `speaker_list.rs` `handle_pick_up_key` (line 737). The `event.rs` version runs first (global key handler), making the widget's Esc branch dead code. If cancellation ever needs additional cleanup (e.g., restoring `selected_index`), the fix would need to go in `event.rs`, not in the widget where a developer would naturally look.

## Findings

- **architecture-strategist:** Split behavior creates ambiguity about which location is canonical.
- **code-simplicity-reviewer:** ~15 LOC removable. Eliminate one of the two paths.

## Proposed Solutions

### Solution A: Remove from event.rs, let widget handle it (Recommended)
Remove the pick-up-specific Esc check from event.rs. Let Esc fall through to the screen-specific handler, which delegates to `speaker_list::handle_key`, which already handles Esc in pick-up mode.

- **Pros:** Better encapsulation — all pick-up logic in one place
- **Cons:** Slightly longer key dispatch path for Esc during pick-up
- **Effort:** Small
- **Risk:** Low — must ensure Esc still works for non-pick-up navigation

### Solution B: Remove from widget, keep in event.rs
Remove the dead Esc branch from `handle_pick_up_key`.

- **Pros:** Minimal change (~3 LOC)
- **Cons:** Pick-up cancellation logic stays split from other pick-up logic
- **Effort:** Trivial
- **Risk:** None

## Technical Details

- **Affected files:** `src/tui/event.rs` (lines 136-154), `src/tui/widgets/speaker_list.rs` (line 737)
- **Depends on:** #030 (speakers_state methods simplify the event.rs match)

## Acceptance Criteria

- [ ] Esc cancels pick-up from exactly one code path
- [ ] Esc still navigates back when not in pick-up mode

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-04-05 | Created from PR #45 code review | |

## Resources

- PR: #45
