//! Track progress bar utilities.

/// Format milliseconds as `M:SS` or `H:MM:SS` for tracks over 1 hour.
pub fn format_time(ms: u64) -> String {
    if ms == 0 {
        return "--:--".to_string();
    }
    let total_seconds = ms / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

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
    fn format_time_zero() {
        assert_eq!(format_time(0), "--:--");
    }

    #[test]
    fn format_time_minutes() {
        assert_eq!(format_time(151_000), "2:31");
    }

    #[test]
    fn format_time_hours() {
        assert_eq!(format_time(3_661_000), "1:01:01");
    }

    #[test]
    fn format_time_under_minute() {
        assert_eq!(format_time(45_000), "0:45");
    }
}
