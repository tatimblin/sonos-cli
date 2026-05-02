//! Shared data helpers for TUI screens — extract common patterns into reusable functions.

use sonos_state::CurrentTrack;

pub fn uri_source_label(uri: &str) -> &str {
    if uri.starts_with("x-rincon:") {
        ""
    } else if uri.is_empty() {
        "Unknown"
    } else {
        "Playing (no metadata)"
    }
}

/// Extract track title and artist as a "title — artist" string.
/// Returns `None` if the track is empty or missing.
pub fn track_summary(track: &Option<CurrentTrack>) -> Option<String> {
    track.as_ref().filter(|t| !t.is_empty()).map(|t| {
        match (&t.title, &t.artist) {
            (Some(title), Some(artist)) => format!("{title} \u{2014} {artist}"),
            (Some(title), None) => title.clone(),
            (None, Some(artist)) => artist.clone(),
            (None, None) => t
                .uri
                .as_deref()
                .map(uri_source_label)
                .filter(|s| !s.is_empty())
                .unwrap_or("Unknown")
                .to_string(),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uri_source_label_rincon() {
        assert_eq!(uri_source_label("x-rincon:RINCON_123"), "");
    }

    #[test]
    fn test_uri_source_label_empty() {
        assert_eq!(uri_source_label(""), "Unknown");
    }

    #[test]
    fn test_uri_source_label_spotify() {
        assert_eq!(
            uri_source_label("x-sonos-spotify:spotify:track:abc"),
            "Playing (no metadata)"
        );
    }

    #[test]
    fn test_uri_source_label_http() {
        assert_eq!(
            uri_source_label("http://stream.example.com/radio"),
            "Playing (no metadata)"
        );
    }

    #[test]
    fn test_track_summary_both_fields() {
        let track = Some(CurrentTrack {
            title: Some("Song".into()),
            artist: Some("Artist".into()),
            album: None,
            album_art_uri: None,
            uri: None,
        });
        assert_eq!(track_summary(&track), Some("Song \u{2014} Artist".into()));
    }

    #[test]
    fn test_track_summary_title_only() {
        let track = Some(CurrentTrack {
            title: Some("Song".into()),
            artist: None,
            album: None,
            album_art_uri: None,
            uri: Some("x-sonos-spotify:abc".into()),
        });
        assert_eq!(track_summary(&track), Some("Song".into()));
    }

    #[test]
    fn test_track_summary_uri_fallback() {
        let track = Some(CurrentTrack {
            title: None,
            artist: None,
            album: None,
            album_art_uri: None,
            uri: Some("x-sonos-vli:abc".into()),
        });
        assert_eq!(
            track_summary(&track),
            Some("Playing (no metadata)".into())
        );
    }

    #[test]
    fn test_track_summary_rincon_uri_shows_unknown() {
        let track = Some(CurrentTrack {
            title: None,
            artist: None,
            album: None,
            album_art_uri: None,
            uri: Some("x-rincon:RINCON_123".into()),
        });
        assert_eq!(track_summary(&track), Some("Unknown".into()));
    }

    #[test]
    fn test_track_summary_empty() {
        let track = Some(CurrentTrack {
            title: None,
            artist: None,
            album: None,
            album_art_uri: None,
            uri: None,
        });
        assert_eq!(track_summary(&track), None);
    }

    #[test]
    fn test_track_summary_none() {
        assert_eq!(track_summary(&None), None);
    }
}
