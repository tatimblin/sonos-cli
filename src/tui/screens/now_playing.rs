//! Group View > Now Playing tab — album art hero, track metadata, playback controls.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use sonos_sdk::{GroupId, PlaybackState};

use crate::tui::hooks::{ProgressState, RenderContext};
use crate::tui::widgets::album_art::{self, ArtProtocolState};
use crate::tui::widgets::{progress_bar, volume_bar};

/// Minimum content width to show album art. Below this, metadata-only layout.
const MIN_ART_WIDTH: u16 = 50;

/// Album art column width (including border).
const ART_COL_WIDTH: u16 = 24;

/// Render the Now Playing tab content.
pub fn render(frame: &mut Frame, area: Rect, ctx: &mut RenderContext, group_id: &GroupId) {
    let group = ctx.app.system.group_by_id(group_id);
    let coordinator = group.as_ref().and_then(|g| g.coordinator());

    if coordinator.is_none() {
        let paragraph = Paragraph::new("Group unavailable")
            .alignment(Alignment::Center)
            .style(ctx.app.theme.muted);
        frame.render_widget(paragraph, area);
        return;
    }
    let coordinator = coordinator.unwrap();
    let group = group.unwrap();

    // --- Hooks: use_watch (returns owned values, borrows released immediately) ---
    let playback_state = ctx.hooks.use_watch(&coordinator.playback_state);
    let current_track = ctx.hooks.use_watch(&coordinator.current_track);
    let position = ctx.hooks.use_watch(&coordinator.position);
    let group_volume = ctx.hooks.use_watch_group(&group.volume);

    let is_playing = playback_state
        .as_ref()
        .is_some_and(|s| *s == PlaybackState::Playing);

    // --- Hook: use_animation (before use_state) ---
    let group_id_str = group_id.to_string();
    ctx.hooks
        .use_animation(&format!("{group_id_str}:now_playing:tick"), is_playing);

    // --- Album art: request fetch if URI changed ---
    let art_uri = current_track
        .as_ref()
        .and_then(|t| t.album_art_uri.clone());

    if let Some(ref uri) = art_uri {
        ctx.app.image_loader.request(uri, coordinator.ip);
    }

    // --- Hook: use_state for progress interpolation (must be last) ---
    let progress_key = format!("{group_id_str}:now_playing:progress");
    let progress_state = ctx
        .hooks
        .use_state::<ProgressState>(&progress_key, ProgressState::default);

    if let Some(pos) = position.as_ref() {
        progress_state.update(pos.position_ms, pos.duration_ms, is_playing);
    } else {
        progress_state.is_playing = is_playing;
    }

    let elapsed_ms = progress_state.interpolated_position_ms();
    let duration_ms = progress_state.last_duration_ms;
    let progress = if duration_ms > 0 {
        elapsed_ms as f64 / duration_ms as f64
    } else {
        0.0
    };

    // Extract track metadata
    let (track_title, track_artist, track_album) = current_track
        .as_ref()
        .filter(|t| !t.is_empty())
        .map(|t| {
            (
                t.title.clone().unwrap_or_default(),
                t.artist.clone().unwrap_or_default(),
                t.album.clone().unwrap_or_default(),
            )
        })
        .unwrap_or_default();

    let volume = group_volume.map(|v| v.value()).unwrap_or(0);

    let playback_icon = match playback_state.as_ref() {
        Some(PlaybackState::Playing) => "▶",
        Some(PlaybackState::Paused) => "⏸",
        _ => "■",
    };

    // Speaker count text
    let members = group.members();
    let speaker_count_text = if members.len() <= 1 {
        coordinator.model_name.clone()
    } else {
        format!("{} + {} more", coordinator.model_name, members.len() - 1)
    };

    // --- Layout ---
    // Vertical: [top section (art + metadata)] [controls] [padding]
    let vertical = Layout::vertical([
        Constraint::Min(8),    // top section: art + metadata
        Constraint::Length(3), // playback controls + progress
        Constraint::Length(1), // bottom padding
    ])
    .split(area);

    let top_area = vertical[0];
    let controls_area = vertical[1];

    let show_art = area.width >= MIN_ART_WIDTH && ctx.app.picker.borrow().is_some();

    if show_art {
        // Horizontal: [album art] [gap] [metadata]
        let art_width = ART_COL_WIDTH.min(top_area.width / 2);
        let horizontal = Layout::horizontal([
            Constraint::Length(art_width),
            Constraint::Length(2), // gap
            Constraint::Min(20),  // metadata
        ])
        .split(top_area);

        render_art_column(frame, horizontal[0], ctx, &art_uri);
        render_metadata_column(
            frame,
            horizontal[2],
            ctx,
            &track_title,
            &track_artist,
            &track_album,
            volume,
            members.len(),
            &speaker_count_text,
        );
    } else {
        // No art — metadata centered
        render_metadata_column(
            frame,
            top_area,
            ctx,
            &track_title,
            &track_artist,
            &track_album,
            volume,
            members.len(),
            &speaker_count_text,
        );
    }

    // Playback controls + progress bar
    render_controls(
        frame,
        controls_area,
        ctx,
        playback_icon,
        progress,
        elapsed_ms,
        duration_ms,
    );
}

/// Render the album art column (left side).
fn render_art_column(
    frame: &mut Frame,
    area: Rect,
    ctx: &mut RenderContext,
    art_uri: &Option<String>,
) {
    let art_state = ctx
        .hooks
        .use_state::<ArtProtocolState>("now_playing:album_art", ArtProtocolState::default);
    art_state.ensure_protocol(art_uri, &ctx.app.image_loader, &ctx.app.picker);

    let border_style = ctx.app.theme.card_border;
    let placeholder_style = ctx.app.theme.muted;

    album_art::render_album_art(
        frame,
        area,
        art_state.protocol.as_mut(),
        border_style,
        placeholder_style,
    );
}

/// Render the track metadata column (right side).
#[allow(clippy::too_many_arguments)]
fn render_metadata_column(
    frame: &mut Frame,
    area: Rect,
    ctx: &RenderContext,
    title: &str,
    artist: &str,
    album: &str,
    volume: u16,
    speaker_count: usize,
    speaker_text: &str,
) {
    let theme = &ctx.app.theme;

    if area.height < 2 {
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    // Top padding
    lines.push(Line::raw(""));

    // Track title (bold)
    if title.is_empty() {
        lines.push(Line::from(Span::styled("Nothing playing", theme.muted)));
    } else {
        lines.push(Line::from(Span::styled(
            title.to_string(),
            theme.card_title,
        )));
    }

    // Artist
    if !artist.is_empty() {
        lines.push(Line::from(Span::styled(artist.to_string(), theme.track_info)));
    }

    // Album (muted)
    if !album.is_empty() {
        lines.push(Line::from(Span::styled(album.to_string(), theme.muted)));
    }

    // Gap
    lines.push(Line::raw(""));

    // Volume bar
    let vol_line = volume_bar::render_volume_bar(
        volume,
        area.width.min(40),
        theme.volume_filled,
        theme.volume_empty,
    );
    let mut vol_spans = vec![Span::raw("🔊  ")];
    vol_spans.extend(vol_line.spans);
    lines.push(Line::from(vol_spans));

    // Gap
    lines.push(Line::raw(""));

    // Speaker count
    lines.push(Line::from(vec![
        Span::raw("🔊×"),
        Span::styled(
            format!("{speaker_count}  "),
            theme.muted,
        ),
        Span::styled(speaker_text.to_string(), theme.muted),
    ]));

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

/// Render playback controls and progress bar.
fn render_controls(
    frame: &mut Frame,
    area: Rect,
    ctx: &RenderContext,
    playback_icon: &str,
    progress: f64,
    elapsed_ms: u64,
    duration_ms: u64,
) {
    let theme = &ctx.app.theme;

    if area.height < 2 {
        return;
    }

    // Line 1: centered playback icons
    let icon_style = match playback_icon {
        "▶" => theme.playing_icon,
        "⏸" => theme.paused_icon,
        _ => theme.stopped_icon,
    };
    let controls_line = Line::from(vec![
        Span::styled("⏮", theme.muted),
        Span::raw("     "),
        Span::styled(playback_icon, icon_style),
        Span::raw("     "),
        Span::styled("⏭", theme.muted),
    ]);
    let controls_paragraph = Paragraph::new(controls_line).alignment(Alignment::Center);
    let controls_row = Rect::new(area.x, area.y, area.width, 1);
    frame.render_widget(controls_paragraph, controls_row);

    // Line 2: progress bar with timestamps
    if area.height >= 2 {
        let elapsed_str = progress_bar::format_time(elapsed_ms);
        let duration_str = progress_bar::format_time(duration_ms);

        // Layout: "elapsed  ━━━━━━━╺────────  duration"
        let time_left_width = elapsed_str.len() + 2;
        let time_right_width = 2 + duration_str.len();
        let bar_width = (area.width as usize)
            .saturating_sub(time_left_width + time_right_width)
            .min(100);

        let bar_spans = progress_bar::render_bar_spans(
            progress,
            bar_width,
            Some("╺"),
            theme.progress_filled,
            theme.progress_cursor,
            theme.progress_empty,
        );
        let mut line_spans = vec![
            Span::styled(elapsed_str, theme.progress_time),
            Span::raw("  "),
        ];
        line_spans.extend(bar_spans);
        line_spans.push(Span::raw("  "));
        line_spans.push(Span::styled(duration_str, theme.progress_time));

        let bar_line = Line::from(line_spans);
        let bar_paragraph = Paragraph::new(bar_line).alignment(Alignment::Center);
        let bar_row = Rect::new(area.x, area.y + 1, area.width, 1);
        frame.render_widget(bar_paragraph, bar_row);
    }
}
