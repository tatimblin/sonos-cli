---
title: "feat: Spotify-style bottom player bar"
type: feat
status: active
date: 2026-04-30
origin: docs/brainstorms/2026-04-30-tui-v1-simplification-brainstorm.md
parallel-group: tui-v1-simplification
---

# Spotify-Style Bottom Player Bar

## Overview

Add a persistent Spotify-style player bar to the bottom of the TUI. The bar follows the cursor — it always shows playback info for the group that contains the currently focused row. This is the primary playback control surface, replacing the cut Now Playing full-screen tab.

The bar renders in two responsive layouts (wide vs narrow) and includes album art thumbnail, track metadata with ticker scrolling, playback controls, progress bar, and group volume.

(see brainstorm: `docs/brainstorms/2026-04-30-tui-v1-simplification-brainstorm.md` — "Spotify-Style Bottom Player Bar" section)

## Problem Statement / Motivation

The current TUI has no playback controls — the mini-player and Now Playing screen were cut during simplification. The dead-code widgets (`progress_bar.rs`, `album_art.rs`) and infrastructure (`image_loader.rs`, `ProgressState` in `hooks.rs`) are all retained and ready to use. This plan activates that code and composes it into the persistent bottom bar.

## Proposed Solution

A new `bottom_bar` widget rendered by `ui.rs` between the content area and the key legend. It receives pre-assembled data from the screen layer (which group is focused, that group's playback state). Two layout branches based on terminal width.

## Technical Approach

### Existing Code to Activate

| Component | File | Status |
|-----------|------|--------|
| Album art rendering | `widgets/album_art.rs` | Dead code — remove `#[allow(dead_code)]`, wire into bottom bar |
| Progress bar spans | `widgets/progress_bar.rs` | Dead code — remove `#[allow(dead_code)]`, wire into bottom bar |
| Image loader | `image_loader.rs` | Running but unused — screens need to call `request()` and `get()` |
| ProgressState | `hooks.rs` | Dead code — activate for client-side progress interpolation |
| Theme progress fields | `theme.rs` | Dead code — `progress_filled`, `progress_empty`, `progress_cursor`, `progress_time` |

### New Code

#### 1. Bottom bar widget — `widgets/bottom_bar.rs`

Render-only widget following the three-layer architecture. Takes a data struct, outputs to frame.

**Data struct:**

```rust
// tui/types.rs
pub struct BottomBarData {
    pub group_name: String,
    pub track_title: Option<String>,
    pub track_artist: Option<String>,
    pub album_art_uri: Option<String>,
    pub playback_state: Option<PlaybackState>,
    pub progress: f64,         // 0.0–1.0
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: u16,
    pub is_wide: bool,         // terminal width >= 100
}
```

**Wide layout** (terminal >= 100 cols) — 3 rows, 3 columns:

```
┌────┐ Bohemian Rhapsody          ⏮  ▶  ⏭                       Living Room
│░░░░│ Queen            2:31 ━━━━━━━━━━━╺──────────── 5:55    ■■■■■■■··· 70%
└────┘
```

- **Left column**: 6-char wide album art area (3x2 half-blocks via `album_art.rs`) + track title (line 1) + artist (line 2) with ticker scrolling
- **Center column**: Playback controls `⏮  ▶  ⏭` (line 1) + progress bar with split times (line 2)
- **Right column**: Group name right-aligned (line 1) + volume bar right-aligned (line 2)

**Narrow layout** (terminal < 100 cols) — 3 rows, 2 columns + full-width row:

```
┌────┐ Bohemian Rhapsody             Living Room
│░░░░│ Queen                      ■■■■■■■··· 70%
└────┘
         ⏮  ▶  ⏭   2:31 ━━━━━━━━━━━━━━━━━━━━━━━╺──────────────── 5:55
```

- **Top-left**: Album art + track/artist
- **Top-right**: Group name + volume
- **Bottom full-width**: Controls + progress bar

**Rendering details:**
- Album art: 3-wide × 2-tall character area, rendered by `album_art::render_album_art()`
- Progress bar: delegate to `progress_bar::render_bar_spans()` with cursor char `╺`
- Volume bar: delegate to `volume_bar::render_volume_bar()`
- Playback icon: `▶` (playing), `⏸` (paused), `■` (stopped) — centered between ⏮ and ⏭

#### 2. Ticker scrolling — `widgets/bottom_bar.rs` (internal)

When track title or artist text exceeds available width:
- Append 3 spaces + repeat of text to create seamless loop
- Use `use_animation("ticker", true)` to advance offset each animation tick
- Slice visible window from the looped string at current offset
- Reset offset to 0 when text changes (detected by `use_state` comparing previous text)

#### 3. Data assembly — `screens/speakers.rs`

The speakers screen already knows which entry is selected. After assembling `SpeakerListData`, also assemble `BottomBarData`:

1. Call `group_for_entry(entries, selected_index)` to find the focused group. **If `None` (standalone/ungrouped speaker):** resolve the speaker's standalone group via the speaker's own group handle — the SDK models every speaker as belonging to a group.
2. Subscribe to the group coordinator's `current_track`, `playback_state`, `position` via `use_watch`. **Only subscribe to `position` for the focused group's coordinator** — position updates are frequent (~1/s) and watching all coordinators wastes event bandwidth.
3. Subscribe to `group.volume` via `use_watch_group`
4. Use `use_animation("progress", is_playing)` for progress interpolation
5. Use `use_state::<ProgressState>("bottom_bar_progress")` for client-side interpolation
6. **Album art loading sequence** (hook order matters):
   a. After `use_watch` returns `current_track` (with `album_art_uri`), call `app.image_loader.request(uri, coordinator.ip)` to trigger background fetch
   b. Call `use_state::<ArtProtocolState>("bottom_bar_art")` (must be after all `use_watch` calls per hook calling order)
   c. Call `art_state.ensure_protocol(uri, &app.image_loader, &app.picker)` to create/update the `StatefulProtocol`
   d. Pass `Option<&mut StatefulProtocol>` to the widget separately from `BottomBarData`
7. **Ticker state:** Use `use_state::<TickerState>("bottom_bar_ticker")` with fields `offset: usize`, `prev_title: String`, `prev_artist: String`. Reset offset to 0 when text changes.
8. Build `BottomBarData` from hook results
9. Return both `SpeakerListData`, `BottomBarData`, and `Option<&mut StatefulProtocol>` from the screen render function

#### 4. Layout integration — `ui.rs`

Update the `render()` function's vertical layout to include the bottom bar:

```
Header (1 line)
Separator (1 line)
Content area (dynamic)
Separator (1 line)
Bottom bar (3 lines — both wide and narrow modes)
Separator (1 line)
Key legend (1 line)
```

The bottom bar is always 3 lines. The wide layout ASCII art shows 3 lines (album art border requires top/content/bottom rows). The narrow layout also uses 3 lines (2-column top + 2-column middle + full-width controls row).

**Minimum height guard:** If `frame.area().height < 12`, suppress the bottom bar entirely and render only the speaker list with header/footer chrome. This prevents the bar from consuming all available space on very short terminals.

Calculate `is_wide` from `frame.area().width >= 100` and pass to the widget.

#### 5. Playback key handling — `handlers/speaker_list.rs`

Add playback control keys that operate on the currently focused group's coordinator:

- `p` — play/pause toggle: check `coordinator.playback_state.get()`, call `.play()` or `.pause()`. If state is `None` (not yet fetched), default to `.play()`.
- `n` — next track: `coordinator.next()`
- `b` — previous track: `coordinator.previous()`

**Standalone speaker fallback:** If `group_for_entry()` returns `None`, resolve the speaker directly via `app.system.speaker_by_id()` and call playback methods on it. This ensures playback controls work for ungrouped speakers.

**Error handling:** Mirror the volume error pattern — `Err(e) => app.status_message = Some(format!("error: {e}"))`.

These keys work in normal mode only — silently swallowed during pick-up mode (handled by the pick-up key dispatch).

### Theme Additions

Activate the existing dead-code theme fields:
- `progress_filled`, `progress_empty`, `progress_cursor`, `progress_time` — already defined in all 4 themes

Add new fields for the bottom bar chrome:
- `bottom_bar_border: Style` — reuse `muted` style in all 4 themes
- `bottom_bar_controls: Style` — reuse `accent` style in all 4 themes

## Design Decisions (from SpecFlow analysis)

| Question | Decision | Rationale |
|----------|----------|-----------|
| Bottom bar on Settings tab? | **Speakers-tab-only for initial implementation.** Follow-up plan can extend it. | Data assembly is tied to `screens/speakers.rs` which knows the focused group. A cross-tab architecture (new `screens/bottom_bar.rs`) adds complexity without clear v1 value. |
| Ungrouped speaker in bottom bar? | **Resolve standalone group via speaker's own group handle.** | The SDK models every speaker as belonging to a group. `group_for_entry()` returning `None` just means the list structure doesn't track it — the data is available. |
| Playback keys for standalone speakers? | **Yes.** Fall back to direct speaker control when no group resolved. | Users with standalone speakers expect playback to work. |
| Bottom bar height: 2 or 3 lines? | **Always 3 lines.** The ASCII art shows 3 rows (album art border requires top/content/bottom). | The plan text said "2 lines wide" but the mockup is clearly 3. Album art rendering requires minimum 3 rows. |
| Very narrow terminal (< 60 cols)? | **Render a minimal 1-line bar:** `[play icon] Track Title — Group Name`. | Below 60 cols, the 3-column layout breaks. A graceful degradation is better than garbled output. |
| Minimum terminal height? | **Suppress bottom bar below height 12.** | Prevents the bar from consuming all content space in split panes. |
| Playback state is `None` on first frame? | **Render stop icon, "No track" placeholder, zero progress.** | Clean fallback; corrects within 50ms as watches return data. |
| Play/pause toggle when state is `None`? | **Default to `.play()`.** | The user pressed `p` intending to start playback. |
| Album art loading trigger? | **Screen calls `image_loader.request(uri, coordinator.ip)` then manages `ArtProtocolState` via `use_state`.** Protocol ref passed to widget separately. | Respects hook calling order (use_watch before use_state). Image loader is already polled in event loop. |
| Ticker scroll speed? | **1 character per animation tick (250ms).** | Standard scrolling feel. ~4 chars/second is readable without being distracting. |
| Key legend update? | **Yes.** Append `p Play/Pause  n Next  b Prev` to the existing speaker list legend. | Users need to discover the playback keys. |
| Mute indicator? | **Defer to follow-up.** | Same decision as Enhanced Speaker List plan. |

## Acceptance Criteria

- [ ] Bottom bar renders below the speaker list, above the key legend (3 lines always)
- [ ] Bar follows cursor — changes group context when cursor moves into a different group
- [ ] Bar works for standalone (ungrouped) speakers by resolving their implicit group
- [ ] Wide layout (>= 100 cols): 3-column layout with album art, controls+progress, group+volume
- [ ] Narrow layout (60–99 cols): 2-column top rows + full-width controls row
- [ ] Minimal layout (< 60 cols): 1-line bar with play icon + track title + group name
- [ ] Bar suppressed entirely when terminal height < 12
- [ ] Album art thumbnail renders via `ratatui-image` (sixel/kitty) or placeholder
- [ ] Album art loads via `image_loader.request()` triggered in screen layer
- [ ] Progress bar animates in real-time when playing, pauses when paused/stopped
- [ ] Ticker scrolling for track title/artist text that exceeds available width (1 char/tick)
- [ ] Playback controls: `p` play/pause, `n` next, `b` previous (with error handling via status message)
- [ ] Playback controls work for standalone speakers (direct speaker fallback)
- [ ] Volume display matches the focused group's volume
- [ ] All existing speaker list functionality continues to work (navigation, volume adjust, pick-up)
- [ ] First-frame fallback: stop icon, "No track" text, zero progress when watches haven't returned
- [ ] `#[allow(dead_code)]` removed from `album_art.rs`, `progress_bar.rs`, `ProgressState`, theme fields
- [ ] Key legend updated with `p Play/Pause  n Next  b Prev`
- [ ] Three-layer architecture maintained: widget is render-only, data assembled in screen, keys handled in handler
- [ ] Bottom bar not rendered on Settings tab (Speakers-only for v1)

## Dependencies & Risks

**Dependencies:**
- The **Enhanced Speaker List** plan runs in parallel but the bottom bar depends on knowing which group is focused. The current `group_for_entry()` function already provides this — no cross-plan dependency. If the Enhanced Speaker List plan removes `UngroupedHeader`, the standalone speaker resolution becomes simpler.
- Image loader infrastructure is already wired into the event loop (`poll()` called each tick).
- The `album_art_mode` config from the **Settings View** plan: the bottom bar should check `config.album_art_mode.is_off()` before rendering album art, and respect `Halfblock` mode by forcing half-block rendering even if the terminal supports sixel/kitty. This is a simple match on the enum — low conflict risk.

**Risks:**
- **Terminal compatibility**: Album art rendering varies across terminals. The existing `Picker::from_query_stdio()` detection + fallback placeholder mitigates this.
- **Progress interpolation accuracy**: Client-side interpolation drifts over time. The existing `ProgressState` resets on SDK position events, which limits drift to the polling interval. After system sleep, the 10s cap in `ProgressState` limits drift until re-subscription.
- **Hook calling order complexity**: The screen layer must call hooks in strict order (use_watch → use_animation → use_state). Album art adds `ArtProtocolState` and ticker adds `TickerState` to the use_state chain. The exact sequence must be followed to satisfy the borrow checker.
- **Position watch bandwidth**: Only the focused group's coordinator gets a `position` watch. Switching groups causes a brief blank progress bar (50–200ms) until the new watch returns data. Acceptable for the interaction feel.

## Sources & References

- **Origin brainstorm:** [docs/brainstorms/2026-04-30-tui-v1-simplification-brainstorm.md](docs/brainstorms/2026-04-30-tui-v1-simplification-brainstorm.md) — bottom player bar design, responsive layouts, ticker scrolling decision
- **TUI architecture:** [docs/references/tui-architecture.md](docs/references/tui-architecture.md) — three-layer pattern, hook calling order, widget signature conventions
- **Existing widgets:** `src/tui/widgets/album_art.rs`, `src/tui/widgets/progress_bar.rs` (dead code to activate)
- **Hooks system:** `src/tui/hooks.rs` — `ProgressState` (dead code), `use_animation`, `use_watch`
- **Image loader:** `src/tui/image_loader.rs` — background HTTP fetcher, LRU cache
