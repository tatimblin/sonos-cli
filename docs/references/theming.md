# Theming Reference

How the TUI theme system works and how to add a new theme.

## Built-in Themes

| Name | Config value | Description |
|------|-------------|-------------|
| **Default** | `default` | Polished dark theme with teal accents and warm grays |
| **Black & White** | `bw` | Grayscale only — Bold/Underline carry hierarchy instead of color |
| **Minimal** | `minimal` | Stripped decorations — no leaders, no connectors, quiet palette |
| **Dance Party** | `dance_party` | Fun glyphs, vivid RGB colors, gradient progress bar |

Set via `~/.config/sonos/config.toml`:

```toml
theme = "dance_party"
```

Or change at runtime in the Settings tab. Unknown names fall back to `default`.

## Architecture

A theme is two structs: `Glyphs` (characters) and `Theme` (pre-computed styles + glyphs). Every widget receives `&Theme` and applies its styles — no hardcoded colors or characters anywhere in the render path.

Styles are pre-computed at construction time so render functions pay zero allocation cost.

```
Config (theme name) -> Theme::from_name() -> App.theme -> screens -> widgets
```

## Glyphs

`Glyphs` holds every UI character as `&'static str` or `char`. Themes can swap characters alongside colors — minimal uses spaces where default uses tree connectors.

| Field | Default | Used for |
|-------|---------|----------|
| `playing` | `▶` | Playback state icon (playing) |
| `paused` | `⏸` | Playback state icon (paused) |
| `stopped` | `■` | Playback state icon (stopped) |
| `connector_branch` | `├─ ` | Speaker list tree connector (middle) |
| `connector_last` | `└─ ` | Speaker list tree connector (last) |
| `cursor` | `❯` | Selection indicator |
| `leader_char` | `:` | Dot leader fill character |
| `model_separator` | ` • ` | Separator between name and model |
| `divider_left` | `+` | Group divider left cap |
| `divider_fill` | `─` | Group divider fill |
| `divider_right` | `+` | Group divider right cap |
| `control_prev` | `⏮` | Previous track button |
| `control_next` | `⏭` | Next track button |
| `logo` | `♪  S O N O S` | Header logo text |
| `tab_active_left` | `[` | Active tab left bracket |
| `tab_active_right` | `]` | Active tab right bracket |
| `tab_active_indicator` | `▸` | Active tab arrow |
| `dropdown_indicator` | `▼` | Dropdown closed indicator |
| `dropdown_active` | `▸` | Dropdown selected item arrow |
| `settings_cursor` | `◀` | Settings row cursor |
| `separator` | `─` | Horizontal separator line |
| `progress_cursor` | `●` | Progress bar position indicator |
| `toast_prefix` | `●` | Toast notification bullet |
| `music_note` | `♪` | Album art placeholder |

### Custom Glyphs

Start from `Glyphs::default_glyphs()` and override fields:

```rust
Glyphs {
    cursor: ">",
    separator: '-',
    connector_branch: "|- ",
    connector_last: "'- ",
    ..Glyphs::default_glyphs()
}
```

## Theme (Styles)

`Theme` holds pre-computed `ratatui::style::Style` objects. Each field is a semantic role, not a raw color.

### Layout Chrome

| Field | Role |
|-------|------|
| `header` | Logo text and active tab |
| `legend` | Key legend bar at bottom |
| `muted` | Inactive/secondary elements (connectors, separators, inactive tabs) |

### Track Info / Bottom Bar

| Field | Role |
|-------|------|
| `track_info` | Track metadata text, group name in bottom bar |
| `bottom_bar_controls` | Playback control buttons |

### Playback State Icons

| Field | Role |
|-------|------|
| `playing_icon` | Color for playing glyph |
| `paused_icon` | Color for paused glyph |
| `stopped_icon` | Color for stopped glyph |

### Volume Bar

| Field | Role |
|-------|------|
| `volume_filled` | Filled portion of volume bar (■) |
| `volume_empty` | Empty portion of volume bar (·) |

### Progress Bar

| Field | Role |
|-------|------|
| `progress_filled` | Filled portion of progress bar (used when no gradient) |
| `progress_empty` | Empty portion of progress bar (─) |
| `progress_cursor` | Progress position indicator (●) |
| `progress_time` | Elapsed / remaining time text |
| `progress_gradient_start` | Start color for gradient fill (`Color`) |
| `progress_gradient_end` | End color for gradient fill (`Color`) |

### Speakers Tab

| Field | Role |
|-------|------|
| `group_header` | Group name row (usually Bold) |
| `speaker_cursor` | Highlight color when a speaker is selected |
| `speaker_name` | Default speaker name color |
| `leader` | Dot leader fill between group name and track info |

### General

| Field | Role |
|-------|------|
| `picked_up` | Background style for a picked-up speaker |
| `accent` | Primary accent — active tab indicator, action highlights |
| `accent_secondary` | Secondary accent — leader fill on focused group |
| `error` | Error toast and error text |

## Gradient Progress Bar

Themes can opt into a gradient fill for the progress bar by setting `progress_gradient_start` and `progress_gradient_end` to different `Color::Rgb` values. Each filled character gets a linearly interpolated color between start and end.

When start and end are the same color (or non-RGB), the bar renders as a single styled span with zero overhead.

```rust
// Gradient: pink -> cyan across the filled region
progress_gradient_start: Color::Rgb(255, 50, 150),
progress_gradient_end: Color::Rgb(50, 200, 255),

// No gradient: solid color
progress_gradient_start: Color::Rgb(180, 180, 180),
progress_gradient_end: Color::Rgb(180, 180, 180),
```

## Adding a New Theme

### 1. Add the factory function

In `src/tui/theme.rs`, add a method on `Theme`:

```rust
pub fn my_theme() -> Self {
    Self {
        header: Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
        // ... all style fields ...
        progress_gradient_start: Color::White, // same = no gradient
        progress_gradient_end: Color::White,
        glyphs: Glyphs::default_glyphs(), // or custom
    }
}
```

### 2. Register it in `from_name()`

```rust
pub fn from_name(name: &str) -> Self {
    match name {
        "my_theme" => Self::my_theme(),
        // ... other themes ...
        _ => Self::default_theme(),
    }
}
```

### 3. Add it to the settings dropdown

In `src/tui/handlers/settings.rs`, add the name to `THEME_OPTIONS`:

```rust
pub(crate) const THEME_OPTIONS: &[&str] = &["default", "bw", "minimal", "dance_party", "my_theme"];
```

### 4. Update the config doc comment

In `src/config.rs`, update the `theme` field doc comment to list the new option.

## Color Types

ratatui supports:

- **Named colors:** `Color::White`, `Color::Cyan`, `Color::DarkGray`, etc. (16 ANSI colors — safe everywhere)
- **RGB:** `Color::Rgb(255, 120, 0)` — requires true-color terminal support
- **Indexed:** `Color::Indexed(42)` — 256-color palette

The `bw` theme uses only named ANSI colors for maximum compatibility. Themes using RGB (default, minimal, dance_party) require a true-color terminal.

## Style Modifiers

Available via `Modifier::BOLD`, `Modifier::UNDERLINED`, `Modifier::ITALIC`, etc. Combine with `|`:

```rust
Style::new().fg(Color::White).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
```

## Design Principles

- **Semantic roles, not raw colors.** A widget uses `theme.accent`, not `Color::Cyan`. This lets themes control the entire look from one place.
- **Zero render-time cost.** All styles are pre-computed. Widgets just reference `theme.field`.
- **Glyphs are part of the theme.** The minimal theme uses spaces where default uses tree connectors — no widget code changes needed.
