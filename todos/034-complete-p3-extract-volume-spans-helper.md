---
status: complete
priority: p3
issue_id: "034"
tags: [code-review, simplification, tui, speaker-list, duplication]
dependencies: []
---

# Extract Volume Spans Rendering Helper

## Problem Statement

The volume rendering block is duplicated for `GroupHeader` and `SpeakerRow` — same `if is_selected` / `else` structure, same `volume_bar::render_volume_bar` call, same `format!("{vol}%")` fallback. ~18 lines copy-pasted.

## Findings

- **code-simplicity-reviewer:** Extract a shared `append_volume_spans` helper. ~15 LOC saved, eliminates copy-paste drift.

## Proposed Solutions

### Solution A: Extract helper function
```rust
fn append_volume_spans(spans: &mut Vec<Span>, vol: u16, is_selected: bool, width: u16, theme: &Theme) {
    if is_selected {
        spans.push(Span::raw("  "));
        let vol_line = volume_bar::render_volume_bar(vol, width, theme.volume_filled, theme.volume_empty);
        spans.extend(vol_line.spans);
    } else {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(format!("{vol}%"), theme.muted));
    }
}
```

- **Effort:** Small
- **Risk:** None

## Technical Details

- **Affected files:** `src/tui/widgets/speaker_list.rs` (lines 329-346 and 372-389)

## Acceptance Criteria

- [ ] Single `append_volume_spans` helper used by both GroupHeader and SpeakerRow branches
- [ ] Identical visual output

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-04-05 | Created from PR #45 code review | |

## Resources

- PR: #45
