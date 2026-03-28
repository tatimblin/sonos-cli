---
date: 2026-03-01
status: active
version: v1
deepened: 2026-03-01
---

# v1 Roadmap — Complete Control

**Theme: Full SDK coverage. Every Sonos capability accessible from the terminal.**

The bar for v1 is completeness: if the Sonos SDK supports it, sonos-cli supports it. No arbitrary gaps. A user should never need to open the Sonos app to do something the SDK makes possible.

## Enhancement Summary

**Deepened on:** 2026-03-01
**Purpose:** Transform PM-level milestones into actionable engineering tasks with concrete types, SDK method mappings, and implementation patterns.

### Key Improvements
1. Every task now references the exact SDK methods, types, and property handles it will use
2. `Action` enum variants are specified per-milestone with concrete signatures
3. TUI tasks include specific ratatui widget patterns, event loop details, and watch subscriptions
4. Clap v4 derive patterns specified for the CLI layer
5. Error handling, exit codes, and TTY detection patterns documented

### Research Sources
- SDK API reference: `docs/references/sonos-sdk.md` — 50 public methods mapped to Action variants
- Ratatui patterns: event loop, `Screen` enum navigation, `ratatui-image` for album art
- Clap v4 derive: `#[command(flatten)]` for shared global flags, `Option<Commands>` for TUI default
- Rust CLI best practices: atomic cache writes, `thiserror` + `anyhow`, `ExitCode`, `IsTerminal`

---

## North Star

A new user installs sonos-cli, runs `sonos`, and is controlling every speaker in their home within 5 minutes — without reading any documentation beyond the TUI's own key legend.

---

## Milestone 1: Project Foundation

The scaffolding that everything else builds on. No user-visible features yet, but the architecture must be right before a single command is wired up.

### Cargo.toml & Dependencies

- [x] `Cargo.toml` with all dependencies:
  - `sonos-sdk` (path = `"../sonos-sdk/sonos-sdk"`)
  - `clap` v4 with `derive` feature
  - `ratatui` + `crossterm`
  - `serde` + `serde_json` + `toml` (for config)
  - `dirs` (for `~/.config/sonos/`)
  - `anyhow` (error propagation)
  - `thiserror` (domain error types)
  - `ratatui-image` (album art rendering) *(not yet added)*
  - `image` (image decoding for album art) *(not yet added)*

### Entry Point — `src/main.rs`

- [x] `main()` returns `ExitCode` (not `Result`) for proper exit code control
- [x] Parse `Cli` struct via clap derive — `Option<Commands>` subcommand field
- [x] No args (`None`) → launch TUI (only if `std::io::stdout().is_terminal()`)
- [x] Subcommand present (`Some(cmd)`) → call `cmd.run(&system, &config)`, print result

```rust
// Pattern: main.rs skeleton
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        None => {
            if !std::io::stdout().is_terminal() {
                eprintln!("error: TUI mode requires an interactive terminal.\nUse 'sonos --help' to see available commands.");
                return ExitCode::from(1);
            }
            match tui::run() {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => { eprintln!("error: {e}"); ExitCode::from(1) }
            }
        }
        Some(cmd) => {
            match run_command(cmd, &cli.global) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => e.exit_code(),
            }
        }
    }
}
```

### ~~Action Enum — `src/actions.rs`~~ (Superseded)

> **Superseded by direct SDK calls.** CLI commands call SDK methods directly from
> `Commands::run()` in `cli/mod.rs`. No intermediate Action enum. See
> `docs/plans/2026-03-10-refactor-cli-architecture-simplification-plan.md`.

### ~~Executor — `src/executor.rs`~~ (Superseded)

> **Superseded by direct SDK calls.** Target resolution lives in `cli/mod.rs` as
> `resolve_speaker()`. The SDK is the shared API layer — both CLI and TUI call
> SDK methods directly.

### Error Types — `src/errors.rs`

- [x] `CliError` enum using `thiserror`:
```rust
#[derive(thiserror::Error, Debug)]
pub enum CliError {
    #[error("speaker \"{0}\" not found")]
    SpeakerNotFound(String),
    #[error("group \"{0}\" not found")]
    GroupNotFound(String),
    #[error("{0}")]
    Sdk(#[from] SdkError),
    #[error("{0}")]
    Config(String),
    #[error("{0}")]
    Cache(String),
    #[error("{0}")]
    Validation(String),
}
```
- [x] `CliError::recovery_hint(&self) -> Option<&str>` — returns actionable follow-up text
- [x] `CliError::exit_code(&self) -> ExitCode` — `1` for runtime, `2` for validation/usage

### ~~Cache — `src/cache.rs`~~ (Superseded)

> **Superseded by SDK-level caching.** Cache now lives in `sonos-sdk/src/cache.rs`
> using `~/.cache/sonos/cache.json` (XDG cache_dir). CLI `src/cache.rs` deleted.

- ~~`CachedSystem` struct~~
- ~~`load()`, `save()`, `is_stale()` functions~~
- ~~`dirs::config_dir()` for cache path~~

### Config — `src/config.rs`

- [x] `Config` struct with `#[serde(default)]`:
```rust
#[derive(Deserialize)]
pub struct Config {
    #[serde(default)]
    pub default_group: Option<String>,
    #[serde(default = "default_ttl")]
    pub cache_ttl_hours: u64,  // default: 24
    #[serde(default = "default_theme")]
    pub theme: String,  // default: "dark"
}
```
- [x] `Config::load() -> Config` — reads `~/.config/sonos/config.toml`, falls back to defaults on any error
- [x] Environment variable overrides: `SONOS_DEFAULT_GROUP`, `SONOS_CONFIG_DIR`

**Exit criteria:** `cargo build` succeeds. `Commands::run()` calls SDK directly — no Action enum or executor. Config loads defaults when no file exists.

---

## Milestone 2: CLI — Discovery & System Commands

The first commands a new user will run. Must work reliably before any playback commands.

### Clap Definitions — `src/cli/mod.rs`

- [x] `Cli` struct with `#[command(flatten)] GlobalFlags` and `Option<Commands>`:
```rust
#[derive(Parser)]
#[command(name = "sonos", about = "Control Sonos speakers from the terminal")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
    #[command(flatten)]
    pub global: GlobalFlags,
}

#[derive(Args)]
pub struct GlobalFlags {
    #[arg(long, global = true)]
    pub speaker: Option<String>,
    #[arg(long, global = true)]
    pub group: Option<String>,
    #[arg(long, short, global = true)]
    pub quiet: bool,
    #[arg(long, global = true)]
    pub verbose: bool,
}
```
- [x] `Commands` enum with `Speakers`, `Groups`, `Status` variants (and playback commands)
- [x] `Commands::run(&self, system, config) -> Result<String, CliError>` — calls SDK methods directly

### ~~Discovery Command~~ (Superseded)

> **Superseded by SDK-level caching.** `SonosSystem::new()` now handles discovery and
> caching transparently. There is no `sonos discover` command — the SDK auto-discovers
> on first run and auto-rediscovers on speaker miss. See
> `docs/plans/2026-03-08-feat-sdk-level-discovery-caching-plan.md`.

- ~~`sonos discover` — calls `sonos_discovery::get_with_timeout(Duration::from_secs(3))`~~
- ~~On TTY: show spinner to stderr during 3s scan~~
- ~~Build `SonosSystem::from_discovered_devices(devices)`~~
- ~~Write cache via `cache::save()`~~
- ~~Print discovered speakers: name, model, IP (one per line)~~

### Speakers Command

- [x] `sonos speakers` — loads from cache (or rediscovers if stale)
- [x] For each speaker: fetch `volume`, `playback_state` via property handles
- [x] Print table: name, model, IP, volume, playback state

**SDK methods used:**
- `system.speakers()` — `Vec<Speaker>`
- `speaker.volume.fetch()` — `Volume(u8)`
- `speaker.playback_state.fetch()` — `PlaybackState`

### Groups Command

- [x] `sonos groups` — loads from cache (or rediscovers)
- [x] Watch `group_membership` on at least one speaker to trigger topology subscription
- [x] For each group: coordinator name, member count, current track, volume, state

**SDK methods used:**
- `system.groups()` — `Vec<Group>`
- `group.coordinator()` — `Option<Speaker>`
- `group.members()` — `Vec<Speaker>`
- `group.volume.fetch()` — `GroupVolume(u16)`
- Coordinator's `speaker.current_track.fetch()` — `CurrentTrack`
- Coordinator's `speaker.playback_state.fetch()` — `PlaybackState`

### Status Command

- [x] `sonos status [--group NAME | --speaker NAME]`
- [x] Resolve target (default group if neither flag given)
- [x] Fetch and display: current track (title, artist, album), playback state, position, volume

**SDK methods used:**
- `speaker.current_track.fetch()` — `CurrentTrack { title, artist, album, album_art_uri, uri }`
- `speaker.playback_state.fetch()` — `PlaybackState`
- `speaker.position.fetch()` — `Position { position_ms, duration_ms }`
- `speaker.volume.fetch()` — `Volume(u8)`

### Auto-Rediscovery & Error Handling

- [x] Auto-rediscovery on cache miss: SDK `get_speaker_by_name()` triggers one-shot SSDP rediscovery per session
- [x] Error format: `error: speaker "X" not found.\nCheck that your speakers are on the same network, then retry.`
- [x] Exit code 1 for runtime errors (speaker not found, network failure)

**Exit criteria:** A user with no config file can run `sonos discover` and then `sonos groups` and see their actual Sonos system state.

---

## Milestone 3: CLI — Playback Commands

The commands used dozens of times a day.

### Commands

- [x] `sonos play [--group NAME | --speaker NAME]`
  - SDK: `speaker.play()` → `Result<(), SdkError>`
- [x] `sonos pause [--group NAME | --speaker NAME]`
  - SDK: `speaker.pause()` → `Result<(), SdkError>`
- [x] `sonos stop [--group NAME | --speaker NAME]`
  - SDK: `speaker.stop()` → `Result<(), SdkError>`
- [x] `sonos next [--group NAME | --speaker NAME]`
  - SDK: `speaker.next()` → `Result<(), SdkError>`
- [x] `sonos prev [--group NAME | --speaker NAME]`
  - SDK: `speaker.previous()` → `Result<(), SdkError>`
- [x] `sonos seek <HH:MM:SS> [--group NAME | --speaker NAME]`
  - SDK: `speaker.seek(SeekTarget::Time(position))` → `Result<(), SdkError>`
  - Validate `HH:MM:SS` format before calling SDK
- [x] `sonos mode <normal|repeat|repeat-one|shuffle|shuffle-no-repeat> [--group NAME | --speaker NAME]`
  - SDK: `speaker.set_play_mode(mode)` → `Result<(), SdkError>`
  - Map CLI string to `PlayMode` enum: `normal` → `PlayMode::Normal`, `repeat` → `PlayMode::RepeatAll`, `repeat-one` → `PlayMode::RepeatOne`, `shuffle` → `PlayMode::Shuffle`, `shuffle-no-repeat` → `PlayMode::ShuffleNoRepeat`

### Targeting & Defaults

- [x] All commands resolve target via `resolve_speaker()` in `cli/mod.rs`
- [x] When targeting a group: find coordinator via `group.coordinator()`, call playback method on coordinator speaker
- [x] Default group fallback: `config.default_group` → first discovered group (alphabetical by name)

### Clap Additions

- [x] Add `Play`, `Pause`, `Stop`, `Next`, `Prev` to `Commands` enum
- [x] Add `Seek { position: String }`, `Mode { mode: String }` to `Commands` enum
- [x] `Commands::run()` handles play/pause/stop/next/prev directly
- [x] Add `Seek { position: String }`, `Mode { mode: String }` to `Commands::run()`

**Exit criteria:** All playback commands work against a live Sonos system. `--group` and `--speaker` targeting tested. Default fallback works.

---

## Milestone 4: CLI — Volume, EQ & Grouping

Gives users precise per-speaker and per-group control.

### Volume & EQ Commands

- [x] `sonos volume <0-100> [--group NAME | --speaker NAME]`
  - Group target → `group.set_volume(level as u16)` (via `GroupVolumeHandle`)
  - Speaker target → `speaker.set_volume(level)` (via `VolumeHandle`)
  - Validation: `SdkError::ValidationFailed` if out of 0–100 range
- [x] `sonos mute [--group NAME | --speaker NAME]`
  - Group → `group.set_mute(true)`, Speaker → `speaker.set_mute(true)`
- [x] `sonos unmute [--group NAME | --speaker NAME]`
  - Group → `group.set_mute(false)`, Speaker → `speaker.set_mute(false)`
- [x] `sonos bass <-10..10> --speaker NAME`
  - SDK: `speaker.set_bass(level)` — speaker-only; error if `--group` given
  - Validation: `SdkError::ValidationFailed` for out of −10 to +10 range
- [x] `sonos treble <-10..10> --speaker NAME`
  - SDK: `speaker.set_treble(level)` — speaker-only
- [x] `sonos loudness <on|off> --speaker NAME`
  - SDK: `speaker.set_loudness(enabled)` — map `"on"` → `true`, `"off"` → `false`

### Grouping Commands

- [x] `sonos join --speaker NAME --group NAME`
  - Resolve both speaker and group
  - SDK: `group.add_speaker(&speaker)` → `Result<(), SdkError>`
  - Cannot add coordinator to itself → `SdkError::InvalidOperation`
- [x] `sonos leave --speaker NAME`
  - SDK: `speaker.leave_group()` → `Result<BecomeCoordinatorOfStandaloneGroupResponse, SdkError>`
  - Cannot remove coordinator → `SdkError::InvalidOperation` (use dissolve instead)

### Sleep Timer Commands

- [x] `sonos sleep <DURATION> [--group NAME | --speaker NAME]`
  - Parse duration: `30m` → `"00:30:00"`, `1h` → `"01:00:00"`, `90m` → `"01:30:00"`
  - SDK: `speaker.configure_sleep_timer(hh_mm_ss)`
- [x] `sonos sleep cancel [--group NAME | --speaker NAME]`
  - SDK: `speaker.cancel_sleep_timer()`

### Clap Additions

- [x] Add `Volume`, `Mute`, `Unmute` to `Commands`
- [x] Add `Bass`, `Treble`, `Loudness`, `Join`, `Leave`, `Sleep` to `Commands`
- [x] `Sleep` has a subcommand: positional duration or `cancel` keyword
- [x] Speaker-only commands (`bass`, `treble`, `loudness`) validate that `--group` is not present

**Exit criteria:** All volume, EQ, grouping, and sleep timer operations work. Validation errors surface clearly (e.g., bass value out of range). Speaker-only commands reject `--group` flag.

---

## Milestone 5: CLI — Queue Management

Rounds out the full SDK surface for v1.

### Commands

- [x] `sonos queue [--group NAME | --speaker NAME]`
  - Fetch queue via `speaker.get_media_info()` → `GetMediaInfoResponse { nr_tracks, media_duration, ... }`
  - Print current queue: track number, title, artist, duration
  - Mark currently playing track with `▶`
  - Use coordinator speaker for group targets

- [x] `sonos queue add <URI> [--group NAME | --speaker NAME]`
  - SDK: `speaker.add_uri_to_queue(uri, "", 0, false)` → `AddURIToQueueResponse`
  - Print: `"Added to queue (position {first_track_number_enqueued})"`

- [x] `sonos queue clear [--group NAME | --speaker NAME]`
  - Prompt for confirmation unless `--no-input` flag is set
  - SDK: `speaker.remove_all_tracks_from_queue()`
  - Print: `"Queue cleared"`

### Clap Additions

- [x] `Queue` command with optional subcommand: `Add { uri: String }`, `Clear`
- [x] No subcommand = show queue (default behavior)
- [x] Add `--no-input` to `GlobalFlags`

**Exit criteria:** Queue commands round-trip correctly. `sonos queue` output is readable and scannable.

---

## Milestone 6: TUI Foundation

The TUI event loop, state management, and navigation skeleton — no real data yet, just the shell.

### TUI State Management Architecture

**Core principle:** The TUI is a stateless wrapper over `SonosSystem`. All speaker and group data lives in `SonosSystem` and is queried on-demand during rendering. The TUI only stores UI state (navigation, selection, scroll position).

**State ownership:**
- `SonosSystem` stores all speaker/group data (single source of truth)
- TUI stores only UI concerns: current view, selected index, scroll offset
- Change events trigger re-renders, which query fresh data from `SonosSystem`
- No duplication of speaker/group state in TUI structs

### App State — `src/tui/app.rs`

- [x] `App` struct holding state:
```rust
pub struct App {
    pub system: SonosSystem,      // Single source of truth for speaker/group data
    pub navigation: Navigation,   // UI state only
    pub should_quit: bool,
    pub config: Config,
}

pub struct Navigation {
    pub stack: Vec<Screen>,       // back-stack for Esc navigation
    pub selected_index: usize,    // UI state: which item is selected
    pub scroll_offset: usize,     // UI state: scroll position
}

pub enum Screen {
    Home { tab: HomeTab },
    GroupView { group_id: GroupId, tab: GroupTab },
    SpeakerDetail { speaker_id: SpeakerId },
}

pub enum HomeTab { Groups, Speakers }
pub enum GroupTab { NowPlaying, Speakers, Queue }
```

- [x] `Navigation::push(screen)`, `Navigation::pop() -> Option<Screen>`, `Navigation::current() -> &Screen`

### Terminal Setup & Teardown

- [x] Raw mode + alternate screen setup via `crossterm::terminal::{enable_raw_mode, EnterAlternateScreen}`
- [x] Clean teardown on exit: `disable_raw_mode`, `LeaveAlternateScreen`
- [x] Panic hook: restore terminal before printing panic message
```rust
let original_hook = std::panic::take_hook();
std::panic::set_hook(Box::new(move |panic| {
    let _ = crossterm::terminal::disable_raw_mode();
    let _ = crossterm::execute!(std::io::stdout(), LeaveAlternateScreen);
    original_hook(panic);
}));
```

### Event Loop

- [x] Tick-based loop: `crossterm::event::poll(Duration::from_millis(50))`
- [x] On each tick:
  1. Poll crossterm events (keyboard input)
  2. Drain `system.iter().try_iter()` for all pending `ChangeEvent`s
  3. Update `App` state from change events
  4. Re-render via `terminal.draw(|frame| ui::render(frame, &app))`

```rust
let change_iter = app.system.iter();
loop {
    // 1. Render
    terminal.draw(|frame| ui::render(frame, &app))?;

    // 2. Poll keyboard (50ms timeout)
    if crossterm::event::poll(Duration::from_millis(50))? {
        if let Event::Key(key) = crossterm::event::read()? {
            handle_key(&mut app, key);
        }
    }

    // 3. Drain SDK events (non-blocking)
    for event in change_iter.try_iter() {
        handle_change_event(&mut app, &event);
    }

    if app.should_quit { break; }
}
```

### Navigation & Key Handling

- [x] `Screen` enum navigation with push/pop stack
- [x] Global key handler:
  - `Esc` = pop navigation (quit from root)
  - `q` = quit from root screen
  - `?` = toggle help overlay
  - `←→` = switch tabs at current level
  - `↑↓` = navigate within current view
  - `Enter` = drill in (push new screen)
  - Media keys dispatch `Action` values to executor

### Key Legend Bar

- [x] Bottom row widget, re-renders based on `app.navigation.current()`
- [x] Each `Screen` variant defines its own key legend text
- [x] Render as `Paragraph` in a bottom `Rect`

### Breadcrumb Header

- [x] Top row: `SONOS`, `SONOS > Living Room`, `SONOS > Living Room > Beam`
- [x] Built from `Navigation::stack` — each `Screen` contributes its label

### Theme System

- [x] Load theme name from `config.theme`
- [x] `Theme` struct with `ratatui::style::Style` values: `bg`, `fg`, `accent`, `highlight`, `muted`, `error`
- [x] Four built-in themes: `dark`, `light`, `neon`, `sonos`
- [x] All widgets reference `app.theme` for styling

**Exit criteria:** TUI launches, shows a blank frame with correct header and legend bar, navigates between screen stubs with arrow keys and Enter/Esc, exits cleanly on `q` and `Esc`. Terminal restores on panic.

---

## Milestone 7: TUI — Home Screen

The landing experience. Must feel live and scannable.

### Groups Tab (default)

- [x] Responsive group card grid: 2 columns on terminals ≥ 100 chars wide, 1 column on narrow
- [x] Each group card widget rendering:
  - Group name (bold)
  - Playback state icon: `▶` Playing, `⏸` Paused, `■` Stopped
  - Current track + artist (marquee scroll if truncated)
  - Volume bar: `████████░░░░ 80%` using `ratatui::widgets::Gauge`
  - Animated progress bar: `━━━━━━━╺──── 2:31/5:55` with elapsed/total
  - Speaker count: `Beam + 2 surrounds`

- [x] Selected card: double border (`BorderType::Double`) + bold + `●` indicator
- [x] Arrow key navigation between cards (`↑↓←→`)

- [x] Mini-player at bottom: tracks focused card
  - Group name + current track + progress + volume
  - Only visible on Home screen

### Property Watching for Groups Tab

- [x] On entering Groups tab, watch these properties for each group's coordinator:
  - `speaker.current_track.watch()` — `CurrentTrack`
  - `speaker.playback_state.watch()` — `PlaybackState`
  - `speaker.position.watch()` — `Position`
  - `group.volume.watch()` — `GroupVolume`

- [x] On leaving Groups tab, watch handles dropped automatically

- [x] Handle `ChangeEvent` in event loop:
  - Match `event.property_key`: `"current_track"`, `"playback_state"`, `"position"`, `"group_volume"`
  - Update corresponding card state, trigger re-render

### Progress Bar Animation

- [x] Client-side interpolation: if last known position is `Position { position_ms: 151000, duration_ms: 355000 }` and state is `Playing`, increment displayed position by elapsed wall-clock time each frame
- [x] Reset interpolation on `ChangeEvent` with `"position"` key (authoritative update)
- [x] Pause interpolation when `PlaybackState::Paused` or `Stopped`

### Speakers Tab

- [x] Speakers organized by group with group name headers
- [x] `"NOT IN A GROUP"` section for ungrouped speakers (standalone groups with `group.is_standalone()`)
- [x] `▸` cursor with `↑↓` navigation
- [x] `n` — create new group: pick a coordinator, calls `system.create_group(&coordinator, &[])`
- [x] `Enter` — move selected speaker into a group (picker modal listing available groups)
  - SDK: `group.add_speaker(&speaker)`
- [x] `d` — ungroup selected speaker
  - SDK: `speaker.leave_group()`

**Exit criteria:** Home screen renders live data. Progress bars animate. State changes from the Sonos app (volume, track change, play/pause) appear on screen within 1 second.

---

## Milestone 8: TUI — Group View

The per-group detail experience. Three tabs.

### Now Playing Tab

- [ ] Album art rendering (left side, ~20x20 chars):
  - Use `ratatui-image` crate with `Picker::from_query_stdio()` for auto-detection
  - Sixel/Kitty protocol for capable terminals (iTerm2, Kitty, WezTerm)
  - Half-block fallback (`▀▄` with truecolor) — broadly compatible
  - ASCII art as universal fallback
  - Fetch image from `current_track.album_art_uri` (HTTP GET, decode with `image` crate)

- [ ] Track metadata (right side): title, artist, album
  - Source: `speaker.current_track.get()` → `CurrentTrack { title, artist, album, ... }`

- [ ] Group volume control: `↑↓` adjusts
  - SDK: `group.set_relative_volume(+2)` / `group.set_relative_volume(-2)` per keypress
  - Display: `████████████████░░░░ 80%` gauge widget

- [ ] Speaker count info line: e.g., "Beam + One SL x 2"
  - From `group.members()`, aggregate by `speaker.model_name`

- [ ] Playback controls row (centered): `⏮  ▶  ⏭` icons
- [ ] `Space` = play/pause toggle
  - Check `speaker.playback_state.get()`, call `speaker.play()` or `speaker.pause()`
- [ ] Media keys work globally (handled at event loop level, not per-screen)

- [ ] Progress bar with elapsed / total time, animates every frame tick
  - Source: `speaker.position.get()` → `Position { position_ms, duration_ms }`
  - Use `position.progress()` for 0.0–1.0 ratio

### Property Watching for Now Playing

- [ ] On entering Now Playing: watch on the group's coordinator:
  - `speaker.current_track.watch()`
  - `speaker.playback_state.watch()`
  - `speaker.position.watch()`
  - `group.volume.watch()`
- [ ] Watch handles dropped when leaving

### Speakers Tab

- [ ] Each group member shown with EQ controls:
  - Volume slider: `speaker.volume` — `↑↓` adjusts via `speaker.set_relative_volume()`
  - Bass slider: `speaker.bass` — `←→` adjusts via `speaker.set_bass()`
  - Treble slider: `speaker.treble` — `←→` adjusts via `speaker.set_treble()`
  - Loudness toggle: `speaker.loudness` — `Enter` toggles via `speaker.set_loudness(!current)`
  - Mute toggle: `speaker.mute` — `m` toggles via `speaker.set_mute(!current)`

- [ ] `↑↓` navigates between speakers and their settings
- [ ] `←→` adjusts focused slider (or switches tabs when no slider focused)

- [ ] Available (non-member) speakers section with `○` bullets
  - `Enter` to add: `group.add_speaker(&speaker)`
  - `d` to remove: `group.remove_speaker(&speaker)`

### Property Watching for Speakers Tab

- [ ] Watch on each in-group speaker:
  - `speaker.volume.watch()`, `speaker.bass.watch()`, `speaker.treble.watch()`
  - `speaker.loudness.watch()`, `speaker.mute.watch()`

### Queue Tab

- [ ] Track list: number, title, artist, duration
- [ ] Track count + total duration header (from `get_media_info().nr_tracks` and `.media_duration`)
- [ ] `▶` marker on currently playing track
- [ ] `Enter` = jump to track: `speaker.seek(SeekTarget::Track(n))`
- [ ] `d` = remove from queue: `speaker.remove_track_from_queue(object_id, update_id)`
- [ ] Scrollable list for long queues using `ratatui::widgets::List` with scroll state

**Exit criteria:** All three tabs render live data. EQ sliders update the speaker in real-time. Queue reflects the actual Sonos queue. Tab switching is instant.

---

## Milestone 9: TUI — Startup Screen & Speaker Detail

### Startup / Discovery Screen

- [ ] Shown on launch while `SonosSystem::new()` runs (or cache loads)
- [ ] Centered `S O N O S` logo (styled text, not emoji)
- [ ] Animated spinner (rotate through `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` braille characters) while scanning
- [ ] Use `sonos_discovery::get_iter()` to show speakers appearing one-by-one as discovered
  - Each `DeviceEvent::Found(device)` adds a line: `device.name` — `device.model_name` — `device.ip_address`
- [ ] `Enter` to skip ahead once any speakers are found
- [ ] `Esc` to quit
- [ ] Transitions to Home screen once discovery completes or user skips

### Speaker Detail Screen

- [ ] Accessible from any Speakers tab (Home or Group level)
- [ ] Product info: `speaker.name`, `speaker.model_name`, `speaker.ip`
  - Firmware version: from `SpeakerInfo` if available in SDK, or omit for v1
- [ ] Group membership: which group this speaker belongs to
  - `system.get_group_for_speaker(&speaker.id)` → group name
  - Coordinator status: `group.is_coordinator(&speaker.id)`
- [ ] Audio controls: volume, bass, treble, loudness, mute
  - `↑↓` navigate between controls
  - `←→` adjust values
  - Same SDK methods as Group View Speakers tab
- [ ] `Esc` returns to calling Speakers tab (pop navigation stack)

### Property Watching for Speaker Detail

- [ ] Watch: `speaker.volume`, `speaker.bass`, `speaker.treble`, `speaker.loudness`, `speaker.mute`
- [ ] Watch handles dropped on leaving

**Exit criteria:** Startup screen shows real discovery progress. Speaker Detail shows accurate device info and live-updates EQ controls.

---

## Milestone 10: Polish & Shipping

Everything needed to hand this to someone who has never heard of it.

### Documentation

- [ ] `README.md`:
  - What it is (one paragraph)
  - Installation: `cargo install sonos-cli` / Homebrew formula
  - First-run screenshot
  - Command reference table (from `docs/goals.md`)
  - Config file documentation with all keys and defaults
  - Terminal compatibility notes

### CLI Flags & Output

- [ ] `--verbose` flag: exposes raw `SdkError` debug output and backtrace
  - When `global.verbose`: print `{:?}` of error before the formatted message
- [ ] `--quiet` / `-q` flag: suppresses non-error output
  - Success messages silenced; only stderr errors shown
- [ ] `--no-input` flag: disables confirmation prompts (for scripts)
- [ ] `--version` / `-v` flag: prints `sonos {version}` from `Cargo.toml`
  - clap: `#[command(version)]` on `Cli` struct

### Error Handling Polish

- [ ] All error messages follow format: `error: <description>\n<recovery action>`
- [ ] `CliError::recovery_hint()` implemented for every variant
- [ ] Graceful handling of speakers going offline mid-session:
  - SDK returns `SdkError::ApiError(ApiError::NetworkError(...))` — catch and suggest rediscovery
  - TUI: mark offline speakers with `✗` indicator, continue operating on remaining speakers

### Terminal Compatibility

- [ ] Test across: iTerm2, Kitty, WezTerm, Alacritty, Terminal.app, tmux
- [ ] Album art auto-detection edge cases:
  - Terminals that lie about Sixel support → fall back to half-block
  - Very small terminal windows → skip album art, show metadata only
  - `ratatui-image::Picker::from_query_stdio()` handles most detection
- [ ] Ensure `crossterm` alternate screen and raw mode work correctly in tmux

### Robustness

- [ ] TUI panic hook restores terminal (from Milestone 6)
- [ ] `Ctrl-C` exits cleanly (crossterm handles SIGINT in raw mode)
- [ ] No panics in normal operation — all `.unwrap()` calls reviewed and replaced with proper error handling

**Exit criteria:** A non-developer Sonos user can install, run, and use sonos-cli without help. The TUI is stable across all target terminals. No panics in normal operation.

---

## What v1 Is Not

These are out of scope. Not deferred — intentionally excluded from v1:

| Feature | Reason |
|---------|--------|
| Music library browsing / search | SDK doesn't expose content directory service yet |
| Alarm CRUD | SDK only exposes snooze + query of active alarm |
| `--json` output flag | Not needed for v1 completeness goal |
| `sonos tui --group NAME` deep-link | Nice-to-have; doesn't block completeness |
| Mouse support in TUI | Keyboard-only is intentional |
| Sonos account / streaming auth | Out of SDK scope entirely |

---

## References

- Full product requirements: `docs/product/prd.md`
- Technical architecture: `docs/brainstorm/2026-03-01-sonos-cli-architecture-brainstorm.md`
- TUI screen designs: `docs/brainstorm/2026-02-26-sonos-tui-brainstorm.md`
- SDK API reference: `docs/references/sonos-sdk.md`
- CLI conventions: `docs/references/cli-guidelines.md`
- Project goals & design decisions: `docs/goals.md`
