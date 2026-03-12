---
status: rejected
priority: p1
issue_id: "012"
tags: [code-review, agent-native, cli, architecture]
dependencies: []
---

# Restore Explicit Discovery Command (`sonos refresh`)

## Problem Statement

Removing `sonos discover` eliminates the only way for scripts and agents to explicitly trigger device discovery. Auto-rediscovery is hidden inside `get_speaker_by_name()` — there's no programmatic way to say "refresh the device list now." This violates agent-native parity: any action the SDK performs implicitly should also be available explicitly.

## Findings

- **Agent-native-reviewer (CRITICAL):** Loss of explicit discovery capability. Agents and scripts have no way to trigger cache refresh or SSDP scan.
- **Architecture-strategist:** The removal of `sonos discover` leaves a gap in the CLI's operational toolkit. Users who add new speakers need a way to force refresh.

## Proposed Solutions

### Option 1: Add `sonos refresh` Command (Recommended)

**Approach:** Add a `sonos refresh` command that:
1. Runs SSDP discovery with a longer timeout (5s)
2. Updates the cache
3. Prints the discovered speakers to stdout
4. Returns exit code 0 on success, 1 if no devices found

This replaces the old `sonos discover` with a clearer verb ("refresh" implies updating existing state).

**Effort:** Small

**Risk:** Low

### Option 2: Add `--refresh` Flag to Existing Commands

**Approach:** Add `--refresh` flag that forces rediscovery before executing any command. E.g., `sonos play --refresh --speaker "Kitchen"`.

**Effort:** Small

**Risk:** Low (but less discoverable for scripts)

## Acceptance Criteria

- [ ] CLI has an explicit way to trigger device discovery/cache refresh
- [ ] Command prints discovered speakers to stdout (machine-parseable)
- [ ] Exit code 0 on success, 1 on no devices found
- [ ] `cli-commands.md` updated with new command

## Work Log

### 2026-03-09 - Discovery during code review (round 2)

**By:** Claude Code
