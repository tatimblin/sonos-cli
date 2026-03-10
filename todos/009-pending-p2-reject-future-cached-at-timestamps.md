---
status: complete
priority: p2
issue_id: "009"
tags: [code-review, security, sdk]
dependencies: []
---

# Reject Future cached_at Timestamps in is_stale()

## Problem Statement

The `is_stale()` function uses `now.saturating_sub(cached.cached_at)`, which saturates to 0 for future timestamps — making the cache appear fresh forever. A tampered cache file could set `cached_at` to a far-future value to prevent cache expiry.

## Findings

- **Security-sentinel Finding 5 (Low):** Future timestamp bypasses staleness check. One-line fix with meaningful security benefit.

## Proposed Solutions

### Option 1: Reject Future Timestamps (Recommended)

**Approach:**
```rust
pub(crate) fn is_stale(cached: &CachedDevices) -> bool {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    if cached.cached_at > now { return true; } // Future = stale
    now - cached.cached_at >= CACHE_TTL_SECS
}
```

**Effort:** Small (one line)

**Risk:** None

## Acceptance Criteria

- [ ] `is_stale()` returns `true` for future timestamps
- [ ] Plan code sample updated

## Work Log

### 2026-03-09 - Discovery during code review

**By:** Claude Code
