---
status: complete
priority: p1
issue_id: "002"
tags: [code-review, sdk, implementation-gap]
dependencies: []
---

# Add state_manager.add_devices() to Rediscovery Flow

## Problem Statement

The plan's `rediscover()` / auto-rediscovery code rebuilds the speaker HashMap but never calls `state_manager.add_devices()` for newly discovered devices. Without this call, new speakers will have handles in the map but no state tracking — all property `get()`/`fetch()`/`watch()` calls will fail.

## Findings

- **Pattern-recognition-specialist (HIGH):** `from_discovered_devices()` at `system.rs:90-92` calls `state_manager.add_devices(devices.clone())`. The plan's rediscovery pseudocode omits this entirely.
- **Pattern-recognition-specialist (MEDIUM):** No handling for removing stale devices from StateManager on rediscover. The map swap replaces all entries, but StateManager still holds old registrations.
- The plan's `build_speakers()` helper (line 176) does not exist — it needs to be extracted from `from_discovered_devices()`.

## Proposed Solutions

### Option 1: Call add_devices() Before Map Swap (Recommended)

**Approach:** In the rediscovery flow, after SSDP returns devices, call `self.state_manager.add_devices(devices)` before rebuilding and swapping the speaker map.

**Pros:**
- Matches existing `from_discovered_devices()` pattern
- New speakers immediately have state tracking

**Cons:**
- StateManager may accumulate stale device registrations over time
- No `remove_devices()` API exists in the SDK

**Effort:** Small

**Risk:** Low

## Technical Details

**Affected files:**
- `../sonos-sdk/sonos-sdk/src/system.rs` — rediscovery method must call `state_manager.add_devices()`
- Plan document — update rediscovery pseudocode to include this step

## Acceptance Criteria

- [ ] Rediscovery calls `state_manager.add_devices()` for newly discovered devices
- [ ] Plan pseudocode updated to show this step
- [ ] Verify Speaker handles from rediscovery can `fetch()` properties

## Work Log

### 2026-03-09 - Discovery during code review

**By:** Claude Code

**Actions:**
- Pattern-recognition agent identified missing state_manager interaction
- Confirmed by reading `from_discovered_devices()` in system.rs
