---
title: "fix: mini-player album art full-size without border and square enforcement"
type: fix
status: active
date: 2026-05-02
---

# Fix: Mini-Player Album Art Sizing

## Overview

The bottom player bar album art renders too small because it's drawn inside a rounded border that consumes 2 columns and 2 rows from the 6x3 art area, leaving only 4x1 for the actual image. Additionally, when TV content is playing, the art URI may point to a non-square image (e.g. 16:9), which renders with awkward aspect ratio in the square art slot.

## Problem Statement

**Issue 1 — Border shrinks art:** `render_album_art()` wraps both images and placeholders in a `Block` with `Borders::ALL` + `BorderType::Rounded`. For the placeholder (music note icon), the border is the desired visual treatment. For actual images, it wastes space — the image should fill the entire 6x3 area.

**Issue 2 — Non-square TV art:** The `ImageLoader` stores raw decoded images with no preprocessing. TV content album art may be 16:9 or other non-square aspect ratios. `ratatui-image` will letterbox/pillarbox these within the rendering area, wasting pixels and looking odd.

## Proposed Solution

### 1. Remove border from image rendering path (`album_art.rs`)

In `render_album_art()`, when `protocol` is `Some`, render the `StatefulImage` directly into the full `area` — skip the `Block` entirely. Keep the existing border for the `None` (placeholder) path.

**File:** `src/tui/widgets/album_art.rs:78-91`

Before:
```rust
Some(proto) => {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width > 0 && inner.height > 0 {
        let image_widget = StatefulImage::new(None);
        frame.render_stateful_widget(image_widget, inner, proto);
    }
}
```

After:
```rust
Some(proto) => {
    let image_widget = StatefulImage::new(None);
    frame.render_stateful_widget(image_widget, area, proto);
}
```

The minimum-size guard at line 74 (`area.width < 3 || area.height < 3`) can be relaxed to `area.width == 0 || area.height == 0` since there's no border eating space anymore. However, `ratatui-image` handles zero-size areas gracefully, so the guard mainly serves to skip unnecessary work.

### 2. Square-crop images after decode (`image_loader.rs`)

In `fetch_and_decode()`, after `image::load_from_memory()` succeeds, center-crop the image to a square using `min(width, height)` as the side length.

**File:** `src/tui/image_loader.rs:168-191`

Add a square-crop step after decode:
```rust
fn fetch_and_decode(agent: &ureq::Agent, url: &str) -> Option<DynamicImage> {
    // ... existing fetch logic ...
    
    let img = image::load_from_memory(&body)
        .map_err(|e| {
            tracing::debug!("Album art decode failed for {url}: {e}");
            e
        })
        .ok()?;

    Some(crop_to_square(img))
}

fn crop_to_square(img: DynamicImage) -> DynamicImage {
    let (w, h) = (img.width(), img.height());
    if w == h {
        return img;
    }
    let side = w.min(h);
    let x = (w - side) / 2;
    let y = (h - side) / 2;
    img.crop_imm(x, y, side, side)
}
```

This runs in the background worker thread, so it adds no latency to the render loop. The cropped image is what gets cached and passed to `Picker::new_resize_protocol()`.

## Acceptance Criteria

- [ ] Album art in the bottom bar renders without a visible border, filling the full 6x3 character area
- [ ] Placeholder (no image) retains its rounded border and centered music note
- [ ] TV content art (non-square source) renders as a square (center-cropped)
- [ ] Standard album art (already square) is unaffected by the crop
- [ ] All three responsive layouts (wide, narrow, minimal) work correctly
- [ ] Art on the Now Playing screen (hero size) is not regressed — it also uses `render_album_art()` and benefits from both fixes

## Technical Considerations

- **`render_album_art` is shared.** Both the bottom bar and any future Now Playing screen call it. Removing the border for the image path improves both callsites. The placeholder path is unchanged.
- **Crop in the loader, not the renderer.** Cropping in `fetch_and_decode()` means the image is square before it enters the cache, before protocol encoding, and for all consumers. No need to crop per-render-site.
- **`image::crop_imm` is non-destructive.** It returns a new `DynamicImage` backed by a view, which is efficient. The original is dropped immediately after.
- **No API change to `render_album_art`.** The `border_style` parameter remains for the placeholder path. Callers don't need to change.

## Files to Modify

1. `src/tui/widgets/album_art.rs` — Remove border from `Some(proto)` branch in `render_album_art()`
2. `src/tui/image_loader.rs` — Add `crop_to_square()` helper, call it in `fetch_and_decode()`
