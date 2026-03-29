# sonos-sdk API Reference

This document is a developer reference for the `sonos-sdk` crate. The crate exposes a
**sync-first, DOM-like API** for controlling Sonos speakers and groups — no `async`/`.await`
required anywhere in the public surface.

The layered internal architecture is:

```
sonos-sdk  (this public API)
    ↓
sonos-state  (property cache + change events)  ←→  sonos-event-manager (UPnP subscriptions)
    ↓                                                       ↓
sonos-api  (typed UPnP operations)                  sonos-stream  (event streaming)
    ↓
soap-client  (blocking HTTP/SOAP transport)
sonos-discovery  (SSDP device discovery)
```

From a CLI author's perspective the only types you will interact with regularly are in
`sonos_sdk::*` (re-exported at the crate root). Everything below describes what those
types contain and how they behave.

---

## Table of Contents

1. [Initialising `SonosSystem`](#1-initialising-sonossystem)
2. [Looking up speakers and groups](#2-looking-up-speakers-and-groups)
3. [Speaker actions](#3-speaker-actions)
4. [Group actions](#4-group-actions)
5. [Property handles](#5-property-handles)
6. [Discovery API](#6-discovery-api)
7. [Cache and optimistic-update model](#7-cache-and-optimistic-update-model)
8. [Change-event iteration](#8-change-event-iteration)
9. [Error types](#9-error-types)
10. [Key types reference](#10-key-types-reference)

---

## 1. Initialising `SonosSystem`

`SonosSystem` is the single entry point. Construction is **blocking** (it runs
discovery synchronously) but **cheap** — the event manager is lazily initialized on
the first `watch()` call, not at construction time.

### `SonosSystem::new() -> Result<SonosSystem, SdkError>`

Cache-first device discovery:

1. Try loading cached devices from `~/.cache/sonos/cache.json`
2. If cache is fresh (< 24 h), use cached devices directly
3. If cache is stale, run SSDP (3 s timeout); fall back to stale cache if SSDP finds nothing
4. If no cache exists, run SSDP discovery
5. If no devices found anywhere, return `Err(SdkError::DiscoveryFailed(...))`

```rust
use sonos_sdk::SonosSystem;

fn main() -> Result<(), sonos_sdk::SdkError> {
    let system = SonosSystem::new()?;
    println!("Found {} speakers", system.speakers().len());
    Ok(())
}
```

### `SonosSystem::from_discovered_devices(devices: Vec<Device>)` (test-support only)

Available when the `test-support` Cargo feature is enabled. Use this in integration
tests when you have already called `sonos_discovery::get()` yourself.

```rust
use sonos_sdk::SonosSystem;

// Only available with `test-support` feature
let devices = sonos_discovery::get_with_timeout(std::time::Duration::from_secs(5));
let system = SonosSystem::from_discovered_devices(devices)?;
```

### `SonosSystem::with_speakers(names: &[&str]) -> SonosSystem` (test-support only)

Creates an in-memory system with synthetic speaker data — no SSDP, no event manager,
no cache reads. Speakers get sequential IPs starting at `192.168.1.100`.

```rust
// Only available with `test-support` feature
let system = SonosSystem::with_speakers(&["Kitchen", "Bedroom"]);
assert_eq!(system.speakers().len(), 2);
assert!(system.speaker("Kitchen").is_some());
```

Construction:

1. Create a `StateManager` (property cache).
2. Register all discovered `Device` objects with the state manager.
3. Create a `Speaker` handle for every device, keyed by friendly name.
4. Event manager is **not** created — it initializes lazily on the first `watch()` call.

---

## 2. Looking up speakers and groups

### Speaker lookups

| Method | Signature | Returns |
|---|---|---|
| `speakers()` | `&self -> Vec<Speaker>` | All speaker handles |
| `speaker_names()` | `&self -> Vec<String>` | All friendly names |
| `speaker(name)` | `(&self, name: &str) -> Option<Speaker>` | By name; auto-rediscovers on miss |
| `speaker_by_id(id)` | `(&self, id: &SpeakerId) -> Option<Speaker>` | By ID; `None` if not found |
| `state_manager()` | `(&self) -> &Arc<StateManager>` | Low-level state access |

`speaker()` triggers an SSDP auto-rediscovery (rate-limited to once per 30 s) when the
name isn't found in the current map. This handles speakers that came online after the
initial discovery.

```rust
let speaker = system
    .speaker("Living Room")
    .ok_or_else(|| SdkError::SpeakerNotFound("Living Room".to_string()))?;

println!("{} — {} at {}", speaker.name, speaker.model_name, speaker.ip);
```

### Group lookups

Groups are populated from the ZoneGroupTopology UPnP service. The first call to any
group method automatically fetches topology on-demand (via `GetZoneGroupState`) if it
hasn't been loaded yet — no need to watch `group_membership` first.

| Method | Signature | Returns |
|---|---|---|
| `groups()` | `&self -> Vec<Group>` | All current groups |
| `group(name)` | `(&self, name: &str) -> Option<Group>` | By coordinator speaker name |
| `group_by_id(id)` | `(&self, id: &GroupId) -> Option<Group>` | By group ID |
| `group_for_speaker(id)` | `(&self, id: &SpeakerId) -> Option<Group>` | Group the speaker belongs to |

Sonos groups don't have independent names — `group(name)` matches by the coordinator
speaker's friendly name.

```rust
for group in system.groups() {
    println!("Group {} — {} members", group.id, group.member_count());
    if let Some(coord) = group.coordinator() {
        println!("  coordinator: {}", coord.name);
    }
}

// Look up by coordinator name
let living_room_group = system.group("Living Room");
```

### Fluent navigation

Speakers and groups link to each other for ergonomic traversal:

```rust
// Speaker → Group
let kitchen = system.speaker("Kitchen").unwrap();
let group = kitchen.group().unwrap();      // group this speaker belongs to

// Group → Speaker
let coord = group.coordinator().unwrap();  // coordinator speaker
let member = group.speaker("Kitchen");     // member by name
let all = group.members();                 // all members
```

### Creating groups from the system level

```rust
let coordinator = system.speaker("Living Room").unwrap();
let member      = system.speaker("Kitchen").unwrap();

let result = system.create_group(&coordinator, &[&member])?;
if !result.is_success() {
    for (id, err) in &result.failed {
        eprintln!("Failed to add {}: {}", id, err);
    }
}
```

`create_group` returns a `GroupChangeResult` (see [Group actions](#4-group-actions)).

---

## 3. Speaker actions

`Speaker` is `Clone`. Its public fields are:

```rust
pub struct Speaker {
    pub id:         SpeakerId,
    pub name:       String,
    pub ip:         IpAddr,
    pub model_name: String,

    // Property handles (see §5)
    pub volume:           VolumeHandle,
    pub mute:             MuteHandle,
    pub bass:             BassHandle,
    pub treble:           TrebleHandle,
    pub loudness:         LoudnessHandle,
    pub playback_state:   PlaybackStateHandle,
    pub position:         PositionHandle,
    pub current_track:    CurrentTrackHandle,
    pub group_membership: GroupMembershipHandle,
}
```

All action methods below make a synchronous UPnP/SOAP network call against the speaker's
IP. On success, methods that change observable state also update the in-memory cache
optimistically (see §7).

### 3.1 Basic playback (AVTransport)

| Method | Parameters | Return | Notes |
|---|---|---|---|
| `play()` | — | `Result<(), SdkError>` | Cache → `PlaybackState::Playing` |
| `pause()` | — | `Result<(), SdkError>` | Cache → `PlaybackState::Paused` |
| `stop()` | — | `Result<(), SdkError>` | Cache → `PlaybackState::Stopped` |
| `next()` | — | `Result<(), SdkError>` | |
| `previous()` | — | `Result<(), SdkError>` | |

```rust
speaker.play()?;
speaker.pause()?;
```

### 3.2 Seek (AVTransport)

```rust
pub fn seek(&self, target: SeekTarget) -> Result<(), SdkError>
```

`SeekTarget` is a type-safe enum that prevents mismatched unit/value combinations:

```rust
pub enum SeekTarget {
    Track(u32),        // 1-based track number → UPnP unit "TRACK_NR"
    Time(String),      // absolute position, e.g. "0:02:30" → "REL_TIME"
    Delta(String),     // time delta, e.g. "+0:00:30" or "-0:00:10" → "TIME_DELTA"
}
```

```rust
speaker.seek(SeekTarget::Time("0:02:30".into()))?;   // jump to 2 min 30 sec
speaker.seek(SeekTarget::Track(3))?;                  // jump to track 3
speaker.seek(SeekTarget::Delta("+0:00:30".into()))?;  // skip forward 30 s
speaker.seek(SeekTarget::Delta("-0:00:10".into()))?;  // rewind 10 s
```

### 3.3 URI / source (AVTransport)

| Method | Parameters | Return |
|---|---|---|
| `set_av_transport_uri(uri, metadata)` | `uri: &str`, `metadata: &str` | `Result<(), SdkError>` |
| `set_next_av_transport_uri(uri, metadata)` | `uri: &str`, `metadata: &str` | `Result<(), SdkError>` |

`metadata` is a DIDL-Lite XML string. Pass `""` when not needed.

```rust
speaker.set_av_transport_uri("x-sonosapi-stream:s123?sid=254", "")?;
```

### 3.4 Transport info queries (AVTransport)

| Method | Return type |
|---|---|
| `get_media_info()` | `Result<GetMediaInfoResponse, SdkError>` |
| `get_transport_settings()` | `Result<GetTransportSettingsResponse, SdkError>` |
| `get_current_transport_actions()` | `Result<GetCurrentTransportActionsResponse, SdkError>` |
| `get_device_capabilities()` | `Result<GetDeviceCapabilitiesResponse, SdkError>` |

**`GetMediaInfoResponse` fields:**

| Field | Type | Description |
|---|---|---|
| `nr_tracks` | `u32` | Number of tracks in the queue |
| `media_duration` | `String` | Total duration (HH:MM:SS) |
| `current_uri` | `String` | Currently playing URI |
| `current_uri_meta_data` | `String` | DIDL-Lite metadata for current URI |
| `next_uri` | `String` | Next URI (for gapless) |
| `next_uri_meta_data` | `String` | DIDL-Lite metadata for next URI |
| `play_medium` | `String` | e.g. `"NETWORK"` |
| `record_medium` | `String` | |
| `write_status` | `String` | |

**`GetTransportSettingsResponse` fields:**

| Field | Type | Description |
|---|---|---|
| `play_mode` | `String` | e.g. `"NORMAL"`, `"SHUFFLE"`, `"REPEAT_ALL"` |
| `rec_quality_mode` | `String` | |

**`GetCurrentTransportActionsResponse` fields:**

| Field | Type | Description |
|---|---|---|
| `actions` | `String` | Comma-separated action names currently available |

**`GetDeviceCapabilitiesResponse` fields:**

| Field | Type | Description |
|---|---|---|
| `play_media` | `String` | Supported play media types |
| `rec_media` | `String` | |
| `rec_quality_modes` | `String` | |

### 3.5 Play mode and crossfade (AVTransport)

| Method | Parameters | Return |
|---|---|---|
| `set_play_mode(mode)` | `mode: PlayMode` | `Result<(), SdkError>` |
| `get_crossfade_mode()` | — | `Result<GetCrossfadeModeResponse, SdkError>` |
| `set_crossfade_mode(enabled)` | `enabled: bool` | `Result<(), SdkError>` |

`PlayMode` enum:

```rust
pub enum PlayMode {
    Normal,           // Sequential, no repeat
    RepeatAll,        // Repeat full queue
    RepeatOne,        // Repeat current track
    ShuffleNoRepeat,  // Shuffle, stop at end
    Shuffle,          // Shuffle with repeat
    ShuffleRepeatOne, // Shuffle, repeat current track
}
```

`GetCrossfadeModeResponse`:

| Field | Type | Description |
|---|---|---|
| `crossfade_mode` | `String` | `"0"` = disabled, `"1"` = enabled |

```rust
speaker.set_play_mode(PlayMode::Shuffle)?;
speaker.set_crossfade_mode(true)?;
```

### 3.6 Sleep timer (AVTransport)

| Method | Parameters | Return |
|---|---|---|
| `configure_sleep_timer(duration)` | `duration: &str` | `Result<(), SdkError>` |
| `cancel_sleep_timer()` | — | `Result<(), SdkError>` |
| `get_remaining_sleep_timer()` | — | `Result<GetRemainingSleepTimerDurationResponse, SdkError>` |

`duration` format is `"HH:MM:SS"`. Pass `""` to cancel (or use `cancel_sleep_timer()`).

`GetRemainingSleepTimerDurationResponse`:

| Field | Type | Description |
|---|---|---|
| `remaining_sleep_timer_duration` | `String` | Time remaining (HH:MM:SS), empty if inactive |
| `current_sleep_timer_generation` | `u32` | Monotonic generation counter |

```rust
speaker.configure_sleep_timer("01:00:00")?;  // sleep after 1 hour
speaker.cancel_sleep_timer()?;
```

### 3.7 Queue operations (AVTransport)

| Method | Parameters | Return |
|---|---|---|
| `add_uri_to_queue(uri, metadata, position, enqueue_as_next)` | `uri: &str`, `metadata: &str`, `position: u32`, `enqueue_as_next: bool` | `Result<AddURIToQueueResponse, SdkError>` |
| `remove_track_from_queue(object_id, update_id)` | `object_id: &str`, `update_id: u32` | `Result<(), SdkError>` |
| `remove_all_tracks_from_queue()` | — | `Result<(), SdkError>` |
| `remove_track_range_from_queue(update_id, starting_index, number_of_tracks)` | all `u32` | `Result<RemoveTrackRangeFromQueueResponse, SdkError>` |
| `save_queue(title, object_id)` | `title: &str`, `object_id: &str` | `Result<SaveQueueResponse, SdkError>` |
| `create_saved_queue(title, uri, metadata)` | all `&str` | `Result<CreateSavedQueueResponse, SdkError>` |
| `backup_queue()` | — | `Result<(), SdkError>` |

**`AddURIToQueueResponse`** fields:

| Field | Type | Description |
|---|---|---|
| `first_track_number_enqueued` | `u32` | Queue position of first added track |
| `num_tracks_added` | `u32` | How many tracks were added |
| `new_queue_length` | `u32` | Total queue length after operation |

**`RemoveTrackRangeFromQueueResponse`** fields:

| Field | Type | Description |
|---|---|---|
| `new_update_id` | `u32` | Updated queue version ID |

**`SaveQueueResponse`** fields:

| Field | Type | Description |
|---|---|---|
| `assigned_object_id` | `String` | Content Directory object ID of the saved playlist |

**`CreateSavedQueueResponse`** fields:

| Field | Type | Description |
|---|---|---|
| `num_tracks_added` | `u32` | |
| `new_queue_length` | `u32` | |
| `assigned_object_id` | `String` | |
| `new_update_id` | `u32` | |

### 3.8 Alarm operations (AVTransport)

| Method | Parameters | Return |
|---|---|---|
| `snooze_alarm(duration)` | `duration: &str` (HH:MM:SS) | `Result<(), SdkError>` |
| `get_running_alarm_properties()` | — | `Result<GetRunningAlarmPropertiesResponse, SdkError>` |

**`GetRunningAlarmPropertiesResponse`** fields:

| Field | Type | Description |
|---|---|---|
| `alarm_id` | `u32` | Alarm identifier |
| `group_id` | `String` | Group the alarm fired in |
| `logged_start_time` | `String` | ISO timestamp when the alarm started |

### 3.9 Volume and EQ (RenderingControl)

| Method | Parameters | Return | Validation |
|---|---|---|---|
| `set_volume(volume)` | `volume: u8` | `Result<(), SdkError>` | 0–100; cache updated |
| `set_relative_volume(adjustment)` | `adjustment: i8` | `Result<SetRelativeVolumeResponse, SdkError>` | −100 to +100; cache updated |
| `set_mute(muted)` | `muted: bool` | `Result<(), SdkError>` | cache updated |
| `set_bass(level)` | `level: i8` | `Result<(), SdkError>` | −10 to +10; cache updated |
| `set_treble(level)` | `level: i8` | `Result<(), SdkError>` | −10 to +10; cache updated |
| `set_loudness(enabled)` | `enabled: bool` | `Result<(), SdkError>` | cache updated |

Out-of-range values are caught before the network call and return
`Err(SdkError::ValidationFailed(...))`.

**`SetRelativeVolumeResponse`** fields:

| Field | Type | Description |
|---|---|---|
| `new_volume` | `u8` | Absolute volume after the adjustment |

### 3.10 Navigation

| Method | Return | Description |
|---|---|---|
| `group()` | `Option<Group>` | The group this speaker belongs to (no network call) |

```rust
let kitchen = system.speaker("Kitchen").unwrap();
if let Some(group) = kitchen.group() {
    println!("Kitchen is in group {} ({} members)", group.id, group.member_count());
}
```

Returns `None` only if topology hasn't been loaded yet (e.g. no group methods or
`watch(group_membership)` have been called).

### 3.11 Group membership (speaker-level)

These are convenience methods that wrap the lower-level group and AVTransport calls.

| Method | Parameters | Return | Notes |
|---|---|---|---|
| `join_group(group)` | `group: &Group` | `Result<(), SdkError>` | Adds this speaker to `group` |
| `leave_group()` | — | `Result<BecomeCoordinatorOfStandaloneGroupResponse, SdkError>` | Becomes standalone |
| `become_standalone()` | — | `Result<BecomeCoordinatorOfStandaloneGroupResponse, SdkError>` | Same as `leave_group()` |
| `delegate_coordination_to(id, rejoin)` | `id: &SpeakerId`, `rejoin: bool` | `Result<(), SdkError>` | Hand off coordinator role |

`join_group` internally calls `group.add_speaker(self)`.

**`BecomeCoordinatorOfStandaloneGroupResponse`** fields:

| Field | Type | Description |
|---|---|---|
| `delegated_group_coordinator_id` | `String` | Who took over as coordinator (may be empty) |
| `new_group_id` | `String` | The new standalone group ID for this speaker |

---

## 4. Group actions

`Group` is `Clone`. Its public fields are:

```rust
pub struct Group {
    pub id:             GroupId,
    pub coordinator_id: SpeakerId,
    pub member_ids:     Vec<SpeakerId>,

    // Property handles (see §5)
    pub volume:            GroupVolumeHandle,
    pub mute:              GroupMuteHandle,
    pub volume_changeable: GroupVolumeChangeableHandle,
}
```

### 4.1 Inspecting membership

| Method | Return | Description |
|---|---|---|
| `coordinator()` | `Option<Speaker>` | Returns the coordinator's `Speaker` handle |
| `members()` | `Vec<Speaker>` | All members including coordinator |
| `speaker(name)` | `Option<Speaker>` | Get a member by friendly name |
| `is_coordinator(id)` | `bool` | Check if a speaker ID is the coordinator |
| `member_count()` | `usize` | Number of members |
| `is_standalone()` | `bool` | `true` when only one member |

```rust
if let Some(coord) = group.coordinator() {
    for member in group.members() {
        let role = if group.is_coordinator(&member.id) { "coordinator" } else { "member" };
        println!("  {} ({})", member.name, role);
    }
}

// Look up a specific member
if let Some(kitchen) = group.speaker("Kitchen") {
    println!("Kitchen volume: {:?}", kitchen.volume.get());
}
```

### 4.2 Membership mutation

| Method | Parameters | Return | Notes |
|---|---|---|---|
| `add_speaker(speaker)` | `speaker: &Speaker` | `Result<(), SdkError>` | Sends `x-rincon:` URI to member; cannot add coordinator to itself |
| `remove_speaker(speaker)` | `speaker: &Speaker` | `Result<(), SdkError>` | Sends `BecomeCoordinatorOfStandaloneGroup` to member; cannot remove coordinator |
| `dissolve()` | — | `GroupChangeResult` | Removes all non-coordinator members, reports per-speaker results |

After any membership change, call `system.groups()` again to see the updated topology.

### 4.3 Group volume and mute (GroupRenderingControl)

All group operations target the coordinator's IP.

| Method | Parameters | Return | Validation |
|---|---|---|---|
| `set_volume(volume)` | `volume: u16` | `Result<(), SdkError>` | 0–100; cache updated |
| `set_relative_volume(adjustment)` | `adjustment: i16` | `Result<SetRelativeGroupVolumeResponse, SdkError>` | −100 to +100; cache updated |
| `set_mute(muted)` | `muted: bool` | `Result<(), SdkError>` | cache updated |
| `snapshot_volume()` | — | `Result<(), SdkError>` | Records per-member volume ratios for proportional changes |

**`SetRelativeGroupVolumeResponse`** fields:

| Field | Type | Description |
|---|---|---|
| `new_volume` | `u16` | New absolute group volume (0–100) |

### 4.4 `GroupChangeResult`

Returned by `dissolve()` and `system.create_group()`. Reports per-speaker outcomes for
operations that touch multiple speakers.

```rust
pub struct GroupChangeResult {
    pub succeeded: Vec<SpeakerId>,
    pub failed:    Vec<(SpeakerId, SdkError)>,
}

impl GroupChangeResult {
    pub fn is_success(&self) -> bool  // all succeeded
    pub fn is_partial(&self) -> bool  // some succeeded, some failed
}
```

---

## 5. Property handles

Every property on `Speaker` and `Group` is exposed through a typed handle. All three
access patterns are synchronous.

### 5.1 `PropertyHandle<P>` (speaker properties)

```rust
// Defined on PropertyHandle<P> for all P: SonosProperty
pub fn get(&self) -> Option<P>
pub fn is_watched(&self) -> bool
pub fn speaker_id(&self) -> &SpeakerId
pub fn speaker_ip(&self) -> IpAddr
pub fn watch(&self) -> Result<WatchHandle<P>, SdkError>

// Defined on PropertyHandle<P> only when P: Fetchable
pub fn fetch(&self) -> Result<P, SdkError>
```

### 5.2 `GroupPropertyHandle<P>` (group properties)

Mirrors the speaker handle but reads from the group property store and targets the
coordinator's IP:

```rust
pub fn get(&self) -> Option<P>
pub fn is_watched(&self) -> bool
pub fn group_id(&self) -> &GroupId
pub fn watch(&self) -> Result<WatchHandle<P>, SdkError>

// When P: GroupFetchable
pub fn fetch(&self) -> Result<P, SdkError>
```

### 5.3 Method semantics

| Method | Network call | Cache | Use case |
|---|---|---|---|
| `get()` | None | Reads | Fast display of last known value |
| `fetch()` | Yes (blocking) | Writes | Show definitive current value |
| `watch()` | No (subscription setup) | Reads current | Returns a `WatchHandle`; hold it to keep the subscription alive. Dropping starts a 50ms grace period. |

`get()` returns `None` until the property has been populated (by `fetch()`, `watch()`, or
an incoming UPnP event).

### 5.4 `WatchHandle<P>`

Returned by `watch()`. RAII handle — dropping it starts a 50ms grace period before the
UPnP subscription is torn down. Re-calling `watch()` within the grace period cancels it
and reuses the existing subscription.

Not `Clone` — each handle is one subscription hold.

```rust
#[must_use]
pub struct WatchHandle<P> {
    // Internal: value, mode, cleanup guard
}

impl<P> WatchHandle<P> {
    pub fn value(&self) -> Option<&P>
    pub fn has_value(&self) -> bool
    pub fn mode(&self) -> WatchMode
    pub fn has_realtime_events(&self) -> bool
}

impl<P> Deref for WatchHandle<P> {
    type Target = Option<P>;
}
```

`WatchMode` indicates how updates will arrive:

```rust
pub enum WatchMode {
    Events,    // UPnP subscription active — real-time
    Polling,   // Subscription failed (firewall?), polling fallback active
    CacheOnly, // No event manager; manual fetch() only
}
```

Check `handle.mode()` to surface firewall warnings to the user.

### 5.5 Fetchable properties

These properties support `fetch()`:

| Handle type | Property | Service |
|---|---|---|
| `VolumeHandle` | `Volume(u8)` | RenderingControl |
| `MuteHandle` | `Mute(bool)` | RenderingControl |
| `BassHandle` | `Bass(i8)` | RenderingControl |
| `TrebleHandle` | `Treble(i8)` | RenderingControl |
| `LoudnessHandle` | `Loudness(bool)` | RenderingControl |
| `PlaybackStateHandle` | `PlaybackState` | AVTransport |
| `PositionHandle` | `Position` | AVTransport |
| `CurrentTrackHandle` | `CurrentTrack` | AVTransport |
| `GroupMembershipHandle` | `GroupMembership` | ZoneGroupTopology |
| `GroupVolumeHandle` | `GroupVolume(u16)` | GroupRenderingControl |
| `GroupMuteHandle` | `GroupMute(bool)` | GroupRenderingControl |

`GroupVolumeChangeableHandle` (`GroupVolumeChangeable(bool)`) is **event-only** — there
is no `fetch()` because Sonos does not expose a `GetGroupVolumeChangeable` UPnP action.

### 5.6 Type aliases

```rust
pub type VolumeHandle              = PropertyHandle<Volume>;
pub type MuteHandle                = PropertyHandle<Mute>;
pub type BassHandle                = PropertyHandle<Bass>;
pub type TrebleHandle              = PropertyHandle<Treble>;
pub type LoudnessHandle            = PropertyHandle<Loudness>;
pub type PlaybackStateHandle       = PropertyHandle<PlaybackState>;
pub type PositionHandle            = PropertyHandle<Position>;
pub type CurrentTrackHandle        = PropertyHandle<CurrentTrack>;
pub type GroupMembershipHandle     = PropertyHandle<GroupMembership>;
pub type GroupVolumeHandle         = GroupPropertyHandle<GroupVolume>;
pub type GroupMuteHandle           = GroupPropertyHandle<GroupMute>;
pub type GroupVolumeChangeableHandle = GroupPropertyHandle<GroupVolumeChangeable>;
```

---

## 6. Discovery API

`sonos_discovery` is an internal crate re-exported through the SDK. You can import it
directly for finer-grained control.

```toml
[dependencies]
sonos-discovery = "0.1"
```

### Free functions

| Function | Signature | Description |
|---|---|---|
| `get()` | `-> Vec<Device>` | Discover with 3-second SSDP timeout |
| `get_with_timeout(timeout)` | `(Duration) -> Vec<Device>` | Custom timeout |
| `get_iter()` | `-> DiscoveryIterator` | Streaming iterator, 3-second timeout |
| `get_iter_with_timeout(timeout)` | `(Duration) -> DiscoveryIterator` | Streaming iterator, custom timeout |

`get_iter()` yields `DeviceEvent::Found(Device)` as each device is discovered, allowing
early termination as soon as you have found the speaker you need.

```rust
use sonos_discovery::{get_iter, DeviceEvent};

for event in get_iter() {
    if let DeviceEvent::Found(device) = event {
        if device.name == "Living Room" {
            println!("Found it at {}", device.ip_address);
            break;
        }
    }
}
```

### `Device` struct

```rust
pub struct Device {
    pub id:           String,   // e.g. "RINCON_000E58A0123456"
    pub name:         String,   // friendly name, e.g. "Living Room"
    pub room_name:    String,   // same as name in most configurations
    pub ip_address:   String,   // IPv4 string, e.g. "192.168.1.100"
    pub port:         u16,      // typically 1400
    pub model_name:   String,   // e.g. "Sonos One"
}
```

---

## 7. Cache and optimistic-update model

### Property cache

The `StateManager` (inside `SonosSystem`) holds a concurrent map of `(SpeakerId, TypeId) -> BoxedProperty`.

- `fetch()` on a handle makes a live SOAP call, stores the result in the cache, and returns it.
- `get()` reads from the cache without touching the network.
- Incoming UPnP events (delivered through the event worker) overwrite cached values and
  emit change events into the `ChangeIterator` channel.

### Optimistic writes

Write methods (`set_volume`, `play`, `pause`, `set_mute`, etc.) update the cache
**after** the SOAP call succeeds. This means:

```rust
speaker.set_volume(75)?;
// speaker.volume.get() now returns Some(Volume(75)) immediately
let v = speaker.volume.get(); // → Some(Volume(75))
```

The optimistic value is correct in the common case. If the device silently rejects the
command or adjusts the value (e.g. clamps to a hardware limit), the cache will remain
stale until the next UPnP event arrives and overwrites it. Use `fetch()` or `watch()` for
authoritative values.

### Group property cache

Group properties (`GroupVolume`, `GroupMute`, `GroupVolumeChangeable`) are stored in a
separate group-keyed map. `group.volume.get()` reads from there; group write methods
update it.

---

## 8. Change-event iteration

`system.iter()` returns a `ChangeIterator`. Events are only emitted for properties that
have been `watch()`ed.

```rust
// 1. Start watching — hold the handles to keep subscriptions alive
let _vol = speaker.volume.watch()?;
let _pb = speaker.playback_state.watch()?;

// 2. Iterate (blocks the calling thread)
for event in system.iter() {
    println!("{} changed on {}", event.property_key, event.speaker_id);
    match event.property_key {
        "volume" => {
            if let Some(vol) = speaker.volume.get() {
                println!("  new volume: {}", vol.0);
            }
        }
        "playback_state" => {
            if let Some(state) = speaker.playback_state.get() {
                println!("  new state: {:?}", state);
            }
        }
        _ => {}
    }
}
```

### `ChangeEvent` struct

```rust
pub struct ChangeEvent {
    pub speaker_id:    SpeakerId,
    pub property_key:  &'static str,  // e.g. "volume", "playback_state"
    pub service:       Service,       // UPnP service that produced the event
    pub timestamp:     std::time::Instant,
}
```

Property keys match the `Property::KEY` constant on each type (see §10).

### `ChangeIterator` methods

`ChangeIterator` itself implements `Iterator<Item = ChangeEvent>` (blocking). Additional
non-blocking methods are available directly on the struct:

| Method | Signature | Behaviour |
|---|---|---|
| `recv()` | `-> Option<ChangeEvent>` | Block until next event; `None` when channel closed |
| `recv_timeout(timeout)` | `(Duration) -> Option<ChangeEvent>` | Block up to `timeout`; `None` on timeout |
| `try_recv()` | `-> Option<ChangeEvent>` | Non-blocking; `None` if queue empty |
| `try_iter()` | `-> TryIter<'_>` | Iterator over all currently queued events (no blocking) |
| `timeout_iter(timeout)` | `(Duration) -> TimeoutIter<'_>` | Iterator that blocks up to `timeout` per item |

```rust
let iter = system.iter();

// Non-blocking poll — drain current queue then move on
for event in iter.try_iter() {
    println!("queued: {:?}", event.property_key);
}

// Wait at most 500 ms for the next event
if let Some(event) = iter.recv_timeout(Duration::from_millis(500)) {
    println!("got: {:?}", event.property_key);
}
```

---

## 9. Error types

### `SdkError`

The top-level error type returned by all SDK methods:

```rust
pub enum SdkError {
    StateError(StateError),          // Internal state manager error
    ApiError(ApiError),              // UPnP/SOAP API error
    EventManager(String),            // Event manager setup failure
    SpeakerNotFound(String),         // No speaker with given name/id
    InvalidIpAddress,                // Could not parse IP from discovery data
    WatcherClosed,                   // Property watcher channel was closed
    FetchFailed(String),             // fetch() could not get a value
    ValidationFailed(ValidationError), // Parameter out of range
    InvalidOperation(String),        // Logical constraint violation (e.g. add coordinator to itself)
    DiscoveryFailed(String),         // No devices found during SSDP discovery
    LockPoisoned,                    // Internal RwLock/Mutex was poisoned
}
```

Pattern matching for CLI error handling:

```rust
match speaker.set_volume(150) {
    Err(SdkError::ValidationFailed(e)) => eprintln!("Invalid input: {}", e),
    Err(SdkError::ApiError(e)) => eprintln!("Network/device error: {}", e),
    Err(e) => eprintln!("Error: {}", e),
    Ok(()) => {}
}
```

### `ApiError` (from `sonos_api`)

Wraps SOAP-level failures:

```rust
pub enum ApiError {
    NetworkError(String),       // TCP/HTTP failure
    ParseError(String),         // Malformed XML response
    SoapFault(u16),             // Device returned SOAP fault (error code)
    InvalidParameter(String),   // Parameter rejected before or by the device
    SubscriptionError(String),  // UPnP event subscription failure
    DeviceError(String),        // Device-specific error (e.g. not coordinator)
}
```

### `StateError` (from `sonos_state`)

Surfaces through `SdkError::StateError`:

```rust
pub enum StateError {
    Init(String),
    Parse(String),
    Api(ApiError),
    AlreadyRunning,
    ShutdownFailed,
    LockError(String),
    SpeakerNotFound(SpeakerId),
    InvalidUrl(String),
    InitializationFailed(String),
    DeviceRegistrationFailed(String),
    SubscriptionFailed(String),
    InvalidIpAddress(String),
    LockPoisoned,
}
```

---

## 10. Key types reference

### `SpeakerId`

Wraps a `String`. The `uuid:` prefix is stripped automatically on construction.

```rust
pub struct SpeakerId(String);

impl SpeakerId {
    pub fn new(id: impl Into<String>) -> Self  // normalises "uuid:RINCON_..." → "RINCON_..."
    pub fn as_str(&self) -> &str
}
// Also implements: Display, PartialEq, Eq, Hash, Clone, From<&str>, From<String>
```

### `GroupId`

Same shape as `SpeakerId` but for groups. Typical format: `"RINCON_xxxxx:n"`.

```rust
pub struct GroupId(String);

impl GroupId {
    pub fn new(id: impl Into<String>) -> Self
    pub fn as_str(&self) -> &str
}
// Also implements: Display, PartialEq, Eq, Hash, Clone, From<&str>, From<String>
```

### `PlaybackState`

```rust
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
    Transitioning,  // briefly set during track changes
}

impl PlaybackState {
    pub fn is_playing(&self) -> bool
    pub fn is_paused(&self) -> bool
    pub fn is_stopped(&self) -> bool
}
```

Property key: `"playback_state"`. Service: `AVTransport`.

### `Volume`

```rust
pub struct Volume(pub u8);   // 0–100 (clamped on construction)

impl Volume {
    pub fn new(value: u8) -> Self   // clamps to 100
    pub fn value(&self) -> u8
}
```

Property key: `"volume"`. Service: `RenderingControl`.

### `Mute`

```rust
pub struct Mute(pub bool);

impl Mute {
    pub fn new(muted: bool) -> Self
    pub fn is_muted(&self) -> bool
}
```

Property key: `"mute"`. Service: `RenderingControl`.

### `Bass` / `Treble`

```rust
pub struct Bass(pub i8);    // −10 to +10 (clamped)
pub struct Treble(pub i8);  // −10 to +10 (clamped)

impl Bass {
    pub fn new(value: i8) -> Self   // clamps to [−10, 10]
    pub fn value(&self) -> i8
}
// Treble is identical
```

Property keys: `"bass"`, `"treble"`. Service: `RenderingControl`.

### `Loudness`

```rust
pub struct Loudness(pub bool);

impl Loudness {
    pub fn new(enabled: bool) -> Self
    pub fn is_enabled(&self) -> bool
}
```

Property key: `"loudness"`. Service: `RenderingControl`.

### `Position`

```rust
pub struct Position {
    pub position_ms: u64,   // current position in milliseconds
    pub duration_ms: u64,   // total track duration in milliseconds
}

impl Position {
    pub fn new(position_ms: u64, duration_ms: u64) -> Self
    pub fn progress(&self) -> f64    // 0.0–1.0; 0.0 when duration is zero
    pub fn parse_time_to_ms(time_str: &str) -> Option<u64>  // "H:MM:SS[.mmm]"
}
```

Property key: `"position"`. Service: `AVTransport`.

### `CurrentTrack`

```rust
pub struct CurrentTrack {
    pub title:         Option<String>,
    pub artist:        Option<String>,
    pub album:         Option<String>,
    pub album_art_uri: Option<String>,
    pub uri:           Option<String>,
}

impl CurrentTrack {
    pub fn is_empty(&self) -> bool          // true when no title, artist, or uri
    pub fn display(&self) -> String         // "Artist - Title" or best available
}
```

Property key: `"current_track"`. Service: `AVTransport`.

### `GroupMembership`

```rust
pub struct GroupMembership {
    pub group_id:       GroupId,
    pub is_coordinator: bool,
}

impl GroupMembership {
    pub fn new(group_id: GroupId, is_coordinator: bool) -> Self
}
```

Property key: `"group_membership"`. Service: `ZoneGroupTopology`.

### `GroupVolume` / `GroupMute` / `GroupVolumeChangeable`

```rust
pub struct GroupVolume(pub u16);         // 0–100 (clamped)
pub struct GroupMute(pub bool);
pub struct GroupVolumeChangeable(pub bool);

impl GroupVolume {
    pub fn new(value: u16) -> Self   // clamps to 100
    pub fn value(&self) -> u16
}
impl GroupMute {
    pub fn new(muted: bool) -> Self
    pub fn is_muted(&self) -> bool
}
impl GroupVolumeChangeable {
    pub fn new(changeable: bool) -> Self
    pub fn is_changeable(&self) -> bool
}
```

Property keys: `"group_volume"`, `"group_mute"`, `"group_volume_changeable"`.
Service: `GroupRenderingControl`.

`GroupVolumeChangeable` is event-only (no `fetch()`).

---

## Complete example: CLI-style polling loop

```rust
use sonos_sdk::{SdkError, SonosSystem};
use std::time::Duration;

fn main() -> Result<(), SdkError> {
    let system = SonosSystem::new()?;

    let speaker = system
        .speaker("Living Room")
        .ok_or_else(|| SdkError::SpeakerNotFound("Living Room".to_string()))?;

    // Fetch current state eagerly
    let volume = speaker.volume.fetch()?;
    let state  = speaker.playback_state.fetch()?;
    let track  = speaker.current_track.fetch()?;

    println!("Volume: {}%", volume.0);
    println!("State:  {:?}", state);
    println!("Track:  {}", track.display());

    // Navigate to group
    if let Some(group) = speaker.group() {
        println!("Group: {} ({} members)", group.id, group.member_count());
    }

    // Start watching for real-time updates — hold handles to keep subscriptions alive
    let vol_handle = speaker.volume.watch()?;
    let _pb = speaker.playback_state.watch()?;
    let _ct = speaker.current_track.watch()?;

    if !vol_handle.has_realtime_events() {
        eprintln!("Warning: running in {:?} mode", vol_handle.mode());
    }

    // React to changes with a 5-second per-event timeout
    let iter = system.iter();
    for event in iter.timeout_iter(Duration::from_secs(5)) {
        println!(
            "[{}] {} changed on {}",
            chrono::Local::now().format("%H:%M:%S"),
            event.property_key,
            event.speaker_id,
        );
    }
    println!("No more events for 5 s — exiting.");
    Ok(())
}
```
