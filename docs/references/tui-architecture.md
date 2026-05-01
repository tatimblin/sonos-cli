# TUI Architecture

## Design Ethos

Render-only widgets as a component library. Screens own data assembly and layout. Handlers own input and SDK mutations. The SDK is the shared data layer — no intermediate state store or event bus. Each layer has a clear boundary enforced by what it imports.

## Three-Layer Architecture

### Widgets (`src/tui/widgets/`)

Stateless rendering components. Each widget takes pre-computed data and a theme, outputs to a ratatui `Frame`. Widgets must not import `hooks`, `App`, `SonosSystem`, or `crossterm::event`.

**Signature pattern:**

```rust
pub fn render(frame: &mut Frame, area: Rect, data: &SpeakerListData, theme: &Theme)
```

**Example:** `speaker_list.rs` takes `SpeakerListData` (entries, volumes, playback state, selection) and renders the grouped speaker list with volume bars and pick-up mode visuals.

**Existing widgets:** `speaker_list`, `volume_bar`, `progress_bar`, `album_art`.

### Screens (`src/tui/screens/`)

Data assembly layer. Screens call hooks to subscribe to SDK properties, transform results into widget data structs, and delegate rendering to widgets. Screens receive `&mut RenderContext` which provides `&App` (read) and `&mut Hooks` (write).

**Signature pattern:**

```rust
pub fn render(frame: &mut Frame, area: Rect, ctx: &mut RenderContext)
```

**Example:** `speakers.rs` subscribes to speaker volumes, group volumes, playback states, and topology changes via hooks, builds `SpeakerListData`, and calls `speaker_list::render()`.

**Hooks calling order:** Within a screen's render function, call hooks in this order:
1. `use_watch` — returns owned values, borrow released immediately
2. `use_animation` — brief `&mut self` borrow, released immediately
3. `use_state` — holds `&mut` for duration, must be last

This satisfies the borrow checker without `RefCell`.

### Handlers (`src/tui/handlers/`)

Key event processing and SDK mutations. Handlers receive `&mut App` and `KeyEvent`, mutate navigation/selection state, and call SDK methods for volume, playback, and regrouping. Handlers must not import `Frame`, `Rect`, or any ratatui rendering types.

**Signature pattern:**

```rust
pub fn handle_key(app: &mut App, key: KeyEvent) -> SpeakerListAction
```

**Example:** `speaker_list.rs` handles Up/Down navigation, Left/Right volume adjust, Space for pick-up/drop, and returns `SpeakerListAction` so the caller can respond (e.g., focus tab bar).

## Shared Modules

### `tui/types.rs`

Cross-layer types that prevent circular imports: `ListEntry`, `PickUpState`, `SpeakerListAction`, `SpeakerListData`, `EntryRenderData`. Also contains list-building functions (`build_list_entries`, `build_display_order`, `group_for_entry`) used by both handlers and screens.

### `tui/helpers.rs`

Shared data transformation functions for screens: `track_summary()` for "title · artist" formatting. Add new helpers here when patterns appear in multiple screens.

### `tui/hooks.rs`

Reactive state system with three primitives: `use_watch` (SDK property subscription), `use_animation` (periodic re-render request), `use_state` (persistent local state). Mark-and-sweep lifecycle evicts unused state between frames. Detailed API is in the code.

### `tui/ui.rs`

Top-level render dispatch: header, separator, screen call, separator, footer. No screen rendering logic — dispatches to `screens/` modules.

## How to Add a New Screen

1. Create `screens/<name>.rs`
2. Add `pub fn render(frame, area, ctx: &mut RenderContext)`
3. Call hooks for SDK subscriptions (watch order: `use_watch` → `use_animation` → `use_state`)
4. Build widget data structs from hook results
5. Call widget render functions with data + theme
6. Add `pub mod <name>;` to `screens/mod.rs`
7. Add dispatch arm in `ui.rs`
8. Add key handler in `handlers/`

## How to Add a New Widget

1. Create `widgets/<name>.rs`
2. Define a data struct for all render inputs
3. Write `pub fn render(frame, area, &Data, &Theme)`
4. Do not import `hooks`, `App`, `SonosSystem`, or `crossterm::event`
5. Add `pub mod <name>;` to `widgets/mod.rs`

## How to Add a New Handler

1. Create or extend a handler in `handlers/`
2. Take `&mut App` + `KeyEvent`
3. Return an action enum if the caller needs to respond
4. Call SDK methods directly for mutations
5. Do not import `Frame`, `Rect`, or rendering types

## Anti-Patterns

- **Hooks in widgets** — breaks render-only. Hooks belong in screens.
- **Rendering in handlers** — breaks separation. Handlers mutate state; the next render cycle picks it up.
- **SDK mutations in screens** — belongs in handlers. Screens are read-only data assembly.
- **Intermediate Action/executor dispatch** — SDK is the shared layer. Call SDK methods directly from handlers.
- **Widget state in App** — use hooks to co-locate state with the screen that needs it (exception: navigation state like `selected_index` which handlers need).

## References

- [TUI separation of concerns plan](../plans/2026-04-30-refactor-tui-v1-separation-of-concerns-plan.md) — full rationale for the three-layer split
- [Hooks architecture brainstorm](../brainstorms/2026-03-29-tui-hooks-architecture-brainstorm.md) — hooks design decisions and calling-order constraint
- [Architecture simplification brainstorm](../brainstorms/2026-03-10-cli-architecture-simplification-brainstorm.md) — "no middleware" principle
