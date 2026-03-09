---
title: "feat: Discovery & System Commands"
type: feat
status: active
date: 2026-03-08
origin: docs/brainstorm/2026-03-01-sonos-cli-architecture-brainstorm.md
milestone: "Milestone 2: CLI — Discovery & System Commands"
---

# feat: Discovery & System Commands

## Overview

Wire up the first real SDK integration in sonos-cli. Today every executor handler is a stub returning placeholder strings — this plan replaces those stubs with live SSDP discovery, cache-backed speaker/group lookup, and four working commands: `discover`, `speakers`, `groups`, and `status`. After this milestone, a user with no config file can run `sonos discover` and then `sonos groups` and see their actual Sonos system state.

Implements **Milestone 2** from `docs/product/roadmap.md`.

## Problem Statement / Motivation

The project foundation (Milestone 1) is complete — the Action enum, cache infrastructure, config system, error types, and CLI parsing for these four commands all exist. But the executor is entirely stubbed: every action returns a hardcoded success string without touching the SDK. There is no way to actually discover or interact with Sonos speakers.

This is the critical path. Every subsequent milestone (playback, volume, TUI) depends on a working discovery + system bootstrap flow.

## Proposed Solution

Three areas of work:

1. **System bootstrap in `main.rs`** — Load cache, check staleness, create `SonosSystem` (from cache or fresh discovery), pass it to the executor.
2. **Executor implementation** — Replace stubs for `Discover`, `ListSpeakers`, `ListGroups`, and `Status` with real SDK calls.
3. **Auto-rediscovery** — When a targeted speaker/group isn't in cache, rediscover once before failing.

### Architecture: System Bootstrap

The core question is how `main.rs` obtains a `SonosSystem` to pass to the executor. The flow:

```
main.rs CLI path:
  1. Config::load()
  2. If command == Discover → skip cache, run SSDP, build SonosSystem, save cache
  3. Else → cache::load()
     a. Cache exists and fresh → SonosSystem::from_discovered_devices(cached_devices)
     b. Cache missing or stale → run SSDP discovery, build SonosSystem, save cache
  4. execute(action, &system) → print result
```

**Key design choice:** The `discover` command always runs fresh SSDP regardless of cache state. All other commands prefer the cache but fall back to discovery.

### Architecture: Cache → SonosSystem Reconstruction

The cache stores `Vec<CachedSpeaker>` and `Vec<CachedGroup>`. To create a `SonosSystem` from cache:

- Convert `CachedSpeaker` entries back to `sonos_discovery::Device` structs (id, name, ip, model_name, port=1400)
- Call `SonosSystem::from_discovered_devices(devices)`

This means the cache effectively stores the SSDP discovery result in a serializable form. The SDK handles all internal wiring (event manager, state manager, speaker handles) from the device list.

### Architecture: Auto-Rediscovery

When `resolve_target()` can't find a speaker/group in the current `SonosSystem`:

1. Run fresh SSDP discovery
2. Build a new `SonosSystem`
3. Save updated cache
4. Retry the lookup
5. If still not found → return `CliError::SpeakerNotFound` / `CliError::GroupNotFound`

This requires the executor to have mutable access to the system, or to return a signal that triggers rediscovery in `main.rs`. The simpler approach: **make the executor own a mutable reference** and handle rediscovery internally.

## Technical Considerations

### Discovery Performance

SSDP discovery blocks for 3 seconds by default. For CLI one-off commands, this is acceptable on first run but annoying on every run. The cache (24h TTL) solves this — subsequent commands are instant.

For TTY sessions, show a stderr spinner during the 3s scan (see brainstorm: `docs/brainstorm/2026-03-01-sonos-cli-architecture-brainstorm.md`). Non-TTY sessions (piped output) skip the spinner.

### Group Topology

Groups require watching `group_membership` on at least one speaker to trigger the ZoneGroupTopology subscription (per SDK docs §2). The `groups` and `status` commands must do this before calling `system.groups()`.

### Thread Safety

`SonosSystem` is not `Send`/`Sync` by default (it manages internal threads). Since the CLI is single-threaded, this isn't a concern. The executor receives `&SonosSystem` and makes blocking SDK calls sequentially.

### Cache Format Stability

The `CachedSpeaker` and `CachedGroup` structs are already defined in `src/cache.rs`. The `save()` and `load()` functions are implemented with atomic writes. No changes needed to the cache schema — just need to actually call `save()` after discovery.

## Acceptance Criteria

### Discovery Command (`sonos discover`)

- [ ] Calls `sonos_discovery::get_with_timeout(Duration::from_secs(3))` for SSDP scan
- [ ] Builds `SonosSystem::from_discovered_devices(devices)`
- [ ] Saves discovered speakers/groups to cache via `cache::save()`
- [ ] Prints each discovered speaker: name, model, IP (one per line)
- [ ] Prints summary line: `"Discovery complete. N speakers in M groups."`
- [ ] Shows stderr spinner during scan when stdout is a TTY
- [ ] On no speakers found: `error: discovery timed out — no speakers found\nCheck that your Sonos speakers are on and connected to the same network.`
- [ ] Exit code 0 on success, 1 on failure

### Speakers Command (`sonos speakers`)

- [ ] Loads from cache (or rediscovers if stale/missing)
- [ ] For each speaker: fetches `volume` and `playback_state` via property handles
- [ ] Prints table: `name  state_icon state_text  vol:N  (group_name)`
- [ ] Format matches CLI reference: `Bedroom One  ▶ Playing  vol:65  (Living Room)`
- [ ] On empty cache with no speakers found: `error: no speakers in cache\nRun 'sonos discover' to find speakers on your network.`

### Groups Command (`sonos groups`)

- [ ] Loads from cache (or rediscovers if stale/missing)
- [ ] Watches `group_membership` on at least one speaker to trigger topology subscription
- [ ] For each group: coordinator name, current track, playback state, volume
- [ ] Format matches CLI reference: `Living Room  ▶ Playing  Bohemian Rhapsody — Queen  vol:80`
- [ ] On empty cache: `error: no groups in cache\nRun 'sonos discover' to find speakers on your network.`

### Status Command (`sonos status`)

- [ ] Resolves target via `--group` / `--speaker` / default
- [ ] Fetches: `current_track`, `playback_state`, `position`, `volume`
- [ ] Prints: `group_name  state_icon state_text  track — artist  position/duration  vol:N`
- [ ] Format matches CLI reference: `Living Room  ▶ Playing  Bohemian Rhapsody — Queen  2:31/5:55  vol:80`
- [ ] On target not found: `error: group "X" not found\nRun 'sonos discover' to refresh the speaker list.`

### System Bootstrap & Auto-Rediscovery

- [ ] `main.rs` loads cache, checks staleness, creates `SonosSystem`
- [ ] Stale or missing cache triggers automatic rediscovery
- [ ] Auto-rediscovery on cache miss: if targeted speaker/group not in system, rediscover once before failing
- [ ] All error messages follow format: `error: <description>\n<recovery action>`
- [ ] Exit code 1 for runtime errors

### resolve_target() Integration

- [ ] `Target::Speaker(name)` → `system.get_speaker_by_name(&name)` → `CliError::SpeakerNotFound` if `None`
- [ ] `Target::Group(name)` → find group where coordinator name matches → `CliError::GroupNotFound` if `None`
- [ ] `Target::Default` → `config.default_group` → first discovered group → error if no groups

## Implementation Plan

### ~~Phase 1: System Bootstrap (`src/main.rs`)~~ (Superseded)

> **Superseded by `docs/plans/2026-03-08-feat-sdk-level-discovery-caching-plan.md`.**
> System bootstrap is now handled by `SonosSystem::new()` which has cache-first logic
> built into the SDK. The CLI calls `SonosSystem::new()?` directly — no CLI-side cache
> orchestration needed.

### ~~Phase 2: Executor — Discovery (`src/executor.rs`)~~ (Superseded)

> **Superseded.** `Action::Discover` and `Commands::Discover` have been removed.
> The SDK handles discovery transparently — no explicit discover command needed.

### Phase 3: Executor — Speakers & Groups (`src/executor.rs`)

Replace `ListSpeakers` and `ListGroups` stubs:

**ListSpeakers:**
- `system.speakers()` → iterate
- `speaker.volume.fetch()` → `Volume(u8)`
- `speaker.playback_state.fetch()` → `PlaybackState`
- Need group name for each speaker: `system.get_group_for_speaker(&speaker.id)` → coordinator name

**ListGroups:**
- Watch `group_membership` on first speaker to trigger topology
- `system.groups()` → iterate
- `group.coordinator()` → `Option<Speaker>`
- Coordinator's `current_track.fetch()`, `playback_state.fetch()`
- `group.volume.fetch()` → `GroupVolume(u16)`

### Phase 4: Executor — Status (`src/executor.rs`)

Replace `Status` stub:
- Resolve target to speaker (for speaker target) or coordinator (for group target)
- `speaker.current_track.fetch()` → `CurrentTrack { title, artist, album, ... }`
- `speaker.playback_state.fetch()` → `PlaybackState`
- `speaker.position.fetch()` → `Position { position_ms, duration_ms }`
- `speaker.volume.fetch()` → `Volume(u8)`

### ~~Phase 5: resolve_target() — Real Lookup (`src/executor.rs`)~~ (Superseded)

> **Superseded.** `resolve_target()` now calls `system.get_speaker_by_name()` which
> handles auto-rediscovery transparently in the SDK. Implemented as part of the
> SDK-level discovery caching plan.

### ~~Phase 6: cache::save() — From SonosSystem (`src/cache.rs`)~~ (Superseded)

> **Superseded.** CLI `cache.rs` has been deleted. Cache is now managed entirely
> by the SDK in `sonos-sdk/src/cache.rs`.

### Phase 7: TTY Spinner (`src/main.rs` or utility)

Simple stderr spinner during the 3s discovery scan:
- Check `std::io::stderr().is_terminal()`
- Spin through braille characters (`⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`) on stderr
- Print `"Discovering speakers..."` during scan
- Clear spinner line when done

Keep it simple — no external crate. A few lines of code with `\r` and `eprint!`.

> **Note:** With SDK-level caching, the spinner is only needed on first run or cache
> expiry. `SonosSystem::new()` blocks during SSDP — the spinner would need to wrap
> that call.

## Success Metrics

- `sonos discover` completes against a live Sonos network and prints speaker list
- `sonos speakers` shows all speakers with live volume and playback state
- `sonos groups` shows all groups with coordinator, track, and volume
- `sonos status` shows current track info for the default group
- Second run of any command (after `discover`) loads from cache instantly (no 3s wait)
- Auto-rediscovery works: rename a speaker in Sonos app, run `sonos status --speaker "NewName"`, get auto-rediscovery then success

## Dependencies & Risks

**Dependencies:**
- `sonos-sdk` at `../sonos-sdk/sonos-sdk` must compile and expose the documented API
- A live Sonos system on the local network is required for integration testing

**Risks:**
- **SDK API mismatch:** The SDK reference doc may not perfectly match the actual crate API. May need to adjust calls during implementation.
- **Group topology timing:** `system.groups()` requires a topology subscription to be active. If `group_membership.watch()` is called but topology hasn't arrived yet, `groups()` may return empty. May need a brief wait or retry.
- **SSDP on restricted networks:** Discovery may fail on some network configurations (VLANs, mDNS disabled). Error messages must be clear.

## Sources & References

### Origin

- **Brainstorm document:** [docs/brainstorm/2026-03-01-sonos-cli-architecture-brainstorm.md](../brainstorm/2026-03-01-sonos-cli-architecture-brainstorm.md) — Key decisions carried forward: Action dispatch pattern, cache-first discovery flow, targeting rules (group wins over speaker).

### Internal References

- Roadmap milestone: `docs/product/roadmap.md` lines 198-285 (Milestone 2)
- SDK API reference: `docs/references/sonos-sdk.md` — discovery (§6), speaker lookups (§2), property handles (§5)
- CLI command reference: `docs/references/cli-commands.md` — discover, speakers, groups, status output formats
- Cache implementation: `src/cache.rs` — CachedSystem, load/save/is_stale
- Config implementation: `src/config.rs` — Config::load(), default_group
- Error types: `src/errors.rs` — CliError enum with recovery_hint()
- Executor stubs: `src/executor.rs` — current stub implementations to replace
- CLI parsing: `src/cli/mod.rs` — Commands enum already includes Discover, Speakers, Groups, Status
