//! Speakers tab screen — data assembly + widget composition.

use ratatui::layout::Rect;
use ratatui::Frame;

use crate::tui::hooks::RenderContext;
use crate::tui::widgets::speaker_list;

/// Render the Speakers tab content.
pub fn render(frame: &mut Frame, area: Rect, ctx: &mut RenderContext) {
    let state = ctx.app.navigation.speakers_state.clone();
    speaker_list::render(frame, area, ctx, &state);
}
