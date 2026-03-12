---
status: complete
priority: p2
issue_id: "006"
tags: [code-review, sdk, architecture]
dependencies: []
---

# Fetch Group Topology On-Demand Via ZoneGroupTopology Service

## Problem Statement

After SSDP rediscovery finds speakers, groups require ZoneGroupTopology data. `system.groups()` may return empty after rediscovery because group info comes from subscription events. `sonos play --group "Living Room"` could fail silently.

## Findings

- **Spec-flow-analyzer (Gap 6.1, Critical Q2):** No specification for how group lookups work after rediscovery.
- **Architecture-strategist (Risk 4.3):** Group topology timing is the plan's most underspecified area.

## Recommended Action

Make a direct (non-streaming) call to the ZoneGroupTopology service whenever group info is needed but not present. This follows the same pattern as other SDK properties — fetch on demand rather than relying on event subscriptions.

## Proposed Solutions

### Option 1: On-Demand ZoneGroupTopology Fetch (Approved)

**Approach:** When `system.groups()` or group-targeting resolution needs group data and it's not present, make a synchronous call to the ZoneGroupTopology service on any known speaker to get the current group state. No subscriptions, no streaming — just a direct fetch.

**Effort:** Medium

**Risk:** Low (follows existing fetch pattern)

## Acceptance Criteria

- [ ] SDK fetches ZoneGroupTopology directly when group info is absent
- [ ] No dependency on event subscriptions for initial group data
- [ ] `system.groups()` returns correct data after rediscovery
- [ ] `--group` flag works reliably on first use

## Work Log

### 2026-03-09 - Discovery during code review

**By:** Claude Code

### 2026-03-09 - Approved during triage

**By:** User
**Actions:** Approach clarified — use direct ZoneGroupTopology service call on demand, no streaming. Status: pending -> ready.
