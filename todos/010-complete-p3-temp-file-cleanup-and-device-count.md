---
status: complete
priority: p3
issue_id: "010"
tags: [code-review, robustness, sdk]
dependencies: []
---

# Temp File Cleanup and Device Count Bound

## Problem Statement

Two minor robustness items:
1. Process crashes leave orphaned `cache.json.{PID}.tmp` files that accumulate over time
2. No upper bound on deserialized device count — a crafted 1MB cache could contain thousands of entries

## Findings

- **Security-sentinel Finding 3 (Low):** Temp file not cleaned up on rename failure.
- **Security-sentinel Finding 4 (Low):** Device count unbounded after deserialization.

## Proposed Solutions

### Option 1: Add Cleanup and Bound Check

**Approach:**
1. Add `let _ = fs::remove_file(&temp_path)` on rename failure
2. Add `if cached.devices.len() > 256 { return None; }` after deserialization

**Effort:** Small

**Risk:** None

## Acceptance Criteria

- [ ] Temp file cleaned up on rename failure
- [ ] Device count capped at reasonable maximum (e.g., 256)

## Work Log

### 2026-03-09 - Discovery during code review

**By:** Claude Code
