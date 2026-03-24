//! Home > Speakers tab — all speakers organized by group.

use ratatui::layout::{Alignment, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::tui::app::{App, HomeSpeakersState};
use crate::tui::widgets::{modal, volume_bar};

/// Render the Speakers tab content.
pub fn render(frame: &mut Frame, area: Rect, app: &App, state: &HomeSpeakersState) {
    let speakers = app.system.speakers();

    if speakers.is_empty() {
        let paragraph = Paragraph::new("No speakers found")
            .alignment(Alignment::Center)
            .style(app.theme.muted);
        frame.render_widget(paragraph, area);
        return;
    }

    let groups = app.system.groups();

    let mut lines: Vec<Line> = Vec::new();
    let mut flat_index: usize = 0;

    // Multi-member groups first
    for group in &groups {
        if group.is_standalone() {
            continue;
        }

        let coordinator_name = group
            .coordinator()
            .map(|c| c.name.clone())
            .unwrap_or_else(|| format!("Group {}", group.id));

        // Group header
        lines.push(Line::from(vec![Span::styled(
            format!(" {coordinator_name} "),
            app.theme.group_header,
        )]));
        lines.push(Line::raw("")); // spacing

        for member in group.members() {
            let cursor = if flat_index == state.selected_index {
                "▸ "
            } else {
                "  "
            };
            let role_suffix = if group.is_coordinator(&member.id) {
                " (coordinator)"
            } else {
                ""
            };

            let volume = member.volume.get().map(|v| v.value() as u16).unwrap_or(0);
            let vol_line = volume_bar::render_volume_bar(
                volume,
                20.min(area.width.saturating_sub(40)),
                app.theme.volume_filled,
                app.theme.volume_empty,
            );

            let cursor_style = if flat_index == state.selected_index {
                app.theme.speaker_cursor
            } else {
                app.theme.speaker_name
            };

            let mut spans = vec![
                Span::styled(cursor.to_string(), cursor_style),
                Span::styled(
                    format!("{}{role_suffix}", member.name),
                    cursor_style,
                ),
                Span::raw("  "),
            ];
            spans.extend(vol_line.spans);

            lines.push(Line::from(spans));
            flat_index += 1;
        }

        lines.push(Line::raw("")); // spacing between groups
    }

    // Standalone groups → "NOT IN A GROUP" section
    let standalone_speakers: Vec<_> = groups
        .iter()
        .filter(|g| g.is_standalone())
        .filter_map(|g| g.coordinator())
        .collect();

    if !standalone_speakers.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            " NOT IN A GROUP ",
            app.theme.group_header,
        )]));
        lines.push(Line::raw(""));

        for speaker in &standalone_speakers {
            let cursor = if flat_index == state.selected_index {
                "▸ "
            } else {
                "  "
            };

            let volume = speaker.volume.get().map(|v| v.value() as u16).unwrap_or(0);
            let vol_line = volume_bar::render_volume_bar(
                volume,
                20.min(area.width.saturating_sub(40)),
                app.theme.volume_filled,
                app.theme.volume_empty,
            );

            let cursor_style = if flat_index == state.selected_index {
                app.theme.speaker_cursor
            } else {
                app.theme.speaker_name
            };

            let mut spans = vec![
                Span::styled(cursor.to_string(), cursor_style),
                Span::styled(
                    format!("{} — {}", speaker.name, speaker.model_name),
                    cursor_style,
                ),
                Span::raw("  "),
            ];
            spans.extend(vol_line.spans);

            lines.push(Line::from(spans));
            flat_index += 1;
        }
    }

    // Render status message if present
    if let Some(ref msg) = app.status_message {
        lines.push(Line::raw(""));
        let style = if msg.starts_with("error:") {
            app.theme.error
        } else {
            app.theme.accent
        };
        lines.push(Line::from(vec![Span::styled(format!(" {msg}"), style)]));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);

    // Render modal overlay if present
    if let Some(ref modal_state) = state.modal {
        modal::render_modal(frame, area, modal_state, &app.theme);
    }
}
