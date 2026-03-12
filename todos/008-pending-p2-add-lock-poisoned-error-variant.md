---
status: complete
priority: p2
issue_id: "008"
tags: [code-review, sdk, error-handling]
dependencies: []
---

# Add SdkError::LockPoisoned Variant and Fix CLI Test Assertions

## Problem Statement

Two related error-handling gaps:
1. The plan's rediscovery code references `SdkError::LockPoisoned` but this variant is never defined
2. The CLI test in `errors.rs:59` asserts `recovery_hint().contains("discover")` which will break when hints are updated

## Findings

- **Pattern-recognition-specialist (Medium):** `LockPoisoned` used in code but never defined in acceptance criteria.
- **Pattern-recognition-specialist (Medium):** `errors.rs` test will fail when recovery hints change from "Run 'sonos discover'" to new text.

## Proposed Solutions

### Option 1: Add Variant + Update Tests (Recommended)

**Approach:**
1. Add `LockPoisoned` to `SdkError` enum
2. Update CLI test assertions to match new recovery hint text

**Effort:** Small

**Risk:** None

## Acceptance Criteria

- [ ] `SdkError::LockPoisoned` added to error enum
- [ ] CLI error tests updated to match new recovery hint text
- [ ] Plan acceptance criteria include both items

## Work Log

### 2026-03-09 - Discovery during code review

**By:** Claude Code
