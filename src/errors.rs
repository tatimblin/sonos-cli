//! Error types for the sonos-cli application.

use std::process::ExitCode;
use thiserror::Error;

use crate::diagnostics;

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

    /// The target can't do this, such as setting the sub level on a speaker
    /// with no sub bonded.
    #[error("{0}")]
    Unsupported(String),
}

impl CliError {
    /// Returns actionable follow-up text for the user.
    pub fn recovery_hint(&self) -> Option<&'static str> {
        match self {
            Self::SpeakerNotFound(_) | Self::GroupNotFound(_) => {
                Some(diagnostics::discovery_hint())
            }
            Self::Sdk(e) => {
                if matches!(e, sonos_sdk::SdkError::DiscoveryFailed(_)) || e.is_network_error() {
                    Some(diagnostics::discovery_hint())
                } else {
                    Some("Check network connectivity and speaker power.")
                }
            }
            Self::Config(_) | Self::Validation(_) | Self::Unsupported(_) => None,
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

/// Whether `err` is, or wraps, the SDK's discovery failure.
///
/// The TUI returns `anyhow::Error`, so the `SdkError` can sit anywhere in the
/// source chain — behind `.context(..)` or inside a `CliError::Sdk`. Matching
/// the variant replaces a search for "DiscoveryFailed" in the `Debug` text,
/// which any error that merely mentioned the word would also satisfy.
pub fn is_discovery_failure(err: &anyhow::Error) -> bool {
    err.chain().any(|e| {
        matches!(
            e.downcast_ref::<sonos_sdk::SdkError>(),
            Some(sonos_sdk::SdkError::DiscoveryFailed(_))
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sonos_sdk::SdkError;

    fn discovery_failed() -> SdkError {
        SdkError::DiscoveryFailed("no speakers responded".to_string())
    }

    #[test]
    fn discovery_failure_detected_directly() {
        assert!(is_discovery_failure(&anyhow::Error::from(
            discovery_failed()
        )));
    }

    #[test]
    fn discovery_failure_detected_behind_context() {
        let err = anyhow::Error::from(discovery_failed()).context("starting TUI");
        assert!(is_discovery_failure(&err));
    }

    #[test]
    fn discovery_failure_detected_inside_cli_error() {
        let err = anyhow::Error::from(CliError::from(discovery_failed()));
        assert!(is_discovery_failure(&err));
    }

    #[test]
    fn other_sdk_errors_are_not_discovery_failures() {
        assert!(!is_discovery_failure(&anyhow::Error::from(
            SdkError::LockPoisoned
        )));
    }

    /// The old check searched the `Debug` text, so a message that merely
    /// contained the word matched.
    #[test]
    fn mentioning_discovery_failed_is_not_enough() {
        let err = anyhow::anyhow!("DiscoveryFailed appears in this message");
        assert!(!is_discovery_failure(&err));
    }

    #[test]
    fn sdk_discovery_failure_gets_discovery_hint() {
        let err = CliError::from(discovery_failed());
        assert_eq!(err.recovery_hint(), Some(diagnostics::discovery_hint()));
    }

    #[test]
    fn speaker_not_found_has_recovery_hint() {
        let err = CliError::SpeakerNotFound("Kitchen".to_string());
        assert!(err.recovery_hint().is_some());
        assert!(err.recovery_hint().unwrap().contains("this could mean:"));
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
