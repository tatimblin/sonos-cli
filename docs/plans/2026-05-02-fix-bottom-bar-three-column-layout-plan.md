---
title: "fix: bottom bar three-column Spotify-style layout"
type: fix
status: active
date: 2026-05-02
---

# Fix: Bottom Bar Three-Column Spotify-Style Layout

## Overview

The wide bottom bar currently puts the progress bar on row 1 alongside artist text and the volume bar. This makes the progress bar span almost the entire width, which looks wrong. The layout should use a three-column "flexbox" approach matching Spotify:

```
Left column          │  Center column         │  Right column
─────────────────────┼────────────────────────┼──────────────────
Title                │  controls              │  Group Name
Artist               │  time ━━━━━━━━━━ time  │  vol_bar vol%
Album                │                        │
```

This also requires adding `album` (from `CurrentTrack.album`) to `BottomBarData`.

## Problem Statement

Current wide layout:
```
Row 0: [art] Title                    controls                  Group Name
Row 1: [art] Artist   0:54 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ 3:22  ■■■··· 23%
Row 2: [art]
```

The progress bar gets all the space between artist and volume — far too wide. Controls float between title and group name on row 0, not centered over the progress bar.

## Proposed Solution

Restructure `render_wide` into three fixed-proportion columns:

```
Left (~30%)          │  Center (~40%)         │  Right (~30%)
─────────────────────┼────────────────────────┼──────────────────
Row 0: Title         │  controls              │  Group Name
Row 1: Artist        │  time ━━━━━━━━━━ time  │  vol_bar vol%
Row 2: Album         │                        │
```

### Column allocation

After the art area (6 cols + 1 gap), the remaining width is divided:
- **Left column:** ~30% — track metadata (title, artist, album), left-aligned
- **Center column:** ~40% — controls (row 0) and progress bar (row 1), centered
- **Right column:** ~30% — group name (row 0, right-aligned), volume bar (row 1, right-aligned)

Exact percentages can flex, but the key constraint is that the progress bar is confined to the center column rather than stretching from after-artist to before-volume.

### Changes required

#### 1. Add `track_album` to `BottomBarData` (`types.rs`)

```rust
pub struct BottomBarData {
    pub group_name: String,
    pub track_title: Option<String>,
    pub track_artist: Option<String>,
    pub track_album: Option<String>,        // NEW
    pub album_art_protocol: Option<StatefulProtocol>,
    pub playback_state: Option<PlaybackState>,
    pub progress: f64,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: u16,
    pub is_wide: bool,
}
```

#### 2. Populate `track_album` in `assemble_bottom_bar` (`screens/speakers.rs`)

```rust
let track_album = current_track.as_ref().and_then(|t| t.album.clone());
```

Add to the `BottomBarData { ... }` struct literal.

#### 3. Rewrite `render_wide` in `bottom_bar.rs`

Replace the current row-based layout with a three-column layout:

```rust
fn render_wide(frame: &mut Frame, area: Rect, data: &mut BottomBarData, theme: &Theme) {
    let art_width: u16 = 6;
    let art_area = Rect::new(area.x, area.y, art_width, 3);
    
    album_art::render_album_art(
        frame, art_area, data.album_art_protocol.as_mut(),
        theme.bottom_bar_border, theme.muted, theme.glyphs.music_note,
    );

    let content_x = area.x + art_width + 1;
    let content_w = area.width.saturating_sub(art_width + 1);
    if content_w == 0 { return; }

    // Three-column split
    let right_w: u16 = 22;  // group name + volume bar
    let left_w = content_w.saturating_sub(right_w) * 30 / 70;  // ~30% of non-right
    let center_w = content_w.saturating_sub(left_w + right_w);

    let left_x = content_x;
    let center_x = left_x + left_w;
    let right_x = center_x + center_w;

    // LEFT COLUMN: Title (row 0), Artist (row 1), Album (row 2)
    // ... render title, artist, album left-aligned in left_w ...

    // CENTER COLUMN: Controls (row 0), Progress bar (row 1)
    // ... render controls centered, progress bar with time labels ...

    // RIGHT COLUMN: Group name (row 0), Volume bar (row 1)
    // ... render group name right-aligned, volume bar ...
}
```

The center column confines the progress bar to ~40% of width, matching Spotify's proportional feel.

## Acceptance Criteria

- [ ] Wide layout uses three distinct columns: metadata | controls+progress | group+volume
- [ ] Progress bar is confined to the center column, not stretching across the full width
- [ ] Album name appears on row 2 of the left column (muted style, like artist)
- [ ] Controls are centered in the center column on row 0
- [ ] Group name is right-aligned in the right column on row 0
- [ ] Volume bar is right-aligned in the right column on row 1
- [ ] Narrow layout (60-99 cols) is not affected
- [ ] Minimal layout (< 60 cols) is not affected

## Technical Considerations

- **Only `render_wide` changes.** Narrow and minimal layouts are unaffected.
- **`BottomBarData` gains one field.** `track_album: Option<String>` sourced from `CurrentTrack.album` in the SDK. This is a non-breaking addition.
- **Column width tuning.** The exact percentages may need visual tuning. The plan provides a starting point; implementation should verify visually.
- **Right column width.** Volume bar is currently 20 chars + 2 padding = 22. This is a reasonable fixed width for the right column.

## Files to Modify

1. `src/tui/types.rs` — Add `track_album` field to `BottomBarData`
2. `src/tui/screens/speakers.rs` — Populate `track_album` from `CurrentTrack.album`
3. `src/tui/widgets/bottom_bar.rs` — Rewrite `render_wide` with three-column layout
