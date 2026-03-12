---
status: complete
priority: p1
issue_id: "001"
tags: [code-review, architecture, sdk, api-design]
dependencies: []
---

# Revert to Transparent Auto-Rediscovery in SDK

## Problem Statement

The deepened plan changed from the brainstorm's transparent auto-rediscovery (inside `get_speaker_by_name()`) to an explicit `rediscover()` method that CLI consumers must call manually. The user has explicitly stated: "I don't want the user of sonos-sdk to deal with discover / refresh. They should just get an experience that works."

The explicit `rediscover()` approach contradicts this vision. Additionally, the current plan only triggers rediscovery on "speaker not found" — it does not handle the critical scenario where a cached speaker's IP is unreachable (router reboot, network change).

## Findings

- **User direction (overrides agent recommendations):** SDK consumers should not deal with discovery/refresh. `SonosSystem::new()` + `get_speaker_by_name()` should just work.
- **Architecture-strategist** and **agent-native-reviewer** recommended keeping `sonos discover` and explicit `rediscover()` — both OVERRULED by user intent.
- **Spec-flow-analyzer Gap 11.1 (CRITICAL):** Fresh cache from a different network passes all validation but every command fails with `ApiError::NetworkError`. No automatic recovery path exists.
- **Spec-flow-analyzer Gap 6.1:** Only `SpeakerNotFound` triggers rediscovery. `ApiError::NetworkError` (wrong IP) does not.
- **Performance-oracle concern:** Hidden 3s SSDP inside a lookup is surprising. Accepted tradeoff per user intent — simplicity over feedback.

## Proposed Solutions

### Option 1: Auto-Rediscovery on Miss AND Network Error (Recommended)

**Approach:** Revert `get_speaker_by_name()` to auto-rediscover on miss (one-shot flag prevents loops). Additionally, add network-error-triggered rediscovery at the CLI layer: when any command returns `ApiError::NetworkError`, the executor triggers one rediscovery and retries.

**Pros:**
- Matches brainstorm vision and user intent
- Handles both "new speaker" and "stale IP" scenarios
- SDK consumer never thinks about discovery

**Cons:**
- Hidden 3s delay on miss (accepted tradeoff)
- CLI cannot show spinner during SDK-internal rediscovery

**Effort:** Medium (revert plan sections, update acceptance criteria)

**Risk:** Low

---

### Option 2: Auto-Rediscovery on Miss Only

**Approach:** Only auto-rediscover inside `get_speaker_by_name()` on miss. Network errors are surfaced to the user with a hint.

**Pros:**
- Simpler implementation
- Matches brainstorm exactly

**Cons:**
- Stale IP scenario (router reboot) has no automatic recovery
- User must wait 24h or delete cache manually

**Effort:** Small

**Risk:** Medium (stale IP scenario is common)

## Recommended Action

To be filled during triage.

## Technical Details

**Affected files:**
- `docs/plans/2026-03-08-feat-sdk-level-discovery-caching-plan.md` — revert "Explicit Rediscovery" section back to transparent auto-rediscovery
- `../sonos-sdk/sonos-sdk/src/system.rs` — `get_speaker_by_name()` with one-shot rediscovery
- `src/executor.rs` — add network-error retry with rediscovery at CLI layer

**Key change:** Remove public `rediscover()` from the plan. Keep it as `pub(crate)` or private for internal use by `get_speaker_by_name()`.

## Acceptance Criteria

- [ ] Plan updated: auto-rediscovery is transparent inside SDK lookups
- [ ] `get_speaker_by_name()` triggers one SSDP scan on miss before returning `None`
- [ ] One-shot flag prevents repeated discovery within same session
- [ ] No public `rediscover()` method exposed to SDK consumers
- [ ] CLI executor retries on `ApiError::NetworkError` (triggers SDK rediscovery)

## Work Log

### 2026-03-09 - Discovery during code review

**By:** Claude Code

**Actions:**
- Identified tension between deepened plan (explicit rediscover) and user intent (transparent)
- User explicitly stated discovery should be invisible to SDK consumers
- Identified stale-IP gap as critical companion issue

## Notes

- This is the single most impactful finding: it changes the plan's API design back to the brainstorm's original vision
- The `sonos discover` CLI command stays removed per user intent
- The `sonos refresh` suggestion from agents is also rejected
