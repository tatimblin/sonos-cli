---
title: "feat: Add album art widget and Now Playing tab layout"
type: feat
status: active
date: 2026-04-01
origin: docs/brainstorms/2026-02-26-sonos-tui-brainstorm.md
milestone: "Milestone 8: TUI — Group View"
---

# feat: Add album art widget and Now Playing tab layout

## Overview

Implement the Now Playing tab layout for the Group View and a reusable, scalable album art widget. The album art widget renders album covers at any size — from a 20×20 hero display down to a 3×3 mini-player thumbnail — using terminal graphics protocols auto-detected at startup. This is the first piece of Milestone 8 and unblocks the remaining Group View tabs.

## Problem Statement / Motivation

The Group View is currently a stub (`"Now Playing — Milestone 8"`). The brainstorm designs show album art as the visual centerpiece of the TUI — it appears in three locations at different sizes (see brainstorm: `docs/brainstorms/2026-02-26-sonos-tui-brainstorm.md`, Section 4). Without album art, the Now Playing screen is just text. The widget must be reusable and size-adaptive since it appears in the hero display, the mini-player, and eventually the queue.

## Proposed Solution

Build the feature in three phases:

1. **Album art infrastructure** — Terminal protocol detection (`Picker`), image fetching (background thread), caching, and the reusable `AlbumArt` widget
2. **Now Playing tab layout** — The full hero layout with album art, track metadata, volume, playback controls, and progress bar
3. **Mini-player integration** — Wire the 3×3 album art into the existing Home screen mini-player

### Key Decisions

These carry forward from the brainstorm and SpecFlow analysis:

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Rendering tiers | Sixel/Kitty/iTerm2 → halfblock fallback | `ratatui-image` handles both natively. Drop the ASCII tier — halfblocks work on any truecolor terminal. Terminals without truecolor show a text placeholder. (see brainstorm: Section 4) |
| HTTP client | `ureq` | Lightweight, sync, no tokio dependency. Matches the SDK's sync-first architecture. |
| Image fetching | Background `std::thread` | Avoids blocking the render thread (50ms tick). Fetch + decode takes 50–500ms on LAN. |
| Image caching | In-memory LRU, ~20 entries by URI | Covers track skips, group switches, and queue without redundant fetches. |
| `album_art_uri` format | Relative path from speaker IP | Sonos returns `/getaa?s=1&u=...`. Construct as `http://{speaker.ip}:1400{uri}`. |
| Picker init | Before `ratatui::init()` | `from_query_stdio()` queries terminal via escape sequences — must happen before raw mode. |
| Queue 1×1 art | Deferred (separate plan) | Dominant-color extraction for 50+ tracks is a different problem than image rendering. Not blocking for Now Playing. |
| Config override | `album_art_mode: "auto" \| "halfblock" \| "off"` | Users can force halfblock or disable art entirely. |

## Technical Considerations

### Architecture: Album Art Loading Pipeline

```
┌─────────────┐     ┌──────────────┐     ┌──────────────┐     ┌────────────┐
│ CurrentTrack │ ──▶ │ URI changed? │ ──▶ │ Background   │ ──▶ │ Image      │
│ .album_art   │     │ (compare     │     │ fetch thread │     │ cache      │
│ _uri         │     │  last URI)   │     │ (ureq + image│     │ (LRU ~20)  │
└─────────────┘     └──────────────┘     │  crate)      │     └──────┬─────┘
                                          └──────────────┘            │
                                                                      ▼
                    ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
                    │ StatefulImage│ ◀── │ Stateful     │ ◀── │ Picker       │
                    │ widget render│     │ Protocol     │     │ .new_resize  │
                    └──────────────┘     │ (per-widget) │     │ _protocol()  │
                                          └──────────────┘     └──────────────┘
```

### Picker Initialization Ordering

`Picker::from_query_stdio()` must execute before `ratatui::init()` enters raw mode. The current `tui::run()` in `src/tui/event.rs` calls `ratatui::init()` first — this must be restructured:

```rust
// BEFORE (broken for Picker)
pub fn run(app: App) -> Result<()> {
    let mut terminal = ratatui::init();
    // ...
}

// AFTER
pub fn run(app: App) -> Result<()> {
    let picker = Picker::from_query_stdio().ok();  // Before raw mode
    let mut terminal = ratatui::init();
    // Store picker in render context
}
```

### Background Image Fetching

Since the SDK is sync and the event loop is single-threaded, image fetching must happen off-thread to avoid blocking the TUI:

```rust
// Simplified flow
struct ImageLoader {
    pending: Option<Receiver<LoadResult>>,
    cache: HashMap<String, Arc<DynamicImage>>,
}

enum LoadResult {
    Loaded { uri: String, image: DynamicImage },
    Failed { uri: String, error: String },
}
```

The widget calls `image_loader.request(uri, speaker_ip)` which spawns a thread if the URI isn't cached. On subsequent frames, the widget checks `image_loader.try_recv()` for completed loads. The hooks system (`use_state`) holds the `ImageLoader` across frames.

### Album Art Widget — Reusable API

The widget is a pure render function following the existing pattern (`group_card.rs`, `volume_bar.rs`):

```rust
pub fn render_album_art(
    frame: &mut Frame,
    area: Rect,
    protocol: Option<&mut dyn StatefulProtocol>,
    placeholder_style: Style,
) {
    match protocol {
        Some(proto) => {
            let image = StatefulImage::default();
            frame.render_stateful_widget(image, area, proto);
        }
        None => {
            // Render placeholder — bordered box with "♪" or "No Art"
            render_placeholder(frame, area, placeholder_style);
        }
    }
}
```

The widget doesn't own state or do fetching — it accepts a `StatefulProtocol` (or `None` for placeholder) and renders into the given `Rect`. The calling screen manages the protocol lifecycle via hooks.

**Scaling behavior:**
- `StatefulProtocol` created via `picker.new_resize_protocol(image)` automatically re-encodes when the render area changes
- Hero (20×20): full detail, primary use case
- Mini-player (3×3): intentionally pixelated, adds visual warmth (see brainstorm: Section 4)
- The widget renders whatever fits in the provided `Rect` — no hardcoded sizes

### Now Playing Tab Layout

From the brainstorm (Screen 2, Tab 1):

```
┌─────────────────────────────────────────────────────────────────────────┐
│  ♪  S O N O S  ›  Living Room             [▸Now Playing]  Speakers  Queue│
│─────────────────────────────────────────────────────────────────────────│
│                                                                         │
│    ┌──────────────────────┐                                             │
│    │                      │     Bohemian Rhapsody                       │
│    │                      │     Queen                                   │
│    │      A L B U M       │     A Night at the Opera (1975)             │
│    │                      │                                             │
│    │       A R T          │                                             │
│    │                      │     🔊  ██████████████████░░░░░░  80%      │
│    │      (hero           │                                             │
│    │       ~20×20)        │     🔊×3  Beam + One SL × 2                 │
│    │                      │                                             │
│    │                      │                                             │
│    └──────────────────────┘                                             │
│                                                                         │
│                          ⏮     ▶     ⏭                                 │
│              ━━━━━━━━━━━━━━━━━━━╺──────────────────────                 │
│              2:31                                 5:55                   │
│                                                                         │
│─────────────────────────────────────────────────────────────────────────│
│ ←→ Tabs   ↑↓ Volume   ⏮ Prev   ␣ Pause   ⏭ Next   ⎋ Back            │
└─────────────────────────────────────────────────────────────────────────┘
```

**Layout splits (ratatui constraints):**

```
Vertical:
  ├── Top section (album art row): ~60% of content area
  │   ├── Left: Album art (fixed width ~24 cols including border)
  │   └── Right: Track metadata + volume + speaker info
  ├── Middle: Playback controls (3 rows: icons, bar, timestamps)
  └── Bottom: Padding

Responsive:
  - Width < 50: Skip album art entirely, show metadata centered
  - Width < 80: Reduce art area proportionally
  - Width >= 80: Full layout as designed
```

### Hooks Integration

Following the established pattern from `home_groups.rs`:

```rust
fn render_now_playing(frame: &mut Frame, area: Rect, ctx: &mut RenderContext, group_id: &GroupId) {
    let group = ctx.app.system.group_by_id(group_id);
    let coordinator = group.and_then(|g| g.coordinator());

    // Property watches (same pattern as home_groups.rs)
    let current_track = ctx.hooks.use_watch(&coordinator.current_track);
    let playback_state = ctx.hooks.use_watch(&coordinator.playback_state);
    let position = ctx.hooks.use_watch(&coordinator.position);
    let group_volume = ctx.hooks.use_watch_group(&group.volume);

    // Animation for progress bar
    ctx.hooks.use_animation(&format!("{}:now_playing:tick", group_id), is_playing);

    // Progress interpolation state
    let progress = ctx.hooks.use_state::<ProgressState>(...);

    // Album art state (ImageLoader + StatefulProtocol)
    let art_state = ctx.hooks.use_state::<AlbumArtState>(...);

    // Render layout...
}
```

### Mini-Player Album Art Integration

The Home screen mini-player currently shows group name + track info + progress + volume. Adding 3×3 album art on the left:

```
│ ▓▓▓ Living Room  ▶ Bohemian Rhapsody — Queen   ━━━━╺──── 2:31/5:55   🔊 80%  │
│ ▓▓▓                                                                            │
│ ▓▓▓                                                                            │
```

The same `render_album_art` function is used, just with a 3×3 `Rect`. Needs its own `StatefulProtocol` instance (different size encoding than the hero). Debounce image loading by 300ms when the focused group changes rapidly (arrow key held down).

### New Files and Dependencies

| File | Purpose |
|------|---------|
| `src/tui/image_loader.rs` | Background fetch thread, LRU cache, URI construction |
| `src/tui/widgets/album_art.rs` | Reusable album art render function + placeholder |
| `src/tui/screens/now_playing.rs` | Now Playing tab layout and hook wiring |

**Cargo.toml addition:**
```toml
ureq = "3"  # Sync HTTP client for image fetching
```

**Already present (unused until now):**
```toml
ratatui-image = "3"
image = "0.25"
```

**ratatui-image feature flags:** Ensure `crossterm` feature is enabled to match the ratatui backend.

## Acceptance Criteria

### Phase 1: Album Art Infrastructure
- [x] `Picker::from_query_stdio()` called before `ratatui::init()` in `src/tui/mod.rs`
- [x] `Picker` stored in `App` (via `RefCell<Option<Picker>>`), accessible from render functions
- [x] `ImageLoader` struct: spawns background thread for HTTP fetch + `image` crate decode
- [x] Image cache (LRU, ~20 entries keyed by URI string)
- [x] URI construction: `http://{speaker.ip}:1400{album_art_uri}` for relative URIs, passthrough for absolute
- [x] HTTP timeout: 3 seconds via `ureq`
- [x] Graceful handling: `album_art_uri` is `None` → placeholder; fetch fails → placeholder; decode fails → placeholder
- [x] `render_album_art()` widget function in `src/tui/widgets/album_art.rs`
- [x] Placeholder rendering: bordered box with `♪` symbol in theme's muted style
- [x] `album_art_mode` config option: `"auto"` (default), `"halfblock"`, `"off"`

### Phase 2: Now Playing Tab Layout
- [x] `src/tui/screens/now_playing.rs` replaces the stub in `render_group_view`
- [x] Layout: album art (left) + track metadata (right) + playback controls (center bottom)
- [x] Track metadata: title (bold), artist, album (muted)
- [x] Group volume bar with `↑↓` adjustment (reuse `volume_bar` widget)
- [x] Speaker count info line (e.g., "Beam + One SL × 2")
- [x] Playback control icons: `⏮  ▶  ⏭` (centered)
- [x] Progress bar with elapsed/total time (reuse `progress_bar` widget + `ProgressState` interpolation)
- [x] `Space` toggles play/pause, `n` next, `p` prev
- [x] Property watches: `current_track`, `playback_state`, `position`, `group.volume`
- [x] Album art updates when track changes (URI change detection)
- [x] Responsive: skip album art when content width < 50, metadata-only layout

### Phase 3: Mini-Player Album Art
- [ ] 3×3 album art rendered on the left side of the existing mini-player bar
- [ ] Separate `StatefulProtocol` instance for the mini-player size
- [ ] Debounce: 300ms delay before fetching art when focused group changes
- [ ] Falls back to no-art mini-player layout when `album_art_mode = "off"` or terminal too narrow

## Success Metrics

- Album art renders correctly in Now Playing tab on iTerm2, Kitty, WezTerm, and Foot (Sixel/Kitty protocols)
- Half-block fallback works on Terminal.app, Alacritty, and other truecolor terminals
- Track changes update art within one render frame after the background fetch completes (~100–300ms on LAN)
- No visible TUI freeze during image loading (background thread)
- Mini-player art tracks the focused group on the Home screen

## Dependencies & Risks

| Risk | Mitigation |
|------|------------|
| `Picker::from_query_stdio()` fails in some terminal multiplexers (tmux, screen) | Fall back to halfblock protocol. Store `Option<Picker>` — `None` means no art. |
| `ratatui-image` v3 `StatefulProtocol` may not satisfy `'static` for hooks `use_state` | Wrap in a newtype struct that implements the necessary trait bounds. Verify early in Phase 1. |
| Image encoding latency > 50ms causes frame drops | `new_resize_protocol` handles this incrementally. If still slow, cap hero art size to 16×16. |
| `ureq` v3 API changes | Pin to `ureq = "3"` (major version). Review changelog. |
| Sonos streaming services return non-image data for `album_art_uri` | Validate Content-Type header before decode. Fall back to placeholder on any error. |
| `album_art_uri` is sometimes an absolute external URL (not speaker-local) | Check if URI starts with `http`. If absolute, fetch directly. If relative, prepend speaker IP. Restrict to private IP ranges if security is a concern. |

## Sources & References

- **Origin brainstorm:** [docs/brainstorms/2026-02-26-sonos-tui-brainstorm.md](docs/brainstorms/2026-02-26-sonos-tui-brainstorm.md) — Album art rendering strategy (Section 4), Now Playing layout (Screen 2 Tab 1), mini-player design, size table
- **Roadmap:** [docs/product/roadmap.md](docs/product/roadmap.md) — Milestone 8 checklist items for Now Playing tab
- **SDK API:** [docs/references/sonos-sdk.md](docs/references/sonos-sdk.md) — `CurrentTrack.album_art_uri`, property handles, `Speaker.ip`
- **ratatui-image docs:** `Picker::from_query_stdio()`, `StatefulImage`, `StatefulProtocol`, protocol auto-detection
- **Existing patterns:** `src/tui/screens/home_groups.rs` (hooks + watch lifecycle), `src/tui/widgets/group_card.rs` (widget composition)
