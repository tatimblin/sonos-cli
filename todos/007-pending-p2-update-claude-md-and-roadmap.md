---
status: complete
priority: p2
issue_id: "007"
tags: [code-review, documentation]
dependencies: []
---

# Update CLAUDE.md and Roadmap After Implementation

## Problem Statement

After this plan executes, several documented references become inaccurate:
- CLAUDE.md line 38 lists `cache.rs` in module structure (will be deleted)
- CLAUDE.md line 51 references `~/.config/sonos/cache.json` (changes to `~/.cache/sonos/cache.json`)
- CLAUDE.md Rule 3 says "`sonos discover` refreshes manually" (command is removed)
- Roadmap line 232 has `sonos discover` as unchecked Milestone 2 task (should be marked superseded)
- Companion plan's caching sections need to be marked as superseded

## Findings

- **Architecture-strategist (Medium):** CLAUDE.md will be inaccurate after plan executes. Add to acceptance criteria.
- **Architecture-strategist (Medium):** Roadmap needs superseded items marked, not just checked off.

## Proposed Solutions

### Option 1: Add Post-Implementation Docs Update to Acceptance Criteria

**Approach:** Add acceptance criteria items for updating CLAUDE.md, roadmap, and companion plan.

**Effort:** Small

**Risk:** None

## Acceptance Criteria

- [ ] CLAUDE.md updated: module structure, cache path, discover command references
- [ ] Roadmap: `sonos discover` item marked as superseded
- [ ] Companion plan's caching/discovery sections marked as superseded

## Work Log

### 2026-03-09 - Discovery during code review

**By:** Claude Code
