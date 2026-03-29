---
status: complete
priority: p1
issue_id: "022"
tags: [code-review, architecture, tui, watch-lifecycle, progress-bar]
dependencies: []
---

# Progress States Never Repopulated After Topology Change

## Problem Statement

After a `group_membership` event, the plan clears `progress_states` and marks dirty. But `handle_change_event()` only *updates existing* entries via `get_mut()` — it never *inserts* new entries. In the old code, `setup_watches()` (lines 167-178 of `event.rs`) eagerly initialized `ProgressState` entries for each group. Without this initialization, progress bar interpolation breaks permanently after any topology change until the app restarts.

## Findings

- **julik-frontend-races-reviewer (BUG):** "After group_membership clears progress_states, no code path re-creates them. Progress bar interpolation breaks permanently."
- **architecture-strategist (Medium):** "There is a bootstrapping problem: handle_change_event only populates entries when a change event arrives. The initial seed is missing."
- **code-simplicity-reviewer:** "Progress state initialization is hand-waved in the plan — needs explicit specification."

## Proposed Solutions

### Option 1: Use entry().or_insert_with() in handle_change_event (Recommended)

Change `handle_change_event` to create entries on first encounter instead of only updating existing ones:

```rust
"position" => {
    if let Some(speaker) = app.system.speaker_by_id(&event.speaker_id) {
        if let Some(pos) = speaker.position.get() {
            if let Some(group) = speaker.group() {
                let ps = app.progress_states
                    .entry(group.id.clone())
                    .or_insert_with(|| ProgressState::new(0, 0, false));
                ps.last_position_ms = pos.position_ms;
                ps.last_duration_ms = pos.duration_ms;
                ps.wall_clock_at_last_update = Instant::now();
            }
        }
    }
}
```

- **Pros:** Simplest fix, self-healing, keeps state mutation in event handler (not render path)
- **Cons:** Slight delay — progress bar interpolation starts only after first position event
- **Effort:** Small
- **Risk:** Low

### Option 2: Initialize progress states in widget render

Have the groups widget seed `ProgressState` when it first sees position data from `app.watch()`.

- **Pros:** Co-located with the code that reads progress states
- **Cons:** Mixes state mutation into render path, requires `&mut App` or RefCell for `progress_states`
- **Effort:** Medium
- **Risk:** Medium — further expands interior mutability

## Acceptance Criteria

- [ ] After a `group_membership` event, progress bar interpolation resumes for new groups
- [ ] Progress bar does not "jump" — interpolation works from the first position event
- [ ] No empty `progress_states` HashMap after topology changes

## Sources

- Current initialization: `src/tui/event.rs:167-178` (setup_watches)
- Current update: `src/tui/event.rs:214,227` (get_mut only)
