---
title: "fix: Use room names for speaker and group identity"
type: fix
status: completed
date: 2026-03-13
origin: docs/brainstorms/2026-03-13-friendly-speaker-names-brainstorm.md
---

# fix: Use room names for speaker and group identity

## Overview

Sonos speakers expose two names in UPnP XML: `<friendlyName>` (verbose, e.g., `"192.168.4.126 - Sonos Roam 2 - RINCON_C43875CA135801400"`) and `<roomName>` (user-assigned, e.g., `"Kitchen"`). The SDK currently uses `friendlyName` everywhere. Switch to `roomName` as the primary identity for speakers and groups, and add case-insensitive matching.

**Before:** `sonos volume 30 --speaker "192.168.4.126 - Sonos Roam 2 - RINCON_C43875CA135801400"`
**After:** `sonos volume 30 --speaker "kitchen"`

## Problem Statement

1. **Bug in `add_devices()`** — `SpeakerInfo.room_name` is set to `device.name` (friendlyName) instead of `device.room_name` (actual room name). Line 313 of `sonos-state/src/state.rs`.
2. **Wrong field used as identity** — `Speaker.name` and the HashMap key in `build_speakers_with_init()` both use `device.name` (friendlyName) instead of `device.room_name`.
3. **Case-sensitive matching** — `system.speaker("kitchen")` fails to match `"Kitchen"`.

(see brainstorm: `docs/brainstorms/2026-03-13-friendly-speaker-names-brainstorm.md`)

## Proposed Solution

### Name resolution function

Add a helper to compute the display name from a `Device`, used consistently everywhere:

```rust
// sonos-sdk/src/system.rs (or a shared util)
fn display_name(device: &Device) -> String {
    if device.room_name.is_empty() || device.room_name == "Unknown" {
        device.name.clone() // fall back to friendlyName
    } else {
        device.room_name.clone()
    }
}
```

### Fix 1: `SpeakerInfo.name` and `SpeakerInfo.room_name` in `add_devices()`

**File:** `sonos-state/src/state.rs:312-313`

```rust
// Before:
name: device.name.clone(),
room_name: device.name.clone(),

// After:
name: display_name(&device),
room_name: device.room_name.clone(),
```

Both fields must change. `SpeakerInfo.name` is read by `group()` at `system.rs:566` and by `Group::coordinator()` / `Group::members()` when reconstructing `Speaker` handles. If only `room_name` is fixed but `name` stays as friendlyName, group lookups and all speaker names from groups will be wrong.

### Fix 2: Speaker HashMap key and `Speaker.name` in `build_speakers_with_init()`

**File:** `sonos-sdk/src/system.rs:253-263`

```rust
// Before:
device.name.clone() // used for Speaker.name and HashMap key

// After:
display_name(&device) // used for both
```

### Fix 3: `Speaker::from_device()` constructor

**File:** `sonos-sdk/src/speaker.rs:199`

```rust
// Before:
device.name.clone()

// After:
display_name(&device)
```

This is a public API — must stay consistent with `build_speakers_with_init()`.

### Fix 4: Case-insensitive lookups

**Strategy:** Keep original-case keys in the HashMap. On miss from `HashMap::get()`, fall back to iterating entries with `eq_ignore_ascii_case()`. This preserves original casing for display (`speaker_names()`, `sonos speakers`) while enabling case-insensitive matching.

**Four lookup sites to update:**

| File | Line | Method | Current |
|------|------|--------|---------|
| `sonos-sdk/src/system.rs` | 280 | `speaker()` initial lookup | `self.speakers.read().ok()?.get(name)` |
| `sonos-sdk/src/system.rs` | 285 | `speaker()` post-rediscovery | `self.speakers.read().ok()?.get(name)` |
| `sonos-sdk/src/system.rs` | 566 | `group()` coordinator match | `si.name == name` |
| `sonos-sdk/src/group.rs` | 202 | `Group::speaker()` | `s.name == name` |

For the HashMap lookups, extract a helper:

```rust
fn find_speaker_by_name(speakers: &HashMap<String, Speaker>, name: &str) -> Option<Speaker> {
    // Try exact match first (O(1))
    if let Some(speaker) = speakers.get(name) {
        return Some(speaker.clone());
    }
    // Fall back to case-insensitive (O(n), n < 50 speakers)
    speakers.values()
        .find(|s| s.name.eq_ignore_ascii_case(name))
        .cloned()
}
```

For the `group()` and `Group::speaker()` comparisons, change `==` to `.eq_ignore_ascii_case()`.

### Fix 5: Duplicate room name warning

In `build_speakers_with_init()`, when `HashMap::insert()` returns `Some(old)` (overwrite), log a warning:

```rust
if let Some(old) = speakers.insert(name.clone(), speaker) {
    eprintln!("warning: duplicate speaker name \"{}\", keeping last discovered", name);
}
```

### Fix 6: `with_speakers()` test helper

**File:** `sonos-sdk/src/system.rs:201-213`

The test helper already sets `name` and `room_name` to the same value. Keep this behavior — tests use short names like `"Kitchen"` which are already room-name-like.

## Acceptance Criteria

- [x] `sonos speakers` shows room names (e.g., "Kitchen") not friendlyNames
- [x] `sonos volume 30 --speaker "Kitchen"` works
- [x] `sonos volume 30 --speaker "kitchen"` works (case-insensitive)
- [x] `sonos groups` shows coordinator room names
- [x] `--group "Living Room"` matches coordinator's room name
- [x] `--group "living room"` works (case-insensitive)
- [x] Speaker with no `<roomName>` in XML falls back to friendlyName
- [x] Speaker with `room_name == "Unknown"` falls back to friendlyName
- [x] Duplicate room names: last-write-wins with warning logged
- [x] `Group::coordinator().name` returns room name
- [x] `Group::members()[n].name` returns room name
- [x] `speaker_names()` returns original-case room names
- [x] All existing SDK tests pass
- [x] All existing CLI tests pass (no CLI changes needed)

## Files Modified

| File | What changes |
|------|-------------|
| `sonos-sdk/src/system.rs` | `display_name()` helper, `build_speakers_with_init()` uses room_name, `speaker()` case-insensitive, `group()` case-insensitive, duplicate warning |
| `sonos-state/src/state.rs` | `add_devices()` fix: `name` and `room_name` fields |
| `sonos-sdk/src/speaker.rs` | `Speaker::from_device()` uses room_name |
| `sonos-sdk/src/group.rs` | `Group::speaker()` case-insensitive |

## Tests to Add

- Case-insensitive `system.speaker("kitchen")` matches `"Kitchen"`
- Case-insensitive `system.group("living room")` matches `"Living Room"`
- `display_name()` returns `room_name` when present, `name` when `room_name` is `"Unknown"` or empty
- `SpeakerInfo.room_name` correctly set from `device.room_name` (not `device.name`)
- `Group::coordinator().name` returns room name, not friendlyName

## Out of Scope

- Fuzzy / "did you mean" suggestions on speaker-not-found errors
- Unicode case folding (ASCII is sufficient for room names)
- Cache migration (cache schema unchanged — behavioral change is transparent)

## Sources

- **Origin brainstorm:** [docs/brainstorms/2026-03-13-friendly-speaker-names-brainstorm.md](docs/brainstorms/2026-03-13-friendly-speaker-names-brainstorm.md) — key decisions: room name only, case-insensitive matching, SDK-level fix, groups inherit coordinator name
