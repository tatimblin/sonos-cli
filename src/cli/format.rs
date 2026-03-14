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

/// Format milliseconds as M:SS or H:MM:SS.
pub fn format_time_ms(ms: u64) -> String {
    let total_secs = ms / 1000;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{}:{:02}", minutes, seconds)
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
}
