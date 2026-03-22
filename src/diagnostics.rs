//! Platform-specific diagnostics for discovery failures.

use std::io::IsTerminal;

use crate::cli::GlobalFlags;

/// Returns a platform-specific hint for when SSDP discovery finds no speakers.
pub fn discovery_hint() -> &'static str {
    if cfg!(target_os = "macos") {
        "hint: on macOS, your terminal needs Local Network access to discover speakers.\n\
         \x20     Check System Settings > Privacy & Security > Local Network."
    } else if cfg!(target_os = "windows") {
        "hint: on Windows, ensure Network Discovery is enabled and your firewall\n\
         \x20     allows UDP traffic on port 1900 (SSDP)."
    } else {
        "hint: ensure your firewall allows UDP multicast on port 1900 (SSDP).\n\
         \x20     For ufw: sudo ufw allow proto udp from any to 239.255.255.250 port 1900"
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
