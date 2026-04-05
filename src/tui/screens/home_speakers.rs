//! Home > Speakers tab — delegates to the shared speaker_list widget.

use ratatui::layout::Rect;
use ratatui::Frame;

use crate::tui::app::SpeakerListScreenState;
use crate::tui::hooks::RenderContext;
use crate::tui::widgets::speaker_list::{self, SpeakerListMode};

/// Render the Speakers tab content.
pub fn render(
    frame: &mut Frame,
    area: Rect,
    ctx: &mut RenderContext,
    state: &SpeakerListScreenState,
) {
    let mode = SpeakerListMode::FullList;
    speaker_list::render(frame, area, ctx, &mode, state);
}
