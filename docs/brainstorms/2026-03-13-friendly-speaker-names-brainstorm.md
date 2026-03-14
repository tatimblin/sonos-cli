# Friendly Speaker and Group Names

**Date:** 2026-03-13
**Status:** decided

## What We're Building

Use Sonos room names (from `<roomName>` XML) as the primary speaker identity instead of the verbose `<friendlyName>` string (which contains IP, model, and RINCON ID). Since groups are identified by their coordinator speaker's name, this fix propagates to group names automatically.

**Before:** `sonos volume 30 --speaker "192.168.4.126 - Sonos Roam 2 - RINCON_C43875CA135801400"`
**After:** `sonos volume 30 --speaker "Kitchen"`

## Why This Approach

The data is already parsed — the discovery layer extracts `<roomName>` from UPnP XML into `Device.room_name`. It's just never used due to a bug in `StateManager::add_devices()` that copies `device.name` (friendly_name) instead of `device.room_name`.

The fix is a single line change in the SDK state layer, plus using `room_name` as the speaker HashMap key and `Speaker.name` source. No new features — just using the correct field.

## Key Decisions

1. **Room name only.** Speaker name is the `<roomName>` value (e.g., "Kitchen"). No model suffix. Falls back to `friendly_name` if `room_name` is absent or "Unknown".

2. **Case-insensitive matching.** `--speaker kitchen` matches "Kitchen". Apply to both `SonosSystem::speaker()` and `SonosSystem::group()` lookups.

3. **Groups inherit coordinator's room name.** `--group "Living Room"` works because groups are identified by their coordinator speaker's name. No separate group naming needed.

4. **SDK-level fix.** Changes go in `sonos-sdk` / `sonos-state` / `sonos-discovery` — the CLI doesn't need to change (it already uses `speaker.name` for display and matching).

## Changes Required

### SDK (sonos-sdk repo)

**sonos-state/src/state.rs — `add_devices()`**
Fix the bug: change `room_name: device.name.clone()` to `room_name: device.room_name.clone()`.

**sonos-sdk/src/system.rs — `build_speakers_with_init()`**
Use `device.room_name` (with fallback to `device.name`) as:
- The `Speaker.name` field
- The HashMap key for speaker lookup

**sonos-sdk/src/system.rs — `speaker()` and `group()` lookups**
Make name comparison case-insensitive (`.eq_ignore_ascii_case()` or lowercase keys).

### CLI (sonos-cli repo)

No changes needed — already uses `speaker.name` everywhere.

## Open Questions

None — all decisions resolved.
