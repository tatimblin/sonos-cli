---
status: complete
priority: p1
issue_id: "028"
tags: [code-review, safety, tui, speaker-list, panic]
dependencies: []
---

# Stale Index Panic in `group_for_entry` and Key Handlers

## Problem Statement

`group_for_entry(entries, index)` does not bounds-check `index` before using it in `entries[i]`. The `drop_index` and `selected_index` values are stored from previous key events when the entry list may have had a different length. If a speaker disappears between frames (powered off, network drop), `drop_index` or `selected_index` can exceed `entries.len()`, causing an index-out-of-bounds panic.

Similarly, `handle_normal_key` reads `selected_index` from persisted state without clamping, though individual match arms have guards. The render path clamps at line 267 but the key handler does not.

## Findings

- **security-sentinel:** `group_for_entry` panics if `index >= entries.len()`. TOCTOU gap: entries rebuilt from live SDK state but `drop_index` stored from prior frame. Same issue for `selected_index` in key handlers.
- **architecture-strategist:** Noted the subtle inconsistency risk where `handle_key` operates on a different entry list than what was last rendered.

## Proposed Solutions

### Solution A: Add bounds guards (Recommended)
Add a bounds check at the top of `group_for_entry`, and clamp `selected`/`drop_index` at the top of both `handle_normal_key` and `handle_pick_up_key`.

- **Pros:** Minimal change (~5 LOC), eliminates all panic paths
- **Cons:** None
- **Effort:** Small
- **Risk:** None

```rust
fn group_for_entry(entries: &[ListEntry], index: usize) -> Option<GroupId> {
    if index >= entries.len() { return None; }
    // ...existing logic
}

// In handle_normal_key:
let selected = get_selected_index(app).min(entries.len().saturating_sub(1));

// In handle_pick_up_key:
let drop_index = pick_up.drop_index.min(entries.len().saturating_sub(1));
```

## Recommended Action

Solution A — straightforward bounds guards.

## Technical Details

- **Affected files:** `src/tui/widgets/speaker_list.rs` (lines 128-138, 527, 667)
- **Trigger:** Speaker powered off or leaves network during active pick-up mode

## Acceptance Criteria

- [ ] `group_for_entry` returns `None` when index >= entries.len()
- [ ] `handle_normal_key` clamps `selected` to valid range
- [ ] `handle_pick_up_key` clamps `drop_index` to valid range
- [ ] No panic possible from stale indices

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-04-05 | Created from PR #45 code review | Multiple agents flagged stale index risk |

## Resources

- PR: #45
- File: `src/tui/widgets/speaker_list.rs`
