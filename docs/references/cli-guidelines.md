# CLI Design Guidelines

This document records the CLI design principles `sonos-cli` follows, derived from [clig.dev](https://clig.dev). Each section states the general rule, then how it applies specifically to this project.

Source: [https://clig.dev](https://clig.dev)

---

## Philosophy

Three core values from clig.dev, in priority order:

1. **Human-first** — Design for the person typing at a terminal, not for scripts. Output should be readable without piping through `jq`.
2. **Composable** — Behave predictably when piped or scripted. Clean stdout, errors to stderr, meaningful exit codes.
3. **Consistent** — Every subcommand behaves the same way. If `play` accepts `--group`, so does `pause`. No surprises.

For a media control tool like `sonos`, human-first matters most: users run commands while music is playing and want instant, legible feedback.

---

## Output

### Rules

- **stdout** is for command output only. A script piping `sonos status` should get clean text.
- **stderr** is for errors, warnings, and progress messages. Never mix them into stdout.
- **Be brief on success.** Show what changed; don't echo back the full request.
- **Detect TTY.** Suppress spinners and color when stdout is piped (`!isatty(1)`).
- **Respect `NO_COLOR`.** If the `NO_COLOR` env var is set, emit no ANSI color codes.
- **Show progress for slow operations.** Discovery takes ~3 seconds. Print a spinner to stderr so the user knows it's working.

### Applied to sonos-cli

```
# Good — stdout only on success, stderr for errors
$ sonos volume 50
Volume set to 50 (Living Room)

$ sonos play --group "Nonexistent"
error: group "Nonexistent" not found. Check that your speakers are on the same network, then retry.
[exits 1]

# Good — no color/spinner when piped
$ sonos status | cat
Living Room  ▶ Playing  Bohemian Rhapsody — Queen  vol:80
```

---

## Arguments and Flags

### Rules

- **Prefer `--flags` over positional arguments.** Flags are self-documenting and order-independent.
- **Full flag names are required.** Single-letter shortcuts only for the most common flags (`-h` for `--help`, `-V` for `--version`, `-q` for `--quiet`, `-v` for `--verbose`, `-s` / `-g` for `--speaker` / `--group`).
- **Use standard flag names where they exist.**

| Flag | Meaning |
|------|---------|
| `--help` / `-h` | Show help |
| `--version` / `-V` | Print version |
| `--quiet` / `-q` | Suppress non-error output |
| `--verbose` / `-v` | Raise log verbosity (`-v` info, `-vv` debug, `-vvv` trace) |

- **Validate early.** Reject invalid argument combinations before doing any network work.
- **Never read secrets via flags.** Flags appear in shell history and `ps` output.
- **Confirm before destructive actions** (e.g., `queue clear`).

### Applied to sonos-cli

**Targeting convention** — every command that operates on a speaker or group accepts:

```
--speaker NAME    target a specific speaker by friendly name
--group NAME      target a group by name
```

If both are given, `--group` wins. If neither is given, the default group is used (configured in [`config.toml`](#config-file-configtoml); falls back to the coordinator of the largest group, then any speaker).

```bash
$ sonos volume 50                        # default group
$ sonos volume 50 --group "Kitchen"      # specific group
$ sonos bass 3 --speaker "Kitchen One"   # speaker-level EQ (group flag not applicable)
```

**Speaker-only flags** — EQ and home theater commands (`bass`, `treble`, `loudness`, `sub`, `surround`, `night`, `speech`) operate on individual speakers, not groups. Passing `--group` to them is a usage error (`bass applies to a single speaker; use --speaker instead of --group`, exit 2) rather than being silently ignored.

---

## Subcommands

### Rules

- **Flat structure.** All commands are direct subcommands of `sonos`. Never `sonos playback play` or `sonos eq bass`.
- **Imperative verbs.** `play`, not `playing`; `seek`, not `seeking`.
- **No abbreviations.** `sonos pl` is not valid. The full command `sonos play` is required.
- **Consistent behavior across subcommands.** If one command accepts `--quiet`, all commands that produce output must accept `--quiet`.

### Applied to sonos-cli

Full command reference — see `docs/goals.md#cli-command-reference`.

Quick examples:
```bash
sonos play
sonos pause
sonos volume 70 --group "Living Room"
sonos mute --speaker "Bedroom One"
sonos groups
```

---

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Runtime error (speaker not found, SDK call failed, network error) |
| `2` | Usage error (unknown flag, invalid argument value, missing required arg) |
| `3` | Output printed, but no speaker responded — nothing in it was verified |

clap returns exit code 2 automatically for usage errors, including out-of-range values (volume, bass, treble ranges are declared on the clap args). Command handlers return `CliError` (a `thiserror` enum in `src/errors.rs`); `main` maps `CliError::Validation` to 2 and every other variant to 1 via `CliError::exit_code()`.

### Unverified output (`3`)

Discovery is cached on disk for 24h, so `sonos speakers`, `sonos groups` and
`sonos status` can print a full, plausible listing without a single packet
reaching a speaker — every per-field fetch fails, each row quietly collapses to
the cached name, and the command exits 0. Code `3` exists so that case is not
indistinguishable from success.

It is a distinct code rather than `1` because the command did run and its stdout
*is* printed: a script that only tests `-ne 0` now catches it, while one that
reads the output still gets it. Two global flags move it:

| Flag | Effect when nothing responded |
|------|-------------------------------|
| *(none)* | stdout printed, warning banner on stderr, exit `3` |
| `--offline` | stdout printed, one-line note on stderr, exit `0` |
| `--require-live` | **stdout suppressed**, warning banner on stderr, exit `1` |

The two flags conflict; passing both is a usage error. A command that reached
even one speaker is unaffected by either.

stdout itself is never annotated. There is no `--json` mode, so the plain
format is the de-facto machine contract — a per-row marker would break
`sonos speakers | awk '{print $1}'` and `sonos speakers | fzf`. The verdict is
carried by the exit code and stderr only.

---

## Configuration

### Precedence (highest → lowest)

1. **CLI flags** — always win
2. **Environment variables** — override config file
3. **Config file** — `config.toml` (see below)
4. **Built-in defaults** — fallback values in code

### Environment Variables

| Variable | Purpose |
|----------|---------|
| `SONOS_DEFAULT_GROUP` | Override default group without editing config |
| `SONOS_CONFIG_DIR` | Override config directory (absolute path only; relative or empty values are ignored) |
| `SONOS_CACHE_DIR` | Override the SDK's discovery-cache directory (absolute path only) |
| `RUST_LOG` | Log filter when no `-v` is given (default `warn`) |

`NO_COLOR` is not implemented yet.

### Config File: `config.toml`

Located at `dirs::config_dir()/sonos/config.toml` unless `SONOS_CONFIG_DIR` is set:

| Platform | Path |
|----------|------|
| Linux | `~/.config/sonos/config.toml` (respects `$XDG_CONFIG_HOME`) |
| macOS | `~/Library/Application Support/sonos/config.toml` |
| Windows | `%APPDATA%\sonos\config.toml` |

```toml
# Default group when --group / --speaker not given
default_group = "Living Room"

# TUI color theme: "default" | "bw" | "minimal" | "dance_party"
theme = "default"

# Album art: "image" | "halfblock" | "off"
album_art_mode = "image"

# Aliases: full speaker/group name = short alias
[aliases]
"Living Room" = "lr"
```

A missing file means defaults. A file that exists but cannot be read or parsed prints one warning, the run continues with defaults, and every save (`sonos config alias`, the TUI Settings tab) is refused so the user's file is never overwritten.

### Discovery Cache: `cache.json`

Owned by the SDK, separate from the config file. Stored at `dirs::cache_dir()/sonos/cache.json` — `~/.cache/sonos/` on Linux (respects `$XDG_CACHE_HOME`), `~/Library/Caches/sonos/` on macOS, `%LOCALAPPDATA%\sonos\` on Windows — or under `SONOS_CACHE_DIR`. Auto-populated by SSDP discovery with a 24h TTL. Contains discovered speaker IPs, names, and IDs. Never edit manually.

### Log File: `sonos.log`

Every run (CLI and TUI) appends to `dirs::data_local_dir()/sonos/sonos.log` — `~/.local/share/sonos/` on Linux, `~/Library/Application Support/sonos/` on macOS, `%LOCALAPPDATA%\sonos\` on Windows. At startup a log larger than 1 MiB is moved to `sonos.log.1` (replacing any previous one). The CLI also logs to stderr; the TUI does not.

---

## Interactivity

- **Only prompt in an interactive TTY.** Commands must never hang waiting for input when piped.
- **`--no-input` disables all prompts.** Scripts should always pass this flag.
- **TUI requires a TTY.** If `sonos` is invoked with no args but stdout is not a TTY, print an error and exit 1:
  ```
  error: no command specified and stdout is not a terminal
  ```

---

## Error Messages

### Format

```
error: <human-readable description>
<optional follow-up action on the next line>
```

### Examples

```
error: speaker "Office Move" not found
hint: this could mean:
  - no Sonos speakers are on this network
  - ...

error: SDK error: ...
Check network connectivity and speaker power.

error: invalid value '150' for '<LEVEL>': 150 is not in 0..=100
```

The follow-up line comes from `CliError::recovery_hint()`: not-found and discovery errors print the platform-specific hint from `diagnostics::discovery_hint()`; other SDK errors print a connectivity hint; `Config` and `Validation` errors have none.

### Rules

- One error per line to stderr.
- Lowercase, no trailing period on the first line.
- Always suggest a next action when one exists.
- The `Debug` form of every error is logged at debug level; run with `-vv` to see it on stderr and in `sonos.log`.

---

## Signals and Process Lifecycle

- **Ctrl-C exits immediately.** The TUI must restore the terminal on SIGINT (crossterm's `LeaveAlternateScreen` + `DisableRawMode`).
- **No cleanup on Ctrl-C for one-off commands.** They complete atomically or fail; there is no partial state to clean up.
- **Exit quickly.** Long-running discovery shows a progress indicator; it does not silently block.

---

## Naming

The binary is named `sonos` — lowercase, no dashes or underscores. Easy to type, memorable, matches the product it controls.

Subcommand naming rules:
- Lowercase only
- Single word where possible (`play`, `pause`, `volume`, `discover`)
- Hyphen for multi-word subcommands if ever needed (`play-uri`, `queue-add`)
