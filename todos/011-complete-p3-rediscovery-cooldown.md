---
status: complete
priority: p3
issue_id: "011"
tags: [code-review, performance, sdk]
dependencies: ["001"]
---

# Add Rediscovery Cooldown / Rate Limit

## Problem Statement

With transparent auto-rediscovery, a TUI event loop could trigger repeated 3s SSDP scans if a speaker is consistently not found. The one-shot flag prevents this within a session, but should be time-based rather than permanent (a speaker added mid-session should be discoverable).

## Findings

- **Spec-flow-analyzer (Gap 6.2):** No rate limit on rediscovery. TUI could trigger repeatedly.
- **Performance-oracle:** Current one-shot boolean is permanent per session. Consider a cooldown timer (e.g., 30 seconds between rediscovery attempts).

## Proposed Solutions

### Option 1: Time-Based Cooldown (Recommended)

**Approach:** Replace the one-shot boolean with a timestamp of last rediscovery. Refuse to rediscover if last attempt was less than 30 seconds ago.

**Effort:** Small

**Risk:** None

## Acceptance Criteria

- [ ] Rediscovery has a minimum interval (e.g., 30s)
- [ ] New speakers added mid-session are eventually discoverable

## Work Log

### 2026-03-09 - Discovery during code review

**By:** Claude Code
