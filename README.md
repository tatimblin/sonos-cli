# sonos-cli

Control Sonos speakers from the command line.

[![CI](https://github.com/tatimblin/sonos-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/tatimblin/sonos-cli/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/sonos-cli.svg)](https://crates.io/crates/sonos-cli)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Install

### Homebrew (macOS)

```bash
brew install tatimblin/tap/sonos
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
| `join` | Add a speaker to a group |
| `leave` | Remove a speaker from its group |
| `sleep <duration>` | Set sleep timer (e.g., 30m, 1h) or "cancel" |
| `queue` | Show the playback queue |
| `queue add <uri>` | Add a URI to the queue |
| `queue clear` | Clear the queue |

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
| `--version` | Print version |
| `--quiet`, `-q` | Suppress non-error output |
| `--verbose` | Show debug output |
| `--no-input` | Disable interactive prompts |

## Configuration

Create `~/.config/sonos/config.toml`:

```toml
# Default group when --speaker/--group is not specified
default_group = "Living Room"

# TUI color theme: "dark" or "light"
theme = "dark"
```

Environment variables override the config file:

| Variable | Overrides |
|----------|-----------|
| `SONOS_DEFAULT_GROUP` | `default_group` |
| `SONOS_CONFIG_DIR` | Config file location (default: `~/.config/sonos/`) |

## License

MIT
