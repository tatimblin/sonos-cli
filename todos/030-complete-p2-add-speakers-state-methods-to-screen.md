---
status: complete
priority: p2
issue_id: "030"
tags: [code-review, architecture, tui, speaker-list, simplification]
dependencies: []
---

# Add speakers_state() Methods to Screen Enum

## Problem Statement

Five helper functions in `speaker_list.rs` (`get_speakers_state`, `get_selected_index`, `set_selected_index`, `get_pick_up_state`, `set_pick_up_state`) each independently match on `Screen::Home` and `Screen::GroupView` to access `speakers_state`. This is the same two-arm match repeated five times. Every new screen with a speaker list requires updating all five.

## Findings

- **architecture-strategist:** Classic expression problem. Recommends `Screen::speakers_state()` / `speakers_state_mut()` methods to collapse five match arms into two.
- **code-simplicity-reviewer:** ~30 LOC saved. Centralizes Screen-variant knowledge in one place.

## Proposed Solutions

### Solution A: Add methods to Screen (Recommended)
```rust
impl Screen {
    pub fn speakers_state(&self) -> Option<&SpeakerListScreenState> {
        match self {
            Screen::Home { speakers_state, .. } => Some(speakers_state),
            Screen::GroupView { speakers_state, .. } => Some(speakers_state),
            _ => None,
        }
    }
    pub fn speakers_state_mut(&mut self) -> Option<&mut SpeakerListScreenState> {
        match self {
            Screen::Home { speakers_state, .. } => Some(speakers_state),
            Screen::GroupView { speakers_state, .. } => Some(speakers_state),
            _ => None,
        }
    }
}
```

Then inline the five wrappers in `speaker_list.rs`.

- **Pros:** ~30 LOC saved, one place to update for new screens, knowledge lives in `Screen` where it belongs
- **Cons:** None
- **Effort:** Small
- **Risk:** None

## Technical Details

- **Affected files:** `src/tui/app.rs` (add methods), `src/tui/widgets/speaker_list.rs` (remove 5 functions), `src/tui/event.rs` (simplify Esc handling)

## Acceptance Criteria

- [ ] `Screen::speakers_state()` and `speakers_state_mut()` added
- [ ] Five wrapper functions in speaker_list.rs removed or inlined
- [ ] Event.rs Esc handling uses the new methods

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-04-05 | Created from PR #45 code review | Two agents flagged accessor boilerplate |

## Resources

- PR: #45
