---
status: complete
priority: p2
issue_id: "029"
tags: [code-review, architecture, tui, speaker-list, safety]
dependencies: []
---

# Replace Parallel Vecs with Single EntryRenderData Struct

## Problem Statement

The render function maintains four parallel `Vec<Option<T>>` (`speaker_volumes`, `group_volumes`, `group_playback_states`, `group_track_info`) indexed by entry position, with `None` padding for irrelevant entries. If any future code path skips a push, the vecs become misaligned and every subsequent index panics or reads wrong data with no compile-time protection.

## Findings

- **security-sentinel:** Parallel vec indexing relies on construction invariant. Error-prone for future maintenance.
- **code-simplicity-reviewer:** 57 lines of setup. A single `Vec<EntryRenderData>` would be clearer and shorter (~15 LOC saved).
- **performance-oracle:** Four heap allocations per frame where one would suffice. Also enables merging the two-pass render (watch + line building) into a single pass.

## Proposed Solutions

### Solution A: Single struct vec (Recommended)
```rust
struct EntryRenderData {
    speaker_volume: Option<u16>,
    group_volume: Option<u16>,
    playback_state: Option<PlaybackState>,
    track_info: Option<String>,
}
```

- **Pros:** Eliminates misalignment bugs, reduces allocations, cleaner code
- **Cons:** Minor refactor effort
- **Effort:** Small-Medium
- **Risk:** Low

### Solution B: Single-pass render
Merge the watch subscription loop and line-building loop into one pass, eliminating intermediate storage entirely.

- **Pros:** Maximum simplification, no intermediate vecs at all
- **Cons:** Longer match arms, harder to read
- **Effort:** Medium
- **Risk:** Low

## Recommended Action

Solution A first. Solution B can follow as a separate cleanup.

## Technical Details

- **Affected files:** `src/tui/widgets/speaker_list.rs` (lines 208-265, 278-411)

## Acceptance Criteria

- [ ] Four parallel Vecs replaced with single `Vec<EntryRenderData>`
- [ ] All render logic indexes into the single vec
- [ ] `cargo clippy` passes

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-04-05 | Created from PR #45 code review | Three agents independently flagged this |

## Resources

- PR: #45
