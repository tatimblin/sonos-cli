//! Album art widget — renders an image or themed placeholder at any size.
//!
//! Size-agnostic: renders whatever fits in the given `Rect`. The caller
//! provides a `StatefulProtocol` (from `ratatui-image`) for image rendering,
//! or `None` for a placeholder. Used at 20×20 in Now Playing, 3×3 in the
//! mini-player, and potentially 1×1 in the queue.

use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
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
    border_style: Style,
    placeholder_style: Style,
) {
    if area.width < 3 || area.height < 3 {
        return;
    }

    match protocol {
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
        None => {
            render_placeholder(frame, area, border_style, placeholder_style);
        }
    }
}

/// Render a placeholder box with a centered music note.
fn render_placeholder(
    frame: &mut Frame,
    area: Rect,
    border_style: Style,
    text_style: Style,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    // Center the music note vertically and horizontally
    let note = "♪";
    let center_y = inner.height / 2;
    let note_area = Rect::new(inner.x, inner.y + center_y, inner.width, 1);
    let paragraph = Paragraph::new(Line::from(note))
        .alignment(Alignment::Center)
        .style(text_style);
    frame.render_widget(paragraph, note_area);
}
