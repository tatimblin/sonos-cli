---
status: complete
priority: p2
issue_id: "015"
tags: [code-review, documentation, agent-native]
dependencies: []
---

# Update cli-commands.md to Remove Stale `sonos discover` References

## Problem Statement

`docs/references/cli-commands.md` still documents the `sonos discover` command which was removed. Error messages throughout the CLI also reference `sonos discover` in recovery hints. These stale references confuse users and agents.

## Findings

- **Agent-native-reviewer (P2):** Documentation still references removed command. Agents parsing docs will attempt to run a nonexistent command.
- **Pattern-recognition-specialist:** Error hint strings in `errors.rs` may still say "Run 'sonos discover' to refresh."

## Proposed Solutions

### Option 1: Audit and Update All References (Recommended)

**Approach:**
1. Remove `sonos discover` from `cli-commands.md`
2. Grep for "sonos discover" across all docs and source files
3. Update error recovery hints — remove references to `sonos discover`, point users to `sonos speakers` to list available devices instead

**Effort:** Small

**Risk:** None

## Acceptance Criteria

- [ ] `cli-commands.md` no longer references `sonos discover`
- [ ] Error messages in `errors.rs` reference correct recovery command
- [ ] `goals.md` and `roadmap.md` updated if they reference `sonos discover`

## Work Log

### 2026-03-09 - Discovery during code review (round 2)

**By:** Claude Code
