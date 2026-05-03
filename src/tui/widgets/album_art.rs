//! Album art widget — renders an image or themed placeholder at any size.
//!
//! Size-agnostic: renders whatever fits in the given `Rect`. The caller
//! provides a `StatefulProtocol` (from `ratatui-image`) for image rendering,
//! or `None` for a placeholder.

use std::cell::RefCell;

use image::imageops::FilterType;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::StatefulImage;

use crate::tui::image_loader::ImageLoader;

const ART_COLS: u16 = 6;
const ART_ROWS: u16 = 3;

/// Hook-friendly state for album art protocol lifecycle.
///
/// Tracks the current album art URI and holds the `StatefulProtocol` used for
/// rendering. Detects URI changes, invalidates stale protocols, and lazily
/// creates new ones from the image cache.
#[derive(Default)]
pub struct ArtProtocolState {
    uri: Option<String>,
    pub protocol: Option<StatefulProtocol>,
}

impl ArtProtocolState {
    /// Update protocol when URI changes. Creates protocol from cached image.
    ///
    /// Call this each render frame with the current `art_uri`. Handles:
    /// - URI change detection (invalidates old protocol)
    /// - Lazy protocol creation from `ImageLoader` cache + `Picker`
    pub fn ensure_protocol(
        &mut self,
        art_uri: &Option<String>,
        image_loader: &ImageLoader,
        picker: &RefCell<Option<Picker>>,
    ) {
        let uri_changed = self.uri.as_deref() != art_uri.as_deref();
        if uri_changed {
            self.uri = art_uri.clone();
            self.protocol = None;
        }

        if self.protocol.is_none() {
            if let Some(ref uri) = art_uri {
                if let Some(img) = image_loader.get(uri) {
                    if let Some(ref mut p) = *picker.borrow_mut() {
                        let (fw, fh) = p.font_size();
                        let target_w = (ART_COLS * fw) as u32;
                        let target_h = (ART_ROWS * fh) as u32;
                        let resized = img.resize_exact(target_w, target_h, FilterType::Lanczos3);
                        self.protocol = Some(p.new_resize_protocol(resized));
                    }
                }
            }
        }
    }
}

/// Render album art or a placeholder within the given area.
///
/// When `protocol` is `Some`, renders the image using the terminal's graphics
/// protocol (Sixel, Kitty, iTerm2, or halfblocks). When `None`, renders a
/// bordered placeholder with a music note.
pub fn render_album_art(
    frame: &mut Frame,
    area: Rect,
    protocol: Option<&mut StatefulProtocol>,
    background_style: Style,
    music_note: &str,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    match protocol {
        Some(proto) => {
            let image_widget = StatefulImage::new(None);
            frame.render_stateful_widget(image_widget, area, proto);
        }
        None => {
            render_placeholder(frame, area, background_style, music_note);
        }
    }
}

fn render_placeholder(frame: &mut Frame, area: Rect, bg_style: Style, note: &str) {
    let block = Block::default().style(bg_style);
    frame.render_widget(block, area);

    let center_y = area.height / 2;
    let note_area = Rect::new(area.x, area.y + center_y, area.width, 1);
    let paragraph = Paragraph::new(Line::from(note.to_string()))
        .alignment(Alignment::Center)
        .style(bg_style);
    frame.render_widget(paragraph, note_area);
}
