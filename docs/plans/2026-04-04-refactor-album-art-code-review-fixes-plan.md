---
title: "refactor: Address album-art-now-playing code review fixes"
type: refactor
status: completed
date: 2026-04-04
origin: Code review of feat/album-art-now-playing branch
---

# refactor: Address album-art-now-playing code review fixes

## Overview

Nine issues identified during code review of the `feat/album-art-now-playing` branch. One bug (speaker count display), one behavioral fix (inverted debounce), and seven internal quality improvements (deduplication, dead code, data structures, type safety, RefCell removal).

## Proposed Solution

Fix all nine issues in a single branch with one commit per issue, ordered to minimize merge conflicts. Prioritize the bug fix and behavioral fix first, then deduplication, then structural improvements.

## Implementation

### Phase 1: Bug fix + behavioral fix (correctness)

#### Fix 5: Speaker count display bug (`now_playing.rs`)

**Problem:** Line 292 displays `speaker_text.chars().count()` (character count of the model name string) instead of `members.len()` (actual speaker count).

**Fix:** Add `speaker_count: usize` parameter to `render_metadata_column`. Pass `members.len()` from the caller. Replace `speaker_text.chars().count().min(99)` with `speaker_count`.

```rust
// now_playing.rs — render_metadata_column signature
fn render_metadata_column(
    frame: &mut Frame,
    area: Rect,
    ctx: &RenderContext,
    title: &str,
    artist: &str,
    album: &str,
    volume: u16,
    speaker_count: usize,   // NEW
    speaker_text: &str,
)

// Line ~292 fix:
Span::styled(format!("{}  ", speaker_count), theme.muted),
```

#### Fix 8: Invert debounce logic (`home_groups.rs`)

**Problem:** Selected group debounces (delays), non-selected groups fetch immediately. Should be the opposite — eagerly fetch for the group the user is looking at, delay background groups until navigation settles.

**Fix:** Swap the branches:

```rust
let should_fetch = if selected {
    true               // always fetch for the focused group
} else {
    selection_stable   // wait for navigation to settle before fetching background groups
};
```

### Phase 2: Dead code + data structure (trivial)

#### Fix 4: Remove dead `is_pending` method (`image_loader.rs`)

Remove the `#[allow(dead_code)] pub fn is_pending()` method entirely. YAGNI — if needed later, it's a one-liner to re-add.

#### Fix 3: Vec to VecDeque for insertion_order (`image_loader.rs`)

Replace `insertion_order: Vec<String>` with `VecDeque<String>`. Change:
- `self.insertion_order.push(...)` → `self.insertion_order.push_back(...)`
- `self.insertion_order.first().cloned()` + `self.insertion_order.remove(0)` → `self.insertion_order.pop_front()`

### Phase 3: Deduplication (code quality)

#### Fix 1: Extract progress bar rendering to `progress_bar.rs`

**Problem:** `PROG_FILLED`, `PROG_EMPTY`, `PROG_CHAR_BYTES` constants and the fill/empty slicing logic are copy-pasted between `group_card.rs` and `now_playing.rs`.

**API design:** The two call sites have different layouts:
- `group_card.rs`: `icon + bar + time` on one line, `"●"` cursor (or no cursor when stopped)
- `now_playing.rs`: `time + bar + time` centered, `"╺"` cursor always shown

Extract a function that returns just the bar portion as `Vec<Span>`, parameterized by cursor:

```rust
// src/tui/widgets/progress_bar.rs

/// Render a progress bar as spans. Caller composes into their layout.
pub fn render_bar_spans(
    progress: f64,       // 0.0..=1.0
    width: usize,        // bar width in chars
    cursor: Option<&str>, // e.g. Some("●"), Some("╺"), or None
    filled_style: Style,
    cursor_style: Style,
    empty_style: Style,
) -> Vec<Span<'static>>
```

Move `PROG_FILLED`, `PROG_EMPTY`, `PROG_CHAR_BYTES` into `progress_bar.rs` as module-level constants. Remove duplicates from `group_card.rs` and `now_playing.rs`.

**Adjacent:** Also export `VOL_FILLED`, `VOL_EMPTY`, and their byte-size constants from `volume_bar.rs` (they are duplicated in `group_card.rs` lines 15-16, 21-22). Consolidate in the same pass since we're already modifying `group_card.rs`.

#### Fix 2: Extract shared album art protocol state (`widgets/album_art.rs`)

**Problem:** `AlbumArtHookState` (now_playing.rs) and `MiniArtState` (home_groups.rs) are identical structs with identical URI-change-detection + protocol-creation logic.

**Fix:** Create `ArtProtocolState` in `widgets/album_art.rs` with a helper method:

```rust
// src/tui/widgets/album_art.rs

/// Hook-friendly state for album art protocol lifecycle.
pub struct ArtProtocolState {
    pub uri: Option<String>,
    pub protocol: Option<StatefulProtocol>,
}

impl Default for ArtProtocolState {
    fn default() -> Self {
        Self { uri: None, protocol: None }
    }
}

impl ArtProtocolState {
    /// Update protocol when URI changes. Creates protocol from cached image.
    /// Returns true if protocol is ready for rendering.
    pub fn ensure_protocol(
        &mut self,
        art_uri: &Option<String>,
        image_loader: &ImageLoader,
        picker: &RefCell<Option<Picker>>,
    ) -> bool {
        let uri_changed = self.uri.as_deref() != art_uri.as_deref();
        if uri_changed {
            self.uri = art_uri.clone();
            self.protocol = None;
        }
        if self.protocol.is_none() {
            if let Some(ref uri) = art_uri {
                if let Some(img) = image_loader.get(uri) {
                    if let Some(ref mut p) = *picker.borrow_mut() {
                        self.protocol = Some(p.new_resize_protocol(img.clone()));
                    }
                }
            }
        }
        self.protocol.is_some()
    }
}
```

Remove `AlbumArtHookState` from `now_playing.rs` and `MiniArtState` from `home_groups.rs`. Both sites use `ctx.hooks.use_state::<ArtProtocolState>(...)` and call `ensure_protocol()`.

#### Fix 7: Collapse duplicate `render_group_card` call sites (`home_groups.rs`)

After Fix 2, the `if show_mini_art` block simplifies. Compute `art_protocol` before the call:

```rust
let art_protocol = if show_mini_art {
    let art_state = ctx.hooks.use_state::<ArtProtocolState>(&art_key, ArtProtocolState::default);
    art_state.ensure_protocol(&art_uri, &ctx.app.image_loader, &ctx.app.picker);
    art_state.protocol.as_mut()
} else {
    None
};
group_card::render_group_card(frame, *col_area, &data, &ctx.app.theme, art_protocol);
```

One call site instead of two.

### Phase 4: Type safety + structural (lower priority)

#### Fix 6: Enum for `album_art_mode` (`config.rs`)

**Problem:** `album_art_mode: String` only checks `"off"`. Typos silently fall through to auto.

**Fix:** Introduce enum with serde fallback:

```rust
// src/config.rs
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlbumArtMode {
    Auto,
    Off,
    /// Catch-all for unrecognized values — behaves like Auto.
    #[serde(other)]
    Other,
}

impl Default for AlbumArtMode {
    fn default() -> Self { Self::Auto }
}
```

Update `tui/mod.rs` check from `config.album_art_mode == "off"` to `config.album_art_mode == AlbumArtMode::Off`.

**Backward compatibility:** `#[serde(other)]` on the `Other` variant means `"halfblock"` or any unrecognized string deserializes to `Other` (behaves as `Auto`) rather than erroring.

#### Fix 9: Remove RefCell from Picker (defer)

**Assessment:** This is the most invasive change. Moving `picker` off `App` requires:
- Storing it alongside `App` in the event loop
- Adding `picker: &'a mut Option<Picker>` to `RenderContext`
- Changing `App::new()` to return picker separately
- Updating all four access sites across `now_playing.rs` and `home_groups.rs`

The split-borrow pattern (`&App` + `&mut Option<Picker>`) is clean but touches the event loop scaffolding. Since `RefCell` with `MAX_CACHE_SIZE = 20` has zero practical risk of runtime panics (borrows are always scoped within a single render call), **defer this to a follow-up** unless the other fixes leave a natural opening.

**Decision:** Skip for now. The `RefCell` is contained, well-documented, and not a correctness risk. Revisit when `RenderContext` needs restructuring for other reasons.

## Commit Order

Ordered to minimize conflicts (issues sharing files are grouped):

| # | Issue | Files Modified | Risk |
|---|-------|---------------|------|
| 1 | Fix 5: Speaker count bug | `now_playing.rs` | Low — parameter addition |
| 2 | Fix 8: Invert debounce | `home_groups.rs` | Low — swap two branches |
| 3 | Fix 4: Remove `is_pending` | `image_loader.rs` | None — dead code removal |
| 4 | Fix 3: Vec → VecDeque | `image_loader.rs` | None — internal data structure |
| 5 | Fix 6: album_art_mode enum | `config.rs`, `tui/mod.rs` | Low — serde fallback handles compat |
| 6 | Fix 1: Progress bar extraction | `progress_bar.rs`, `group_card.rs`, `now_playing.rs` | Medium — API design |
| 7 | Fix 2: Art protocol extraction | `album_art.rs`, `now_playing.rs`, `home_groups.rs` | Medium — shared state struct |
| 8 | Fix 7: Collapse call sites | `home_groups.rs` | Low — depends on Fix 2 |

Fix 9 (RefCell removal) deferred.

## Acceptance Criteria

- [x] Speaker count in Now Playing shows actual member count, not string length
- [x] Selected group's album art loads immediately; background groups wait for navigation to settle
- [x] `is_pending` dead code removed from `image_loader.rs`
- [x] `insertion_order` uses `VecDeque`
- [x] `album_art_mode` is an enum with serde fallback for unknown values
- [x] Progress bar constants and slicing logic live only in `progress_bar.rs`
- [x] Album art protocol state struct is defined once in `widgets/album_art.rs`
- [x] Single `render_group_card` call site in `home_groups.rs`
- [x] `cargo clippy` passes with no new warnings
- [x] `cargo test` passes
- [x] Volume bar constants consolidated into `volume_bar.rs` (adjacent cleanup)

## Sources

- Code review of `feat/album-art-now-playing` branch (this conversation)
- Existing plan: `docs/plans/2026-04-01-feat-album-art-now-playing-layout-plan.md`
- Hooks architecture: `docs/plans/2026-03-29-refactor-tui-hooks-architecture-plan.md`
