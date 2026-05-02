# Data Freshness & Completeness Fixes

**Date:** 2026-05-01  
**Status:** Validated  
**Scope:** sonos-sdk + sonos-cli

## What We're Building

Fixes for 5 validated issues causing incomplete or stale data in both the SDK and CLI. The symptoms users see: "Unknown" track metadata, missing playback state for entire groups, duplicate speaker names (satellite surround speakers), and timeouts when a portable speaker changes IP.

## Why This Approach

The root causes span both repos but cluster into three natural workstreams:

1. **Topology initialization overhaul (SDK)** — Fixes 2+3+4 all touch `ensure_topology()` and the speaker map. Combining them avoids repeated churn in the same function and produces a coherent startup sequence: try multiple speakers, refresh IPs from the authoritative topology response, prune invisible satellites.

2. **Coordinator routing for fetch (SDK)** — Fix 1 is an isolated change to `PropertyHandle::fetch()` that mirrors what `watch()` already does. Independent of topology.

3. **URI fallback display (CLI)** — Fix 5 is CLI-only polish. When track metadata is genuinely unavailable (older firmware, VLI protocol), show useful info from the URI instead of "Unknown".

## Architectural Principle: TUI as Thin Rendering Engine

The sonos-cli TUI should be a **pure rendering layer** over SDK state. All dynamic data flows through watched properties via hooks. The SDK owns the truth; the TUI just displays it.

**Audit result (2026-05-01):** The TUI largely follows this pattern correctly (84/85 code locations audited). Key findings:

- **Correct:** All screen data comes from `use_watch()` / `use_watch_group()` hooks that create fresh snapshots each frame. Widgets are pure render functions with no SDK calls. Handlers mutate via SDK methods and let watch events propagate state changes reactively.

- **One minor deviation:** `handlers/speaker_list.rs:183` calls `playback_state.get()` synchronously in the play/pause handler to decide whether to call `play()` or `pause()`. The SDK's `get()` is a cached lookup (no network I/O), so this is safe in practice, but for consistency should read from the last watched value.

- **Implication for these fixes:** Fix 1 (fetch routing) primarily benefits the **CLI commands** since the TUI uses `watch()` which already routes correctly. Fixes 2-4 (topology overhaul) benefit both — the TUI reads topology via `system.groups()` which depends on `ensure_topology()`. Fix 5 (URI fallback) benefits both CLI and TUI display layers.

- **Rule going forward:** Never add `fetch()` calls to screens or widgets. Use `use_watch()` for SDK properties, `use_state()` for derived/local state. Handlers should mutate via SDK methods and trust the event system to propagate changes.

## Key Decisions

### Topology overhaul scope (Fixes 2+3+4)

`ensure_topology()` becomes the single source of truth at startup:

- **Resilient speaker selection (Fix 2):** Try all speaker IPs in sequence until one responds to `GetZoneGroupState`. Current code picks one random speaker; if unreachable, silently fails.

- **IP refresh from topology (Fix 4):** The topology response's `ZoneGroupMemberInfo.location` contains each speaker's current IP (e.g., `http://192.168.4.200:1400/...`). Extract these and update the speaker map + state manager. This is the highest-impact fix — it resolves stale cached IPs from DHCP changes (Roam .198 -> .200). Must work both at startup AND mid-session — see below.

- **Satellite filtering (Fix 3):** After topology loads, remove speakers whose IDs appear as satellites (`Invisible="1"`) in the topology. Requires propagating satellite IDs from `decode_topology_event()` — currently parsed by sonos-api but discarded by the decoder. Add a `satellite_ids: Vec<SpeakerId>` field to `TopologyChanges`.

### fetch() coordinator routing (Fix 1)

Add coordinator routing to `PropertyHandle::fetch()` using the existing `resolve_subscription_target()` method. The `SonosProperty::SERVICE` trait constant provides the `Service` enum needed for routing. Store the result under the coordinator's speaker ID. This makes `fetch()` consistent with `watch()`, which already routes PerCoordinator services.

**Nuance:** This primarily benefits CLI commands (`sonos status`, `sonos groups`). The TUI already gets correct data because it uses `watch()` which routes correctly. The TUI would still benefit indirectly — any code path that calls `fetch()` as a fallback (e.g., hooks.rs `use_watch` error path calls `prop.get()` which reads the cache populated by `fetch()`).

### URI fallback display (Fix 5)

When `CurrentTrack.display()` returns "Unknown", check `CurrentTrack.uri` for recognizable patterns:

| URI Pattern | Display |
|-------------|---------|
| `x-sonos-spotify:...` or `spotify:` in URI | "Spotify" |
| `x-sonos-vli:...,spotify:...` | "Spotify (connect)" |
| `x-rincon:RINCON_...` | Skip (group member pointer) |
| `x-sonos-http:...` | "Web stream" |
| Other | Truncated URI |

Applies to CLI (`cmd_status`, `cmd_groups`) and TUI (`track_summary` helper, bottom bar).

### Mid-session IP refresh (Fix 4b)

Fix 4 at startup is necessary but not sufficient. Portable speakers (Roam, Move) can change IPs while the TUI is running. The infrastructure for mid-session refresh already exists:

- `event_worker.rs:49-54` already processes ZoneGroupTopology events via `decode_topology_event()` + `apply_topology_changes()`
- `apply_topology_changes()` at event_worker.rs:141-210 already updates group membership and boot_seq for each speaker
- `ZoneGroupMemberInfo.location` contains the current IP but is **currently discarded** by the decoder

The fix: extend `TopologyChanges` to include `speaker_ips: Vec<(SpeakerId, IpAddr)>`, extract IPs from `location` URLs in `decode_topology_event()`, and apply them in `apply_topology_changes()` alongside the existing boot_seq update. This means every time a topology event arrives (group changes, speaker joins/leaves, speaker reboots), IPs get refreshed automatically.

This is the same data path for both startup (Fix 4) and mid-session (Fix 4b) — the decoder produces `TopologyChanges`, consumers apply it. One code path, two contexts.

Additionally, IP updates must also refresh the `ip_to_speaker` reverse map in the state store (used by the event worker to route incoming UPnP events) and be visible to existing `PropertyHandle` instances (resolved by having `fetch()`/`watch()` look up IPs from the state manager at call time rather than using the frozen `SpeakerContext.speaker_ip`).

## Live Evidence

Validated against real hardware on 2026-05-01:

| Speaker | IP (cached) | IP (actual) | Issue |
|---------|-------------|-------------|-------|
| Office/Roam (Roam 2) | .198 | .200 | Stale cache, coordinator unreachable |
| Living Room (Amp) | .191 | .191 | Group member, gets NOT_IMPLEMENTED from own IP |
| Bedroom (Connect:Amp) | .193 | .193 | Coordinator but VLI protocol returns no metadata |
| Basement (Playbar) | .167 | .167 | OK, but satellites at .48/.47 cause duplicate names |

## Implementation Order

1. **PR 1 (SDK): Topology overhaul** — Fixes 2+3+4, all in `system.rs` + `decoder.rs`
2. **PR 2 (SDK): fetch() routing** — Fix 1, isolated to `handles.rs`
3. **PR 3 (CLI): URI fallback** — Fix 5, `run.rs` + `helpers.rs`

## Resolved Questions

Architecture review (2026-05-01) surfaced three issues. Resolutions:

### SpeakerContext IP is frozen in Arc (was HIGH gap)

Every `PropertyHandle` captures `speaker_ip` inside `Arc<SpeakerContext>` at construction. Updating `SpeakerInfo.ip_address` in the state store doesn't propagate to existing handles.

**Resolution:** Have `fetch()` and `watch()` look up the IP from the state manager at call time instead of using the frozen `self.context.speaker_ip`. This is already partially in place — Fix 1 calls `resolve_subscription_target()` which reads the state manager. Extend this so the IP is always resolved fresh from state, not from the captured context. The `SpeakerContext.speaker_ip` becomes the fallback when the state manager doesn't have an IP (e.g., speaker not yet in state).

### ip_to_speaker reverse map must be updated (was MEDIUM gap)

The event worker uses `ip_to_speaker: HashMap<IpAddr, SpeakerId>` to route incoming UPnP events. A stale IP means events from the new IP are silently dropped.

**Resolution:** When Fix 4/4b updates `SpeakerInfo.ip_address`, also update the `ip_to_speaker` map: remove the old IP entry, insert the new one. This happens in the same `apply_topology_changes()` path.

### fetch() cache key mismatch with get() (was HIGH risk)

Fix 1 stores fetched data under the coordinator's speaker ID. But `PropertyHandle::get()` reads `self.context.speaker_id` (the member). For PerCoordinator properties, `get()` already uses `get_resolved()` which reads from the coordinator's property bag. So storing under coordinator ID is correct — `get()` will find it via the existing resolution path.

**Verification needed during implementation:** Confirm `get()` calls `get_resolved()` for PerCoordinator properties, not a raw lookup.

## Open Questions

None — all gaps identified by architecture review have been resolved.
