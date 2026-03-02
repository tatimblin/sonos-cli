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
error: group "Nonexistent" not found. Run 'sonos discover' to refresh.
[exits 1]

# Good — no color/spinner when piped
$ sonos status | cat
Living Room  ▶ Playing  Bohemian Rhapsody — Queen  vol:80
```

---

## Arguments and Flags

### Rules

- **Prefer `--flags` over positional arguments.** Flags are self-documenting and order-independent.
- **Full flag names are required.** Single-letter shortcuts only for the most common flags (`-h` for `--help`, `-v` for `--version`, `-q` for `--quiet`).
- **Use standard flag names where they exist.**

| Flag | Meaning |
|------|---------|
| `--help` / `-h` | Show help |
| `--version` / `-v` | Print version |
| `--quiet` / `-q` | Suppress non-error output |
| `--verbose` | Show debug / raw SDK errors |

- **Validate early.** Reject invalid argument combinations before doing any network work.
- **Never read secrets via flags.** Flags appear in shell history and `ps` output.
- **Confirm before destructive actions** (e.g., `queue clear`).

### Applied to sonos-cli

**Targeting convention** — every command that operates on a speaker or group accepts:

```
--speaker NAME    target a specific speaker by friendly name
--group NAME      target a group by name
```

If both are given, `--group` wins. If neither is given, the default group is used (configured in `~/.config/sonos/config.toml`; falls back to the first discovered group).

```bash
$ sonos volume 50                        # default group
$ sonos volume 50 --group "Kitchen"      # specific group
$ sonos bass 3 --speaker "Kitchen One"   # speaker-level EQ (group flag not applicable)
```

**Speaker-only flags** — EQ commands (`bass`, `treble`, `loudness`) operate on individual speakers, not groups. `--group` is not accepted for these.

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
sonos discover
sonos groups
```

---

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Runtime error (speaker not found, SDK call failed, network error) |
| `2` | Usage error (unknown flag, invalid argument value, missing required arg) |

clap returns exit code 2 automatically for usage errors. Our error-handling code returns 1 for all runtime failures via `anyhow::Error` propagation from `main`.

---

## Configuration

### Precedence (highest → lowest)

1. **CLI flags** — always win
2. **Environment variables** — override config file
3. **Config file** — `~/.config/sonos/config.toml`
4. **Built-in defaults** — fallback values in code

### Environment Variables

| Variable | Purpose |
|----------|---------|
| `SONOS_DEFAULT_GROUP` | Override default group without editing config |
| `SONOS_CONFIG_DIR` | Override config directory (default: `~/.config/sonos/`) |
| `NO_COLOR` | Disable all ANSI color output |

### Config File: `~/.config/sonos/config.toml`

```toml
# Default group when --group / --speaker not given
default_group = "Living Room"

# How long the discovery cache is valid before auto-rediscovery (hours)
cache_ttl_hours = 24

# TUI color theme: "dark" | "light" | "neon" | "sonos"
theme = "dark"
```

### Discovery Cache: `~/.config/sonos/cache.json`

Separate from the config file. Written by `sonos discover` and auto-refreshed on TTL expiry. Contains discovered speaker IPs, names, and IDs. Never edit manually.

---

## Interactivity

- **Only prompt in an interactive TTY.** Commands must never hang waiting for input when piped.
- **`--no-input` disables all prompts.** Scripts should always pass this flag.
- **TUI requires a TTY.** If `sonos` is invoked with no args but stdout is not a TTY, print an error and exit 1:
  ```
  error: TUI mode requires an interactive terminal.
  Use 'sonos --help' to see available commands.
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
error: speaker "Office Move" not found.
Run 'sonos discover' to refresh the speaker list.

error: failed to connect to Living Room (192.168.1.42): connection refused
Check that your Sonos speakers are online.

error: volume must be between 0 and 100 (got: 150)
```

### Rules

- One error per line to stderr.
- Lowercase, no trailing period on the first line.
- Always suggest a next action when one exists.
- Use `--verbose` to expose raw `SdkError` details and stack traces for debugging.

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
