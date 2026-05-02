---
title: "feat: TUI settings view"
type: feat
status: active
date: 2026-04-30
origin: docs/brainstorms/2026-04-30-tui-v1-simplification-brainstorm.md
parallel-group: tui-v1-simplification
---

# TUI Settings View

## Overview

Implement the Settings tab — the second and final view in the simplified TUI. Three settings only: theme, default group, and album art toggle. Changes write to `~/.config/sonos/config.toml` and take effect immediately.

(see brainstorm: `docs/brainstorms/2026-04-30-tui-v1-simplification-brainstorm.md` — "Settings View" section)

## Problem Statement / Motivation

The Settings tab is currently a "coming soon" placeholder in `ui.rs`. The brainstorm scopes it to exactly three settings that a user would actually change in-app: theme (visual preference), default group (targeting), and album art on/off (terminal compatibility). Everything else is config-file-only.

## Proposed Solution

A new settings screen + widget following the three-layer architecture. The screen assembles the current config values and available options, the widget renders the dropdown-style form, and the handler manages focus/selection and persists changes.

## Technical Approach

### Target Layout

```
♪ S O N O S                         Speakers  [▸Settings]
──────────────────────────────────────────────────────────
  Settings

  Theme:          [ dark ▼ ]
  Default group:  [ Living Room ▼ ]
  Album art:      [ image ▼ ]
```

Three settings:
- **Theme**: dark, light, neon, sonos
- **Default group**: dropdown of discovered groups
- **Album art**: image / halfblock / off
  - `image` — rendered via terminal graphics protocol (sixel/kitty), detected by `Picker`
  - `halfblock` — Unicode half-block pixel art (`▀▄` with truecolor), works in most terminals
  - `off` — music note placeholder in a bordered box (♪), no image fetching

### Phase 1: Data Structures

**New types in `types.rs`:**

```rust
pub struct SettingsData {
    pub items: Vec<SettingsItem>,
    pub selected_index: usize,
    pub is_dropdown_open: bool,
    pub dropdown_selected: usize,
}

pub struct SettingsItem {
    pub label: &'static str,
    pub current_value: String,
    pub options: Vec<String>,
}
```

**Add to `Navigation`/`App`:**

```rust
pub struct SettingsScreenState {
    pub selected_index: usize,       // which setting row is focused (0–2)
    pub dropdown_open: bool,         // is the option picker expanded
    pub dropdown_selected: usize,    // which option in the open dropdown
}
```

### Phase 2: Screen — `screens/settings.rs`

New screen module. Assembles `SettingsData` from current `App` state:

1. **Theme**: options = `["dark", "light", "neon", "sonos"]`, current = `app.config.theme`
2. **Default group**: options = group names from `app.system.groups()` (sorted alphabetically), current = `app.config.default_group` or `"(auto)"` if None
3. **Album art**: options = `["image", "halfblock", "off"]`, current = from `app.config.album_art_mode` (defaults to `"image"`)

```rust
pub fn render(frame: &mut Frame, area: Rect, ctx: &mut RenderContext) {
    let groups = ctx.app.system.groups();
    let group_names: Vec<String> = groups.iter()
        .filter_map(|g| g.coordinator().map(|c| c.name.clone()))
        .collect();
    
    let items = vec![
        SettingsItem {
            label: "Theme",
            current_value: ctx.app.config.theme.clone(),
            options: vec!["dark", "light", "neon", "sonos"].into_iter().map(String::from).collect(),
        },
        SettingsItem {
            label: "Default group",
            current_value: ctx.app.config.default_group.clone().unwrap_or_else(|| "(auto)".into()),
            options: std::iter::once("(auto)".into()).chain(group_names).collect(),
        },
        SettingsItem {
            label: "Album art",
            current_value: ctx.app.config.album_art_mode.to_string(),
            options: vec!["image".into(), "halfblock".into(), "off".into()],
        },
    ];
    
    let data = SettingsData {
        items,
        selected_index: ctx.app.navigation.settings_state.selected_index,
        is_dropdown_open: ctx.app.navigation.settings_state.dropdown_open,
        dropdown_selected: ctx.app.navigation.settings_state.dropdown_selected,
    };
    
    widgets::settings::render(frame, area, &data, &ctx.app.theme);
}
```

### Phase 3: Widget — `widgets/settings.rs`

New render-only widget.

**Layout:**
- Title "Settings" in accent style, 2 lines below the top of the content area
- Each setting row: `  {label}:` left-padded and right-aligned to a fixed column (16 chars) + `[ {value} ▼ ]` rendered as a bordered inline dropdown

**Normal state:**
- Each row shows `[ {current_value} ▼ ]`
- Selected row highlighted with accent background on the dropdown bracket area
- Cursor indicator `◀` at far right of selected row

**Dropdown open state:**
- The selected setting's options expand vertically below the bracket:
  ```
  Theme:          [ dark ▼ ]
                    dark        ◀
                    light
                    neon
                    sonos
  ```
- Active option in the dropdown highlighted with accent style + `◀` indicator
- Other settings remain visible but non-interactive while dropdown is open

**Rendering approach:**
- Use `ratatui::widgets::Paragraph` with styled `Line`/`Span` sequences
- Dropdown items rendered as additional lines after the setting row when open
- Fixed label column width (longest label "Default group" = 13 chars + `:` + padding)

### Phase 4: Handler — `handlers/settings.rs`

New handler module.

**When dropdown is closed:**
- `Up/Down`: move `selected_index` between 0–2
- `Enter` or `Right`: open dropdown for current setting (`dropdown_open = true`, `dropdown_selected` set to index of `current_value` in options)
- `Up` from index 0: return `FocusTabBar` action (same pattern as speaker list)

**When dropdown is open:**
- `Up/Down`: move `dropdown_selected` within options
- `Enter` or `Right`: confirm selection → apply change, close dropdown
- `Esc` or `Left`: close dropdown without changing

**Esc key priority:** When a dropdown is open, the settings handler must intercept Esc before the global quit handler in `event.rs`. The current dispatch order in `event.rs` handles `q` and `Ctrl+C` globally, then Esc cancels pick-up mode or quits. The settings handler needs the same pattern: `handlers/home.rs` dispatches to the settings handler first, which returns `Handled` for Esc when a dropdown is open. Only if the handler does NOT consume Esc does the global handler see it.

**Applying changes:**

When a setting value is confirmed:

1. Update the in-memory `app.config` field:
   - Theme: `app.config.theme = value`, then `app.theme = Theme::from_name(&value)` for immediate effect
   - Default group: `app.config.default_group = if value == "(auto)" { None } else { Some(value) }`
   - Album art: map dropdown value to `AlbumArtMode` variant — `"image"` → `Image`, `"halfblock"` → Halfblock, `"off"` → `Off`. Show status message: "Album art change takes effect on restart" (Picker is initialized once at startup).

2. Persist to disk: call `app.config.save()`. On error, show `app.status_message = Some(format!("error: {e}"))` — keep the in-memory change so the user sees their selection, but they know it didn't persist.

### Phase 5: Config Updates — `config.rs`

**Replace the existing `AlbumArtMode` enum** with three explicit variants. The current enum has `Auto`, `Off`, and a catch-all `Other` (with `#[serde(other)]`). The new enum gives users explicit control over the rendering mode:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AlbumArtMode {
    /// Rendered via terminal graphics protocol (sixel/kitty) — best quality, requires capable terminal
    #[default]
    Image,
    /// Unicode half-block pixel art (▀▄ with truecolor) — works in most terminals
    Halfblock,
    /// Music note placeholder (♪) in bordered box — no image fetching
    Off,
    /// Catch-all for unrecognized config values — behaves like Image
    #[serde(other)]
    Other,
}
```

**Add `Serialize` derive** to both `Config` and `AlbumArtMode`. The `Other` variant uses `#[serde(other)]` which is deserialize-only. For serialization, `Other` should serialize as `"image"` (since it behaves like Image). Implement `Serialize` manually for `AlbumArtMode` to map `Other` → `"image"`.

**Add `Display` impl** for the dropdown's `current_value`:

```rust
impl std::fmt::Display for AlbumArtMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Image | Self::Other => write!(f, "image"),
            Self::Halfblock => write!(f, "halfblock"),
            Self::Off => write!(f, "off"),
        }
    }
}
```

**Update `is_off()`** — keep as-is, still returns `true` only for `Off`.

**Update `tui/mod.rs`** — the Picker initialization at startup currently checks `config.album_art_mode.is_off()`. Update to also handle `Halfblock`:
- `Image` or `Other` → run `Picker::from_query_stdio()` for protocol detection
- `Halfblock` → create a Picker forced to halfblock protocol (skip sixel/kitty detection)
- `Off` → skip Picker entirely, no image loading

**Backward compatibility:** Existing config files with `album_art_mode = "auto"` will deserialize as `Other` (catch-all), which behaves like `Image`. Users upgrading see no change in behavior. On next save, it normalizes to `"image"`.

**Add `#[serde(skip_serializing_if = "Option::is_none")]` to `default_group`:**

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub default_group: Option<String>,
```

This avoids writing `default_group = ""` to the TOML file when no default is set.

**Extract a shared `config_path()` helper** used by both `load()` and `save()`:

```rust
fn config_path() -> Option<PathBuf> {
    std::env::var("SONOS_CONFIG_DIR")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| dirs::config_dir().map(|p| p.join("sonos")))
        .map(|d| d.join("config.toml"))
}
```

**Add `Config::save()` method:**

```rust
impl Config {
    pub fn save(&self) -> anyhow::Result<()> {
        let path = config_path()
            .ok_or_else(|| anyhow::anyhow!("cannot determine config directory"))?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
```

This is the only plan that requires changes to `config.rs`.

### Phase 6: Integration — Wire Into Existing Navigation

**`handlers/home.rs`:**

Update tab dispatch to route to the new settings handler:

```rust
Tab::Settings => {
    match handlers::settings::handle_key(app, key) {
        SettingsAction::Handled => {}
        SettingsAction::FocusTabBar => { app.navigation.tab_focused = true; }
    }
}
```

**`ui.rs`:**

Replace the "coming soon" placeholder with the settings screen call:

```rust
Tab::Settings => screens::settings::render(frame, content_area, ctx),
```

**`app.rs`:**

Add `settings_state: SettingsScreenState` to `Navigation`.

**Dropdown state cleanup on tab switch:** In `handlers/home.rs`, when switching away from Settings tab (Left/Right on tab bar), reset `app.navigation.settings_state.dropdown_open = false`. This prevents returning to a stale open dropdown.

**Key legend update:** In `ui.rs` `render_key_legend()`, add Settings-specific legend:
- Dropdown closed: `↑↓ Navigate  Enter Open  ←→ Tabs  Esc Quit`
- Dropdown open: `↑↓ Select  Enter Confirm  Esc Cancel`

## Design Decisions (from SpecFlow analysis)

| Question | Decision | Rationale |
|----------|----------|-----------|
| `album_art` options? | **Replace `AlbumArtMode` with three explicit variants: `Image`, `Halfblock`, `Off`.** Drop the old `Auto` variant. | Gives users explicit control: `image` = sixel/kitty protocol, `halfblock` = Unicode `▀▄` pixel art, `off` = music note placeholder. Old `auto` config values deserialize as `Other` → behaves like `Image`. |
| `Config::save()` config path? | **Extract shared `config_path()` helper** respecting `SONOS_CONFIG_DIR`. | Both `load()` and `save()` must use the same path. |
| `Config` missing `Serialize` derive? | **Add `Serialize` to both `Config` and `AlbumArtMode`.** Handle `#[serde(other)]` conflict. | Required for `toml::to_string_pretty()`. |
| Save error handling? | **Keep in-memory change, show error via `app.status_message`.** | User sees their selection; error is visible but non-blocking. |
| Album art mode change runtime effect? | **Show "Album art change takes effect on restart."** Picker protocol is initialized once at startup. | Honest UX. Re-initializing Picker mid-session is complex and error-prone. Switching between image/halfblock/off requires different Picker configurations. |
| Esc key priority when dropdown open? | **Settings handler intercepts Esc first.** Only passes through to global quit when dropdown is closed. | Prevents quitting the app when user intends to cancel a dropdown. |
| Dropdown state on tab switch? | **Reset `dropdown_open = false` when switching away from Settings.** | Prevents returning to stale open dropdown. |
| Current value not in options list? | **Prepend saved value even if not in discovered groups.** Default `dropdown_selected` to 0 if not found. | Ensures the index always resolves. |
| Key legend for Settings? | **Dynamic based on dropdown state.** Closed: navigate/open/tabs/quit. Open: select/confirm/cancel. | Users need to discover the interaction. |
| Right arrow confirm vs Enter only? | **Keep both** (Right opens AND confirms). | Matches "Right = expand/drill-in" mental model from speaker list. |
| Unknown TOML keys destroyed on save? | **Acceptable for v1.** File has only 3 keys. Document in code comment for future reference. | Low risk given the minimal config surface. |
| Visual save confirmation? | **Show `app.status_message`** for non-theme changes: "Default group set to Kitchen" or "Album art: off (restart to apply)". Theme change is self-evident. | Uses existing status message pattern from speaker list handler. |

## Acceptance Criteria

- [ ] Settings tab renders three settings: Theme, Default group, Album art
- [ ] Up/Down navigates between settings rows
- [ ] Enter/Right opens dropdown for the focused setting
- [ ] Dropdown shows all available options with the current value highlighted
- [ ] Up/Down navigates dropdown options; Enter confirms; Esc cancels
- [ ] Esc in open dropdown closes dropdown (does NOT quit app)
- [ ] Theme change takes effect immediately (re-renders with new theme)
- [ ] Default group change persists to `config.toml` with status message confirmation
- [ ] Album art dropdown shows three options: image, halfblock, off
- [ ] Album art mode change persists to `config.toml` with "restart to apply" status message
- [ ] `AlbumArtMode` enum updated: `Image` (default), `Halfblock`, `Off`, `Other` (catch-all)
- [ ] Old `auto` config values deserialize as `Other` → behaves like `Image` (backward compatible)
- [ ] `tui/mod.rs` Picker initialization updated for `Image` vs `Halfblock` vs `Off`
- [ ] Default group dropdown populated from live discovered groups + "(auto)" + saved value if not discovered
- [ ] Config persisted via `Config::save()` respecting `SONOS_CONFIG_DIR`
- [ ] `Serialize` derive added to `Config` and `AlbumArtMode`
- [ ] `config_path()` helper extracted and used by both `load()` and `save()`
- [ ] `default_group` uses `skip_serializing_if = "Option::is_none"`
- [ ] Save errors shown via `app.status_message` (in-memory change preserved)
- [ ] Dropdown state reset when switching away from Settings tab
- [ ] Key legend updates dynamically for dropdown-open vs closed state
- [ ] "coming soon" placeholder removed from `ui.rs`
- [ ] Settings tab accessible via Left/Right on tab bar (existing navigation)
- [ ] Three-layer architecture maintained: data in screen, rendering in widget, input in handler

## Dependencies & Risks

**Dependencies:**
- The `album_art_mode` config field coordinates with the **Bottom Player Bar** plan: the bar checks `config.album_art_mode.is_off()` before rendering album art. This is a single `if` guard — low conflict risk.
- The `toml` crate is already in `Cargo.toml` for deserialization. The `toml` crate includes both `serde::Serialize` and `serde::Deserialize` support by default — no feature flag needed.

**Risks:**
- **`#[serde(other)]` + `Serialize` conflict**: The `Other` variant's `#[serde(other)]` is deserialize-only. Need a custom `Serialize` impl or a `#[serde(rename = "auto")]` on `Other` to make it serialize as `"auto"`. Test this explicitly.
- **Config file format stability**: Writing via `toml::to_string_pretty` normalizes formatting and drops unknown keys. Acceptable for v1 with only 3 keys. Add a code comment noting this.
- **Group list staleness**: The default group dropdown shows groups discovered at launch. Acceptable for v1.
- **Terminal height overflow**: Three settings + one open dropdown (max 4 options for theme) = ~10 visual lines. Fits comfortably in any reasonable terminal. No scrolling needed.

## Sources & References

- **Origin brainstorm:** [docs/brainstorms/2026-04-30-tui-v1-simplification-brainstorm.md](docs/brainstorms/2026-04-30-tui-v1-simplification-brainstorm.md) — settings view design, three settings scope decision
- **Config module:** `src/config.rs` — existing `Config` struct with `load()`, `AlbumArtMode` enum. Needs `save()`, `Serialize`, `config_path()`.
- **UI dispatch:** `src/tui/ui.rs` — "coming soon" placeholder to replace
- **Handler pattern:** `src/tui/handlers/home.rs` — tab dispatch, `FocusTabBar` action pattern
- **Theme system:** `src/tui/theme.rs` — `Theme::from_name()` for immediate theme switching
- **Picker init:** `src/tui/mod.rs:28` — `config.album_art_mode.is_off()` check at startup
