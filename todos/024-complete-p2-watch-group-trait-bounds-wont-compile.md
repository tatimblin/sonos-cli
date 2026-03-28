---
status: complete
priority: p2
issue_id: "024"
tags: [code-review, architecture, tui, watch-lifecycle, compilation]
dependencies: []
---

# watch_group() Generic Bounds Insufficient for fetch() Fallback

## Problem Statement

The plan's `watch_group()` method has generic bounds `P: SonosProperty + Clone + 'static`, but calls `prop.fetch()` as a cold-cache fallback. `GroupPropertyHandle::fetch()` requires the `GroupFetchable` trait bound (SDK line 922). Not all group properties implement `GroupFetchable` (e.g., `GroupVolumeChangeable`). The code as proposed will not compile.

## Findings

- **pattern-recognition-specialist (High):** "watch_group() generic bounds are insufficient for fetch(). The code as written will not compile."

## Proposed Solutions

### Option 1: Remove fetch() from watch_group() (Recommended)

If fetch() is removed from the render path per todo #021, this problem disappears — both `watch()` and `watch_group()` just return `wh.value().cloned()` or fall back to `get()`.

- **Pros:** Solves two problems at once, simplest code
- **Cons:** One blank frame on cold cache for group properties
- **Effort:** Small
- **Risk:** None

### Option 2: Add GroupFetchable bound

Add `P: GroupFetchable` to the trait bounds, and provide a separate non-fetching variant for properties that don't implement it.

- **Pros:** Preserves cold-cache fetch behavior
- **Cons:** More complexity, two method variants
- **Effort:** Medium
- **Risk:** Low

## Acceptance Criteria

- [ ] `cargo check` passes with the watch_group() implementation
- [ ] All group property types can be watched (GroupVolume, GroupMute, GroupVolumeChangeable)

## Sources

- SDK: `../sonos-sdk/sonos-sdk/src/property/handles.rs` — GroupFetchable trait
- Plan: `docs/plans/2026-03-28-refactor-watch-api-migration-plan.md` section 1
