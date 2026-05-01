//! Speaker list widget — renders grouped speakers with volume, playback state,
//! and pick-up/drop regrouping visuals.

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use sonos_sdk::PlaybackState;

use crate::tui::app::SpeakerListScreenState;
use crate::tui::hooks::RenderContext;
use crate::tui::theme::Theme;
use crate::tui::types::{build_display_order, build_list_entries, ListEntry};
use crate::tui::widgets::volume_bar;

/// Per-entry render data collected during the watch-subscription pass.
struct EntryRenderData {
    speaker_volume: Option<u16>,
    group_volume: Option<u16>,
    playback_state: Option<PlaybackState>,
    track_info: Option<String>,
}

/// Render the speaker list widget.
pub fn render(
    frame: &mut Frame,
    area: Rect,
    ctx: &mut RenderContext,
    state: &SpeakerListScreenState,
) {
    let speakers = ctx.app.system.speakers();

    if speakers.is_empty() {
        let paragraph = Paragraph::new("No speakers found")
            .alignment(ratatui::layout::Alignment::Center)
            .style(ctx.app.theme.muted);
        frame.render_widget(paragraph, area);
        return;
    }

    let entries = build_list_entries(&ctx.app.system);

    if entries.is_empty() {
        let paragraph = Paragraph::new("No speakers in group")
            .alignment(ratatui::layout::Alignment::Center)
            .style(ctx.app.theme.muted);
        frame.render_widget(paragraph, area);
        return;
    }

    let mut render_data: Vec<EntryRenderData> = Vec::with_capacity(entries.len());

    for entry in &entries {
        match entry {
            ListEntry::SpeakerRow(speaker_id) => {
                let speaker = ctx.app.system.speaker_by_id(speaker_id);
                let vol = speaker
                    .as_ref()
                    .and_then(|s| ctx.hooks.use_watch(&s.volume))
                    .map(|v| v.value() as u16);
                if let Some(ref s) = speaker {
                    ctx.hooks.use_watch(&s.group_membership);
                }
                render_data.push(EntryRenderData {
                    speaker_volume: vol,
                    group_volume: None,
                    playback_state: None,
                    track_info: None,
                });
            }
            ListEntry::GroupHeader(group_id) => {
                let group = ctx.app.system.group_by_id(group_id);
                let coordinator = group.as_ref().and_then(|g| g.coordinator());

                let gvol = group
                    .as_ref()
                    .and_then(|g| ctx.hooks.use_watch_group(&g.volume))
                    .map(|v| v.value());

                let pb = coordinator
                    .as_ref()
                    .and_then(|c| ctx.hooks.use_watch(&c.playback_state));

                let track = coordinator
                    .as_ref()
                    .and_then(|c| ctx.hooks.use_watch(&c.current_track))
                    .filter(|t| !t.is_empty())
                    .map(|t| {
                        let title = t.title.as_deref().unwrap_or("Unknown");
                        let artist = t.artist.as_deref().unwrap_or("Unknown");
                        format!("{title} \u{00b7} {artist}")
                    });

                render_data.push(EntryRenderData {
                    speaker_volume: None,
                    group_volume: gvol,
                    playback_state: pb,
                    track_info: track,
                });
            }
            ListEntry::UngroupedHeader => {
                render_data.push(EntryRenderData {
                    speaker_volume: None,
                    group_volume: None,
                    playback_state: None,
                    track_info: None,
                });
            }
        }
    }

    let selected_index = state.selected_index.min(entries.len().saturating_sub(1));
    let is_pick_up = state.pick_up.is_some();
    let pick_up_speaker_id = state.pick_up.as_ref().map(|p| p.speaker_id.clone());

    let display_order = build_display_order(&entries, &state.pick_up);

    let vol_width = 16.min(area.width.saturating_sub(50));

    let mut lines: Vec<Line> = Vec::new();

    for &orig_idx in &display_order {
        let entry = &entries[orig_idx];
        let data = &render_data[orig_idx];

        let is_selected = if is_pick_up {
            pick_up_speaker_id
                .as_ref()
                .is_some_and(|pid| matches!(entry, ListEntry::SpeakerRow(sid) if sid == pid))
        } else {
            orig_idx == selected_index
        };

        match entry {
            ListEntry::GroupHeader(group_id) => {
                let group_name = ctx
                    .app
                    .system
                    .group_by_id(group_id)
                    .and_then(|g| g.coordinator())
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| "Unknown Group".to_string());

                let (icon, icon_style) = match &data.playback_state {
                    Some(PlaybackState::Playing) => ("\u{25b6} ", ctx.app.theme.playing_icon),
                    Some(PlaybackState::Paused) => ("\u{23f8} ", ctx.app.theme.paused_icon),
                    _ => ("\u{25a0} ", ctx.app.theme.stopped_icon),
                };

                let track_info = data.track_info.as_deref().unwrap_or("");

                let name_style = if is_selected {
                    ctx.app.theme.speaker_cursor
                } else {
                    ctx.app.theme.group_header
                };

                let mut spans = vec![
                    Span::styled(icon, icon_style),
                    Span::styled(group_name, name_style),
                ];

                if !track_info.is_empty() {
                    spans.push(Span::raw("  "));
                    spans.push(Span::styled(
                        track_info.to_string(),
                        ctx.app.theme.track_info,
                    ));
                }

                if let Some(vol) = data.group_volume {
                    append_volume_spans(&mut spans, vol, is_selected, vol_width, &ctx.app.theme);
                }

                lines.push(Line::from(spans));
            }
            ListEntry::SpeakerRow(speaker_id) => {
                let speaker_name = ctx
                    .app
                    .system
                    .speaker_by_id(speaker_id)
                    .map(|s| s.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string());

                let cursor = if is_selected { "  \u{25b8} " } else { "    " };

                let name_style = if is_selected {
                    ctx.app.theme.speaker_cursor
                } else {
                    ctx.app.theme.speaker_name
                };

                let mut spans = vec![
                    Span::styled(cursor.to_string(), name_style),
                    Span::styled(speaker_name, name_style),
                ];

                if let Some(vol) = data.speaker_volume {
                    append_volume_spans(&mut spans, vol, is_selected, vol_width, &ctx.app.theme);
                }

                lines.push(Line::from(spans));
            }
            ListEntry::UngroupedHeader => {
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![Span::styled(
                    " NOT IN A GROUP ",
                    ctx.app.theme.group_header,
                )]));
            }
        }
    }

    if let Some(ref pick_up) = state.pick_up {
        let name = ctx
            .app
            .system
            .speaker_by_id(&pick_up.speaker_id)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "Speaker".to_string());
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![Span::styled(
            format!(" Moving {name} \u{2014} Space to drop, Esc to cancel"),
            ctx.app.theme.accent,
        )]));
    }

    if state.pick_up.is_none() {
        if let Some(ref msg) = ctx.app.status_message {
            lines.push(Line::raw(""));
            let style = if msg.starts_with("error:") {
                ctx.app.theme.error
            } else {
                ctx.app.theme.accent
            };
            lines.push(Line::from(vec![Span::styled(format!(" {msg}"), style)]));
        }
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

/// Append volume bar (when selected) or percentage text to a span list.
fn append_volume_spans(
    spans: &mut Vec<Span>,
    vol: u16,
    is_selected: bool,
    width: u16,
    theme: &Theme,
) {
    spans.push(Span::raw("  "));
    if is_selected {
        let vol_line =
            volume_bar::render_volume_bar(vol, width, theme.volume_filled, theme.volume_empty);
        spans.extend(vol_line.spans);
    } else {
        spans.push(Span::styled(format!("{vol}%"), theme.muted));
    }
}
