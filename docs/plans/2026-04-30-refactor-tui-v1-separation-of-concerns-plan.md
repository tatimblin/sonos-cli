---
title: "refactor: TUI v1 separation of concerns — widgets, screens, handlers"
type: refactor
status: completed
date: 2026-04-30
---

# refactor: TUI v1 Separation of Concerns

## Overview

Restructure the TUI's ~2,700 lines across 18 files into three clean layers:

1. **Widgets** — render-only component library. Take data structs + theme, output to frame. No hooks, no SDK, no key handling.
2. **Screens** — layout + data assembly. Call hooks, transform SDK data into widget-friendly structs, define layout areas, compose widgets.
3. **Handlers** — key handling + SDK mutations. Dispatch key events, navigate, call SDK methods for volume/playback/regrouping.

This is a pure structural refactor — zero user-visible behavior changes. It prepares the codebase for the remaining Milestone 8–10 work (Queue tab, Speaker Detail, Startup screen) by making it easy to add new screens and widgets without accumulating more mixed-concern code.

## Problem Statement

The current TUI works well but has growing structural debt:

- **`speaker_list.rs` (723 lines)** is the largest file and mixes rendering (lines 195–434), normal-mode key handling (lines 486–575), pick-up-mode key handling (lines 634–722), and direct SDK calls for volume adjustment (lines 577–598) and speaker regrouping (lines 679–706). Adding features to this file means touching all three concerns at once.

- **Screen render functions mix data fetching and layout.** `home_groups.rs` and `now_playing.rs` interleave hook calls (SDK subscriptions) with layout calculations and widget composition in single large functions. The hooks calling-order constraint (`use_watch` → `use_animation` → `use_state`) forces data assembly and rendering to be interleaved rather than separated.

- **`ui.rs` contains inline screen stubs.** `render_group_view()` (lines 236–262) and `render_speaker_detail()` (lines 264–274) live in the top-level render dispatch instead of in their own screen modules.

- **Shared data helpers are duplicated.** Playback icon mapping, track info extraction, and speaker count text formatting appear in `home_groups.rs`, `now_playing.rs`, and `speaker_list.rs`.

- **Types shared across layers live in widget files.** `SpeakerListMode`, `ListEntry`, `PickUpState`, and `SpeakerListAction` are defined in `widgets/speaker_list.rs` but used by `handlers/home.rs` and `handlers/group.rs`.

## Proposed Solution

Six phases, each a self-contained PR that compiles and runs:

1. **Extract shared types to `tui/types.rs`** — Move cross-layer types out of widgets
2. **Extract speaker list key handling to `handlers/speaker_list.rs`** — Split the 723-line file
3. **Extract screen stubs from `ui.rs`** — Create proper screen modules
4. **Make speaker list widget render-only** — Replace hooks + SDK reads with a data struct
5. **Extract shared data helpers** — Deduplicate playback/track/speaker formatting
6. **Write `docs/references/tui-architecture.md`** — Codify the architecture for future agents

## Technical Approach

### Architecture After Refactor

```
src/tui/
  mod.rs              ← module root, run()
  app.rs              ← App state, Navigation, Screen enum
  event.rs            ← event loop (unchanged)
  types.rs            ← NEW: shared types (SpeakerListMode, ListEntry, PickUpState, etc.)
  helpers.rs          ← NEW: shared data helpers (playback_icon, track_info, speaker_count)
  hooks.rs            ← hooks system (unchanged)
  image_loader.rs     ← background image fetcher (unchanged)
  theme.rs            ← theme system (unchanged)
  ui.rs               ← SLIMMED: header + footer + dispatch only (no inline screens)

  handlers/
    mod.rs            ← re-exports
    home.rs           ← Home screen key handling (mostly unchanged)
    group.rs          ← GroupView key handling (mostly unchanged)
    speaker_list.rs   ← NEW: extracted from widgets/speaker_list.rs

  screens/
    mod.rs            ← re-exports
    home_groups.rs    ← Home > Groups tab (data assembly + layout)
    home_speakers.rs  ← NEW: thin wrapper calling speaker_list widget with assembled data
    now_playing.rs    ← GroupView > Now Playing tab (data assembly + layout)
    group_speakers.rs ← NEW: thin wrapper calling speaker_list widget with assembled data
    group_view.rs     ← NEW: extracted from ui.rs render_group_view()
    speaker_detail.rs ← NEW: extracted from ui.rs render_speaker_detail()
    queue.rs          ← NEW: extracted from ui.rs Queue stub

  widgets/
    mod.rs            ← re-exports
    album_art.rs      ← render-only (already clean)
    group_card.rs     ← render-only (already clean)
    progress_bar.rs   ← render-only (already clean)
    volume_bar.rs     ← render-only (already clean)
    speaker_list.rs   ← SLIMMED: render-only, takes SpeakerListRenderData
```

### Layer Responsibilities

| Layer | Knows about | Does NOT know about |
|-------|-------------|---------------------|
| Widgets | `Frame`, `Rect`, `Theme`, own data structs | Hooks, SDK, App, key events |
| Screens | `Frame`, `Rect`, `RenderContext` (app + hooks), widgets | Key events, SDK mutations |
| Handlers | `App` (mutable), key events, SDK mutations | `Frame`, `Rect`, rendering |

### Implementation Phases

#### Phase 1: Extract shared types to `tui/types.rs`

**Why first:** Every subsequent phase needs these types to live outside `widgets/speaker_list.rs` to avoid circular imports. Moving them first is a clean, low-risk change.

**What moves:**

From `widgets/speaker_list.rs` → `tui/types.rs`:
- `SpeakerListMode` (used by handlers + screens + widgets) — `speaker_list.rs:25–30`
- `ListEntry` (used by handlers + widgets) — `speaker_list.rs:33–39`
- `PickUpState` (used by app.rs + handlers + widgets) — `speaker_list.rs:48–53`
- `SpeakerListAction` (used by handlers) — `speaker_list.rs:56–61`
- `build_list_entries()` and helpers (`build_full_list`, `build_scoped_list`, `group_for_entry`, `build_display_order`) — `speaker_list.rs:76–188`

From `widgets/group_card.rs` → `tui/types.rs`:
- `PlaybackIcon` enum — `group_card.rs:31–53`

**Changes to existing files:**
- `widgets/speaker_list.rs`: replace definitions with `use crate::tui::types::*`
- `widgets/group_card.rs`: replace `PlaybackIcon` definition with import
- `handlers/home.rs`: update import path
- `handlers/group.rs`: update import path
- `app.rs`: import `PickUpState` from `types` instead of `widgets::speaker_list`
- `tui/mod.rs`: add `pub mod types;`

**Tasks:**
- [x] Create `src/tui/types.rs` with moved types and functions
- [x] Update all import paths (6 files)
- [x] `cargo check` passes
- [x] All existing tests pass unchanged

---

#### Phase 2: Extract speaker list key handling to `handlers/speaker_list.rs`

**Why:** This splits the 723-line `widgets/speaker_list.rs` along its natural seam — rendering vs. key handling. After this phase, the widget file drops from 723 to ~440 lines.

**What moves:**

From `widgets/speaker_list.rs` → `handlers/speaker_list.rs`:
- `handle_key()` — `speaker_list.rs:459–476`
- `next_selectable()` / `prev_selectable()` — `speaker_list.rs:478–484`
- `handle_normal_key()` — `speaker_list.rs:486–575`
- `handle_volume_adjust()` — `speaker_list.rs:577–598`
- `enter_add_speaker_mode()` — `speaker_list.rs:600–632`
- `handle_pick_up_key()` — `speaker_list.rs:634–722`

**Interface between layers:**

The handler module needs:
- `App` (mutable) — for navigation and SDK calls
- `KeyEvent` — the input
- `SpeakerListMode` — from `types.rs` (moved in Phase 1)
- `ListEntry`, `PickUpState`, `SpeakerListAction` — from `types.rs`
- `build_list_entries()`, `group_for_entry()`, `build_full_list()` — from `types.rs`

The handler does NOT need any rendering types (`Frame`, `Rect`, `Theme`).

**Changes to existing files:**
- `handlers/mod.rs`: add `pub mod speaker_list;`
- `handlers/home.rs`: update `speaker_list::handle_key` → `handlers::speaker_list::handle_key` (already calls this)
- `handlers/group.rs`: same import update
- `widgets/speaker_list.rs`: delete moved functions (~280 lines removed)

**Tasks:**
- [x] Create `src/tui/handlers/speaker_list.rs` with moved key handling
- [x] Update imports in `handlers/home.rs` and `handlers/group.rs`
- [x] Remove moved functions from `widgets/speaker_list.rs`
- [x] `cargo check` passes
- [x] Key handling works identically (manual test: navigate speakers, adjust volume, pick up/drop)

---

#### Phase 3: Extract screen stubs from `ui.rs`

**Why:** `ui.rs` currently has two inline rendering functions that belong in `screens/`. Extracting them now — before Phase 4 modifies the speaker list call sites — avoids touching the same code path twice across consecutive PRs.

**What moves:**

From `ui.rs` → `screens/group_view.rs`:
- `render_group_view()` — `ui.rs:236–262`

From `ui.rs` → `screens/speaker_detail.rs`:
- `render_speaker_detail()` — `ui.rs:264–274`

From `ui.rs` → `screens/queue.rs`:
- The Queue stub logic inside `render_group_view` match arm — `ui.rs:254–260`

**`group_view.rs` becomes the sub-dispatcher** for GroupView tabs (NowPlaying, Speakers, Queue), matching the pattern where `ui.rs` dispatches to screens and screens compose widgets.

**After this phase, `ui.rs` contains only:**
- `render()` — top-level dispatch: header, separator, screen call, separator, footer
- `render_header()` — logo + tab bar
- `render_key_legend()` — footer
- `draw_separator()` — utility
- `build_logo()`, `build_tab_spans()`, `render_tab_labels()` — header helpers

No screen rendering logic remains.

**Tasks:**
- [x] Create `screens/group_view.rs` with extracted `render_group_view()`
- [x] Create `screens/speaker_detail.rs` with extracted `render_speaker_detail()`
- [x] Create `screens/queue.rs` with Queue stub
- [x] Update `screens/mod.rs` to re-export new modules
- [x] Update `ui.rs` to call screen modules instead of local functions
- [x] `cargo check` passes
- [x] All screens render identically

---

#### Phase 4: Make speaker list widget render-only

**Why:** After Phase 2 removed key handling and Phase 3 created proper screen modules, the widget still calls hooks and reads SDK data during rendering (lines 222–283). This phase makes it a true render-only widget by introducing a data struct the screen layer prepares. The screen modules from Phase 3 (`group_view.rs`) give the data assembly code a natural home.

**Core change:** The widget currently takes `&mut RenderContext` and internally subscribes to SDK properties, resolves speaker/group names, and builds render data. After this phase, a new data struct captures all pre-computed display information, and the widget signature becomes `fn render(frame, area, &SpeakerListData, &Theme)`.

The data struct must pre-resolve all names and display strings. The widget must not access `SonosSystem` — otherwise "render-only" is meaningless. Follow the `GroupCardData` pattern already established in `group_card.rs`.

**Screen layer (new files):**

Create `screens/home_speakers.rs` and `screens/group_speakers.rs` — thin functions that:
1. Call hooks (`use_watch` on speaker volumes, group volumes, playback states, current tracks, group membership — see Side-Effect Inventory for the full list)
2. Build the data struct from hook results, pre-resolving all names via `&App.system`
3. Call `widgets::speaker_list::render(frame, area, &data, &theme)`

**Borrow checker consideration:** The current widget takes `&mut RenderContext` (which holds `&App` + `&mut Hooks`). After this change, the screen functions own the `&mut RenderContext` exclusively during data assembly, and the widget only needs `&data` + `&Theme`. This actually *simplifies* the borrow situation.

**Tasks:**
- [x] Define data structs for the speaker list widget in `tui/types.rs`
- [x] Create `screens/home_speakers.rs` — data assembly + calls widget
- [x] Create `screens/group_speakers.rs` — data assembly + calls widget
- [x] Refactor `widgets/speaker_list.rs` to take data struct + `Theme` only
- [x] Remove hooks and SDK imports from `widgets/speaker_list.rs`
- [x] Update `screens/group_view.rs` and `ui.rs` dispatch to call screen functions
- [x] `cargo check` passes
- [x] Speaker list renders identically in both Full and GroupScoped modes
- [x] Volume bars, playback icons, track info all still appear
- [x] Pick-up mode visual behavior unchanged

---

#### Phase 5: Extract shared data helpers to `tui/helpers.rs`

**Why:** Three render files duplicate the same data-extraction logic. Centralizing it reduces copy-paste bugs and makes the patterns available to new screens.

**Duplicated patterns to extract:**

1. **Playback icon mapping** — appears in `home_groups.rs:113–117`, `now_playing.rs:91–95`, `speaker_list.rs:323–327` (render data)

   Already partially solved by `PlaybackIcon` enum in `group_card.rs` (moved to `types.rs` in Phase 1). Extend with a `from_playback_state()` constructor:

   ```rust
   impl PlaybackIcon {
       pub fn from_state(state: Option<&PlaybackState>) -> Self {
           match state {
               Some(PlaybackState::Playing) => Self::Playing,
               Some(PlaybackState::Paused) => Self::Paused,
               _ => Self::Stopped,
           }
       }
   }
   ```

2. **Track info extraction** — appears in `home_groups.rs:119–128`, `now_playing.rs:77–87`, `speaker_list.rs:261–264`

   ```rust
   pub fn extract_track_info(track: &Option<CurrentTrack>) -> (String, String, String) {
       track.as_ref()
           .filter(|t| !t.is_empty())
           .map(|t| (
               t.title.clone().unwrap_or_default(),
               t.artist.clone().unwrap_or_default(),
               t.album.clone().unwrap_or_default(),
           ))
           .unwrap_or_default()
   }
   ```

3. **Speaker count text** — appears in `home_groups.rs:178–182` (`"+ {n}"`), `now_playing.rs:99–103` (`"+ {n} more"`)

   ```rust
   pub fn speaker_count_text(coordinator_model: &str, member_count: usize, verbose: bool) -> String {
       if member_count <= 1 {
           coordinator_model.to_string()
       } else {
           let suffix = if verbose { " more" } else { "" };
           format!("{coordinator_model} + {}{suffix}", member_count - 1)
       }
   }
   ```

4. **Progress state update** — appears in `home_groups.rs:161–166`, `now_playing.rs:62–66`

   ```rust
   pub fn update_progress(state: &mut ProgressState, position: &Option<Position>, is_playing: bool) {
       if let Some(pos) = position.as_ref() {
           state.update(pos.position_ms, pos.duration_ms, is_playing);
       } else {
           state.is_playing = is_playing;
       }
   }
   ```

**Tasks:**
- [x] Create `src/tui/helpers.rs` with extracted functions
- [x] Add `PlaybackIcon::from_state()` constructor on the type in `types.rs`
- [x] Replace duplicated code in `home_groups.rs`, `now_playing.rs`, `home_speakers.rs`, `group_speakers.rs`
- [x] `tui/mod.rs`: add `mod helpers;`
- [x] `cargo check` passes
- [x] All rendering identical

---

#### Phase 6: Write `docs/references/tui-architecture.md`

**Why:** The three-layer architecture, hooks calling order, render-only enforcement, and data-struct boundary pattern are all decisions that exist only in developers' heads (and this plan). Future agents adding screens, widgets, or handlers will not know these rules and will regress toward mixed-concern code. A reference doc makes the architecture self-documenting and durable.

**This is not a README or tutorial.** It's a terse reference for agents and contributors — what the layers are, why they exist, and how to add new things without violating the structure. It belongs alongside `cli-guidelines.md` and `sonos-sdk.md` in `docs/references/`.

**Document structure:**

```markdown
# TUI Architecture

## Design Ethos

[3-4 sentences: render-only widgets as a component library, screens own
data assembly and layout, handlers own input and SDK mutations. The SDK
is the shared data layer — no intermediate state store or event bus.]

## Three-Layer Architecture

### Widgets (`src/tui/widgets/`)

[What they are, what they receive, what they must NOT import.
Signature pattern: `fn render(frame, area, &Data, &Theme)`.
Example: `group_card.rs` takes `GroupCardData`, outputs to frame.]

### Screens (`src/tui/screens/`)

[What they do: call hooks, transform SDK data into widget data structs,
define layout areas with ratatui `Layout`, compose widgets.
Signature pattern: `fn render(frame, area, &mut RenderContext, ...)`.
Example: `home_groups.rs` subscribes to playback state, builds
`GroupCardData` for each group, renders a responsive grid of cards.]

### Handlers (`src/tui/handlers/`)

[What they do: receive key events, mutate `App` state (navigation,
selection), call SDK methods for volume/playback/regrouping.
Signature pattern: `fn handle_key(app: &mut App, key, ...) -> Action`.
They must NOT import ratatui rendering types.]

## Shared Modules

### `tui/types.rs`
[Cross-layer types that prevent circular imports. ListEntry,
SpeakerListMode, PickUpState, PlaybackIcon, data structs, list
building functions.]

### `tui/helpers.rs`
[Shared data transformation functions: track info extraction,
speaker count text, progress state update.]

### `tui/hooks.rs`
[Reactive state system. Calling order constraint and why.
Mark-and-sweep lifecycle. Brief description — detailed API is
in the code.]

## Hooks Calling Order

[The constraint: `use_watch` first (returns owned values, borrow
released immediately), `use_animation` second (brief borrow),
`use_state` last (holds `&mut` for duration). Why: satisfies Rust
borrow checker without RefCell. Where: inside screen render functions,
never in widgets.]

## How to Add a New Screen

[Step-by-step: 1. Create screen module in screens/. 2. Add render
function taking RenderContext. 3. Call hooks for SDK subscriptions.
4. Build widget data structs. 5. Compose widgets with Layout. 6. Add
Screen enum variant to app.rs. 7. Add dispatch arm in ui.rs. 8. Add
key handler in handlers/.]

## How to Add a New Widget

[Step-by-step: 1. Create widget module in widgets/. 2. Define a data
struct for all render inputs. 3. Write render function taking
(frame, area, &Data, &Theme). 4. Do NOT import hooks, App, or SDK.
5. Re-export from widgets/mod.rs.]

## How to Add a New Handler

[Step-by-step: 1. Create or extend handler in handlers/. 2. Take
&mut App + KeyEvent. 3. Return an action enum if the caller needs
to respond. 4. Call SDK methods directly for mutations. 5. Do NOT
import Frame, Rect, or rendering types.]

## Anti-Patterns

[What NOT to do, with the "why":
- Don't call hooks inside widgets (breaks render-only)
- Don't render inside handlers (breaks separation)
- Don't add SDK mutation calls in screens (belongs in handlers)
- Don't scatter widget state across App + event loop + render
  (use hooks to co-locate state with the widget's screen)
- Don't add an intermediate Action/executor dispatch layer
  (SDK is the shared layer — see architecture simplification)]
```

**What to reference:**
- This plan (`docs/plans/2026-04-30-refactor-tui-v1-separation-of-concerns-plan.md`) for the full rationale
- The hooks architecture brainstorm (`docs/brainstorms/2026-03-29-tui-hooks-architecture-brainstorm.md`) for the hooks design decisions
- The architecture simplification brainstorm (`docs/brainstorms/2026-03-10-cli-architecture-simplification-brainstorm.md`) for the "no middleware" principle

**Update CLAUDE.md** to add the new doc to the Reference Documentation table:

```
| `docs/references/tui-architecture.md` | TUI three-layer architecture, hooks system, how to add screens/widgets/handlers |
```

**Tasks:**
- [x] Create `docs/references/tui-architecture.md` following the structure above
- [x] Keep it under 200 lines — terse reference, not a tutorial
- [x] Add entry to CLAUDE.md Reference Documentation table
- [x] Every rule in the doc is verifiable by reading the code (no aspirational rules)
- [x] Cross-reference to this plan and the hooks brainstorm for deeper rationale

## System-Wide Impact

Pure structural refactor — no behavior, API, or state lifecycle changes. The call chain after refactor:

```
event.rs → handle_key() → handlers/*.rs → App mutations + SDK calls
event.rs → render()     → ui.rs → screens/*.rs → hooks + data assembly → widgets/*.rs → Frame
```

## Acceptance Criteria

### Functional Requirements

- [x] All 6 phases compile (`cargo check`) individually
- [x] TUI launches and renders identically to pre-refactor
- [x] Home > Groups tab: cards render with live progress, volume, track info, album art
- [x] Home > Speakers tab: grouped speaker list with volume bars, pick-up/drop works
- [x] GroupView > NowPlaying: album art, metadata, controls, progress bar
- [x] GroupView > Speakers: scoped speaker list with add-speaker flow
- [x] GroupView > Queue: stub renders
- [x] SpeakerDetail: stub renders
- [x] Navigation: Enter drills in, Esc pops back, tab switching works
- [x] SDK mutations: volume adjust, play/pause, next/prev, regrouping all work
- [x] Status messages appear and clear correctly
- [x] Pick-up mode: visual reordering, drop confirmation, cancel with Esc

### Non-Functional Requirements

- [x] No widget file imports `hooks`, `App`, `SonosSystem`, or `crossterm::event`
- [x] No handler file imports `Frame`, `Rect`, or any ratatui rendering type
- [x] `widgets/speaker_list.rs` is under 250 lines
- [x] No new `#[allow(clippy::too_many_arguments)]` annotations (use data structs instead)
- [x] Existing `app.rs` tests pass unchanged

### Documentation Requirements

- [x] `docs/references/tui-architecture.md` exists and covers all three layers
- [x] Every "how to add" section is accurate against the post-refactor code
- [x] CLAUDE.md references the new doc in the Reference Documentation table
- [x] No aspirational rules — every statement is verifiable by reading the codebase

### Quality Gates

- [x] `cargo clippy -- -D warnings` passes
- [x] Each phase is a separate commit/PR with a clear description
- [x] No dead code warnings (clean up unused imports at each phase)

## Side-Effect Inventory

Before any phase begins, every `use_watch` call in render functions must be classified as **display data** (needed for rendering) or **side effect** (subscription that triggers refresh behavior). Side-effect subscriptions must be preserved in the screen's data assembly, not lost when the widget becomes render-only.

| File | Line | Hook Call | Purpose |
|------|------|-----------|---------|
| `speaker_list.rs` | 232 | `use_watch(&s.volume)` | Display data — speaker volume bar |
| `speaker_list.rs` | 235 | `use_watch(&s.group_membership)` | **Side effect** — topology refresh after regrouping |
| `speaker_list.rs` | 249 | `use_watch_group(&g.volume)` | Display data — group volume |
| `speaker_list.rs` | 254 | `use_watch(&c.playback_state)` | Display data — play/pause icon |
| `speaker_list.rs` | 258 | `use_watch(&c.current_track)` | Display data — track info |
| `home_groups.rs` | 108 | `use_watch(&coordinator.playback_state)` | Display data |
| `home_groups.rs` | 109 | `use_watch(&coordinator.current_track)` | Display data |
| `home_groups.rs` | 110 | `use_watch(&coordinator.position)` | Display data — progress bar |
| `home_groups.rs` | 111 | `use_watch_group(&group.volume)` | Display data |
| `now_playing.rs` | 35 | `use_watch(&coordinator.playback_state)` | Display data |
| `now_playing.rs` | 36 | `use_watch(&coordinator.current_track)` | Display data |
| `now_playing.rs` | 37 | `use_watch(&coordinator.position)` | Display data — progress bar |
| `now_playing.rs` | 38 | `use_watch_group(&group.volume)` | Display data |

The critical one is `speaker_list.rs:235` — without this `group_membership` subscription, the speaker list won't auto-refresh after regrouping. The screen's data assembly function (`home_speakers.rs` / `group_speakers.rs`) must call `use_watch(&s.group_membership)` for every speaker in the list.

## Render-Only Enforcement

Widget "render-only" status is enforced at two levels:

1. **Signature level:** Render-only widgets take `(frame: &mut Frame, area: Rect, data: &Data, theme: &Theme)`. They never receive `&mut RenderContext`, `&mut Hooks`, or `&App`.

2. **Import level:** Widget modules must NOT import `crate::tui::hooks`, `crate::tui::app::App`, or `sonos_sdk::SonosSystem`. Verified during code review. (The existing clean widgets — `volume_bar`, `progress_bar`, `group_card`, `album_art` — already follow this pattern.)

## Known Pre-Existing Issues (Do Not Fix in This Refactor)

- **Missing play/pause on Home > Groups:** The key legend (`ui.rs:208`) shows "Play/Pause" but `handle_home_groups_key` in `handlers/home.rs` does not handle `KeyCode::Char(' ')`. This predates the refactor — fix in a separate PR.

- **Inconsistent speaker count format:** `home_groups.rs:178` uses `"+ {n}"` while `now_playing.rs:99` uses `"+ {n} more"`. Phase 5 preserves both variants via parameterization: `speaker_count_text(model, count, verbose: bool)`.

## Dependencies & Risks

**Dependencies:** None. This is an internal refactor with no SDK or dependency changes.

**Risks:**

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| Borrow checker issues when splitting `RenderContext` across layers | Medium | Phase 3 actually simplifies borrows: widgets take `&data` + `&Theme` instead of `&mut RenderContext`. Screen functions own the `&mut RenderContext` exclusively during data assembly. |
| Phase ordering mistake causes intermediate state that doesn't compile | Low | Each phase is designed to compile independently. Phase 1 (types) unblocks all others. Phases 2–5 can be reordered if needed. |
| Hooks calling order violated after splitting data assembly from rendering | Medium | Screen functions call hooks in the correct order (`use_watch` → `use_animation` → `use_state`), then pass owned results to widgets. The constraint stays within one function — the screen's render function. |
| Clone overhead from data structs at layer boundaries | Low | Data is small (strings + numbers). Already cloning `GroupCardData` for each card. The speaker list data struct replaces per-entry hook lookups, so it's net-neutral. |
| Regression in pick-up mode visual behavior | Medium | Pick-up mode is the most complex interaction (visual reordering + SDK calls on drop). Test manually at Phase 2 (key handling extraction) and Phase 3 (render data struct). |

## Sources & References

### Internal References

- Current widget architecture: `src/tui/widgets/speaker_list.rs` (723 lines — primary target)
- Hooks system: `src/tui/hooks.rs` — `use_watch`, `use_animation`, `use_state` with mark-and-sweep
- Existing clean widget pattern: `src/tui/widgets/group_card.rs` — takes `GroupCardData` struct, render-only
- Event loop: `src/tui/event.rs` — `begin_frame()` → render → `end_frame()` lifecycle
- Top-level dispatch: `src/tui/ui.rs` — header/footer + screen routing

### Institutional Learnings

- Hooks architecture plan (completed 2026-03-29): `docs/plans/2026-03-29-refactor-tui-hooks-architecture-plan.md` — established the calling-order constraint and mark-and-sweep pattern
- Architecture simplification (completed 2026-03-10): `docs/plans/2026-03-10-refactor-cli-architecture-simplification-plan.md` — "SDK is the shared layer, no middleware" principle applies to TUI too
- Album art code review (completed 2026-04-04): `docs/plans/2026-04-04-refactor-album-art-code-review-fixes-plan.md` — widget quality gotchas (explicit parameters prevent off-by-one bugs)

### Roadmap Alignment

This refactor prepares the codebase for:
- **Milestone 8** (TUI — Group View): Queue tab, Speaker Detail EQ controls — easier to implement as new screen modules
- **Milestone 9** (TUI — Startup & Speaker Detail): New screens drop into `screens/` cleanly
- **Milestone 10** (Polish): Easier to audit and maintain with clear layer boundaries
