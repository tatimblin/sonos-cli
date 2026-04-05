---
status: complete
priority: p3
issue_id: "033"
tags: [code-review, simplification, tui, speaker-list]
dependencies: []
---

# Inline home_speakers.rs Trivial Passthrough

## Problem Statement

`src/tui/screens/home_speakers.rs` is a 19-line file that does nothing but delegate to `speaker_list::render` with `FullList` mode. The `GroupView > Speakers` branch in `ui.rs` already calls `speaker_list::render` directly. The inconsistency adds a needless indirection layer.

## Findings

- **code-simplicity-reviewer:** Single-line delegation. Caller in `ui.rs` should call `speaker_list::render` directly.

## Proposed Solutions

### Solution A: Inline the call in ui.rs, delete the file
- **Effort:** Small
- **Risk:** None

## Technical Details

- **Affected files:** `src/tui/screens/home_speakers.rs` (delete), `src/tui/ui.rs`, `src/tui/screens/mod.rs`

## Acceptance Criteria

- [ ] `home_speakers.rs` deleted
- [ ] `ui.rs` calls `speaker_list::render` directly for Home > Speakers
- [ ] `cargo build` passes

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-04-05 | Created from PR #45 code review | |

## Resources

- PR: #45
