---
title: "feat: platform-specific discovery failure diagnostics"
type: feat
status: completed
date: 2026-03-22
origin: docs/brainstorms/2026-03-22-discovery-failure-diagnostics-brainstorm.md
---

# feat: Platform-specific discovery failure diagnostics

## Overview

When SSDP discovery finds zero speakers, show a platform-specific hint explaining the most likely cause (macOS Local Network permissions, Windows firewall, Linux firewall rules). On macOS and Windows, prompt to open the relevant settings pane. This replaces the current generic "Check that your speakers are on the same network" message that gives users no actionable path forward.

## Problem Statement

A new user installs `sonos` on a machine where the terminal lacks network permissions. They run a command. SSDP multicast is silently dropped by the OS. The CLI says "No speakers found" or "discovery failed" with no indication the problem is permissions. (See brainstorm: `docs/brainstorms/2026-03-22-discovery-failure-diagnostics-brainstorm.md`)

## Proposed Solution

Add a `src/diagnostics.rs` module that provides platform-specific hint text and a settings-opener. Wire it into the two places where "no speakers" surfaces:

1. **`main.rs:36-43`** — `SonosSystem::new()` returns `Err(SdkError::DiscoveryFailed(...))`
2. **`errors.rs:28-36`** — `recovery_hint()` for `SpeakerNotFound`/`GroupNotFound` (covers `sonos play` etc. via resolve functions)

### Target output

**macOS:**
```
error: no speakers discovered
hint: on macOS, your terminal needs Local Network access to discover speakers.
      Check System Settings > Privacy & Security > Local Network.

Open settings? [Y/n]
```

**Windows:**
```
error: no speakers discovered
hint: on Windows, ensure Network Discovery is enabled and your firewall
      allows UDP traffic on port 1900 (SSDP).

Open settings? [Y/n]
```

**Linux:**
```
error: no speakers discovered
hint: ensure your firewall allows UDP multicast on port 1900 (SSDP).
      For ufw: sudo ufw allow proto udp from any to 239.255.255.250 port 1900
```

## Acceptance Criteria

- [x] When `SonosSystem::new()` returns `DiscoveryFailed`, show platform-specific hint on stderr
- [x] On macOS/Windows: prompt `Open settings? [Y/n]` (default Yes) when stdin is a TTY and `--no-input` is not set
- [x] On Linux: text-only hint, no prompt
- [x] Prompt suppressed when `--no-input` is set or stdin is not a TTY
- [x] If `open`/`start` fails to spawn, print fallback: "Navigate to [path] manually."
- [x] Non-`DiscoveryFailed` errors from `SonosSystem::new()` retain existing generic hint
- [x] `recovery_hint()` on `SpeakerNotFound`/`GroupNotFound` returns platform-specific text (covers `sonos play`, `sonos volume`, etc. on empty systems)
- [x] All diagnostic output goes to stderr; stdout unchanged
- [x] `--verbose` still prints `debug: {e:?}` before the hint

## Technical Considerations

### New module: `src/diagnostics.rs`

```
src/
  diagnostics.rs   ← NEW: platform detection, hint text, settings opener
```

Responsibilities:
- `discovery_hint() -> &str` — returns platform-specific hint text using `cfg!(target_os = "...")`
- `offer_open_settings()` — prompts `[Y/n]`, spawns `open`/`start` on confirm, handles spawn failure gracefully
- Platform detection at compile time via `cfg!` (no runtime overhead, all three targets already in CI)

### Injection points

**Point 1 — `src/main.rs:36-43`:**
Pattern-match on `SdkError::DiscoveryFailed` specifically. Other `SdkError` variants (e.g., `LockPoisoned`, `InvalidIpAddress`) keep the existing generic hint.

```rust
Err(e) => {
    if cli.global.verbose {
        eprintln!("debug: {e:?}");
    }
    eprintln!("error: {e}");
    if matches!(&e, sonos_sdk::SdkError::DiscoveryFailed(_)) {
        eprintln!("{}", diagnostics::discovery_hint());
        diagnostics::offer_open_settings(&cli.global);
    } else {
        eprintln!("Check that your speakers are on the same network, then retry.");
    }
    return ExitCode::from(1);
}
```

**Point 2 — `src/errors.rs:28-36`:**
Make `recovery_hint()` return the platform-specific hint for `SpeakerNotFound`/`GroupNotFound`. This covers `sonos play`, `sonos volume`, etc. when resolve functions hit an empty system.

```rust
Self::SpeakerNotFound(_) | Self::GroupNotFound(_) => {
    Some(diagnostics::discovery_hint())
}
```

Note: `offer_open_settings()` is NOT called from `recovery_hint()` — the prompt is only offered at the `main.rs` `DiscoveryFailed` path where we know for certain it's a discovery problem, not a typo or offline speaker.

### Platform settings URLs

| Platform | Command | URL |
|----------|---------|-----|
| macOS | `open` | `x-apple.systempreferences:com.apple.preference.security?Privacy_LocalNetwork` |
| Windows | `start` | `ms-settings:privacy-localnetwork` (Win 11) or `ms-settings:network` (fallback) |
| Linux | N/A | Text-only |

### Edge cases

- **Prompt default is Yes** (`[Y/n]`) — brainstorm decision. Pressing Enter opens settings. This differs from the existing `queue clear` prompt (`[y/N]`) because opening settings is low-risk and the user explicitly needs help.
- **`--quiet` does not suppress stderr** — consistent with current behavior. `--quiet` suppresses stdout output only.
- **`open`/`start` spawn failure** — print "Navigate to [settings path] manually." and continue. Don't crash.
- **Unknown OS** (FreeBSD, etc.) — falls through to a generic "check your firewall allows UDP multicast on port 1900" hint, same as Linux.
- **WSL** — compiles as `target_os = "linux"`, gets Linux hints. Acceptable — detecting WSL adds complexity for a rare edge case.
- **TUI mode** — not affected. TUI path (`main.rs:24-27`) doesn't call `SonosSystem::new()` yet. When it does, TUI should show diagnostics in-screen, not via prompt. That's a future concern.

## Sources

- **Origin brainstorm:** [docs/brainstorms/2026-03-22-discovery-failure-diagnostics-brainstorm.md](docs/brainstorms/2026-03-22-discovery-failure-diagnostics-brainstorm.md) — key decisions: CLI-only (not SDK), platform-specific hints, prompt before auto-open, stderr only
- **Primary injection point:** `src/main.rs:34-44`
- **Recovery hints:** `src/errors.rs:26-36`
- **Resolve functions (covered by hint change):** `src/cli/resolve.rs:43-46, 70-75`
- **Existing TTY + no-input pattern:** `src/cli/run.rs:341`
- **Global flags:** `src/cli/mod.rs:28-45`
- **CI targets (all 3 platforms):** `Cargo.toml:73-78`
