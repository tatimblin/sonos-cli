---
title: "fix: Data Freshness & Completeness Across SDK and CLI"
type: fix
status: completed
date: 2026-05-01
origin: docs/brainstorms/2026-05-01-data-freshness-fixes-brainstorm.md
milestone: "Milestone 2: CLI — Discovery & System Commands, Milestone 7: TUI — Home Screen, Milestone 8: TUI — Group View"
---

# fix: Data Freshness & Completeness Across SDK and CLI

## Overview

Five validated issues cause incomplete or stale data across sonos-sdk and sonos-cli. Users see: "Unknown" track metadata, missing playback state for entire groups, duplicate speaker names (satellite surround speakers), and timeouts when a portable speaker changes IP.

Root causes span both repos but cluster into three PRs:

1. **PR 1 (SDK): Topology Overhaul** — Resilient speaker selection, satellite filtering, IP refresh at startup + mid-session (Fixes 2+3+4+4b)
2. **PR 2 (SDK): fetch() Coordinator Routing** — Route PerCoordinator property fetches to the group coordinator (Fix 1)
3. **PR 3 (CLI): URI Fallback Display** — Show useful info from track URI when metadata is unavailable (Fix 5)

Validated against live hardware on 2026-05-01. (see brainstorm: `docs/brainstorms/2026-05-01-data-freshness-fixes-brainstorm.md`)

## Problem Statement

### Live Evidence

| Speaker | IP (cached) | IP (actual) | Issue |
|---------|-------------|-------------|-------|
| Office/Roam (Roam 2) | .198 | .200 | Stale cache, coordinator unreachable |
| Living Room (Amp) | .191 | .191 | Group member, gets NOT_IMPLEMENTED from own IP |
| Bedroom (Connect:Amp) | .193 | .193 | Coordinator but VLI protocol returns no metadata |
| Basement (Playbar) | .167 | .167 | OK, but satellites at .48/.47 cause duplicate names |

### Root Causes

1. **fetch() doesn't route to coordinator** (`handles.rs:496-518`): `PropertyHandle::fetch()` always calls the speaker's own IP. For PerCoordinator services (AVTransport), non-coordinators return NOT_IMPLEMENTED. `watch()` already routes correctly via `resolve_subscription_target()`.

2. **ensure_topology() is fragile** (`system.rs:481-526`): Uses `speakers.values().next()` — HashMap iteration is non-deterministic. If it picks an unreachable speaker, topology fetch fails silently. No fallback to other speakers.

3. **Satellites appear as standalone speakers** (`system.rs:307-339`): SSDP discovers satellite speakers (surround/sub) with the same `room_name` as the main speaker. The `HashMap<String, Speaker>` keyed by name overwrites one device. Topology already marks satellites with `Invisible="1"`.

4. **Stale cached IPs** (`system.rs:481-526` + `event_worker.rs:141-210`): Discovery cache has stale IPs from DHCP changes. The topology response's `ZoneGroupMemberInfo.location` contains each speaker's current IP but is discarded by the decoder. Must refresh at startup (Fix 4) AND mid-session (Fix 4b).

5. **Empty metadata shows "Unknown"** (`run.rs:203-242`, `helpers.rs:7-12`): When track metadata is genuinely unavailable (older firmware, VLI protocol), the CLI/TUI shows "Unknown" instead of extracting useful info from the track URI.

## Proposed Solution

### Architectural Principle: TUI as Thin Rendering Engine

The sonos-cli TUI is a **pure rendering layer** over SDK state. All dynamic data flows through `use_watch()` / `use_watch_group()` hooks. The SDK owns truth; the TUI displays it. (see brainstorm: architectural audit confirmed 84/85 code locations follow this pattern)

**Rule going forward:** Never add `fetch()` calls to screens or widgets. Use `use_watch()` for SDK properties, `use_state()` for derived/local state. Handlers mutate via SDK methods and trust the event system to propagate changes.

**Implication for these fixes:** Fixes 1-4 are SDK-only. They improve data quality at the source, and the TUI automatically benefits because it watches SDK state. Fix 5 touches display logic in both CLI command handlers and TUI helpers.

### Three-PR Structure

| PR | Repo | Scope | Files |
|----|------|-------|-------|
| PR 1: Topology Overhaul | sonos-sdk | Fixes 2+3+4+4b | `decoder.rs`, `state.rs`, `event_worker.rs`, `system.rs` |
| PR 2: fetch() Routing | sonos-sdk | Fix 1 | `handles.rs` |
| PR 3: URI Fallback | sonos-cli | Fix 5 | `run.rs`, `helpers.rs`, `screens/speakers.rs` |

PRs 1 and 2 ship to crates.io first. PR 3 updates the sonos-sdk dependency version, then ships.

## Technical Approach

### Phase 1: SDK Topology Overhaul (PR 1)

#### 1a. Extend `TopologyChanges` with IP and Satellite Data

**File:** `sonos-state/src/decoder.rs`

The `TopologyChanges` struct (line 33-41) currently has `groups`, `memberships`, `boot_seqs`. Add two new fields:

```rust
// decoder.rs — TopologyChanges struct
pub struct TopologyChanges {
    pub groups: Vec<GroupData>,
    pub memberships: Vec<(SpeakerId, GroupId)>,
    pub boot_seqs: Vec<(SpeakerId, u32)>,
    pub speaker_ips: Vec<(SpeakerId, IpAddr)>,    // NEW: extracted from location URLs
    pub satellite_ids: Vec<SpeakerId>,              // NEW: speakers with Invisible="1"
}
```

In `decode_topology_event()` (line 308-344), extract IPs from `member.location` and satellite IDs:

```rust
// decoder.rs — inside decode_topology_event()
// For each ZoneGroupMemberInfo:
if let Some(ip) = extract_ip_from_location(&member.location) {
    changes.speaker_ips.push((speaker_id.clone(), ip));
}

// For each satellite in member.satellites:
for sat in &member.satellites {
    if sat.invisible == "1" {
        let sat_id = SpeakerId::from(&sat.uuid);
        changes.satellite_ids.push(sat_id.clone());
        // Also extract satellite IP
        if let Some(ip) = extract_ip_from_location(&sat.location) {
            changes.speaker_ips.push((sat_id, ip));
        }
    }
}
```

Helper to parse IP from location URL:

```rust
fn extract_ip_from_location(location: &str) -> Option<IpAddr> {
    // location format: "http://192.168.4.200:1400/xml/device_description.xml"
    let url_part = location.strip_prefix("http://")?;
    let host_port = url_part.split('/').next()?;
    let host = host_port.split(':').next()?;
    host.parse().ok()
}
```

**SpecFlow gap addressed:** The decoder must check `Invisible` field on satellites and exclude them from normal group membership. Currently `decode_topology_event()` at line 318 collects all members unconditionally.

#### 1b. Apply IP Refreshes in State Manager

**File:** `sonos-state/src/state.rs`

The `ip_to_speaker` reverse map lives on `StateManager` as a separate `Arc<RwLock<HashMap>>` (line 293), not inside `StateStore`. Add the update method on `StateManager` so it can acquire both locks:

```rust
// state.rs — new method on StateManager (not StateStore)
impl StateManager {
    pub fn update_speaker_ip(&self, speaker_id: &SpeakerId, new_ip: IpAddr) {
        // Update SpeakerInfo in the store
        if let Ok(mut store) = self.store.write() {
            if let Some(info) = store.speakers.get_mut(speaker_id) {
                let old_ip = info.ip_address;
                if old_ip == new_ip { return; }
                info.ip_address = new_ip;
            }
        }
        // Update the reverse map (separate lock)
        if let Ok(mut map) = self.ip_to_speaker.write() {
            // Remove old IP entry
            map.retain(|_ip, id| id != speaker_id);
            map.insert(new_ip, speaker_id.clone());
        }
    }
}
```

For `event_worker.rs` (which holds the `StateStore` write lock directly), add a parallel method on `StateStore` that updates `SpeakerInfo.ip_address` only — the event worker updates `ip_to_speaker` via the `StateManager` wrapper separately:

```rust
// state.rs — on StateStore, for use inside apply_topology_changes()
pub fn update_speaker_ip_address(&mut self, speaker_id: &SpeakerId, new_ip: IpAddr) -> Option<IpAddr> {
    if let Some(info) = self.speakers.get_mut(speaker_id) {
        let old_ip = info.ip_address;
        if old_ip != new_ip {
            info.ip_address = new_ip;
            return Some(old_ip);
        }
    }
    None
}
```

**SpecFlow gap addressed:** The `ip_to_speaker` reverse map (used by event_worker.rs:70-76 to route incoming UPnP events) must have old IPs removed, not just new ones inserted. Stale entries cause events from recycled IPs to route to wrong speakers.

**Review fix:** `ip_to_speaker` lives on `StateManager`, not `StateStore`. The plan provides methods at both layers — `StateManager::update_speaker_ip()` for external callers (system.rs), `StateStore::update_speaker_ip_address()` for internal callers (event_worker.rs apply_topology_changes).

#### 1c. Apply Topology Changes Including IPs

**File:** `sonos-state/src/event_worker.rs`

Extend `apply_topology_changes()` (line 141-210) to apply IP updates:

```rust
// event_worker.rs — inside apply_topology_changes(), after boot_seq updates
// Uses StateStore method (we already hold the write lock)
let mut ip_updates: Vec<(IpAddr, IpAddr, SpeakerId)> = Vec::new();
for (speaker_id, new_ip) in &changes.speaker_ips {
    if let Some(old_ip) = store.update_speaker_ip_address(speaker_id, *new_ip) {
        ip_updates.push((old_ip, *new_ip, speaker_id.clone()));
    }
}
// After releasing store lock, update ip_to_speaker via StateManager
drop(store); // release write lock
for (old_ip, new_ip, speaker_id) in ip_updates {
    // StateManager::update_ip_to_speaker_map() or equivalent
    state_manager.update_reverse_ip_map(&speaker_id, old_ip, new_ip);
}
```

This is the same code path for both startup (Fix 4) and mid-session (Fix 4b). Every topology event (group changes, speaker joins/leaves, speaker reboots) refreshes IPs automatically.

**SpecFlow finding — race condition:** IP refresh vs. active UPnP subscriptions. Since Sonos uses HTTP callbacks (events are pushed to the SDK's listener IP, not pulled from the speaker IP), changing the speaker's IP does not invalidate existing subscriptions. The speaker continues pushing events to our listener regardless of its own IP change. No subscription teardown needed.

**SpecFlow finding — momentary inconsistency:** `apply_topology_changes` calls `store.clear_groups()` at line 165 which wipes `speaker_to_group`. Any concurrent `get_resolved()` for a PerCoordinator property falls back to the speaker's own (empty) props during the write lock gap. This is pre-existing and acceptable — the next frame's watch event restores correct state.

#### 1d. Resilient `ensure_topology()`

**File:** `sonos-sdk/src/system.rs`

Replace the single-speaker attempt (line 481-526) with a loop over all known speakers:

```rust
// system.rs — ensure_topology()
fn ensure_topology(&self) -> Result<(), SdkError> {
    let speaker_ips: Vec<IpAddr> = self.speakers.read()
        .map(|map| map.values().map(|s| s.ip).collect())
        .unwrap_or_default();

    for ip in &speaker_ips {
        match self.api_client.get_zone_group_state(*ip) {
            Ok(topology_xml) => {
                let changes = decode_topology_event(&topology_xml);
                // apply_topology_changes() now handles IP refresh (step 1c)
                // and satellite ID storage — no duplicate application needed
                self.state_manager.apply_topology_changes(&changes);
                return Ok(());
            }
            Err(e) => {
                tracing::debug!("Topology fetch failed for {ip}: {e}");
                continue;
            }
        }
    }

    tracing::warn!("ensure_topology: no speakers responded");
    Ok(())
}
```

**Review fix:** IP updates are consolidated into `apply_topology_changes()` (step 1c). `ensure_topology()` does not duplicate them — single code path for both startup and mid-session.

**SpecFlow gap addressed:** The spec didn't specify attempt strategy. Sequential with first-success is correct here — topology data is identical from any speaker, so we just need one to respond. Parallel would waste bandwidth and complicate error handling.

#### 1e. Filter Satellite Speakers After Topology

**File:** `sonos-sdk/src/system.rs`

In `from_devices_inner()` (line 161-222), after `ensure_topology()` returns, prune satellites from the speaker map:

```rust
// system.rs — in from_devices_inner(), after ensure_topology()
let satellite_ids = self.state_manager.get_satellite_ids();
if !satellite_ids.is_empty() {
    if let Ok(mut speakers) = self.speakers.write() {
        speakers.retain(|_name, speaker| !satellite_ids.contains(&speaker.id));
    }
    tracing::debug!("Filtered {} satellite speakers", satellite_ids.len());
}
```

The `satellite_ids` are populated by `apply_topology_changes()` which was called inside `ensure_topology()`. Add storage for satellite IDs in the state store.

**Note:** `SpeakerInfo.satellites: Vec<SpeakerId>` already exists in the state model but isn't populated. The topology changes now provide the data to populate it.

#### 1f. Refresh Speaker Map IPs After Topology

**File:** `sonos-sdk/src/system.rs`

After `ensure_topology()` updates state store IPs, the `Speaker` handles in the `speakers` map still hold stale IPs. Update them:

```rust
// system.rs — in from_devices_inner(), after ensure_topology()
if let Ok(mut speakers) = self.speakers.write() {
    for speaker in speakers.values_mut() {
        if let Some(info) = self.state_manager.speaker_info(&speaker.id) {
            speaker.ip = info.ip_address;
        }
    }
}
```

### Phase 2: SDK Coordinator Routing for fetch() (PR 2)

**File:** `sonos-sdk/src/property/handles.rs`

#### 2a. Route fetch() Through Coordinator

Modify `fetch()` (line 496-518) to resolve the coordinator target before making the SOAP call, mirroring what `watch()` does at line 359:

```rust
// handles.rs — PropertyHandle<P: Fetchable>::fetch()
pub fn fetch(&self) -> Result<P::Output, SdkError> {
    // Resolve coordinator for PerCoordinator services (3-arg form, matching watch() at line 359)
    let (target_id, target_ip) = self.context.state_manager
        .resolve_subscription_target(
            &self.context.speaker_id,
            self.context.speaker_ip,  // fallback IP when resolution fails
            P::SERVICE,
        )
        .unwrap_or_else(|| {
            (self.context.speaker_id.clone(), self.context.speaker_ip)
        });

    let response = self.context.api_client.call(target_ip, &P::operation())?;
    let value = P::parse(response)?;

    // Store under target_id (coordinator), not self.context.speaker_id
    self.context.state_manager
        .set_property(&target_id, P::PROPERTY_KEY, &value);

    Ok(value)
}
```

**Review fix:** `resolve_subscription_target()` takes 3 arguments `(speaker_id, speaker_ip, service)`, not 2. The `speaker_ip` serves as fallback when coordinator lookup fails.

**Critical detail:** The result is stored under `target_id` (the coordinator), not `self.context.speaker_id` (the requesting member). This is correct because `get()` at line 304-307 calls `get_resolved()` for PerCoordinator properties, which reads from the coordinator's property bag.

**Verification required during implementation:** Confirm `get()` calls `get_resolved()` for PerCoordinator properties, not a raw lookup. The brainstorm resolved this as correct but flagged for verification.

#### 2b. Fresh IP Lookup at Call Time

The `SpeakerContext.speaker_ip` is frozen in `Arc` at construction time (line 26-31). After Phase 1's IP refresh, the state store has the correct IP but existing `PropertyHandle` instances still hold the stale one.

The `resolve_subscription_target()` method already reads from the state manager, returning the current coordinator IP. The fallback `(self.context.speaker_id, self.context.speaker_ip)` only fires when the speaker isn't found in state at all (edge case during initialization).

**SpecFlow gap addressed:** Existing Speaker handles hold stale IPs after refresh. Since `resolve_subscription_target()` returns the fresh IP from state, this is resolved for PerCoordinator properties. For PerSpeaker properties, fetch() should also look up the current IP from state:

```rust
// For PerSpeaker properties, look up current IP from state
let current_ip = self.context.state_manager
    .speaker_info(&self.context.speaker_id)
    .map(|info| info.ip_address)
    .unwrap_or(self.context.speaker_ip);
```


### Phase 3: CLI URI Fallback Display (PR 3)

#### 3a. URI Pattern Matching Helper

**File:** `sonos-cli/src/tui/helpers.rs`

Add a URI fallback function alongside `track_summary()` (line 7-12):

```rust
// helpers.rs
pub fn uri_source_label(uri: &str) -> &str {
    if uri.starts_with("x-rincon:") {
        ""  // grouped member pointer — not actually playing, skip
    } else if uri.is_empty() {
        "Unknown"
    } else {
        "Playing (no metadata)"
    }
}
```

**Review simplification:** The original version matched specific streaming services (Spotify, Web stream, etc.) but this is YAGNI — users need to know "something is playing" when metadata is missing, not which service. The specific-service matching also had a logic bug where the outer `uri.contains("spotify:")` branch would never reach the inner `x-sonos-vli:` check. The simplified version covers the actual user need without maintaining a brittle URI pattern registry.

**SpecFlow gap addressed:** The `track_summary()` function at line 8 checks `t.is_empty()` which tests `uri.is_none()`. Tracks with a URI but no metadata pass this filter but still show "Unknown -- Unknown". The fix adds a fallback path when title and artist are both None but URI is present.

#### 3b. Update CLI Command Handlers

**File:** `sonos-cli/src/cli/run.rs`

In `cmd_status()` (line 203-242) and `cmd_groups()` (line 116-156), when `track.display()` returns "Unknown", check URI:

```rust
// run.rs — in cmd_status(), after fetching current_track
let track_display = if track.display() == "Unknown" {
    track.uri.as_deref()
        .map(uri_source_label)
        .filter(|s| !s.is_empty())
        .unwrap_or("Unknown")
} else {
    &track.display()
};
```

#### 3c. Update TUI Track Display

**File:** `sonos-cli/src/tui/helpers.rs`

Update `track_summary()` to use URI fallback:

```rust
// helpers.rs — updated track_summary()
pub fn track_summary(t: &CurrentTrack) -> String {
    if t.is_empty() {
        return String::new();
    }
    match (&t.title, &t.artist) {
        (Some(title), Some(artist)) => format!("{title} — {artist}"),
        (Some(title), None) => title.clone(),
        (None, Some(artist)) => artist.clone(),
        (None, None) => t.uri.as_deref()
            .map(uri_source_label)
            .filter(|s| !s.is_empty())
            .unwrap_or("Unknown")
            .to_string(),
    }
}
```

**File:** `sonos-cli/src/tui/screens/speakers.rs`

Update bottom bar track display (line 264-267) to use `track_summary()` instead of direct field extraction, so the URI fallback applies consistently.

## System-Wide Impact

### Interaction Graph

**Fix 1 (fetch routing):** `fetch()` → `resolve_subscription_target()` → state_manager read lock → SOAP call to coordinator IP → `set_property()` under coordinator ID → `get()` via `get_resolved()` reads same coordinator bag. No new callbacks or observers introduced.

**Fixes 2+3+4 (topology overhaul):** `ensure_topology()` → `get_zone_group_state()` on each speaker → `decode_topology_event()` → `apply_topology_changes()` → `update_speaker_ip()` → `clear_groups()` + rebuild groups. Mid-session: topology UPnP event → event_worker → same `decode_topology_event()` + `apply_topology_changes()` path. The IP update additionally writes to `ip_to_speaker` reverse map.

**Fix 5 (URI fallback):** Pure display logic. No SDK calls, no state mutations. Reads `CurrentTrack.uri` which is already populated by existing SOAP responses.

### Error Propagation

- `ensure_topology()` failures are non-fatal (log + continue) — same as current behavior, but now with fallback to other speakers
- `fetch()` routing failure falls back to `(self.context.speaker_id, self.context.speaker_ip)` — graceful degradation to current behavior
- `extract_ip_from_location()` returns `None` on parse failure — speaker IP simply not updated, no error propagated
- URI fallback returns "Unknown" when no pattern matches — same as current behavior

### State Lifecycle Risks

- **IP update atomicity:** `update_speaker_ip()` removes old IP and inserts new IP in the same write lock. No window where neither mapping exists.
- **Satellite filtering after topology:** Satellites are removed from the speaker map after `ensure_topology()` completes. If a satellite was already being watched, the watch handle holds an Arc to the Speaker — the handle becomes stale but won't crash. The next TUI frame won't request a watch for a speaker that's no longer in `system.speakers()`.
- **Topology not found for unknown speakers:** `apply_topology_changes()` at event_worker.rs:186 only updates `boot_seq` for speakers already in `store.speakers`. Speakers in topology but not in SSDP discovery get memberships stored but no SpeakerInfo. This is pre-existing and acceptable — such speakers become visible on next rediscovery.
- **Stale IP in Speaker handles for action methods:** `play()`, `pause()`, etc. use `speaker.ip` directly. After a mid-session IP change, these methods fail until the Speaker handle is re-obtained from `system.speakers()`. This is an edge case (portable speaker changes IP while TUI is running) and is acceptable for v1 — the TUI rebuilds Speaker references each frame via hooks. A future improvement could add `Speaker::current_ip()` that reads from state.

### API Surface Parity

All five fixes are internal to the SDK or display-only in the CLI. No public API changes. `PropertyHandle::fetch()` signature unchanged. `track_summary()` is CLI-internal.

## Acceptance Criteria

### PR 1: SDK Topology Overhaul

- [x] `TopologyChanges` has `speaker_ips: Vec<(SpeakerId, IpAddr)>` and `satellite_ids: Vec<SpeakerId>` fields (`decoder.rs`)
- [x] `decode_topology_event()` extracts IPs from `member.location` URLs (`decoder.rs`)
- [x] `decode_topology_event()` collects satellite IDs from members with `Invisible="1"` (`decoder.rs`)
- [x] `extract_ip_from_location()` helper parses `http://IP:PORT/...` format (`decoder.rs`)
- [x] `StateManager::update_speaker_ip()` updates `SpeakerInfo.ip_address` AND `ip_to_speaker` reverse map (separate locks), removing old IP entry (`state.rs`)
- [x] `StateStore::update_speaker_ip_address()` updates `SpeakerInfo.ip_address` only, for use inside `apply_topology_changes()` (`state.rs`)
- [x] `apply_topology_changes()` applies IP updates from `changes.speaker_ips`, then updates reverse map via `StateManager` (`event_worker.rs`)
- [x] `ensure_topology()` tries all known speaker IPs sequentially, breaks on first success (`system.rs`)
- [x] IP updates are consolidated in `apply_topology_changes()` — no duplicate application in `ensure_topology()` (`system.rs`)
- [x] After `ensure_topology()`, satellite speakers are pruned from the speaker map (`system.rs`)
- [x] After `ensure_topology()`, Speaker handle IPs are refreshed from state store (`system.rs`)
- [x] `SpeakerInfo.satellites` field is populated from topology data (`state.rs`) — stored as `satellite_ids: HashSet<SpeakerId>` on StateStore instead
- [x] All existing tests pass: `cargo test` in sonos-sdk workspace
- [x] New tests for `extract_ip_from_location()` — valid URL, missing prefix, malformed
- [x] New tests for `update_speaker_ip()` — verifies both forward and reverse map update
- [ ] New tests for resilient `ensure_topology()` — first speaker fails, second succeeds (requires network mocking, deferred)

### PR 2: SDK fetch() Coordinator Routing

- [x] `fetch()` calls `resolve_subscription_target()` before SOAP call (`handles.rs`)
- [x] `fetch()` stores result under coordinator's speaker ID for PerCoordinator properties (`handles.rs`)
- [x] `fetch()` falls back to `(self.context.speaker_id, self.context.speaker_ip)` when resolution fails (`handles.rs`)
- [x] For PerSpeaker properties, `fetch()` looks up current IP from state manager (`handles.rs`)
- [x] Verify `get()` calls `get_resolved()` for PerCoordinator properties (confirmed: `get_property()` → `get_resolved()`)
- [x] All existing tests pass: `cargo test` in sonos-sdk workspace
- [ ] New test: fetch() on a group member returns coordinator's data for PerCoordinator property (requires network mocking, deferred)

### PR 3: CLI URI Fallback Display

- [x] `uri_source_label()` function in `helpers.rs` maps URI patterns to display labels
- [x] `cmd_status()` shows URI-based label when track metadata is "Unknown" (`run.rs`)
- [x] `cmd_groups()` shows URI-based label when track metadata is "Unknown" (`run.rs`)
- [x] `track_summary()` falls back to URI label when title and artist are both None (`helpers.rs`)
- [x] Bottom bar in TUI uses `track_summary()` for consistent fallback (`screens/speakers.rs`)
- [x] `x-rincon:` URIs produce empty string (grouped member pointer, no display) (`helpers.rs`)
- [x] All existing tests pass: `cargo test` in sonos-cli
- [x] `track_summary()` signature unchanged — updated internal logic only
- [x] New tests for `uri_source_label()` — x-rincon (empty), empty string, non-empty URI ("Playing (no metadata)")

## Implementation Phases

### Phase 1: SDK Topology Overhaul (Fixes 2+3+4+4b)

**Repo:** `sonos-sdk`  
**Branch:** `fix/topology-overhaul`

| Step | File | Change |
|------|------|--------|
| 1a | `sonos-state/src/decoder.rs` | Add `speaker_ips` and `satellite_ids` to `TopologyChanges`; extract from topology XML |
| 1b | `sonos-state/src/state.rs` | Add `StateManager::update_speaker_ip()` and `StateStore::update_speaker_ip_address()`; add satellite ID storage |
| 1c | `sonos-state/src/event_worker.rs` | Apply IP updates in `apply_topology_changes()` |
| 1d | `sonos-sdk/src/system.rs` | Resilient `ensure_topology()` — try all speakers sequentially |
| 1e | `sonos-sdk/src/system.rs` | Filter satellites from speaker map after topology |
| 1f | `sonos-sdk/src/system.rs` | Refresh Speaker handle IPs from state store after topology |
| 1g | Tests | Unit tests for IP extraction, state update, resilient topology |

### Phase 2: SDK fetch() Coordinator Routing (Fix 1)

**Repo:** `sonos-sdk`  
**Branch:** `fix/fetch-coordinator-routing`

| Step | File | Change |
|------|------|--------|
| 2a | `sonos-sdk/src/property/handles.rs` | Route `fetch()` via `resolve_subscription_target()` |
| 2b | `sonos-sdk/src/property/handles.rs` | Store result under coordinator ID; fresh IP lookup for PerSpeaker |
| 2c | Tests | Test fetch routing for PerCoordinator and PerSpeaker properties |

### Phase 3: CLI URI Fallback Display (Fix 5)

**Repo:** `sonos-cli`  
**Branch:** `fix/uri-fallback-display`  
**Depends on:** PR 1 and PR 2 published to crates.io

| Step | File | Change |
|------|------|--------|
| 3a | `src/tui/helpers.rs` | Add `uri_source_label()`, update `track_summary()` |
| 3b | `src/cli/run.rs` | Apply URI fallback in `cmd_status()` and `cmd_groups()` |
| 3c | `src/tui/screens/speakers.rs` | Use `track_summary()` in bottom bar for consistent fallback |
| 3d | Tests | Unit tests for URI pattern matching |

## Dependencies & Prerequisites

- SDK changes (PRs 1+2) must merge and publish to crates.io before CLI PR 3
- `sonos-api` crate's `ZoneGroupMemberInfo` already has `location: String` and `SatelliteInfo` with `uuid`, `location`, `invisible` fields — no API crate changes needed
- `SpeakerInfo.satellites: Vec<SpeakerId>` field already exists in state model but isn't populated

## Risk Analysis & Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| `extract_ip_from_location()` encounters unexpected URL formats | Low — IP just not updated | Log warning, continue with cached IP |
| All speakers unreachable during `ensure_topology()` | Medium — no groups visible | Same as current behavior (silent failure), but now attempted all speakers first |
| Satellite filtering removes a speaker the user expects to see | Low — satellites are invisible by design | Only filter speakers marked `Invisible="1"` in topology |
| fetch() routing changes break existing CLI commands | Medium — "Unknown" metadata | Fallback to current behavior on resolution failure |
| Mid-session IP change during active playback | Low — events continue via HTTP callbacks | UPnP push model means speaker IP change doesn't break subscriptions |

## Verification Plan

1. `cargo test` in sonos-sdk workspace
2. `cargo test` in sonos-cli
3. `cargo run -- speakers` — Basement should appear once (not duplicated by satellites), Office/Roam should show playback state + track
4. `cargo run -- status` — Should show track metadata for the default group
5. `cargo run -- status --speaker "Living Room"` — Should show the coordinator's track data (routed via fetch)
6. `cargo run -- groups` — All groups should show track info where available, URI-based fallback where not
7. Launch TUI (`cargo run`) — verify bottom bar shows track/progress for actively playing groups
8. Bedroom group — should show "Spotify (connect)" instead of "Unknown" (URI fallback)

## Sources & References

### Origin

- **Brainstorm document:** [docs/brainstorms/2026-05-01-data-freshness-fixes-brainstorm.md](../brainstorms/2026-05-01-data-freshness-fixes-brainstorm.md) — Key decisions: combined topology overhaul (fixes 2+3+4), mid-session IP refresh via existing event_worker path, TUI as thin rendering engine principle, all architecture review gaps resolved.

### Internal References

- SDK handles: `../sonos-sdk/sonos-sdk/src/property/handles.rs` — `fetch()` at line 496, `watch()` at line 334, `SpeakerContext` at line 26
- SDK system: `../sonos-sdk/sonos-sdk/src/system.rs` — `ensure_topology()` at line 481, `build_speakers()` at line 307, `from_devices_inner()` at line 161
- State decoder: `../sonos-sdk/sonos-state/src/decoder.rs` — `decode_topology_event()` at line 308, `TopologyChanges` at line 33
- State store: `../sonos-sdk/sonos-state/src/state.rs` — `resolve_subscription_target()` at line 694, `ip_to_speaker` map
- Event worker: `../sonos-sdk/sonos-state/src/event_worker.rs` — `apply_topology_changes()` at line 141, topology event handling at line 49
- API topology types: `../sonos-sdk/sonos-api/src/services/zone_group_topology/events.rs` — `ZoneGroupMemberInfo` at line 152, `SatelliteInfo` at line 188
- CLI commands: `src/cli/run.rs` — `cmd_status()` at line 203, `cmd_groups()` at line 116
- CLI resolve: `src/cli/resolve.rs` — target resolution priority chain
- TUI helpers: `src/tui/helpers.rs` — `track_summary()` at line 7
- TUI hooks: `src/tui/hooks.rs` — `use_watch()` at line 230, `use_watch_group()` at line 260
- TUI speakers screen: `src/tui/screens/speakers.rs` — bottom bar assembly at line 189
- SDK API reference: `docs/references/sonos-sdk.md`
- Roadmap: `docs/product/roadmap.md` — Milestones 2, 7, 8
- Precedent cross-repo plan: `docs/plans/2026-03-08-feat-sdk-level-discovery-caching-plan.md`
