//! Album art widget — renders an image or themed placeholder at any size.
//!
//! Size-agnostic: renders whatever fits in the given `Rect`. The caller
//! provides a `StatefulProtocol` (from `ratatui-image`) for image rendering,
//! or `None` for a placeholder.

use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::StatefulImage;

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
