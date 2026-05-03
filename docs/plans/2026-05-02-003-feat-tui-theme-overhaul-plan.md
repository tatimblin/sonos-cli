---
title: "feat: Replace theme lineup and add gradient progress bar"
type: feat
status: active
date: 2026-05-02
origin: docs/brainstorms/2026-05-02-tui-themes-brainstorm.md
---

# feat: Replace theme lineup and add gradient progress bar

## Summary

Replace the four existing themes (dark, light, neon, sonos) with a new set of four (default, bw, minimal, dance_party) and add a gradient progress bar feature to the `Theme` struct. The default theme upgrades the color palette for a polished aesthetic; bw is grayscale-only; minimal strips decorations via custom glyphs; dance_party brings fun glyphs, wild RGB colors, and a pink-to-cyan gradient progress bar.

---

## Problem Frame

The current theme lineup (dark/light/neon/sonos) has mismatched colors that feel random rather than designed. There's no monochrome option, no stripped-down aesthetic, and no playful option. The progress bar is single-color only, limiting expressive potential for themes like dance_party.

---

## Requirements

- R1. Remove the four existing themes (dark, light, neon, sonos) and replace with four new themes (default, bw, minimal, dance_party)
- R2. Default theme: same glyphs as current, refined teal-accented warm-gray palette
- R3. Black & white theme: grayscale only (White/Gray/DarkGray/Black), Bold/Underline for hierarchy
- R4. Minimal theme: custom glyphs that strip leaders, connectors, tab brackets; quiet RGB palette
- R5. Dance party theme: fun glyphs (★, ♫, 💤), vivid RGB colors, gradient progress bar
- R6. Add `progress_gradient_start` and `progress_gradient_end` to Theme struct; when different, progress bar renders per-character color interpolation
- R7. Gradient must be zero-overhead for non-gradient themes (same start/end = single span)
- R8. Fallback theme changes from "dark" to "default" in `Theme::from_name()`
- R9. Theme switching in Settings continues to work with the new theme names
- R10. Existing config files with old theme names gracefully fall back to default

---

## Scope Boundaries

- No new theme-creation UI or user-defined custom themes
- No gradient on the volume bar — only the progress bar
- No animation or time-varying color effects
- No changes to the Theme struct's existing style fields — only additions (gradient colors)

### Deferred to Follow-Up Work

- User-configurable themes via TOML: separate effort after the theme system stabilizes

---

## Context & Research

### Relevant Code and Patterns

- `src/tui/theme.rs` — Theme/Glyphs structs, four factory methods, `from_name` dispatch (lines 153-162)
- `src/tui/widgets/progress_bar.rs` — `render_bar_spans()` takes `filled_style`, `cursor_style`, `empty_style` (lines 16-46); pre-computed bar strings `PROG_FILLED`/`PROG_EMPTY` with `PROG_CHAR_BYTES = 3`
- `src/tui/widgets/bottom_bar.rs` — Two call sites for `render_bar_spans`: wide layout (line 121) and narrow layout (line 252); both pass `theme.progress_filled`, `theme.progress_cursor`, `theme.progress_empty`
- `src/tui/handlers/settings.rs` — `THEME_OPTIONS` constant (line 11), `apply_setting` for theme switching (line 154)
- `src/config.rs` — `theme: String` field with `"dark"` default (line 61), doc comment listing valid names (line 51)

### Patterns to Follow

- Zero-allocation design: all styles pre-computed at construction, render functions pay zero cost
- Factory method pattern: `Theme::theme_name() -> Self` with `from_name` dispatch
- Glyphs override via struct update syntax: `Glyphs { field: "override", ..Glyphs::default_glyphs() }`

---

## Key Technical Decisions

- **Gradient fields are `Color`, not `Option<Color>`**: Using `Color` directly (with same start/end = no gradient) is simpler than wrapping in `Option`. The equality check (`start != end`) is trivial and avoids `Option` unwrapping in the hot path
- **Per-character spans for gradient**: Each filled character gets its own `Span` with an interpolated `Style`. This is only constructed when `start != end`, so non-gradient themes pay zero cost
- **Gradient only interpolates RGB colors**: If either endpoint is a named ANSI color, gradient falls back to `start` color. This avoids complex ANSI-to-RGB conversion and is consistent with the brainstorm sketch
- **`render_bar_spans` gains two new parameters**: Adding `gradient_start` and `gradient_end` to the function signature (rather than a separate function) keeps all call sites explicit about gradient behavior
- **💤 emoji in dance_party**: The `paused` glyph uses 💤 which is multi-byte. Terminal support varies, but it's fine for a fun theme — users who want reliability pick a different theme

---

## Open Questions

### Resolved During Planning

- **Should `render_bar_spans` take gradient colors or should the caller build per-character spans?** Resolution: `render_bar_spans` handles it internally. The function already owns the filled/empty split logic; adding gradient there keeps callers simple
- **Do old theme names in config files cause errors?** Resolution: No — `from_name` already has a catch-all fallback. Changing the fallback from `dark()` to `default()` handles this gracefully

### Deferred to Implementation

- **Exact visual tuning of RGB values**: The brainstorm specifies target values, but fine-tuning may happen during visual testing
- **💤 emoji width**: Some terminals render 💤 as 2 cells. If it causes layout issues in dance_party, a fallback glyph can be chosen during implementation

---

## Implementation Units

- U1. **Add gradient fields to Theme struct and update progress bar rendering**

**Goal:** Extend the theme system with gradient progress bar support and update the rendering function to interpolate colors.

**Requirements:** R6, R7

**Dependencies:** None

**Files:**
- Modify: `src/tui/theme.rs`
- Modify: `src/tui/widgets/progress_bar.rs`
- Modify: `src/tui/widgets/bottom_bar.rs`
- Test: `src/tui/widgets/progress_bar.rs` (inline tests)

**Approach:**
- Add `progress_gradient_start: Color` and `progress_gradient_end: Color` to the `Theme` struct
- Add a `lerp` helper and `gradient_color` function to `progress_bar.rs`
- Modify `render_bar_spans` to accept gradient start/end colors. When `start != end`, emit per-character `Span`s with interpolated colors instead of a single filled span. When `start == end`, emit a single span as today (zero overhead)
- Update both call sites in `bottom_bar.rs` to pass `theme.progress_gradient_start` and `theme.progress_gradient_end`
- Set gradient start/end to the same value as `progress_filled` in all existing themes temporarily (will be replaced in U2)

**Patterns to follow:**
- `progress_bar.rs` existing pre-computed string slicing pattern
- Zero-allocation principle: gradient spans still reference slices of the pre-computed `PROG_FILLED` string

**Test scenarios:**
- Happy path: gradient with distinct RGB start/end colors produces N filled spans with interpolated colors at 50% progress
- Happy path: same start/end color produces exactly 1 filled span (zero-overhead check)
- Edge case: progress at 0.0 with gradient produces no filled spans
- Edge case: progress at 1.0 with gradient fills entire width
- Edge case: non-RGB gradient colors (e.g., named Color::White) fall back to start color as single span
- Edge case: filled count of 1 with gradient produces 1 span at start color

**Verification:**
- All existing progress bar tests pass
- New gradient tests pass
- `cargo check` succeeds

---

- U2. **Replace all four themes with the new lineup**

**Goal:** Remove dark/light/neon/sonos factory methods, add default/bw/minimal/dance_party, update the `from_name` dispatch.

**Requirements:** R1, R2, R3, R4, R5, R8, R10

**Dependencies:** U1 (gradient fields must exist on Theme)

**Files:**
- Modify: `src/tui/theme.rs`
- Test: `src/tui/theme.rs` (inline tests)

**Approach:**
- Remove `dark()`, `light()`, `neon()`, `sonos()` methods
- Add `default()`, `bw()`, `minimal()`, `dance_party()` methods using the exact colors and glyphs from the brainstorm
- `default()` and `bw()` use `Glyphs::default_glyphs()`
- `minimal()` uses custom Glyphs with stripped decorations (no connectors, no leaders, no tab brackets, lowercase logo `"sonos"`, `›` cursor)
- `dance_party()` uses custom Glyphs (★ cursor, ♫ playing/music_note, 💤 paused, ✖ stopped, ◆ progress cursor, ◄◄/►► controls, party logo)
- `dance_party()` sets `progress_gradient_start: Color::Rgb(255, 50, 150)` and `progress_gradient_end: Color::Rgb(50, 200, 255)`; all other themes set start == end (no gradient)
- Update `from_name` match: `"bw"` → `bw()`, `"minimal"` → `minimal()`, `"dance_party"` → `dance_party()`, `_` → `default()`
- Update existing tests: remove dark-specific assertions, add tests for all four new themes resolving correctly, test unknown name falls back to default

**Patterns to follow:**
- Existing factory method pattern: `pub fn theme_name() -> Self { Self { ... } }`
- Glyphs override: `Glyphs { cursor: "›", ..Glyphs::default_glyphs() }`

**Test scenarios:**
- Happy path: `Theme::from_name("default")` returns default theme
- Happy path: `Theme::from_name("bw")` returns bw theme
- Happy path: `Theme::from_name("minimal")` returns minimal theme with custom glyphs (check cursor == "›", logo == "sonos")
- Happy path: `Theme::from_name("dance_party")` returns dance_party theme with gradient (start != end) and custom glyphs (check cursor == "★", logo contains "DANCE")
- Edge case: `Theme::from_name("dark")` falls back to default (old name gracefully handled)
- Edge case: `Theme::from_name("nonexistent")` falls back to default
- Happy path: bw theme uses no Color::Rgb values — all named ANSI colors

**Verification:**
- All theme tests pass
- `cargo check` succeeds
- No references to old theme names remain in `theme.rs`

---

- U3. **Update settings handler and config for new theme names**

**Goal:** Wire the new theme names into the settings dropdown and config defaults so users can select and persist themes.

**Requirements:** R9, R10

**Dependencies:** U2 (new theme factory methods must exist)

**Files:**
- Modify: `src/tui/handlers/settings.rs`
- Modify: `src/config.rs`

**Approach:**
- Change `THEME_OPTIONS` from `&["dark", "light", "neon", "sonos"]` to `&["default", "bw", "minimal", "dance_party"]`
- Change `Config::default()` theme from `"dark"` to `"default"`
- Update the doc comment on `Config::theme` to list the new valid names

**Patterns to follow:**
- Existing settings handler pattern — only the constant and default change, no structural changes

**Test scenarios:**
- Happy path: settings dropdown shows all four new theme names
- Happy path: selecting "dance_party" in settings updates `app.theme` and `app.config.theme`
- Edge case: existing config file with `theme = "dark"` loads and falls back to default at runtime (via `from_name` catch-all — no config validation needed)

**Verification:**
- Theme dropdown shows: default, bw, minimal, dance_party
- Selecting each theme visually changes the TUI
- Config persists the selected theme name

---

- U4. **Update theming reference documentation**

**Goal:** Update `docs/references/theming.md` to reflect the new theme lineup and gradient feature.

**Requirements:** R1, R6

**Dependencies:** U2, U3

**Files:**
- Modify: `docs/references/theming.md`

**Approach:**
- Replace references to dark/light/neon/sonos with default/bw/minimal/dance_party
- Document the gradient progress bar fields and how themes opt in
- Update the "Adding a New Theme" example to show gradient fields
- Add a "Gradient Progress Bar" section explaining the start/end color mechanism

**Test expectation:** none — documentation only

**Verification:**
- Documentation accurately describes the current theme system after U1-U3 land

---

## System-Wide Impact

- **Interaction graph:** Theme switching in settings handler calls `Theme::from_name()` → reconstructs all styles and glyphs. All widgets consume `&Theme` from `App` on next render frame. No callbacks or observers involved
- **Error propagation:** No failure modes — theme construction is infallible. Unknown names fall back to default
- **State lifecycle risks:** None — theme is fully reconstructed atomically via factory method. No partial state possible
- **API surface parity:** CLI commands don't use themes, only the TUI. No parity concern
- **Integration coverage:** Settings handler → Theme::from_name → App.theme → widget rendering chain should be verified visually for each theme
- **Unchanged invariants:** Volume bar rendering, speaker list layout, bottom bar layout structure — all unchanged. Only colors and glyphs change

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| RGB colors don't render on terminals without true-color support | Default theme uses RGB — users on limited terminals can be advised to use bw theme (ANSI only). This matches current behavior where the sonos theme already uses RGB |
| 💤 emoji may render as 2 cells on some terminals | Dance party is explicitly a fun theme; minor layout quirks are acceptable. Can swap to a single-cell glyph during implementation if testing reveals issues |
| Gradient progress bar allocates N spans per frame | Only active when start != end (dance_party only by default). N is bounded by terminal width (~100-200 chars max). Profiling not needed for this scale |
| Users with existing config files referencing old theme names | `from_name` fallback handles this — "dark", "light", "neon", "sonos" all resolve to default. No migration needed |

---

## Sources & References

- **Origin document:** [docs/brainstorms/2026-05-02-tui-themes-brainstorm.md](docs/brainstorms/2026-05-02-tui-themes-brainstorm.md)
- **Theming reference:** [docs/references/theming.md](docs/references/theming.md)
- Related code: `src/tui/theme.rs`, `src/tui/widgets/progress_bar.rs`, `src/tui/handlers/settings.rs`
