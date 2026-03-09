//! Error types for the sonos-cli application.

use std::process::ExitCode;
use thiserror::Error;

/// Domain error type with recovery hints and exit codes.
#[derive(Error, Debug)]
pub enum CliError {
    #[error("speaker \"{0}\" not found")]
    SpeakerNotFound(String),

    #[error("group \"{0}\" not found")]
    GroupNotFound(String),

    #[error("SDK error: {0}")]
    Sdk(#[from] sonos_sdk::SdkError),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("validation error: {0}")]
    Validation(String),
}

impl CliError {
    /// Returns actionable follow-up text for the user.
    pub fn recovery_hint(&self) -> Option<&str> {
        match self {
            Self::SpeakerNotFound(_) | Self::GroupNotFound(_) => {
                Some("Check that your speakers are on the same network, then retry.")
            }
            Self::Sdk(_) => Some("Check network connectivity and speaker power."),
            Self::Config(_) | Self::Validation(_) => None,
        }
    }

    /// Returns the appropriate exit code.
    pub fn exit_code(&self) -> ExitCode {
        match self {
            Self::Validation(_) => ExitCode::from(2), // usage error
            _ => ExitCode::from(1),                   // runtime error
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speaker_not_found_has_recovery_hint() {
        let err = CliError::SpeakerNotFound("Kitchen".to_string());
        assert!(err.recovery_hint().is_some());
        assert!(err.recovery_hint().unwrap().contains("same network"));
    }

    #[test]
    fn group_not_found_has_recovery_hint() {
        let err = CliError::GroupNotFound("Living Room".to_string());
        assert!(err.recovery_hint().is_some());
    }

    #[test]
    fn validation_error_has_no_hint() {
        let err = CliError::Validation("invalid volume".to_string());
        assert!(err.recovery_hint().is_none());
    }

    #[test]
    fn validation_error_returns_exit_code_2() {
        let err = CliError::Validation("bad input".to_string());
        assert_eq!(err.exit_code(), ExitCode::from(2));
    }

    #[test]
    fn runtime_errors_return_exit_code_1() {
        let err = CliError::SpeakerNotFound("x".to_string());
        assert_eq!(err.exit_code(), ExitCode::from(1));
    }
}
