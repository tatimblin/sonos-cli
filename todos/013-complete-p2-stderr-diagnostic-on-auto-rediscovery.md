---
status: complete
priority: p2
issue_id: "013"
tags: [code-review, agent-native, observability, sdk]
dependencies: []
---

# Emit Stderr Diagnostic When Auto-Rediscovery Fires

## Problem Statement

When `get_speaker_by_name()` triggers auto-rediscovery, the 3-second SSDP scan happens silently. Users, scripts, and agents see a mysterious pause with no indication of what's happening. This makes debugging difficult and blocks agents from distinguishing "searching" from "hung."

## Findings

- **Agent-native-reviewer (P2):** Hidden 3s blocks are invisible to agents. Agents need observable signals to understand system behavior.
- **Performance-oracle:** The hidden latency should at minimum be documented; stderr diagnostic makes it observable.

## Proposed Solutions

### Option 1: Add `tracing::info!` in SDK + stderr in CLI (Recommended)

**Approach:**
- SDK: Add `tracing::info!("speaker '{}' not found, running auto-rediscovery...", name)` in `try_rediscover()`
- CLI: Since CLI uses `eprintln!`, this surfaces automatically if tracing subscriber is configured. Alternatively, add `eprintln!("Searching for speakers...")` in the executor when resolution triggers rediscovery.

**Effort:** Small

**Risk:** None

## Acceptance Criteria

- [ ] Auto-rediscovery emits a diagnostic message visible on stderr
- [ ] Message includes the speaker name being searched for
- [ ] Message appears before the 3s scan, not after

## Work Log

### 2026-03-09 - Discovery during code review (round 2)

**By:** Claude Code
