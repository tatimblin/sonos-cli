//! Modal overlay — centered bordered list for pickers and confirmations.
//!
//! Retained for future milestones (queue picker, etc.).

use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::tui::theme::Theme;

/// State for a modal overlay (e.g. group picker).
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct ModalState {
    pub title: String,
    pub items: Vec<String>,
    pub selected_index: usize,
}

/// Render a modal overlay centered in the terminal.
#[allow(dead_code)]
pub fn render_modal(frame: &mut Frame, area: Rect, modal: &ModalState, theme: &Theme) {
    let modal_width = 40.min(area.width.saturating_sub(4));
    let modal_height = (modal.items.len() as u16 + 4).min(area.height.saturating_sub(2));

    let [modal_area] = Layout::horizontal([Constraint::Length(modal_width)])
        .flex(Flex::Center)
        .areas(area);
    let [modal_area] = Layout::vertical([Constraint::Length(modal_height)])
        .flex(Flex::Center)
        .areas(modal_area);

    // Clear the background
    frame.render_widget(Clear, modal_area);

    let block = Block::default()
        .title(format!(" {} ", modal.title))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(theme.modal_border)
        .title_style(theme.modal_title);

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let lines: Vec<Line> = modal
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if i == modal.selected_index {
                theme.modal_selected
            } else {
                theme.speaker_name
            };
            let prefix = if i == modal.selected_index {
                " ▸ "
            } else {
                "   "
            };
            Line::from(vec![Span::styled(format!("{prefix}{item}"), style)])
        })
        .collect();

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}
