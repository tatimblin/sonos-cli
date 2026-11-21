# CLI Command Reference

This document is the authoritative reference for every `sonos` CLI command. For design rules and conventions, see [`cli-guidelines.md`](cli-guidelines.md). For a brief command table, see [`../goals.md`](../goals.md#cli-command-reference).

---

## Global Flags

These flags are accepted by every command.

| Flag | Short | Description |
|------|-------|-------------|
| `--help` | `-h` | Show command help and exit (exit 0) |
| `--version` | `-V` | Print `sonos <version>` and exit (exit 0) |
| `--quiet` | `-q` | Suppress all non-error stdout output |
| `--verbose` | `-v` | Raise log verbosity on stderr: `-v` info, `-vv` debug, `-vvv` trace (overrides `RUST_LOG`) |
| `--no-input` | — | Disable all interactive prompts; required in scripts |
| `--offline` | — | Accept cached data when no speaker responds (exit 0, no warning banner) |
| `--require-live` | — | Fail with no stdout (banner on stderr, exit 1) unless a speaker was actually reached |
| `--speaker NAME` | `-s` | Target a speaker (see [Targeting](#targeting)) |
| `--group NAME` | `-g` | Target a group (see [Targeting](#targeting)) |

`--offline` and `--require-live` conflict. Without either, a read command (`speakers`, `groups`, `status`) whose every request failed still prints its cached output, adds a warning banner on stderr, and exits 3.

Every run appends a log to `sonos.log` in the platform data directory (`~/.local/share/sonos/` on Linux, `~/Library/Application Support/sonos/` on macOS, `%LOCALAPPDATA%\sonos\` on Windows). At startup a log over 1 MiB is rotated to `sonos.log.1`, replacing the previous one.

Argument values are validated by clap: an out-of-range or unknown value prints clap's message (`error: invalid value '...' for '<ARG>': ...`) and exits 2. Not-found errors are followed by a platform-specific `hint:` block listing likely discovery causes (abbreviated `hint: ...` below).

---

## Targeting

Most commands operate on a group or speaker. Every command that targets a device accepts:

```
--speaker NAME    target a specific speaker by friendly name
--group NAME      target a group by name
```

If both flags are given, `--group` wins. If neither is given, the default group is used — set via `default_group` in the [config file](cli-guidelines.md#config-file-configtoml) or the `SONOS_DEFAULT_GROUP` environment variable. If no default is configured (or it no longer exists), the coordinator of the largest group is used, then any speaker.

Names given to `--speaker` / `--group` may be aliases (see [config alias](#config-alias)).

EQ commands (`bass`, `treble`, `loudness`) operate on individual speakers only. Passing `--group` to them is a usage error (exit 2):

```
error: validation error: bass applies to a single speaker; use --speaker instead of --group
```

---

## Discovery & System

### speakers

List all speakers in the cache with their current state and volume.

```
sonos speakers
```

**Flags:** none

**Example:**

```bash
$ sonos speakers
Bedroom One       ▶ Playing   vol:65   (Living Room)
Beam              ▶ Playing   vol:80   (Living Room)
Kitchen One       ⏸ Paused    vol:50   (Kitchen)
```

**Errors:**

With no speakers discovered, the `hint:` block goes to stderr and `No speakers found` to stdout (exit 0). If every speaker request fails, the cached rows still print, followed by the unverified-output banner on stderr (exit 3; see `--offline` / `--require-live`).

---

### groups

List all groups in the cache with their current playback state and volume.

```
sonos groups
```

**Flags:** none

**Example:**

```bash
$ sonos groups
Living Room   ▶ Playing   Bohemian Rhapsody — Queen   vol:80
Kitchen       ⏸ Paused    Hotel California — Eagles    vol:50
```

**Errors:**

With no groups discovered, the `hint:` block goes to stderr and `No groups found` to stdout (exit 0). As with `speakers`, output that no speaker confirmed gets the unverified-output banner (exit 3).

---

### status

Show the current track, playback state, and volume for a group or speaker.

```
sonos status [--speaker NAME | --group NAME]
```

**Flags:**

| Flag | Type | Required | Description |
|------|------|----------|-------------|
| `--group NAME` | string | no | Target group by name |
| `--speaker NAME` | string | no | Target speaker by name |

**Example:**

```bash
$ sonos status
Living Room  ▶ Playing  Bohemian Rhapsody — Queen  2:31/5:55  vol:80

$ sonos status --group "Kitchen"
Kitchen  ⏸ Paused  Hotel California — Eagles  4:12/6:30  vol:50
```

**Errors:**

```
error: group "Office" not found
hint: ...
```

---

## Playback

All playback commands accept `[--speaker NAME | --group NAME]`. If neither is given, the default group is targeted.

### play

Resume playback on the targeted group or speaker.

```
sonos play [--speaker NAME | --group NAME]
```

**Flags:**

| Flag | Type | Required | Description |
|------|------|----------|-------------|
| `--group NAME` | string | no | Target group by name |
| `--speaker NAME` | string | no | Target speaker by name |

**Example:**

```bash
$ sonos play
Playing (Living Room)

$ sonos play --group "Kitchen"
Playing (Kitchen)
```

**Errors:**

```
error: group "Kitchen" not found
hint: ...
```

---

### pause

Pause playback on the targeted group or speaker.

```
sonos pause [--speaker NAME | --group NAME]
```

**Flags:**

| Flag | Type | Required | Description |
|------|------|----------|-------------|
| `--group NAME` | string | no | Target group by name |
| `--speaker NAME` | string | no | Target speaker by name |

**Example:**

```bash
$ sonos pause
Paused (Living Room)

$ sonos pause --group "Kitchen"
Paused (Kitchen)
```

---

### stop

Stop playback on the targeted group or speaker.

```
sonos stop [--speaker NAME | --group NAME]
```

Unlike `pause`, `stop` clears the transport state. Resume with `sonos play`.

**Flags:**

| Flag | Type | Required | Description |
|------|------|----------|-------------|
| `--group NAME` | string | no | Target group by name |
| `--speaker NAME` | string | no | Target speaker by name |

**Example:**

```bash
$ sonos stop
Stopped (Living Room)
```

---

### next

Skip to the next track in the queue.

```
sonos next [--speaker NAME | --group NAME]
```

**Flags:**

| Flag | Type | Required | Description |
|------|------|----------|-------------|
| `--group NAME` | string | no | Target group by name |
| `--speaker NAME` | string | no | Target speaker by name |

**Example:**

```bash
$ sonos next
Next track (Living Room)
```

**Errors:**

```
error: no next track in queue
```

---

### prev

Go back to the previous track or restart the current track.

```
sonos prev [--speaker NAME | --group NAME]
```

**Flags:**

| Flag | Type | Required | Description |
|------|------|----------|-------------|
| `--group NAME` | string | no | Target group by name |
| `--speaker NAME` | string | no | Target speaker by name |

**Example:**

```bash
$ sonos prev
Previous track (Living Room)
```

---

### seek

Seek to a specific position in the current track.

```
sonos seek <HH:MM:SS> [--speaker NAME | --group NAME]
```

The position argument uses `HH:MM:SS` format. Single-digit hours are accepted (e.g., `0:02:30` for 2 minutes 30 seconds).

**Arguments:**

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<HH:MM:SS>` | string | yes | Target position in `H:MM:SS` or `HH:MM:SS` format |

**Flags:**

| Flag | Type | Required | Description |
|------|------|----------|-------------|
| `--group NAME` | string | no | Target group by name |
| `--speaker NAME` | string | no | Target speaker by name |

**Example:**

```bash
$ sonos seek 1:30:00
Seeked to 1:30:00 (Living Room)

$ sonos seek 0:02:30 --group "Kitchen"
Seeked to 0:02:30 (Kitchen)
```

**Errors:**

```
error: validation error: invalid position "0:05:70" — seconds must be 0–59
[exits 2]
```

---

### mode

Set the play mode for the targeted group or speaker.

```
sonos mode <normal|repeat|repeat-one|shuffle|shuffle-no-repeat> [--speaker NAME | --group NAME]
```

**Arguments:**

| Argument | Values | Description |
|----------|--------|-------------|
| `<MODE>` | `normal` | Play queue once through |
| | `repeat` | Repeat the entire queue |
| | `repeat-one` | Repeat the current track |
| | `shuffle` | Shuffle and repeat |
| | `shuffle-no-repeat` | Shuffle once, no repeat |

**Flags:**

| Flag | Type | Required | Description |
|------|------|----------|-------------|
| `--group NAME` | string | no | Target group by name |
| `--speaker NAME` | string | no | Target speaker by name |

**Example:**

```bash
$ sonos mode shuffle
Mode set to shuffle (Living Room)

$ sonos mode normal --group "Kitchen"
Mode set to normal (Kitchen)
```

**Errors:**

```
error: invalid value 'loop' for '<MODE>'
  [possible values: normal, repeat, repeat-one, shuffle, shuffle-no-repeat]
[exits 2]
```

---

## Volume & EQ

### volume

Set the volume for a group or speaker.

```
sonos volume <0-100> [--speaker NAME | --group NAME]
```

**Arguments:**

| Argument | Type | Required | Range | Description |
|----------|------|----------|-------|-------------|
| `<LEVEL>` | integer | yes | 0–100 | Target volume level |

**Flags:**

| Flag | Type | Required | Description |
|------|------|----------|-------------|
| `--group NAME` | string | no | Target group by name |
| `--speaker NAME` | string | no | Target speaker by name |

**Example:**

```bash
$ sonos volume 50
Volume set to 50 (Living Room)

$ sonos volume 75 --group "Kitchen"
Volume set to 75 (Kitchen)

$ sonos volume 30 --speaker "Bedroom One"
Volume set to 30 (Bedroom One)
```

**Errors:**

```
error: invalid value '150' for '<LEVEL>': 150 is not in 0..=100
[exits 2]
```

---

### mute

Mute a group or speaker.

```
sonos mute [--speaker NAME | --group NAME]
```

**Flags:**

| Flag | Type | Required | Description |
|------|------|----------|-------------|
| `--group NAME` | string | no | Target group by name |
| `--speaker NAME` | string | no | Target speaker by name |

**Example:**

```bash
$ sonos mute
Muted (Living Room)

$ sonos mute --speaker "Kitchen One"
Muted (Kitchen One)
```

---

### unmute

Unmute a group or speaker.

```
sonos unmute [--speaker NAME | --group NAME]
```

**Flags:**

| Flag | Type | Required | Description |
|------|------|----------|-------------|
| `--group NAME` | string | no | Target group by name |
| `--speaker NAME` | string | no | Target speaker by name |

**Example:**

```bash
$ sonos unmute
Unmuted (Living Room)
```

---

### bass

Set the bass level for a specific speaker. This command operates on individual speakers only — group targeting is not supported.

```
sonos bass <-10..10> --speaker NAME
```

**Arguments:**

| Argument | Type | Required | Range | Description |
|----------|------|----------|-------|-------------|
| `<LEVEL>` | integer | yes | -10 to 10 | Target bass level |

**Flags:**

| Flag | Type | Required | Description |
|------|------|----------|-------------|
| `--speaker NAME` | string | yes | Target speaker by friendly name |

**Example:**

```bash
$ sonos bass 3 --speaker "Beam"
Bass set to 3 (Beam)

$ sonos bass -2 --speaker "Bedroom One"
Bass set to -2 (Bedroom One)
```

Negative levels are parsed as values, not flags, so no `--` separator is needed (same for `treble`).

**Errors:**

```
error: invalid value '15' for '<LEVEL>': 15 is not in -10..=10
[exits 2]

error: validation error: --speaker is required for bass
[exits 2]

error: validation error: bass applies to a single speaker; use --speaker instead of --group
[exits 2]
```

---

### treble

Set the treble level for a specific speaker. This command operates on individual speakers only — group targeting is not supported.

```
sonos treble <-10..10> --speaker NAME
```

**Arguments:**

| Argument | Type | Required | Range | Description |
|----------|------|----------|-------|-------------|
| `<LEVEL>` | integer | yes | -10 to 10 | Target treble level |

**Flags:**

| Flag | Type | Required | Description |
|------|------|----------|-------------|
| `--speaker NAME` | string | yes | Target speaker by friendly name |

**Example:**

```bash
$ sonos treble 5 --speaker "Kitchen One"
Treble set to 5 (Kitchen One)
```

**Errors:**

```
error: invalid value '-15' for '<LEVEL>': -15 is not in -10..=10
[exits 2]

error: validation error: --speaker is required for treble
[exits 2]

error: validation error: treble applies to a single speaker; use --speaker instead of --group
[exits 2]
```

---

### loudness

Enable or disable loudness compensation for a specific speaker. This command operates on individual speakers only — group targeting is not supported.

Loudness compensation boosts bass and treble at low volumes to counteract the way human hearing perceives sound.

```
sonos loudness <on|off> --speaker NAME
```

**Arguments:**

| Argument | Values | Description |
|----------|--------|-------------|
| `<STATE>` | `on` | Enable loudness compensation |
| | `off` | Disable loudness compensation |

**Flags:**

| Flag | Type | Required | Description |
|------|------|----------|-------------|
| `--speaker NAME` | string | yes | Target speaker by friendly name |

**Example:**

```bash
$ sonos loudness on --speaker "Beam"
Loudness enabled (Beam)

$ sonos loudness off --speaker "Kitchen One"
Loudness disabled (Kitchen One)
```

**Errors:**

```
error: invalid value 'yes' for '<STATE>'
  [possible values: on, off]
[exits 2]

error: validation error: --speaker is required for loudness
[exits 2]

error: validation error: loudness applies to a single speaker; use --speaker instead of --group
[exits 2]
```

---

## Queue

`queue` is a subcommand with optional sub-subcommands. Running `sonos queue` with no sub-subcommand shows the current queue.

### queue

Show the number of tracks in the current queue for a group or speaker. (Listing individual tracks is not implemented yet.)

```
sonos queue [--speaker NAME | --group NAME]
```

**Flags:**

| Flag | Type | Required | Description |
|------|------|----------|-------------|
| `--group NAME` | string | no | Target group by name |
| `--speaker NAME` | string | no | Target speaker by name |

**Example:**

```bash
$ sonos queue
Living Room — 3 tracks

$ sonos queue --group "Kitchen"
queue is empty (Kitchen)
```

---

### queue add

Add a URI to the end of the queue.

```
sonos queue add <URI> [--speaker NAME | --group NAME]
```

URIs use the Sonos URI format. For example: `x-sonosapi-radio:...` for a radio stream or `x-file-cifs:...` for a local file.

**Arguments:**

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<URI>` | string | yes | Sonos URI of the track or stream to add |

**Flags:**

| Flag | Type | Required | Description |
|------|------|----------|-------------|
| `--group NAME` | string | no | Target group by name |
| `--speaker NAME` | string | no | Target speaker by name |

**Example:**

```bash
$ sonos queue add "x-file-cifs://nas/music/track.mp3" --group "Kitchen"
Added to queue (Kitchen)
```

**Errors:**

```
error: invalid URI format
```

---

### queue clear

Clear the entire queue for a group or speaker. This action cannot be undone.

```
sonos queue clear [--speaker NAME | --group NAME]
```

**Flags:**

| Flag | Type | Required | Description |
|------|------|----------|-------------|
| `--group NAME` | string | no | Target group by name |
| `--speaker NAME` | string | no | Target speaker by name |

When running in an interactive terminal, this command prompts for confirmation. Pass `--no-input` to skip the prompt in scripts.

**Example:**

```bash
$ sonos queue clear
Clear queue for Living Room? [y/N] y
Queue cleared (Living Room)

$ sonos queue clear --no-input
Queue cleared (Living Room)
```

---

## Grouping

### join

Add a speaker to an existing group.

```
sonos join --speaker NAME --group NAME
```

Both `--speaker` and `--group` are required. After joining, the speaker plays in sync with the rest of the group.

**Flags:**

| Flag | Type | Required | Description |
|------|------|----------|-------------|
| `--speaker NAME` | string | yes | Speaker to add to the group |
| `--group NAME` | string | yes | Group to join |

**Example:**

```bash
$ sonos join --speaker "Bedroom One" --group "Living Room"
Bedroom One joined Living Room
```

**Errors:**

```
error: speaker "Bedroom One" not found
hint: ...

error: group "Living Room" not found
hint: ...
```

---

### leave

Remove a speaker from its current group. The speaker becomes a standalone single-speaker group.

```
sonos leave --speaker NAME
```

**Flags:**

| Flag | Type | Required | Description |
|------|------|----------|-------------|
| `--speaker NAME` | string | yes | Speaker to remove from its group |

**Example:**

```bash
$ sonos leave --speaker "Bedroom One"
Bedroom One left Living Room
```

**Errors:**

```
error: speaker "Bedroom One" not found
hint: ...

error: validation error: --speaker is required for leave
[exits 2]
```

---

## Sleep Timer

`sleep` is a subcommand with an optional `cancel` sub-subcommand. Running `sonos sleep <DURATION>` sets a timer; `sonos sleep cancel` cancels it.

### sleep

Set a sleep timer. Playback will stop automatically after the duration expires.

```
sonos sleep <DURATION> [--speaker NAME | --group NAME]
```

Duration format uses a number followed by a unit: `m` for minutes, `h` for hours. Examples: `30m`, `1h`, `90m`.

**Arguments:**

| Argument | Type | Required | Examples | Description |
|----------|------|----------|---------|-------------|
| `<DURATION>` | string | yes | `30m`, `1h`, `90m` | Time until playback stops |

**Flags:**

| Flag | Type | Required | Description |
|------|------|----------|-------------|
| `--group NAME` | string | no | Target group by name |
| `--speaker NAME` | string | no | Target speaker by name |

**Example:**

```bash
$ sonos sleep 30m
Sleep timer set for 30 minutes (Living Room)

$ sonos sleep 1h --group "Kitchen"
Sleep timer set for 1 hour (Kitchen)
```

**Errors:**

```
error: validation error: invalid duration "30" — use a unit suffix: 30m or 1h
[exits 2]
```

---

### sleep cancel

Cancel the active sleep timer.

```
sonos sleep cancel [--speaker NAME | --group NAME]
```

**Flags:**

| Flag | Type | Required | Description |
|------|------|----------|-------------|
| `--group NAME` | string | no | Target group by name |
| `--speaker NAME` | string | no | Target speaker by name |

**Example:**

```bash
$ sonos sleep cancel
Sleep timer cancelled (Living Room)
```

**Errors:**

```
error: no active sleep timer (Living Room)
```

---

## Configuration

`config` commands never contact a speaker, so they run without discovery. With no action, `sonos config` prints its help and exits 2.

### config alias

List, set, or clear short aliases for speaker and group names. Aliases are accepted anywhere `--speaker` / `--group` take a name and match case-insensitively.

```
sonos config alias                 # list aliases
sonos config alias <NAME> <ALIAS>  # set (or replace) the alias for NAME
sonos config alias <NAME>          # clear the alias for NAME
```

**Example:**

```bash
$ sonos config alias "Living Room" lr
Alias set: lr → Living Room

$ sonos config alias
lr → Living Room

$ sonos config alias "Living Room"
Alias cleared: lr → Living Room
```

With no aliases, the list prints `No aliases configured`.

**Errors:**

```
error: validation error: alias must not contain whitespace
[exits 2]

error: validation error: alias "lr" is already used by "Living Room"
[exits 2]

error: validation error: no alias configured for "Kitchen"
[exits 2]

error: configuration error: refusing to overwrite <path>/config.toml, which could not be loaded; fix or remove it, then retry
[exits 1]
```

The last error appears when the config file exists but cannot be read or parsed: every command then warns and runs with default settings, and nothing overwrites the broken file.
