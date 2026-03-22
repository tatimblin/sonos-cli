---
title: "feat: scaffold TUI foundation"
type: feat
status: completed
date: 2026-03-22
origin: docs/brainstorms/2026-02-26-sonos-tui-brainstorm.md
milestone: "Milestone 6: TUI Foundation"
deepened: 2026-03-22
---

## Enhancement Summary

**Deepened on:** 2026-03-22
**Sections enhanced:** 6 phases + acceptance criteria
**Research agents used:** Architecture Strategist, Performance Oracle, Code Simplicity Reviewer, Pattern Recognition Specialist, SpecFlow Analyzer, Best Practices Researcher, Context7 (ratatui + crossterm docs)

### Key Improvements

1. **Use `ratatui::init()` / `ratatui::restore()`** — eliminates ~30 lines of manual terminal setup/teardown/panic hook boilerplate. Available since ratatui 0.28, handles raw mode, alternate screen, AND panic hooks automatically.
2. **Switch from `event::read()` to `event::poll(250ms)`** from the start — blocking read starves the SDK `ChangeIterator::try_iter()` drain, preventing live updates in M7+. 250ms is cheap (4 wakeups/sec idle) and establishes correct loop semantics now.
3. **Collapse `screens/` and `widgets/` into `ui.rs`** as private functions — 7 fewer files for M6 stubs. Extract to subdirectories when content grows in M7+.
4. **Remove `selected_index` / `scroll_offset` from Navigation** — unused by M6 stubs, and the shared-across-screens design causes bugs (lost position on pop, out-of-bounds on tab switch). Add per-screen state in M7 when lists exist.
5. **Trim Theme to 3 fields** (`header`, `legend`, `muted`) with only `dark()` — YAGNI; other themes and fields add visual surface we can't verify without real content.
6. **Add `dirty` flag and handle `Ctrl+C` / `Event::Resize`** — foundational correctness that prevents subtle bugs in later milestones.

### New Considerations Discovered

- `ratatui::init()` returns `DefaultTerminal` type alias — use it instead of `Terminal<CrosstermBackend<io::Stdout>>`
- `Layout::vertical([...]).areas(frame.area())` returns a fixed-size array — cleaner destructuring than `.split()` + indexing
- `? Help` in legend has no help overlay — remove from M6 legend text
- Discovery failure in TUI mode should show platform diagnostics (matching CLI's pattern in `main.rs:42-44`)
- `Screen::label()` couples `app.rs` types to SDK — move to `ui.rs` where it's used

---

# feat: scaffold TUI foundation

## Overview

Scaffold the TUI core: module structure, terminal lifecycle, event loop, navigation system, theme engine, breadcrumb header, and context-sensitive key legend. This is the foundation that every future TUI milestone (7–10) builds on.

When complete, `sonos` (no args) launches a full-screen TUI that renders a blank frame with a breadcrumb header and key legend, navigates between screen stubs with arrow keys and Enter/Esc, and exits cleanly. Terminal restores on panic.

**Roadmap reference:** Implements [Milestone 6: TUI Foundation](../product/roadmap.md#milestone-6-tui-foundation) (see brainstorm: `docs/brainstorms/2026-02-26-sonos-tui-brainstorm.md`).

## Problem Statement

All CLI milestones (1–5) are complete. The TUI entry point in `main.rs:27` prints "TUI not yet implemented" and exits. There is no `src/tui/` directory. The four TUI dependencies (`ratatui`, `crossterm`, `ratatui-image`, `image`) are already in `Cargo.toml` but unused.

The goal is to build the structural core so that Milestones 7–10 can focus purely on screen content — they should never need to touch the event loop, navigation system, terminal setup, or theme engine.

## Proposed Solution

Create a `src/tui/` module following the same structural patterns as `src/cli/` (separation of concerns: types in one file, logic in another, helpers split out). The TUI is a stateless wrapper over `SonosSystem` — it stores only UI state (navigation stack, quit flag) and queries the SDK on-demand during rendering.

### Research Insights

**Best Practices (ratatui community patterns):**
- The TEA (Elm Architecture) pattern — single App struct, pure render functions, centralized event handling — is the established community pattern for TUIs of this complexity. Our plan follows this correctly.
- Function-based render dispatch (match on Screen enum in `ui.rs`) is preferred over trait-based (`Box<dyn ScreenRenderer>`) — simpler, compiler-checked exhaustiveness, no lifetime headaches.
- `Widget` trait consumes `self` by design (widgets are constructed inline, used once per frame). For M6 stubs, render functions are simpler than implementing the trait.

**Performance Considerations:**
- ratatui's double buffering diffs automatically — no need to track dirty regions for rendering. But a `dirty` flag still saves the layout computation + diff work when nothing changed during a poll timeout.
- Layout results are cached by ratatui (default `layout-cache` feature). Ensure we don't disable default features in `Cargo.toml`.

---

## Developer Guide: Where Things Go

This section is the authoritative reference for where to place new TUI code. Consult this when adding screens, widgets, or features in Milestones 7–10.

### Module Map

```
src/
  main.rs              ← arg parse; no args → tui::run(config)
  lib.rs               ← add `pub mod tui;` for library target
  config.rs            ← unchanged (already has `theme` field)
  errors.rs            ← unchanged (CliError stays CLI-focused)
  cli/                 ← unchanged
  tui/
    mod.rs             ← pub fn run(config) → Result<()>; re-exports
    app.rs             ← App, Navigation, Screen, HomeTab, GroupTab enums
    event.rs           ← event loop, handle_key(), handle_change_event()
    theme.rs           ← Theme struct, built-in themes, from_name()
    ui.rs              ← top-level render() dispatch + screen stubs + widgets
```

**Note:** Screens and widgets start as private functions in `ui.rs` for M6. When a screen or widget grows beyond ~80 lines (expected in M7), extract it to `tui/screens/<name>.rs` or `tui/widgets/<name>.rs` and add the subdirectory's `mod.rs`. This keeps M6 minimal while establishing the correct extraction path.

### Decision Matrix: Where Does New Code Go?

| I need to...                              | Put it in...           | Why                                      |
|-------------------------------------------|------------------------|------------------------------------------|
| Add a new screen                          | `tui/ui.rs` (then extract to `tui/screens/<name>.rs` when >80 lines) | One render function per screen |
| Add a reusable UI component               | `tui/ui.rs` (then extract to `tui/widgets/<name>.rs` when >80 lines) | Widgets are standalone ratatui renderers  |
| Add new App state fields                  | `tui/app.rs`           | All UI state lives in App/Navigation      |
| Add a new key binding                     | `tui/event.rs`         | All key dispatch lives in handle_key()    |
| Add a new theme                           | `tui/theme.rs`         | All themes defined in one file            |
| React to an SDK change event              | `tui/event.rs`         | handle_change_event() drains SDK events   |
| Add formatting helpers (icons, time, etc) | `src/cli/format.rs`    | Shared between CLI and TUI via `crate::cli::*` |
| Change how screens are composed/laid out  | `tui/ui.rs`            | Top-level render dispatch only            |

### Key Rules

1. **Screens are pure render functions.** A screen function takes `(frame: &mut Frame, area: Rect, app: &App)` and draws widgets. It does NOT handle input or mutate state.

2. **Event handling is centralized.** All keyboard input flows through `handle_key()` in `event.rs`. Screen-specific keys are dispatched based on `app.navigation.current()`. This keeps input handling in one place.

3. **SDK calls happen in event handlers, never during rendering.** When a keypress triggers an action (play, volume change), the handler calls the SDK directly (e.g., `speaker.play()`). Render functions may read cached SDK data via `get()` (which reads from the SDK's local cache) but must never call `fetch()` (which makes a network request).

4. **App owns all mutable UI state.** Screens read from `&App`, event handlers write to `&mut App`. No other mutable state exists.

5. **Widgets are stateless ratatui components.** A widget takes data as constructor args and implements `Widget` or is a render function. It does not hold references to `App`.

---

## Technical Approach

### Phase 1: Module Structure & Core Types

Create the file skeleton and define all types that the rest of the TUI depends on.

#### `src/tui/mod.rs`

Public entry point and re-exports:

```rust
// src/tui/mod.rs
mod app;
mod event;
mod theme;
mod ui;

pub use app::App;

use crate::config::Config;
use anyhow::Result;

/// Launch the interactive TUI. Blocks until the user quits.
pub fn run(config: Config) -> Result<()> {
    let theme = theme::Theme::from_name(&config.theme);
    let app = App::new(config, theme)?;
    event::run_event_loop(app)
}
```

#### Research Insights — Module Exports

**Simplicity:** Use `pub use app::App` (named export) instead of `pub use app::*` (glob re-export). No external consumer needs `Navigation`, `Screen`, or tab enums — they're internal to the TUI. The glob export was flagged by the Pattern Recognition Specialist as unnecessarily broad.

#### `src/tui/app.rs`

All UI state. The TUI is a stateless wrapper over `SonosSystem` — `App` holds the system handle plus purely UI concerns.

```rust
// src/tui/app.rs
use crate::config::Config;
use crate::tui::theme::Theme;
use sonos_sdk::{GroupId, SonosSystem, SpeakerId};

pub struct App {
    pub system: SonosSystem,
    pub navigation: Navigation,
    pub should_quit: bool,
    pub dirty: bool,
    pub config: Config,
    pub theme: Theme,
}

pub struct Navigation {
    pub stack: Vec<Screen>,
}

#[derive(Clone, Debug)]
pub enum Screen {
    Home { tab: HomeTab },
    GroupView { group_id: GroupId, tab: GroupTab },
    SpeakerDetail { speaker_id: SpeakerId },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum HomeTab {
    #[default]
    Groups,
    Speakers,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum GroupTab {
    #[default]
    NowPlaying,
    Speakers,
    Queue,
}
```

#### Research Insights — App State

**Simplicity (YAGNI):** `selected_index` and `scroll_offset` are removed from `Navigation`. They are unused by M6 screen stubs, and the shared-across-screens design is buggy — pushing a new screen resets the position, but popping back also resets it (losing the user's position). In M7, add selection state as fields on the `Screen` enum variants that need it (e.g., `Home { tab: HomeTab, selected: usize }`). This gives each screen independent cursor state.

**Architecture:** `dirty: bool` flag added to `App`. Set to `true` on any state mutation (key press, SDK event, resize). The event loop only calls `terminal.draw()` when dirty. Prevents unnecessary layout computation + diff on poll timeouts with no events.

Navigation methods:

```rust
impl Navigation {
    pub fn new() -> Self {
        Self {
            stack: vec![Screen::Home { tab: HomeTab::default() }],
        }
    }

    pub fn current(&self) -> &Screen {
        self.stack.last().expect("navigation stack is never empty")
    }

    pub fn current_mut(&mut self) -> &mut Screen {
        self.stack.last_mut().expect("navigation stack is never empty")
    }

    pub fn push(&mut self, screen: Screen) {
        self.stack.push(screen);
    }

    /// Returns true if a screen was popped. Returns false if at root.
    pub fn pop(&mut self) -> bool {
        if self.stack.len() > 1 {
            self.stack.pop();
            true
        } else {
            false
        }
    }

    pub fn at_root(&self) -> bool {
        self.stack.len() == 1
    }
}
```

`App` construction:

```rust
impl App {
    pub fn new(config: Config, theme: Theme) -> anyhow::Result<Self> {
        let system = SonosSystem::new()?;
        Ok(Self {
            system,
            navigation: Navigation::new(),
            should_quit: false,
            dirty: true, // first frame always renders
            config,
            theme,
        })
    }
}
```

**Note:** `SonosSystem::new()` is blocking (cache load + possible 3s SSDP). For Milestone 6 this is acceptable. Milestone 9 adds the animated discovery screen that runs discovery in a background thread.

#### Tasks

- [x] Create `src/tui/mod.rs` with `pub fn run(config: Config) -> Result<()>` — `src/tui/mod.rs`
- [x] Create `src/tui/app.rs` with `App`, `Navigation`, `Screen`, `HomeTab`, `GroupTab` — `src/tui/app.rs`
- [x] Navigation methods: `new()`, `current()`, `current_mut()`, `push()`, `pop()`, `at_root()` — `src/tui/app.rs`

---

### Phase 2: Terminal Lifecycle (ratatui::init / restore)

#### Research Insights — ratatui 0.28+ convenience API

**Critical simplification:** ratatui 0.28+ provides `ratatui::init()` and `ratatui::restore()` that handle ALL of the following automatically:
- Enable/disable raw mode
- Enter/leave alternate screen
- Install panic hook that restores terminal before printing panic message
- Return `DefaultTerminal` type alias (no manual `Terminal<CrosstermBackend<io::Stdout>>`)

This eliminates the need for manual `setup_terminal()`, `restore_terminal()`, and `install_panic_hook()` functions (~30 lines of boilerplate). The panic hook installed by `ratatui::init()` chains with any existing hook, so it handles the ordering correctly (restore terminal FIRST, then print panic).

```rust
// Before (plan original — 30 lines):
fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> { ... }
fn restore_terminal(terminal: &mut Terminal<...>) { ... }
fn install_panic_hook() { ... }

// After (using ratatui::init — 2 lines):
let mut terminal = ratatui::init();
// ... event loop ...
ratatui::restore();
```

**Edge cases handled automatically:**
- Panic during event loop → panic hook fires, restores terminal, then prints panic
- Normal exit → `ratatui::restore()` cleans up
- `Event::Resize` → handled by ratatui's backend automatically (no manual handler needed for terminal size, but we should set `dirty = true` to trigger a redraw)

#### Tasks

- [x] Use `ratatui::init()` in `run_event_loop()` — returns `DefaultTerminal` — `src/tui/event.rs`
- [x] Use `ratatui::restore()` after event loop exits — `src/tui/event.rs`
- [x] No manual `setup_terminal()`, `restore_terminal()`, or `install_panic_hook()` needed

---

### Phase 3: Event Loop

The main loop coordinates two event sources: keyboard input and SDK change events. Uses `event::poll(250ms)` to avoid starving the SDK event drain.

```rust
// src/tui/event.rs
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

pub fn run_event_loop(mut app: App) -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    let change_iter = app.system.iter();

    loop {
        // 1. Render (only when state changed)
        if app.dirty {
            terminal.draw(|frame| crate::tui::ui::render(frame, &app))?;
            app.dirty = false;
        }

        // 2. Poll for keyboard input (non-blocking, 250ms timeout)
        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key) => {
                    handle_key(&mut app, key);
                    app.dirty = true;
                }
                Event::Resize(_, _) => {
                    app.dirty = true;
                }
                _ => {}
            }
        }

        // 3. Drain all pending SDK events (non-blocking)
        for _event in change_iter.try_iter() {
            // Milestone 7+ will handle change events here
            app.dirty = true;
        }

        if app.should_quit {
            break;
        }
    }

    ratatui::restore();
    Ok(())
}
```

#### Research Insights — Why `poll(250ms)` Instead of Blocking `read()`

**Performance Oracle (CRITICAL):** Blocking `event::read()` starves the SDK event drain. The loop structure is render → **block forever on keyboard** → drain SDK events. If the user doesn't press a key, SDK events accumulate without being processed. When M7 adds property watching, this means playback state updates (track changes, volume changes from the Sonos app) would be invisible until the user presses a key.

Switching to `poll(250ms)` means:
- 4 wakeups/sec when idle — negligible CPU cost
- SDK events drain every 250ms even without keyboard input
- The `dirty` flag prevents unnecessary renders on timeout with no events
- No code change needed when M7 adds animations — just shorten the timeout to 50ms

**Edge Cases:**
- `Event::Resize` sets `dirty = true` to trigger a redraw with the new terminal dimensions
- `Ctrl+C` handled as universal quit (raw mode intercepts the default SIGINT behavior)

#### Key handling

Centralized in `handle_key()`. Global keys are processed first, then screen-specific keys:

```rust
fn handle_key(app: &mut App, key: KeyEvent) {
    // Ignore key releases — only handle presses
    if key.kind != event::KeyEventKind::Press {
        return;
    }

    // Global keys (work on every screen)
    match key.code {
        KeyCode::Char('q') if app.navigation.at_root() => {
            app.should_quit = true;
            return;
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
            return;
        }
        KeyCode::Esc => {
            if !app.navigation.pop() {
                // At root — Esc quits
                app.should_quit = true;
            }
            return;
        }
        _ => {}
    }

    // Screen-specific keys
    match app.navigation.current().clone() {
        Screen::Home { tab } => handle_home_key(app, key, &tab),
        Screen::GroupView { group_id, tab } => handle_group_key(app, key, &group_id, &tab),
        Screen::SpeakerDetail { .. } => handle_speaker_key(app, key),
    }
}
```

#### Research Insights — Key Handling

**SpecFlow Analyzer:** `Ctrl+C` must be handled explicitly — raw mode intercepts the default SIGINT. Without this, `Ctrl+C` does nothing and users think the app is frozen.

**Architecture Strategist:** `.clone()` on the `Screen` enum in `handle_key()` releases the borrow on `app.navigation` so handlers can take `&mut App`. The clone is cheap (~50ns, just an enum with small IDs).

**Best Practices:** Always filter `key.kind != KeyEventKind::Press` — crossterm on some platforms generates Press, Release, and Repeat events. Without this filter, every key press fires twice.

Screen-specific handlers for Milestone 6 (stubs with tab switching + navigation):

```rust
fn handle_home_key(app: &mut App, key: KeyEvent, tab: &HomeTab) {
    match key.code {
        KeyCode::Left | KeyCode::Right => {
            let new_tab = match tab {
                HomeTab::Groups => HomeTab::Speakers,
                HomeTab::Speakers => HomeTab::Groups,
            };
            *app.navigation.current_mut() = Screen::Home { tab: new_tab };
        }
        KeyCode::Enter => {
            // Milestone 7+: drill into selected group/speaker
        }
        _ => {}
    }
}

fn handle_group_key(app: &mut App, key: KeyEvent, group_id: &GroupId, tab: &GroupTab) {
    match key.code {
        KeyCode::Left => {
            let new_tab = match tab {
                GroupTab::NowPlaying => GroupTab::Queue,
                GroupTab::Speakers => GroupTab::NowPlaying,
                GroupTab::Queue => GroupTab::Speakers,
            };
            *app.navigation.current_mut() = Screen::GroupView {
                group_id: group_id.clone(),
                tab: new_tab,
            };
        }
        KeyCode::Right => {
            let new_tab = match tab {
                GroupTab::NowPlaying => GroupTab::Speakers,
                GroupTab::Speakers => GroupTab::Queue,
                GroupTab::Queue => GroupTab::NowPlaying,
            };
            *app.navigation.current_mut() = Screen::GroupView {
                group_id: group_id.clone(),
                tab: new_tab,
            };
        }
        _ => {}
    }
}

fn handle_speaker_key(_app: &mut App, _key: KeyEvent) {
    // Milestone 8+: speaker detail key handling
}
```

#### Research Insights — Tab Wrapping

**SpecFlow Analyzer:** GroupView has 3 tabs but the original plan didn't specify wrapping behavior. The implementation above wraps: `Left` on NowPlaying goes to Queue, `Right` on Queue goes to NowPlaying. This is consistent with standard tab bar behavior.

#### Tasks

- [x] `run_event_loop(app)` — main loop with poll(250ms) → render-when-dirty → keyboard → SDK drain cycle — `src/tui/event.rs`
- [x] `handle_key()` — global key dispatch (q, Ctrl+C, Esc) then screen-specific routing — `src/tui/event.rs`
- [x] `handle_home_key()` — tab switching (←→) — `src/tui/event.rs`
- [x] `handle_group_key()` — tab switching with wrapping between NowPlaying/Speakers/Queue — `src/tui/event.rs`
- [x] `handle_speaker_key()` — navigation stub — `src/tui/event.rs`

---

### Phase 4: Theme System

Themes define the colors used by the TUI. Every widget references `app.theme` — no hardcoded colors anywhere.

#### `src/tui/theme.rs`

```rust
use ratatui::style::{Color, Modifier, Style};

#[derive(Clone, Debug)]
pub struct Theme {
    pub header: Style,
    pub legend: Style,
    pub muted: Style,
}

impl Theme {
    pub fn from_name(name: &str) -> Self {
        match name {
            // Milestone 7+: add "light", "neon", "sonos" when visual surface exists
            _ => Self::dark(),
        }
    }

    pub fn dark() -> Self {
        Self {
            header: Style::new()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            legend: Style::new()
                .fg(Color::DarkGray),
            muted: Style::new()
                .fg(Color::DarkGray),
        }
    }
}
```

#### Research Insights — Theme Scope

**Simplicity Reviewer (YAGNI):** The original plan defined 14 fields (`bg`, `fg`, `accent`, `highlight`, `muted`, `error`, `header`, `legend`, `border`, `border_focus`, `progress`, `progress_bg`, `gauge`) and 4 built-in themes. M6 screen stubs only use 3 styles: `header` (breadcrumb bar), `legend` (key legend bar), and `muted` (placeholder text). The other 11 fields and 3 extra themes have no visual surface to verify in M6.

**Plan:** Start with 3 fields and `dark()` only. `from_name()` accepts any string and falls back to dark — the match arms for `"light"`, `"neon"`, `"sonos"` are added in M7 when real content exists to test colors against. The `Theme` struct grows as screens need new semantic styles.

**Best Practices:** Pre-compute `Style` values in the constructor (not per-frame in render functions). Our approach is correct — `Style::new().fg(...).add_modifier(...)` is called once in `dark()`, not on every render.

#### Tasks

- [x] Create `src/tui/theme.rs` with `Theme` struct (3 fields) and `from_name()` → `dark()` — `src/tui/theme.rs`

---

### Phase 5: Render Pipeline & Chrome

#### `src/tui/ui.rs` — Render dispatch, screen stubs, and widgets

All rendering lives in one file for M6. Includes the top-level layout dispatch, screen stub functions, and breadcrumb/legend widgets.

```rust
// src/tui/ui.rs
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::tui::app::{App, GroupTab, HomeTab, Screen};

pub fn render(frame: &mut Frame, app: &App) {
    let [header_area, content_area, legend_area] = Layout::vertical([
        Constraint::Length(1), // breadcrumb header
        Constraint::Min(0),   // content area
        Constraint::Length(1), // key legend
    ])
    .areas(frame.area());

    render_breadcrumb(frame, header_area, app);

    match app.navigation.current() {
        Screen::Home { tab } => render_home(frame, content_area, app, tab),
        Screen::GroupView { group_id, tab } => {
            render_group_view(frame, content_area, app, group_id, tab)
        }
        Screen::SpeakerDetail { speaker_id } => {
            render_speaker_detail(frame, content_area, app, speaker_id)
        }
    }

    render_key_legend(frame, legend_area, app);
}
```

#### Research Insights — Layout API

**Context7 (ratatui docs):** `Layout::vertical([...]).areas(frame.area())` returns a fixed-size array `[Rect; N]` — destructure directly with `let [a, b, c] = ...` instead of `.split()` + `chunks[0]`, `chunks[1]`, `chunks[2]`. Cleaner and compiler-checked.

**Pattern Specialist:** `Screen::label()` was defined on the `Screen` enum in `app.rs` in the original plan, coupling type definitions to SDK types. Move the label logic to `ui.rs` as a private function — it's only used for breadcrumb rendering.

#### Breadcrumb header

```rust
fn screen_label(screen: &Screen, system: &sonos_sdk::SonosSystem) -> String {
    match screen {
        Screen::Home { .. } => "SONOS".to_string(),
        Screen::GroupView { group_id, .. } => {
            system.group_by_id(group_id)
                .and_then(|g| g.coordinator())
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "Group".to_string())
        }
        Screen::SpeakerDetail { speaker_id } => {
            system.speaker_by_id(speaker_id)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "Speaker".to_string())
        }
    }
}

fn render_breadcrumb(frame: &mut Frame, area: Rect, app: &App) {
    let labels: Vec<String> = app.navigation.stack
        .iter()
        .map(|screen| screen_label(screen, &app.system))
        .collect();
    let breadcrumb = labels.join(" > ");

    let mut spans = vec![Span::styled(breadcrumb, app.theme.header)];

    // Tab indicators (right-aligned)
    if let Some(tab_text) = current_tab_text(app.navigation.current()) {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(tab_text, app.theme.muted));
    }

    let paragraph = Paragraph::new(Line::from(spans))
        .style(app.theme.header);
    frame.render_widget(paragraph, area);
}

fn current_tab_text(screen: &Screen) -> Option<String> {
    match screen {
        Screen::Home { tab } => {
            let groups = if *tab == HomeTab::Groups { "[Groups]" } else { " Groups " };
            let speakers = if *tab == HomeTab::Speakers { "[Speakers]" } else { " Speakers " };
            Some(format!("{groups} {speakers}"))
        }
        Screen::GroupView { tab, .. } => {
            let np = if *tab == GroupTab::NowPlaying { "[NowPlaying]" } else { " NowPlaying " };
            let sp = if *tab == GroupTab::Speakers { "[Speakers]" } else { " Speakers " };
            let q = if *tab == GroupTab::Queue { "[Queue]" } else { " Queue " };
            Some(format!("{np} {sp} {q}"))
        }
        Screen::SpeakerDetail { .. } => None,
    }
}
```

#### Key legend

Context-sensitive bottom bar. Returns different text based on `app.navigation.current()` (see brainstorm: `docs/brainstorms/2026-02-26-sonos-tui-brainstorm.md`, key legend table):

```rust
fn render_key_legend(frame: &mut Frame, area: Rect, app: &App) {
    let text = match app.navigation.current() {
        Screen::Home { tab: HomeTab::Groups } =>
            "<-> Tabs  Enter Open group  q Quit",
        Screen::Home { tab: HomeTab::Speakers } =>
            "<-> Tabs  Enter Open speaker  q Quit",
        Screen::GroupView { tab: GroupTab::NowPlaying, .. } =>
            "<-> Tabs  Esc Back",
        Screen::GroupView { tab: GroupTab::Speakers, .. } =>
            "<-> Tabs  Enter Open speaker  Esc Back",
        Screen::GroupView { tab: GroupTab::Queue, .. } =>
            "<-> Tabs  Esc Back",
        Screen::SpeakerDetail { .. } =>
            "Esc Back",
    };

    let paragraph = Paragraph::new(text)
        .style(app.theme.legend);
    frame.render_widget(paragraph, area);
}
```

#### Research Insights — Key Legend

**SpecFlow Analyzer:** The original plan included `? Help` in legend text, but there's no help overlay in M6. Removed to avoid showing non-functional hints.

**Simplicity:** M6 legend shows only the keys that actually work in M6 stubs. `^v Select`, `Space Pause`, etc. are removed since there are no lists or playback controls yet. M7 adds them back when the keys gain function.

#### Screen stubs

Each screen draws a centered placeholder. This proves the navigation system works.

```rust
fn render_home(frame: &mut Frame, area: Rect, app: &App, tab: &HomeTab) {
    let text = match tab {
        HomeTab::Groups => "Groups — Milestone 7",
        HomeTab::Speakers => "Speakers — Milestone 7",
    };
    let paragraph = Paragraph::new(text)
        .alignment(Alignment::Center)
        .style(app.theme.muted);
    frame.render_widget(paragraph, area);
}

fn render_group_view(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    _group_id: &sonos_sdk::GroupId,
    tab: &GroupTab,
) {
    let text = match tab {
        GroupTab::NowPlaying => "Now Playing — Milestone 7",
        GroupTab::Speakers => "Group Speakers — Milestone 7",
        GroupTab::Queue => "Queue — Milestone 7",
    };
    let paragraph = Paragraph::new(text)
        .alignment(Alignment::Center)
        .style(app.theme.muted);
    frame.render_widget(paragraph, area);
}

fn render_speaker_detail(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    _speaker_id: &sonos_sdk::SpeakerId,
) {
    let paragraph = Paragraph::new("Speaker Detail — Milestone 8")
        .alignment(Alignment::Center)
        .style(app.theme.muted);
    frame.render_widget(paragraph, area);
}
```

#### Tasks

- [x] Create `src/tui/ui.rs` — `render()` layout split, screen dispatch, breadcrumb, legend, screen stubs — `src/tui/ui.rs`

---

### Phase 6: Integration

Wire the TUI into the existing binary and library targets.

#### `src/main.rs` changes

Replace the "TUI not yet implemented" placeholder:

```rust
// Before (lines 26-28):
eprintln!("TUI not yet implemented");
ExitCode::from(1)

// After:
match tui::run(config) {
    Ok(()) => ExitCode::SUCCESS,
    Err(e) => {
        eprintln!("error: {e}");
        if e.to_string().contains("DiscoveryFailed")
            || e.to_string().contains("discovery")
        {
            eprintln!("{}", diagnostics::discovery_hint());
        }
        ExitCode::from(1)
    }
}
```

Add `mod tui;` to the module declarations (after `mod errors;`).

#### Research Insights — Integration

**SpecFlow Analyzer:** The CLI path (lines 42-44) shows platform diagnostics on discovery failure. The TUI error path must do the same — when `SonosSystem::new()` fails (inside `App::new()`), the error propagates up to `main.rs` where it should show the same `discovery_hint()` message. The code above matches this pattern.

**Architecture Strategist:** The `is_terminal()` check on line 26 must be preserved. The `None` arm structure stays the same — only the `is_terminal()` branch body changes.

#### `src/lib.rs` changes

Add `pub mod tui;` so the library target exports the TUI module (consistent with `pub mod cli;`).

#### Tasks

- [x] Add `mod tui;` to `src/main.rs` — `src/main.rs:15`
- [x] Replace TUI placeholder in `main.rs` with `tui::run(config)` call + discovery diagnostics — `src/main.rs:26-28`
- [x] Add `pub mod tui;` to `src/lib.rs` — `src/lib.rs`

---

## File Inventory

Every file created or modified by this plan:

| File | Action | Purpose |
|------|--------|---------|
| `src/main.rs` | Modify | Add `mod tui`, replace placeholder with `tui::run(config)` + discovery diagnostics |
| `src/lib.rs` | Modify | Add `pub mod tui` |
| `src/tui/mod.rs` | Create | Entry point `run()`, named re-export of `App` |
| `src/tui/app.rs` | Create | `App`, `Navigation`, `Screen`, tab enums |
| `src/tui/event.rs` | Create | Event loop (poll-based), key handling, `Ctrl+C` + resize |
| `src/tui/theme.rs` | Create | `Theme` struct (3 fields), `dark()` theme |
| `src/tui/ui.rs` | Create | Render dispatch, breadcrumb, key legend, screen stubs (all in one file) |

**7 files total** (5 new, 2 modified). Down from 14 in the original plan.

**No changes to:** `Cargo.toml` (dependencies already present), `config.rs`, `errors.rs`, `cli/` (all unchanged).

## Acceptance Criteria

### Functional Requirements

- [ ] `sonos` (no args, in a terminal) launches the TUI in full-screen alternate screen mode
- [ ] Breadcrumb header shows `SONOS` on the home screen, with tab indicators
- [ ] Key legend bar shows context-appropriate hints that change per screen/tab
- [ ] `←→` switches between Home tabs (Groups / Speakers) — breadcrumb and legend update
- [ ] `←→` in GroupView switches between NowPlaying / Speakers / Queue tabs with wrapping
- [ ] `Esc` pops the navigation stack (back one level); at root, `Esc` quits
- [ ] `q` quits from the root screen
- [ ] `Ctrl+C` quits from any screen
- [ ] Terminal restores cleanly on normal exit (raw mode off, alternate screen left, cursor shown)
- [ ] Terminal restores on panic (ratatui::init panic hook)
- [ ] Terminal redraws on resize
- [ ] Theme loads from `config.theme` field; defaults to `"dark"` when not configured
- [ ] Discovery failure shows platform diagnostics (matching CLI error path)

### Non-Functional Requirements

- [ ] ~0% CPU when idle (poll-based with 250ms timeout + dirty flag)
- [ ] No `unwrap()` or `expect()` in the main code path (except the nav stack invariant)
- [ ] All TUI code compiles with `cargo clippy -- -D warnings`
- [ ] `cargo fmt --check` passes

### Quality Gates

- [ ] Unit tests for `Navigation` methods (push, pop, at_root, current)
- [ ] Unit tests for `Theme::from_name()` (unknown → dark fallback)
- [ ] `cargo test` passes
- [ ] Manual verification: launch TUI, navigate all screens, verify breadcrumb + legend update, quit cleanly

## Implementation Notes

### Why `poll(250ms)` instead of blocking `read()`

The SDK's `ChangeIterator::try_iter()` drains pending events non-blocking, but the drain only runs after the keyboard input step. With blocking `event::read()`, the drain never runs unless the user presses a key. This means SDK events (playback changes, volume changes from the Sonos app) accumulate silently.

Using `poll(250ms)` means the loop completes every 250ms even without keyboard input, draining SDK events and checking for quit. The `dirty` flag ensures we don't waste CPU rendering unchanged frames. When M7 adds progress bar animation, change the timeout to `50ms` — one line edit.

### Why `SonosSystem::new()` blocks at startup

For Milestone 6, the TUI calls `SonosSystem::new()` synchronously before entering the event loop. This means a 0–3 second pause on startup. This is acceptable for the foundation milestone.

**Milestone 9 adds the discovery screen** — it will move `SonosSystem::new()` into a background thread, show the animated discovery screen, and transition to Home when ready. The `App::new()` signature will change to accept an `Option<SonosSystem>` or split into two phases.

### Shared formatting helpers

`playback_icon()`, `playback_label()`, and `format_time_ms()` in `src/cli/format.rs` are useful for TUI rendering too. They're accessible via `crate::cli::playback_icon()`. No need to move or duplicate them — the `cli` module is always compiled (it's part of the binary).

### What this plan does NOT include

These are explicitly deferred to later milestones:

| Deferred to | Feature |
|-------------|---------|
| Milestone 7 | Live data rendering, property watching, progress bar animation |
| Milestone 7 | Mini-player widget, `selected_index` per-screen state |
| Milestone 7 | Additional themes (light, neon, sonos), additional theme fields |
| Milestone 7 | `^v` list navigation (no lists in M6 stubs) |
| Milestone 7 | `Enter` drill-in to real groups/speakers (requires live data) |
| Milestone 8 | Album art rendering, per-speaker EQ widgets |
| Milestone 9 | Discovery screen, background discovery |
| Milestone 9 | Speaker detail with real device info |
| Future | `error_message: Option<String>` on App (for in-TUI error display) |
| Future | `FocusMode` enum for dual-purpose arrow keys (tab switch vs slider adjust) |
| Future | Help overlay (`?` key) |

## Sources & References

### Origin

- **Brainstorm document:** [docs/brainstorms/2026-02-26-sonos-tui-brainstorm.md](../brainstorms/2026-02-26-sonos-tui-brainstorm.md) — Key decisions carried forward: groups-first navigation, two-level tabbed views (Home + Group), keyboard-only input model, context-sensitive key legend, 4 built-in themes.
- **Architecture simplification:** [docs/brainstorms/2026-03-10-cli-architecture-simplification-brainstorm.md](../brainstorms/2026-03-10-cli-architecture-simplification-brainstorm.md) — Direct SDK calls from TUI handlers, no intermediate dispatch layer.

### Internal References

- Roadmap Milestone 6: `docs/product/roadmap.md` (lines 370–486)
- CLI module pattern to follow: `src/cli/mod.rs` (mod.rs with re-exports, split files by concern)
- Config with theme field: `src/config.rs:13` (`pub theme: String`)
- Main.rs TUI entry point: `src/main.rs:26-28` (placeholder to replace)
- SDK API reference: `docs/references/sonos-sdk.md` (SonosSystem, Speaker, Group, ChangeIterator, property handles)
- Shared format helpers: `src/cli/format.rs` (playback_icon, format_time_ms)

### External References

- ratatui 0.29 documentation (via Context7): `ratatui::init()`, `ratatui::restore()`, `DefaultTerminal`, `Layout::vertical().areas()`
- crossterm 0.28 documentation: `event::poll()`, `event::read()`, `KeyEventKind::Press` filtering
- ratatui community patterns: TEA architecture, function-based render dispatch, panic hook best practices

### Research Agents

- **Architecture Strategist:** Module alignment, per-screen state design, SDK-call-during-render rules, error handling gaps
- **Performance Oracle:** poll vs read analysis, dirty flag pattern, resize handling, selected_index bounds
- **Code Simplicity Reviewer:** File count reduction (14→7), Theme field trimming (14→3), selected_index removal, developer guide deferral
- **Pattern Recognition Specialist:** Export scope (`pub use app::*` → `pub use app::App`), Screen::label coupling, anyhow vs CliError strategy
- **SpecFlow Analyzer:** Ctrl+C handling, resize events, discovery diagnostics parity, tab wrapping, legend accuracy
- **Best Practices Researcher:** TEA pattern validation, Widget trait patterns, per-screen selection state, `Block::inner()` pattern
