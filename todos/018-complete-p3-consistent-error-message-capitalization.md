---
status: complete
priority: p3
issue_id: "018"
tags: [code-review, patterns, sdk]
dependencies: []
---

# Consistent Error Message Capitalization in SDK

## Problem Statement

Error messages in `SdkError` have inconsistent capitalization. Some start with uppercase ("State management error"), others with lowercase ("discovery failed", "internal lock poisoned"). This is a minor style inconsistency.

## Findings

- **Pattern-recognition-specialist:** Error message style varies — some uppercase, some lowercase. Pick one convention and apply it consistently.

## Proposed Solutions

### Option 1: Lowercase All Error Messages (Recommended)

**Approach:** Rust convention (per `thiserror` and stdlib) is lowercase error messages. Standardize all `SdkError` variants to lowercase:
- "State management error" → "state management error"
- "API error" → "api error"
- "Event manager error" → "event manager error"
- etc.

**Effort:** Small

**Risk:** None (error messages are for display, not machine-parsed)

## Acceptance Criteria

- [ ] All `SdkError` display messages start with lowercase
- [ ] Consistent style across all error variants

## Work Log

### 2026-03-09 - Discovery during code review (round 2)

**By:** Claude Code
