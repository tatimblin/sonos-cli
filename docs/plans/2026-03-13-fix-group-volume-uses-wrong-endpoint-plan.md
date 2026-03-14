---
title: "fix: Group volume/mute sets coordinator only instead of using GroupRenderingControl"
type: fix
status: completed
date: 2026-03-13
---

# fix: Group volume/mute sets coordinator only instead of using GroupRenderingControl

## Overview

When `sonos volume 50 --group "Kitchen"` is run, the CLI resolves the group to its coordinator `Speaker` and calls `Speaker.set_volume()`, which sends `RenderingControl::SetVolume` — affecting only the coordinator. It should call `Group.set_volume()`, which sends `GroupRenderingControl::SetGroupVolume` to adjust all members proportionally.

The same bug affects `mute` and `unmute`.

## Root Cause

`resolve_speaker()` in `src/cli/resolve.rs:17-24` always extracts the coordinator `Speaker` from a group:

```rust
if let Some(group_name) = &global.group {
    let g = system.group(group_name)...;
    return g.coordinator()...;  // ← returns Speaker, not Group
}
```

The volume/mute/unmute handlers in `src/cli/run.rs:69-83` then call `Speaker.set_volume()` / `Speaker.set_mute()` which use `RenderingControl` (single-speaker) instead of `GroupRenderingControl` (whole group).

The SDK already has the correct group methods — `Group.set_volume(u16)` and `Group.set_mute(bool)` in `sonos-sdk/src/group.rs:332-354` — they just aren't being called.

## Affected Commands

| Command | Current (broken) | Correct |
|---------|-----------------|---------|
| `volume` | `Speaker.set_volume(u8)` → `RenderingControl::SetVolume` | `Group.set_volume(u16)` → `GroupRenderingControl::SetGroupVolume` |
| `mute` | `Speaker.set_mute(bool)` → `RenderingControl::SetMute` | `Group.set_mute(bool)` → `GroupRenderingControl::SetGroupMute` |
| `unmute` | `Speaker.set_mute(bool)` → `RenderingControl::SetMute` | `Group.set_mute(bool)` → `GroupRenderingControl::SetGroupMute` |

**Not affected:** `play`, `pause`, `stop`, `next`, `previous`, `seek`, `mode`, `sleep` — these use AVTransport which is inherently group-scoped when sent to the coordinator. They are correct as-is.

## Resolution Matrix

| Scenario | Target type | Volume method | Mute method |
|----------|------------|---------------|-------------|
| `--group "Kitchen"` | Group | `Group.set_volume(level as u16)` | `Group.set_mute(bool)` |
| `--speaker "Kitchen One"` | Speaker | `Speaker.set_volume(level)` | `Speaker.set_mute(bool)` |
| Neither flag (default group) | Group | `Group.set_volume(level as u16)` | `Group.set_mute(bool)` |
| Both flags (`--group` wins) | Group | `Group.set_volume(level as u16)` | `Group.set_mute(bool)` |
| Neither flag, no groups | Speaker (fallback) | `Speaker.set_volume(level)` | `Speaker.set_mute(bool)` |

## Acceptance Criteria

- [x] `sonos volume 50 --group "Kitchen"` calls `Group.set_volume(50)` via GroupRenderingControl
- [x] `sonos mute --group "Kitchen"` calls `Group.set_mute(true)` via GroupRenderingControl
- [x] `sonos unmute --group "Kitchen"` calls `Group.set_mute(false)` via GroupRenderingControl
- [x] `sonos volume 50 --speaker "Kitchen One"` still calls `Speaker.set_volume(50)` (no regression)
- [x] `sonos volume 50` (no flags) resolves default group and calls `Group.set_volume(50)`
- [x] Output shows coordinator/group name when targeting group, speaker name when targeting speaker
- [x] `cargo test` passes with new tests covering group resolution paths
- [x] No changes to `play`, `pause`, `stop`, `next`, `prev`, `seek`, `mode`, `sleep` commands

## Implementation

### 1. Add `resolve_group()` to `src/cli/resolve.rs`

Add a new function alongside the existing `resolve_speaker()`:

```rust
use sonos_sdk::Group;

/// Resolve --group / --speaker flags to a Group handle.
///
/// Priority: --group wins. If --speaker is given (without --group),
/// returns the speaker's containing group. If neither, uses config
/// default or first available group.
pub fn resolve_group(
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
) -> Result<Group, CliError> {
    if let Some(group_name) = &global.group {
        return system
            .group(group_name)
            .ok_or_else(|| CliError::GroupNotFound(group_name.to_string()));
    }

    // Default: config group → first group
    if let Some(default_group) = &config.default_group {
        if let Some(g) = system.group(default_group) {
            return Ok(g);
        }
    }

    system
        .groups()
        .into_iter()
        .next()
        .ok_or_else(|| CliError::GroupNotFound("no groups available".to_string()))
}
```

Export it from `src/cli/mod.rs` alongside the existing exports.

### 2. Update volume/mute/unmute handlers in `src/cli/run.rs`

Replace the inline match arms with dedicated functions that branch on `--speaker` vs group:

```rust
Commands::Volume { level } => cmd_volume(system, config, global, level),
Commands::Mute => cmd_mute(system, config, global, true),
Commands::Unmute => cmd_mute(system, config, global, false),
```

```rust
fn cmd_volume(
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
    level: u8,
) -> Result<String, CliError> {
    // Explicit --speaker → Speaker.set_volume(u8)
    if global.speaker.is_some() && global.group.is_none() {
        let s = resolve_speaker(system, config, global)?;
        s.set_volume(level)?;
        return Ok(format!("Volume set to {} ({})", level, s.name));
    }
    // Otherwise → Group.set_volume(u16)
    let g = resolve_group(system, config, global)?;
    let name = g.coordinator().map(|c| c.name).unwrap_or_default();
    g.set_volume(level as u16)?;
    Ok(format!("Volume set to {} ({})", level, name))
}

fn cmd_mute(
    system: &SonosSystem,
    config: &Config,
    global: &GlobalFlags,
    muted: bool,
) -> Result<String, CliError> {
    let label = if muted { "Muted" } else { "Unmuted" };
    // Explicit --speaker → Speaker.set_mute(bool)
    if global.speaker.is_some() && global.group.is_none() {
        let s = resolve_speaker(system, config, global)?;
        s.set_mute(muted)?;
        return Ok(format!("{} ({})", label, s.name));
    }
    // Otherwise → Group.set_mute(bool)
    let g = resolve_group(system, config, global)?;
    let name = g.coordinator().map(|c| c.name).unwrap_or_default();
    g.set_mute(muted)?;
    Ok(format!("{} ({})", label, name))
}
```

### 3. Add tests for group resolution in `src/cli/resolve.rs`

Test `resolve_group()` paths:
- `--group` resolves to the named group
- Default group from config resolves to that group
- Falls back to first group when no flags and no config
- Group not found returns `CliError::GroupNotFound`

**Note:** Check whether `SonosSystem::with_speakers()` creates group topology. If not, a test helper like `SonosSystem::with_groups()` may need to be added to the SDK.

### 4. Type note

The volume CLI arg is `u8` (0-255), `Speaker.set_volume()` takes `u8`, `Group.set_volume()` takes `u16`. The `u8 as u16` cast is lossless. Both the SDK and Sonos firmware validate the 0-100 range.

## Files to Change

| File | Change |
|------|--------|
| `src/cli/resolve.rs` | Add `resolve_group()` function |
| `src/cli/mod.rs` | Export `resolve_group` |
| `src/cli/run.rs` | Update volume/mute/unmute to branch on speaker vs group |
| `src/cli/resolve.rs` (tests) | Add tests for `resolve_group()` |

## Out of Scope

- Relative volume (`volume-up`/`volume-down`) — no CLI command exists yet
- `snapshot_volume()` — only needed for proportional relative adjustments
- TUI volume controls — TUI not yet implemented
- CLI-level 0-100 validation — separate improvement, not related to this endpoint bug

## Sources

- Sonos GroupRenderingControl API: https://sonos.svrooij.io/services/group-rendering-control#setgroupvolume
- SDK Group methods: `../sonos-sdk/sonos-sdk/src/group.rs:332-354`
- SDK Speaker methods: `../sonos-sdk/sonos-sdk/src/speaker.rs:633-638`
- CLI resolve logic: `src/cli/resolve.rs:11-47`
- CLI volume handler: `src/cli/run.rs:69-83`
- SDK API reference: `docs/references/sonos-sdk.md`
