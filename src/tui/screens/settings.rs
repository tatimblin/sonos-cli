//! Settings tab screen — data assembly, delegates rendering to widget.

use ratatui::layout::Rect;
use ratatui::Frame;

use crate::tui::hooks::RenderContext;
use crate::tui::types::{SettingsData, SettingsItem, SettingsRow};
use crate::tui::widgets::settings;

/// Render the Settings tab content.
pub fn render(frame: &mut Frame, area: Rect, ctx: &mut RenderContext) {
    let state = &ctx.app.navigation.settings_state;

    let items = SettingsRow::ALL
        .iter()
        .map(|row| SettingsItem {
            label: row.label(),
            value: row.current_value(&ctx.app.config),
            options: row.options(&ctx.app.system),
        })
        .collect();

    let data = SettingsData {
        items,
        selected_row: state.selected_row,
        dropdown_open: state.dropdown_open,
        dropdown_index: state.dropdown_index,
    };

    settings::render(frame, area, &data, &ctx.app.theme);
}
