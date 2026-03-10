---
status: complete
priority: p2
issue_id: "016"
tags: [code-review, simplicity, sdk]
dependencies: []
---

# Remove MAX_CACHE_SIZE Guard (YAGNI)

## Problem Statement

The `MAX_CACHE_SIZE` (1MB) check in `cache::load()` is redundant. If the file is corrupt or oversized, `serde_json::from_str()` will fail gracefully and return `None` via `.ok()`. The size check adds code that solves a problem the existing error handling already covers.

## Findings

- **Code-simplicity-reviewer (P2):** `MAX_CACHE_SIZE` is YAGNI — serde parse already handles corrupt files. Remove the constant and the metadata size check.

## Proposed Solutions

### Option 1: Remove the Guard (Recommended)

**Approach:** Remove the `MAX_CACHE_SIZE` constant and the `meta.len() > MAX_CACHE_SIZE` check in `load()`. The `fs::metadata()` call can also be removed if it was only used for the size check.

```rust
pub(crate) fn load() -> Option<CachedDevices> {
    let path = cache_dir()?.join("cache.json");
    let contents = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&contents).ok()
}
```

**Effort:** Small

**Risk:** None — serde handles malformed input gracefully

## Acceptance Criteria

- [ ] `MAX_CACHE_SIZE` constant removed
- [ ] `load()` reads file directly without size pre-check
- [ ] Cache still handles corrupt files gracefully (returns None)

## Work Log

### 2026-03-09 - Discovery during code review (round 2)

**By:** Claude Code
