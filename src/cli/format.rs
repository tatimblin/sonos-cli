/// Format a playback state as an icon string.
pub fn playback_icon(state: &sonos_sdk::PlaybackState) -> &'static str {
    match state {
        sonos_sdk::PlaybackState::Playing => "\u{25b6}",
        sonos_sdk::PlaybackState::Paused => "\u{23f8}",
        _ => "\u{25a0}",
    }
}

/// Format a playback state as a word.
pub fn playback_label(state: &sonos_sdk::PlaybackState) -> &'static str {
    match state {
        sonos_sdk::PlaybackState::Playing => "Playing",
        sonos_sdk::PlaybackState::Paused => "Paused",
        sonos_sdk::PlaybackState::Stopped => "Stopped",
        sonos_sdk::PlaybackState::Transitioning => "Transitioning",
    }
}

/// Format a playback state as icon + word, e.g. "▶ Playing".
pub fn state_display(state: &sonos_sdk::PlaybackState) -> String {
    format!("{} {}", playback_icon(state), playback_label(state))
}

/// Format the current track for read commands (`status`, `groups`).
///
/// A track with no title or artist is shown as "Playing (no metadata)" when
/// something is actually loaded — e.g. a radio stream — rather than the SDK's
/// "Unknown". An empty URI or an `x-rincon:` URI (a grouped member following
/// its coordinator) means nothing of its own is loaded, so "Unknown" stays.
pub fn track_display(track: &sonos_sdk::CurrentTrack) -> String {
    let d = track.display();
    if d != "Unknown" {
        return d;
    }
    track
        .uri
        .as_deref()
        .filter(|u| !u.is_empty() && !u.starts_with("x-rincon:"))
        .map(|_| "Playing (no metadata)".to_string())
        .unwrap_or(d)
}

/// Format milliseconds as M:SS or H:MM:SS.
pub fn format_time_ms(ms: u64) -> String {
    let total_secs = ms / 1000;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_time_ms_values() {
        assert_eq!(format_time_ms(0), "0:00");
        assert_eq!(format_time_ms(151_000), "2:31");
        assert_eq!(format_time_ms(355_000), "5:55");
        assert_eq!(format_time_ms(3_661_000), "1:01:01");
    }

    #[test]
    fn state_display_values() {
        use sonos_sdk::PlaybackState;
        assert_eq!(state_display(&PlaybackState::Playing), "\u{25b6} Playing");
        assert_eq!(state_display(&PlaybackState::Paused), "\u{23f8} Paused");
        assert_eq!(state_display(&PlaybackState::Stopped), "\u{25a0} Stopped");
        assert_eq!(
            state_display(&PlaybackState::Transitioning),
            "\u{25a0} Transitioning"
        );
    }

    fn track(
        artist: Option<&str>,
        title: Option<&str>,
        uri: Option<&str>,
    ) -> sonos_sdk::CurrentTrack {
        sonos_sdk::CurrentTrack {
            artist: artist.map(Into::into),
            title: title.map(Into::into),
            uri: uri.map(Into::into),
            ..Default::default()
        }
    }

    #[test]
    fn track_display_uses_metadata_when_present() {
        assert_eq!(
            track_display(&track(Some("Artist"), Some("Song"), Some("x-file:1"))),
            "Artist - Song"
        );
        assert_eq!(track_display(&track(None, Some("Song"), None)), "Song");
    }

    #[test]
    fn track_display_flags_loaded_stream_without_metadata() {
        assert_eq!(
            track_display(&track(None, None, Some("x-sonosapi-stream:s123"))),
            "Playing (no metadata)"
        );
    }

    #[test]
    fn track_display_keeps_unknown_when_nothing_loaded() {
        assert_eq!(track_display(&track(None, None, None)), "Unknown");
        assert_eq!(track_display(&track(None, None, Some(""))), "Unknown");
        assert_eq!(
            track_display(&track(None, None, Some("x-rincon:RINCON_123"))),
            "Unknown"
        );
    }
}
