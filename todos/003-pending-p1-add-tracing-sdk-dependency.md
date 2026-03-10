---
status: complete
priority: p1
issue_id: "003"
tags: [code-review, sdk, dependency]
dependencies: []
---

# Add tracing to SDK Dependencies

## Problem Statement

The plan uses `tracing::warn!` for logging cache save failures and stale cache fallback warnings, but `tracing` is not listed in the new SDK dependencies. The SDK's existing `Cargo.toml` already has `tracing = "0.1"`, but the plan's dependency section (line 196, 282) only lists `serde`, `serde_json`, and `dirs` as new additions. The plan's prose and code samples reference `tracing` without acknowledging it's already available.

## Findings

- **Spec-flow-analyzer (Gap 4.3):** "tracing is used in sample code but not listed as a dependency."
- **Existing SDK Cargo.toml** at `../sonos-sdk/sonos-sdk/Cargo.toml` line 19: `tracing = "0.1"` is already present.
- This is a documentation gap in the plan, not a missing dependency. The plan should note that `tracing` is already available.

## Proposed Solutions

### Option 1: Update Plan Text (Recommended)

**Approach:** Note in the plan that `tracing` is already an SDK dependency. Remove it from the "gap" category.

**Effort:** Small (plan text edit only)

**Risk:** None

## Acceptance Criteria

- [ ] Plan acknowledges tracing is already in SDK deps
- [ ] Code samples showing `tracing::warn!` are consistent with actual usage

## Work Log

### 2026-03-09 - Discovery during code review

**By:** Claude Code

**Actions:**
- Multiple agents flagged tracing as missing but it's already in the SDK
