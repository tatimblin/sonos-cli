# Fix EventInitFn Propagation Through Groups

## What We're Building

A fix for the bug where the TUI shows speakers as "not playing" with no volume. The root cause: speakers obtained via `Group::coordinator()` and `Group::members()` are created with `event_init: None`, so `watch()` always falls back to `CacheOnly` mode — no UPnP event subscriptions, no live data.

## Problem Diagnosis

Tracing instrumentation revealed the full chain:

1. TUI renders → calls `app.watch(&coordinator.playback_state)`
2. `coordinator` comes from `group.coordinator()` → `Speaker::new()` → `event_init: None`
3. `PropertyHandle::watch()` checks `self.context.event_init` → finds `None`
4. Falls back to `CacheOnly` mode → no subscriptions → no events → stale UI

The `EventInitFn` closure (which lazily creates the `SonosEventManager`) is currently stored per-`SpeakerContext`. It's set correctly for speakers created by `SonosSystem::from_devices_inner()`, but `Group` doesn't know about it. Any speaker constructed through Group gets `None`.

## Why This Approach

**Decision: Move EventInitFn from per-speaker SpeakerContext to StateManager as the single source of truth.**

The init closure is identical for ALL speakers — it's a system-level concern, not per-speaker. Storing it on `StateManager` (already shared via `Arc` everywhere) means:

- **Zero changes to Group, Speaker, or SpeakerContext constructors** — no threading through factories
- **Any speaker from any code path gets access automatically** — no forgetting to pass it
- **Single source of truth** — remove the redundant `event_init` field from `SpeakerContext` entirely
- **Eliminates the "test mode?" log noise** — cleaner separation between "no init fn set yet" and "test mode"

### Alternatives Considered

1. **Thread EventInitFn through Group** — Add the field to Group, pass through `from_info()`, propagate to `coordinator()`/`members()`. Rejected: verbose, every new Group factory must remember to pass it.

2. **StateManager fallback** — Keep per-speaker `event_init` but add StateManager as fallback. Rejected: two sources of truth is worse than one.

## Key Decisions

- EventInitFn moves to StateManager exclusively (remove from SpeakerContext)
- `SonosSystem::from_devices_inner()` sets the init fn on StateManager after construction
- `PropertyHandle::watch()` resolves the init fn from `self.context.state_manager` instead of `self.context.event_init`
- `Speaker::new_with_event_init()` and `SpeakerContext::with_event_init()` are removed
- `build_speakers_with_init()` loses the `event_init` parameter

## Scope

**SDK files affected:**
- `sonos-state/src/state.rs` — add storage + setter + getter for EventInitFn
- `sonos-sdk/src/property/handles.rs` — remove `event_init` from SpeakerContext, update `watch()` to use StateManager
- `sonos-sdk/src/speaker.rs` — remove `new_with_event_init()`, simplify `new()`
- `sonos-sdk/src/system.rs` — set init fn on StateManager, simplify `build_speakers_with_init()`

**CLI files:** None (TUI automatically benefits from SDK fix)

## Success Criteria

- TUI `watch()` calls resolve to `Events` mode (not `CacheOnly`)
- Log shows "Lazy-initializing event manager (first watch() call)" on TUI startup
- Log shows "Event worker started" and event delivery traces
- TUI displays live playback state, volume, and track info
