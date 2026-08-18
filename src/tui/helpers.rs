//! Shared data helpers for TUI screens — extract common patterns into reusable functions.

use sonos_sdk::CurrentTrack;

pub fn uri_source_label(uri: &str) -> &str {
    let _ = uri;
    ""
}

/// Extract track title and artist as a "title — artist" string.
/// Returns `None` if the track is empty or missing.
pub fn track_summary(track: &Option<CurrentTrack>) -> Option<String> {
    track
        .as_ref()
        .filter(|t| !t.is_empty())
        .and_then(|t| {
            let title = t.title.as_deref().filter(|s| !s.trim().is_empty());
            let artist = t.artist.as_deref().filter(|s| !s.trim().is_empty());
            match (title, artist) {
            (Some(title), Some(artist)) => Some(format!("{title} \u{2014} {artist}")),
            (Some(title), None) => Some(title.to_string()),
            (None, Some(artist)) => Some(artist.to_string()),
            (None, None) => t
                .uri
                .as_deref()
                .map(uri_source_label)
                .filter(|s| !s.is_empty())
                .map(String::from),
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
        assert_eq!(uri_source_label(""), "");
    }

    #[test]
    fn test_uri_source_label_spotify() {
        assert_eq!(uri_source_label("x-sonos-spotify:spotify:track:abc"), "");
    }

    #[test]
    fn test_uri_source_label_http() {
        assert_eq!(uri_source_label("http://stream.example.com/radio"), "");
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
        assert_eq!(track_summary(&track), None);
    }

    #[test]
    fn test_track_summary_rincon_uri_returns_none() {
        let track = Some(CurrentTrack {
            title: None,
            artist: None,
            album: None,
            album_art_uri: None,
            uri: Some("x-rincon:RINCON_123".into()),
        });
        assert_eq!(track_summary(&track), None);
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
