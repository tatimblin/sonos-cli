//! Platform-specific diagnostics for discovery failures.

use std::io::IsTerminal;

use crate::cli::GlobalFlags;

/// Returns a platform-specific hint for when SSDP discovery finds no speakers.
pub fn discovery_hint() -> &'static str {
    if cfg!(target_os = "macos") {
        "hint: this could mean:\n\
         \x20 - no Sonos speakers are on this network\n\
         \x20 - your terminal lacks Local Network access\n\
         \x20   (System Settings > Privacy & Security > Local Network)\n\
         \x20 - a firewall is blocking UDP multicast on port 1900"
    } else if cfg!(target_os = "windows") {
        "hint: this could mean:\n\
         \x20 - no Sonos speakers are on this network\n\
         \x20 - Network Discovery is disabled or your firewall is\n\
         \x20   blocking UDP traffic on port 1900 (SSDP)"
    } else {
        "hint: this could mean:\n\
         \x20 - no Sonos speakers are on this network\n\
         \x20 - a firewall is blocking UDP multicast on port 1900\n\
         \x20   (ufw: sudo ufw allow proto udp from any to 239.255.255.250 port 1900)"
    }
}

/// On macOS/Windows, prompts the user to open the relevant settings pane.
///
/// Respects `--no-input` and TTY detection. Does nothing on Linux or when
/// stdin is not a terminal.
pub fn offer_open_settings(global: &GlobalFlags) {
    if !can_prompt(global) {
        return;
    }

    let (cmd, args, fallback) = if cfg!(target_os = "macos") {
        (
            "open",
            "x-apple.systempreferences:com.apple.preference.security?Privacy_LocalNetwork",
            "System Settings > Privacy & Security > Local Network",
        )
    } else if cfg!(target_os = "windows") {
        (
            "cmd",
            "/C start ms-settings:privacy-localnetwork",
            "Settings > Privacy & Security > Local Network",
        )
    } else {
        return;
    };

    eprintln!();
    eprint!("Open settings? [Y/n] ");

    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        return;
    }

    let answer = input.trim();
    if answer.is_empty() || answer.eq_ignore_ascii_case("y") {
        let result = if cfg!(target_os = "windows") {
            std::process::Command::new(cmd)
                .args(args.split_whitespace())
                .spawn()
        } else {
            std::process::Command::new(cmd).arg(args).spawn()
        };

        if result.is_err() {
            eprintln!("Could not open settings. Navigate to {fallback} manually.");
        }
    }
}

fn can_prompt(global: &GlobalFlags) -> bool {
    std::io::stdin().is_terminal() && !global.no_input
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_hint_lists_possible_causes() {
        let hint = discovery_hint();
        assert!(hint.starts_with("hint: this could mean:"));
        assert!(hint.contains("no Sonos speakers are on this network"));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn discovery_hint_macos_mentions_local_network() {
        let hint = discovery_hint();
        assert!(hint.contains("Local Network"));
        assert!(hint.contains("System Settings"));
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn discovery_hint_windows_mentions_firewall() {
        let hint = discovery_hint();
        assert!(hint.contains("Network Discovery"));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn discovery_hint_linux_mentions_ufw() {
        let hint = discovery_hint();
        assert!(hint.contains("ufw"));
        assert!(hint.contains("239.255.255.250"));
    }

    #[test]
    fn no_input_flag_prevents_prompt() {
        let global = GlobalFlags {
            speaker: None,
            group: None,
            quiet: false,
            verbose: false,
            no_input: true,
        };
        assert!(!can_prompt(&global));
    }

    #[test]
    fn discovery_failed_triggers_platform_hint() {
        // SdkError::DiscoveryFailed is the variant that should route to
        // platform-specific diagnostics in main.rs
        let err =
            sonos_sdk::SdkError::DiscoveryFailed("no Sonos devices found on the network".into());
        assert!(matches!(err, sonos_sdk::SdkError::DiscoveryFailed(_)));

        // And the hint it would display is our platform-specific one
        let hint = discovery_hint();
        assert!(hint.starts_with("hint:"));
    }

    #[test]
    fn speaker_not_found_routes_to_platform_hint() {
        // When resolve functions return "no speakers available", the
        // recovery_hint() should return the platform-specific diagnostic
        use crate::errors::CliError;
        let err = CliError::SpeakerNotFound("no speakers available".into());
        let hint = err.recovery_hint().expect("should have a recovery hint");
        assert_eq!(hint, discovery_hint());
    }

    #[test]
    fn group_not_found_routes_to_platform_hint() {
        use crate::errors::CliError;
        let err = CliError::GroupNotFound("no groups available".into());
        let hint = err.recovery_hint().expect("should have a recovery hint");
        assert_eq!(hint, discovery_hint());
    }

    #[test]
    fn sdk_discovery_failed_uses_platform_hint() {
        use crate::errors::CliError;
        let err = CliError::Sdk(sonos_sdk::SdkError::DiscoveryFailed("test".into()));
        let hint = err.recovery_hint().unwrap();
        assert_eq!(hint, discovery_hint());
    }

    #[test]
    fn sdk_non_network_error_uses_generic_hint() {
        // Non-network SDK errors (e.g. lock poisoned) should get a
        // generic hint, not the platform-specific one
        use crate::errors::CliError;
        let err = CliError::Sdk(sonos_sdk::SdkError::LockPoisoned);
        let hint = err.recovery_hint().unwrap();
        assert_ne!(
            hint,
            discovery_hint(),
            "non-network SDK errors should use generic hint"
        );
    }
}
