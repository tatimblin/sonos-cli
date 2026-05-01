//! Settings tab screen — data assembly, delegates rendering to widget.

use ratatui::layout::Rect;
use ratatui::Frame;

use crate::tui::handlers::settings::{ALBUM_ART_OPTIONS, AUTO_GROUP_LABEL, THEME_OPTIONS};
use crate::tui::hooks::RenderContext;
use crate::tui::types::{SettingsData, SettingsItem};
use crate::tui::widgets::settings;

/// Render the Settings tab content.
pub fn render(frame: &mut Frame, area: Rect, ctx: &mut RenderContext) {
    let state = &ctx.app.navigation.settings_state;

    // Build group option list from live speaker topology
    let mut group_options = vec![AUTO_GROUP_LABEL.to_string()];
    for group in ctx.app.system.groups() {
        if let Some(coordinator) = group.coordinator() {
            group_options.push(coordinator.name.clone());
        }
    }

    let items = vec![
        SettingsItem {
            label: "Theme",
            value: ctx.app.config.theme.clone(),
            options: THEME_OPTIONS.iter().map(|s| s.to_string()).collect(),
        },
        SettingsItem {
            label: "Default group",
            value: ctx
                .app
                .config
                .default_group
                .clone()
                .unwrap_or_else(|| AUTO_GROUP_LABEL.to_string()),
            options: group_options,
        },
        SettingsItem {
            label: "Album art",
            value: ctx.app.config.album_art_mode.to_string(),
            options: ALBUM_ART_OPTIONS.iter().map(|s| s.to_string()).collect(),
        },
    ];

    let data = SettingsData {
        items,
        selected_row: state.selected_row,
        dropdown_open: state.dropdown_open,
        dropdown_index: state.dropdown_index,
        status_message: ctx.app.status_message.clone(),
    };

    settings::render(frame, area, &data, &ctx.app.theme);
}
