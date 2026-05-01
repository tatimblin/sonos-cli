//! Shared data helpers for TUI screens — extract common patterns into reusable functions.

use sonos_state::CurrentTrack;

/// Extract track title and artist as a "title — artist" string.
/// Returns `None` if the track is empty or missing.
pub fn track_summary(track: &Option<CurrentTrack>) -> Option<String> {
    track.as_ref().filter(|t| !t.is_empty()).map(|t| {
        let title = t.title.as_deref().unwrap_or("Unknown");
        let artist = t.artist.as_deref().unwrap_or("Unknown");
        format!("{title} \u{2014} {artist}")
    })
}
