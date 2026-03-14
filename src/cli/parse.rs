use crate::errors::CliError;

/// Validate a seek time string (H:MM:SS or HH:MM:SS format).
pub fn validate_seek_time(input: &str) -> Result<(), CliError> {
    let parts: Vec<&str> = input.split(':').collect();
    if parts.len() != 3 {
        return Err(CliError::Validation(format!(
            "invalid position \"{input}\" — expected H:MM:SS format"
        )));
    }
    parts[0].parse::<u32>().map_err(|_| {
        CliError::Validation(format!(
            "invalid position \"{input}\" — hours must be a number"
        ))
    })?;
    let minutes: u32 = parts[1].parse().map_err(|_| {
        CliError::Validation(format!(
            "invalid position \"{input}\" — minutes must be a number"
        ))
    })?;
    let seconds: u32 = parts[2].parse().map_err(|_| {
        CliError::Validation(format!(
            "invalid position \"{input}\" — seconds must be a number"
        ))
    })?;
    if minutes > 59 {
        return Err(CliError::Validation(format!(
            "invalid position \"{input}\" — minutes must be 0–59"
        )));
    }
    if seconds > 59 {
        return Err(CliError::Validation(format!(
            "invalid position \"{input}\" — seconds must be 0–59"
        )));
    }
    Ok(())
}

/// Parse a duration string (e.g. "30m", "1h", "90m") into HH:MM:SS format.
pub fn parse_duration(input: &str) -> Result<String, CliError> {
    let (num_str, unit) = if let Some(s) = input.strip_suffix('m') {
        (s, 'm')
    } else if let Some(s) = input.strip_suffix('h') {
        (s, 'h')
    } else {
        return Err(CliError::Validation(format!(
            "invalid duration \"{input}\" — use a unit suffix: 30m or 1h"
        )));
    };

    let value: u32 = num_str.parse().map_err(|_| {
        CliError::Validation(format!(
            "invalid duration \"{input}\" — use a unit suffix: 30m or 1h"
        ))
    })?;

    let total_minutes = match unit {
        'h' => value * 60,
        'm' => value,
        _ => unreachable!(),
    };

    let hours = total_minutes / 60;
    let minutes = total_minutes % 60;
    Ok(format!("{hours:02}:{minutes:02}:00"))
}

/// Format a duration string for human-readable output.
pub fn format_duration_human(input: &str) -> String {
    if let Some(num) = input.strip_suffix('m') {
        if num == "1" {
            "1 minute".to_string()
        } else {
            format!("{num} minutes")
        }
    } else if let Some(num) = input.strip_suffix('h') {
        if num == "1" {
            "1 hour".to_string()
        } else {
            format!("{num} hours")
        }
    } else {
        input.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_seek_times() {
        assert!(validate_seek_time("0:00:00").is_ok());
        assert!(validate_seek_time("0:02:30").is_ok());
        assert!(validate_seek_time("1:30:00").is_ok());
        assert!(validate_seek_time("12:59:59").is_ok());
    }

    #[test]
    fn invalid_seek_time_bad_seconds() {
        let result = validate_seek_time("0:02:70");
        assert!(matches!(result, Err(CliError::Validation(ref s)) if s.contains("seconds")));
    }

    #[test]
    fn invalid_seek_time_bad_minutes() {
        let result = validate_seek_time("0:70:00");
        assert!(matches!(result, Err(CliError::Validation(ref s)) if s.contains("minutes")));
    }

    #[test]
    fn invalid_seek_time_bad_format() {
        assert!(validate_seek_time("2:30").is_err());
        assert!(validate_seek_time("abc").is_err());
    }

    #[test]
    fn parse_duration_minutes() {
        assert_eq!(parse_duration("30m").unwrap(), "00:30:00");
        assert_eq!(parse_duration("90m").unwrap(), "01:30:00");
        assert_eq!(parse_duration("1m").unwrap(), "00:01:00");
    }

    #[test]
    fn parse_duration_hours() {
        assert_eq!(parse_duration("1h").unwrap(), "01:00:00");
        assert_eq!(parse_duration("2h").unwrap(), "02:00:00");
    }

    #[test]
    fn parse_duration_invalid() {
        assert!(parse_duration("30").is_err());
        assert!(parse_duration("abc").is_err());
        assert!(parse_duration("").is_err());
    }

    #[test]
    fn format_duration_human_values() {
        assert_eq!(format_duration_human("30m"), "30 minutes");
        assert_eq!(format_duration_human("1m"), "1 minute");
        assert_eq!(format_duration_human("1h"), "1 hour");
        assert_eq!(format_duration_human("2h"), "2 hours");
    }
}
