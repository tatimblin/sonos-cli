---
status: complete
priority: p3
issue_id: "019"
tags: [code-review, patterns, sdk]
dependencies: []
---

# Extract `now_secs()` Helper to Deduplicate Timestamp Computation

## Problem Statement

The pattern `SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()` appears in both `save()` and `is_stale()` in the SDK cache module. This is minor duplication.

## Findings

- **Pattern-recognition-specialist:** Timestamp computation duplicated in two functions. Extract a `now_secs()` helper.

## Proposed Solutions

### Option 1: Extract Helper Function

**Approach:**
```rust
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
```

**Effort:** Small

**Risk:** None

## Acceptance Criteria

- [ ] `now_secs()` helper exists in cache module
- [ ] Both `save()` and `is_stale()` use the helper

## Work Log

### 2026-03-09 - Discovery during code review (round 2)

**By:** Claude Code
