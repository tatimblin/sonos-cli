# sonos-cli

Control Sonos speakers from the command line.

<img width="2660" height="1498" alt="sonos-cli" src="https://github.com/user-attachments/assets/9532c52e-604b-4c4d-8032-dbecc069bb51" />

[![CI](https://github.com/tatimblin/sonos-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/tatimblin/sonos-cli/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/sonos-cli.svg)](https://crates.io/crates/sonos-cli)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Install

### Homebrew (macOS)

```bash
brew install tatimblin/tap/sonos-cli
```

### Cargo

```bash
cargo install sonos-cli
```

### Binary download

Pre-built binaries for macOS, Linux, and Windows are available on the [Releases](https://github.com/tatimblin/sonos-cli/releases) page.

## Quick start

```bash
# List your speakers
sonos speakers

# See what's playing
sonos status

# Set the volume
sonos volume 50 --group "Living Room"

# Skip to the next track
sonos next
```

## Commands

| Command | Description |
|---------|-------------|
| `speakers` | List all speakers with state and volume |
| `groups` | List all groups with playback state |
| `status` | Show current track, state, and volume |
| `play` | Start playback |
| `pause` | Pause playback |
| `stop` | Stop playback |
| `next` | Skip to next track |
| `prev` | Skip to previous track |
| `volume <0-100>` | Set volume level |
| `mute` | Mute playback |
| `unmute` | Unmute playback |
| `seek <H:MM:SS>` | Seek to position in current track |
| `mode <mode>` | Set play mode (normal, repeat, repeat-one, shuffle, shuffle-no-repeat) |
| `bass <-10..10>` | Set bass level (speaker only) |
| `treble <-10..10>` | Set treble level (speaker only) |
| `loudness <on\|off>` | Set loudness compensation (speaker only) |
| `sub <-15..15\|on\|off>` | Set the sub level or turn the sub on/off (home theater) |
| `surround <-15..15\|on\|off> [--tv]` | Set the music (or TV) surround level, or turn the surrounds on/off (home theater) |
| `night <on\|off>` | Set night mode (home theater) |
| `speech <on\|off>` | Set speech enhancement (home theater) |
| `join` | Add a speaker to a group |
| `leave` | Remove a speaker from its group |
| `sleep <duration>` | Set sleep timer (e.g., 30m, 1h) or "cancel" |
| `queue` | Show the playback queue |
| `queue add <uri>` | Add a URI to the queue |
| `queue clear` | Clear the queue |
| `config alias [name] [alias]` | List, set, or clear speaker/group aliases |

Run `sonos` with no arguments in a terminal to open the interactive TUI.

### Targeting

Most commands accept `--speaker` and `--group` flags to target a specific device:

```bash
sonos volume 80 --group "Kitchen"
sonos bass 5 --speaker "Beam"
```

If both are given, `--group` wins. If neither is given, the default group is used.

### Global flags

| Flag | Description |
|------|-------------|
| `--help`, `-h` | Show help |
| `--version`, `-V` | Print version |
| `--quiet`, `-q` | Suppress non-error output |
| `--verbose`, `-v` | More log output on stderr (`-vv` debug, `-vvv` trace) |
| `--no-input` | Disable interactive prompts |
| `--offline` | Accept cached data when no speaker responds |
| `--require-live` | Fail unless a speaker actually responded |

Exit codes: `0` success, `1` runtime error, `2` usage error, `3` output printed but no speaker responded.

## Configuration

Settings live in `config.toml` in the platform config directory:

| Platform | Path |
|----------|------|
| Linux | `~/.config/sonos/config.toml` (or `$XDG_CONFIG_HOME/sonos/`) |
| macOS | `~/Library/Application Support/sonos/config.toml` |
| Windows | `%APPDATA%\sonos\config.toml` |

The TUI Settings tab and `sonos config alias` write this file for you. To edit it by hand:

```toml
# Default group when --speaker/--group is not specified
default_group = "Living Room"

# TUI color theme: "default", "bw", "minimal", or "dance_party"
theme = "default"

# Album art: "image", "halfblock", or "off"
album_art_mode = "image"

# Short names usable with --speaker/--group (full name = alias)
[aliases]
"Living Room" = "lr"
```

If the file cannot be parsed, sonos warns, runs with defaults, and refuses to overwrite it until it is fixed.

Environment variables override the config file:

| Variable | Overrides |
|----------|-----------|
| `SONOS_DEFAULT_GROUP` | `default_group` |
| `SONOS_CONFIG_DIR` | Config directory (absolute path; replaces the platform default above) |

The SDK caches discovered speakers in `cache.json` under the platform cache directory (`~/.cache/sonos/` on Linux, `~/Library/Caches/sonos/` on macOS, `%LOCALAPPDATA%\sonos\` on Windows; override with `SONOS_CACHE_DIR`) for 24 hours. Logs are appended to `sonos.log` under the platform data directory (`~/.local/share/sonos/` on Linux, `~/Library/Application Support/sonos/` on macOS) and rotated to `sonos.log.1` past 1 MiB.

## Related Projects

This CLI is built on the [sonos-sdk](https://github.com/tatimblin/sonos-sdk) which handles device discovery, event streaming, and the local SOAP API.

It replaces my prior [sonos-local-controller](https://github.com/tatimblin/sonos-local-controller) which was a sonos CLI built in 2024 without event streaming in mind.


## License

MIT
