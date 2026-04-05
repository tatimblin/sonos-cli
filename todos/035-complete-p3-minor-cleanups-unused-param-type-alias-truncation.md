---
status: complete
priority: p3
issue_id: "035"
tags: [code-review, simplification, tui, speaker-list, hygiene]
dependencies: []
---

# Minor Cleanups: Unused Parameter, Type Alias, Volume Delta Type

## Problem Statement

Three small issues flagged by multiple agents:

1. **Unused `_entries` parameter** in `enter_add_speaker_mode` (line 626) — never used, calls `build_full_list` directly instead.
2. **`HomeSpeakersState` type alias** (app.rs:140) — adds a second name for `SpeakerListScreenState` with zero value. Only used in `Screen::Home`.
3. **`i16` to `i8` truncation** in `handle_volume_adjust` (line 617) — `delta as i8` silently truncates. Currently safe (only ±2), but latent defect if caller changes.

## Findings

- **security-sentinel:** `as i8` truncation is a latent defect.
- **code-simplicity-reviewer:** Type alias is unnecessary indirection. Unused parameter is dead code.
- **architecture-strategist:** Unused `_entries` parameter suggests speculative design.

## Proposed Solutions

1. Remove `_entries` parameter from `enter_add_speaker_mode` and both call sites.
2. Delete `HomeSpeakersState` alias, use `SpeakerListScreenState` directly in `Screen::Home`.
3. Change `handle_volume_adjust` to accept `i8` directly instead of `i16`.

- **Effort:** Small (trivial per item)
- **Risk:** None

## Technical Details

- **Affected files:** `src/tui/widgets/speaker_list.rs`, `src/tui/app.rs`

## Acceptance Criteria

- [ ] `enter_add_speaker_mode` has no `_entries` parameter
- [ ] No `HomeSpeakersState` type alias
- [ ] `handle_volume_adjust` takes `i8` delta
- [ ] `cargo clippy` passes

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-04-05 | Created from PR #45 code review | |

## Resources

- PR: #45
