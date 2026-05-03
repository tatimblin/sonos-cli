# TUI Themes Brainstorm

**Date:** 2026-05-02

## What We're Building

Replace the existing four themes (dark, light, neon, sonos) with a new set of four:

| Theme | Name (config) | Personality |
|-------|---------------|-------------|
| **Default** | `default` | Polished dark theme with cohesive palette — same glyphs as current |
| **Black & White** | `bw` | Grayscale only (White/Gray/DarkGray/Black) — elegant monochrome |
| **Minimal** | `minimal` | Stripped decorations — fewer leaders, simplified connectors, quiet palette |
| **Dance Party** | `dance_party` | Fun glyphs, wild RGB colors, gradient progress bar |

### New Feature: Gradient Progress Bar

Add `progress_gradient_start` and `progress_gradient_end` (`Color::Rgb`) to the `Theme` struct. The progress bar renders each filled character with a color interpolated between start and end based on position. When start == end, it behaves as a solid color (backwards compatible).

This is a theme-level feature available to all themes. The `render_bar_spans` function changes from producing one filled span to producing N individually-colored spans when a gradient is active.

## Why This Approach

- **Replaces instead of adds** — 4 themes is the right number for a settings dropdown. 8 is clutter.
- **Gradient at the theme level** — avoids special-casing dance_party in widget code. Any theme can opt in.
- **Glyphs + styles together** — minimal and dance_party demonstrate why the theme system bundles both.

## Theme Proposals

### 1. Default (`default`)

The everyday theme. Clean dark background, cohesive warm-neutral palette. No garish accent colors — everything feels intentional.

**Design direction:** Soft white text, warm gray secondaries, a single teal accent for interactive elements. Green/amber/gray for playback state (same semantic meaning, refined tones).

#### Colors

| Role | Color | Rationale |
|------|-------|-----------|
| `header` | White, Bold | Clean and readable |
| `legend` | `Rgb(88, 88, 88)` | Visible but not competing |
| `muted` | `Rgb(68, 68, 68)` | Subtle chrome |
| `track_info` | `Rgb(158, 158, 158)` | Readable secondary text |
| `bottom_bar_controls` | White | High contrast for controls |
| `playing_icon` | `Rgb(80, 200, 120)` | Fresh green — active, alive |
| `paused_icon` | `Rgb(230, 180, 60)` | Warm amber — waiting |
| `stopped_icon` | `Rgb(88, 88, 88)` | Faded — inactive |
| `volume_filled` | `Rgb(100, 200, 200)` | Teal accent |
| `volume_empty` | `Rgb(58, 58, 58)` | Recedes into background |
| `progress_filled` | `Rgb(180, 180, 180)` | Neutral fill, doesn't compete with content |
| `progress_empty` | `Rgb(48, 48, 48)` | Near-invisible track |
| `progress_cursor` | White | Pops against the bar |
| `progress_time` | `Rgb(88, 88, 88)` | Quiet timestamps |
| `group_header` | White, Bold | Clear group hierarchy |
| `speaker_cursor` | `Rgb(100, 200, 200)` | Teal highlight matches accent |
| `speaker_name` | `Rgb(158, 158, 158)` | Readable but secondary |
| `leader` | `Rgb(48, 48, 48)` | Near-invisible fill |
| `picked_up` | bg `Rgb(40, 50, 55)` | Subtle teal-tinted background |
| `accent` | `Rgb(100, 200, 200)` | Teal — primary interactive color |
| `accent_secondary` | `Rgb(80, 130, 160)` | Muted steel blue |
| `error` | `Rgb(220, 80, 80)` | Warm red, not aggressive |
| `progress_gradient_start` | `Rgb(180, 180, 180)` | No gradient (same as end) |
| `progress_gradient_end` | `Rgb(180, 180, 180)` | No gradient (same as start) |

**Glyphs:** `Glyphs::default_glyphs()` (unchanged)

#### Preview

```
 ♪  S O N O S                                                [▸Speakers]      Settings
────────────────────────────────────────────────────────────────────────────────────────
   ⏸ Bathroom :::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::  32%
     ├─ Kitchen • Sonos Connect:Amp                                             27%
     ├─ Bathroom • Sonos Connect:Amp                                            35%
 ❯   └─ Bedroom • Sonos Connect:Amp                                 ■■■······· 34%

   ▶ Office / Roam ::::: Shine — Meek Mill ::::::::::::::::::::::::::::::::::::::  23%
     └─ Office / Roam • Sonos Roam 2                                            23%
────────────────────────────────────────────────────────────────────────────────────────
     Shine — Meek Mill                      ⏮  ▶  ⏭                     Office / Roam
 ♪                        1:23 ━━━━━━━●──────────────────────────── 4:01  ■■■··· 23%
────────────────────────────────────────────────────────────────────────────────────────
 ↑↓ Navigate   ←→ Volume   ␣ Pick up/Drop   p Play/Pause   ⎋ Quit

 Colors: teal accents, warm grays, soft green ▶
```

---

### 2. Black & White (`bw`)

Monochrome with grayscale hierarchy. Feels like a well-typeset terminal. Bold and underline carry the visual weight instead of color.

#### Colors

| Role | Color | Rationale |
|------|-------|-----------|
| `header` | White, Bold | Maximum contrast |
| `legend` | DarkGray | Recedes |
| `muted` | DarkGray | Chrome fades away |
| `track_info` | Gray | Secondary text |
| `bottom_bar_controls` | White | Controls pop |
| `playing_icon` | White, Bold | Bold = active (no color) |
| `paused_icon` | Gray | Dimmer = paused |
| `stopped_icon` | DarkGray | Faded = stopped |
| `volume_filled` | White | Bright fill |
| `volume_empty` | DarkGray | Empty recedes |
| `progress_filled` | White | Clean bar |
| `progress_empty` | DarkGray | Quiet track |
| `progress_cursor` | White, Bold | Stands out |
| `progress_time` | DarkGray | Quiet |
| `group_header` | White, Bold, Underlined | Underline replaces color for emphasis |
| `speaker_cursor` | White, Bold | Bold highlight for selection |
| `speaker_name` | Gray | Standard text |
| `leader` | DarkGray | Minimal fill |
| `picked_up` | bg DarkGray | Simple inversion |
| `accent` | White | No color accent — just brightness |
| `accent_secondary` | Gray | Subtle |
| `error` | White, Bold, Underlined | Underline signals error without color |
| `progress_gradient_start` | White | No gradient |
| `progress_gradient_end` | White | No gradient |

**Glyphs:** `Glyphs::default_glyphs()` (unchanged)

#### Preview

```
 ♪  S O N O S                                                [▸Speakers]      Settings
────────────────────────────────────────────────────────────────────────────────────────
   ⏸ Bathroom :::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::  32%
     ├─ Kitchen • Sonos Connect:Amp                                             27%
 ❯   └─ Bedroom • Sonos Connect:Amp                                 ■■■······· 34%

   ▶ Office / Roam ::::: Shine — Meek Mill ::::::::::::::::::::::::::::::::::::::  23%
────────────────────────────────────────────────────────────────────────────────────────
     Shine — Meek Mill                      ⏮  ▶  ⏭                     Office / Roam
 ♪                        1:23 ━━━━━━━●──────────────────────────── 4:01  ■■■··· 23%
────────────────────────────────────────────────────────────────────────────────────────

 Colors: White (bold for emphasis), Gray, DarkGray. No color at all.
 Hierarchy through brightness and modifiers (bold, underline).
```

---

### 3. Minimal (`minimal`)

Stripped-down decorations. No leaders, simplified connectors, plain dividers. The palette is dark and quiet — only the selected item and playback state draw attention.

#### Colors

| Role | Color | Rationale |
|------|-------|-----------|
| `header` | `Rgb(180, 180, 180)` | Not quite white — understated |
| `legend` | `Rgb(68, 68, 68)` | Nearly hidden |
| `muted` | `Rgb(50, 50, 50)` | Very subtle chrome |
| `track_info` | `Rgb(130, 130, 130)` | Quiet text |
| `bottom_bar_controls` | `Rgb(180, 180, 180)` | Readable controls |
| `playing_icon` | `Rgb(130, 180, 130)` | Muted green |
| `paused_icon` | `Rgb(180, 170, 100)` | Muted amber |
| `stopped_icon` | `Rgb(60, 60, 60)` | Nearly invisible |
| `volume_filled` | `Rgb(150, 150, 150)` | Neutral |
| `volume_empty` | `Rgb(40, 40, 40)` | Barely there |
| `progress_filled` | `Rgb(140, 140, 140)` | Understated bar |
| `progress_empty` | `Rgb(35, 35, 35)` | Recedes |
| `progress_cursor` | `Rgb(180, 180, 180)` | Subtle pop |
| `progress_time` | `Rgb(68, 68, 68)` | Quiet |
| `group_header` | `Rgb(200, 200, 200)`, Bold | Hierarchy through weight |
| `speaker_cursor` | `Rgb(180, 200, 210)` | Barely-blue highlight |
| `speaker_name` | `Rgb(120, 120, 120)` | Understated |
| `leader` | `Rgb(30, 30, 30)` | Essentially invisible |
| `picked_up` | bg `Rgb(30, 35, 38)` | Barely-visible tint |
| `accent` | `Rgb(180, 200, 210)` | Cool-gray accent |
| `accent_secondary` | `Rgb(100, 110, 115)` | Muted secondary |
| `error` | `Rgb(200, 100, 100)` | Soft red |
| `progress_gradient_start` | `Rgb(140, 140, 140)` | No gradient |
| `progress_gradient_end` | `Rgb(140, 140, 140)` | No gradient |

#### Custom Glyphs

| Field | Value | Why |
|-------|-------|-----|
| `connector_branch` | `"  "` (2 spaces) | No tree lines |
| `connector_last` | `"  "` (2 spaces) | No tree lines |
| `leader_char` | `' '` (space) | No visible leaders — just whitespace |
| `divider_left` | `""` | No divider caps |
| `divider_fill` | `" "` | Blank line instead of rule |
| `divider_right` | `""` | No divider caps |
| `model_separator` | `"  "` | Space instead of bullet |
| `cursor` | `"›"` | Lighter arrow |
| `progress_cursor` | `"•"` | Smaller dot (U+2022 vs U+25CF) |
| `logo` | `"sonos"` | Lowercase, no music note |
| `tab_active_left` | `""` | No brackets |
| `tab_active_right` | `""` | No brackets |
| `tab_active_indicator` | `""` | No indicator arrow |

#### Preview

```
 sonos                                                         Speakers       Settings
────────────────────────────────────────────────────────────────────────────────────────
   ⏸ Bathroom                                                                    32%
      Kitchen  Sonos Connect:Amp                                                27%
      Bathroom  Sonos Connect:Amp                                               35%
 ›     Bedroom  Sonos Connect:Amp                                    ■■■······· 34%



   ▶ Office / Roam       Shine — Meek Mill                                       23%
      Office / Roam  Sonos Roam 2                                               23%
────────────────────────────────────────────────────────────────────────────────────────
     Shine — Meek Mill                      ⏮  ▶  ⏭                     Office / Roam
 ♪                        1:23 ━━━━━━━•──────────────────────────── 4:01  ■■■··· 23%
────────────────────────────────────────────────────────────────────────────────────────

 No leaders. No tree connectors. No tab brackets. Lowercase logo.
 Just content and whitespace breathing room.
```

---

### 4. Dance Party (`dance_party`)

Maximalist fun. Emoji-adjacent glyphs, vivid RGB colors, and a gradient progress bar that transitions across the filled region.

#### Colors

| Role | Color | Rationale |
|------|-------|-----------|
| `header` | `Rgb(255, 100, 255)` (hot pink), Bold | Party starts at the top |
| `legend` | `Rgb(100, 100, 180)` | Purple-tinted hint text |
| `muted` | `Rgb(90, 70, 120)` | Purple-gray chrome |
| `track_info` | `Rgb(255, 200, 100)` | Warm gold metadata |
| `bottom_bar_controls` | `Rgb(100, 255, 255)` | Cyan controls |
| `playing_icon` | `Rgb(50, 255, 50)` | Neon green |
| `paused_icon` | `Rgb(255, 255, 50)` | Electric yellow |
| `stopped_icon` | `Rgb(120, 50, 150)` | Dark purple — sleeping |
| `volume_filled` | `Rgb(255, 50, 150)` | Hot pink volume |
| `volume_empty` | `Rgb(60, 30, 80)` | Dark purple empty |
| `progress_filled` | (gradient — see below) | Rainbow fill |
| `progress_empty` | `Rgb(40, 20, 60)` | Deep purple track |
| `progress_cursor` | `Rgb(255, 255, 100)` | Yellow beacon |
| `progress_time` | `Rgb(100, 100, 180)` | Purple-tinted |
| `group_header` | `Rgb(255, 150, 50)` (orange), Bold | Warm group headings |
| `speaker_cursor` | `Rgb(255, 100, 255)` | Hot pink selection |
| `speaker_name` | `Rgb(200, 150, 255)` | Lavender names |
| `leader` | `Rgb(60, 30, 80)` | Dark purple dots |
| `picked_up` | bg `Rgb(80, 30, 100)` | Purple glow |
| `accent` | `Rgb(255, 100, 255)` | Hot pink |
| `accent_secondary` | `Rgb(100, 255, 200)` | Mint green |
| `error` | `Rgb(255, 80, 80)` | Still clearly an error |
| `progress_gradient_start` | `Rgb(255, 50, 150)` | Hot pink start |
| `progress_gradient_end` | `Rgb(50, 200, 255)` | Cyan end |

**Gradient:** Progress bar transitions from hot pink → cyan across the filled region. Each character gets an interpolated RGB color.

#### Custom Glyphs

| Field | Value | Why |
|-------|-------|-----|
| `playing` | `"♫"` | Double music note for extra energy |
| `paused` | `"💤"` | Sleepy snooze (falls back to ZZ if no emoji) |
| `stopped` | `"✖"` | Dramatic X |
| `cursor` | `"★"` | Star selection |
| `music_note` | `"♫"` | Double note |
| `logo` | `"★ D A N C E  P A R T Y ★"` | On brand |
| `progress_cursor` | `"◆"` | Diamond cursor |
| `toast_prefix` | `"★"` | Star toasts |
| `control_prev` | `"◄◄"` | Double arrow |
| `control_next` | `"►►"` | Double arrow |

#### Preview

```
 ★ D A N C E  P A R T Y ★                                    [▸Speakers]      Settings
────────────────────────────────────────────────────────────────────────────────────────
   💤 Bathroom :::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::  32%
     ├─ Kitchen • Sonos Connect:Amp                                             27%
 ★   └─ Bedroom • Sonos Connect:Amp                                 ■■■······· 34%

   ♫ Office / Roam ::::: Shine — Meek Mill ::::::::::::::::::::::::::::::::::::::  23%
     └─ Office / Roam • Sonos Roam 2                                            23%
────────────────────────────────────────────────────────────────────────────────────────
     Shine — Meek Mill                     ◄◄  ♫  ►►                    Office / Roam
 ♫                   1:23 ━━━━━━━◆──────────────────────────── 4:01  ■■■······· 23%
                           ^^^^^^^^^^^^
                           pink → cyan gradient across filled region
────────────────────────────────────────────────────────────────────────────────────────

 Hot pink, cyan, neon green, electric yellow, lavender, gold.
 Gradient progress bar. Star cursor. Party logo. Vibes.
```

---

## Gradient Progress Bar — Implementation Sketch

### Theme Additions

```rust
pub struct Theme {
    // ... existing fields ...

    // Gradient endpoints for progress bar filled region.
    // When start == end, renders as solid color (backwards compatible).
    pub progress_gradient_start: Color,
    pub progress_gradient_end: Color,
}
```

### Rendering

In `progress_bar.rs`, when `start != end`, replace the single filled span with per-character spans:

```rust
fn gradient_color(start: Color, end: Color, t: f64) -> Color {
    // t in [0.0, 1.0]
    if let (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) = (start, end) {
        Color::Rgb(
            lerp(r1, r2, t),
            lerp(g1, g2, t),
            lerp(b1, b2, t),
        )
    } else {
        start // non-RGB colors don't interpolate
    }
}

fn lerp(a: u8, b: u8, t: f64) -> u8 {
    (a as f64 + (b as f64 - a as f64) * t) as u8
}
```

Each filled character at position `i` out of `filled_count` gets color `gradient_color(start, end, i as f64 / filled_count as f64)`.

When `start == end`, skip the loop and emit a single span (zero overhead for non-gradient themes).

## Key Decisions

1. **Replace all 4 existing themes** — new lineup is default, bw, minimal, dance_party.
2. **Default theme** keeps all standard glyphs, just upgrades the color palette for a polished aesthetic.
3. **B&W** uses grayscale (White/Gray/DarkGray/Black) with Bold/Underline for hierarchy — not strictly 2-color.
4. **Minimal** strips decorations: no tree connectors, no leaders, no tab brackets, lowercase logo.
5. **Dance party** uses fun glyphs (★, ♫, 💤) AND wild RGB colors AND a gradient progress bar.
6. **Gradient is theme-level** — any theme can set `progress_gradient_start`/`progress_gradient_end`. Same start/end = no gradient.
7. **Fallback theme** changes from `dark` → `default` in `Theme::from_name()`.

## Open Questions

None — all design directions were clarified during brainstorming.
