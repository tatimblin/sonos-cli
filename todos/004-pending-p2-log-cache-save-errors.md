---
status: complete
priority: p2
issue_id: "004"
tags: [code-review, observability, sdk]
dependencies: []
---

# Log Cache Save Errors Instead of Silently Discarding

## Problem Statement

The plan's prose says "Log warning, continue without caching (non-fatal)" but the sample code uses `let _ = cache::save(&devices)` which silently discards errors. If save persistently fails (e.g., read-only filesystem), every CLI invocation runs a 3-second SSDP scan with no diagnostic output.

## Findings

- **Spec-flow-analyzer (Gap 8.1):** Code/prose mismatch — plan says "log warning" but code discards error.
- **Pattern-recognition-specialist (7c):** `tracing::warn!` should be at call site, not inside `save()`.

## Proposed Solutions

### Option 1: Replace `let _` with Logged Error (Recommended)

**Approach:** Change all `let _ = cache::save(...)` to:
```rust
if let Err(e) = cache::save(&devices) {
    tracing::warn!("Failed to save discovery cache: {}", e);
}
```

**Effort:** Small

**Risk:** None

## Acceptance Criteria

- [ ] All cache save call sites log errors via tracing::warn!
- [ ] Plan sample code updated to match

## Work Log

### 2026-03-09 - Discovery during code review

**By:** Claude Code
