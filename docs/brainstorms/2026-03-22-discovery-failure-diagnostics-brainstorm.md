# Discovery Failure Diagnostics

**Date:** 2026-03-22
**Status:** Draft
**Relates to:** Milestone 2 (CLI core infrastructure)

## What We're Building

Platform-specific diagnostic hints when SSDP discovery finds zero speakers. When no speakers are discovered, the CLI detects the OS and presents an actionable hint explaining the most likely cause — then offers to open the relevant settings pane (where possible) so the user can fix it.

### The Problem

A new user installs `sonos` on a machine where the terminal lacks network permissions (macOS Local Network privacy, Windows Firewall, Linux firewall rules). They run a command. SSDP multicast packets are silently dropped by the OS. The CLI says "No speakers found" with no indication that the problem is permissions, not missing speakers.

### Target Behavior

When discovery returns zero speakers:

1. Print a platform-specific hint to stderr explaining the likely cause
2. On macOS and Windows, prompt `Open settings? [Y/n]` and auto-open the relevant settings pane if confirmed
3. On Linux, print troubleshooting text only (no unified settings app to open)

**Example (macOS):**
```
No speakers found.

hint: On macOS, your terminal needs Local Network access to discover speakers.
      Check System Settings > Privacy & Security > Local Network.

Open settings? [Y/n]
```

**Example (Windows):**
```
No speakers found.

hint: On Windows, ensure Network Discovery is enabled and your firewall
      allows UDP traffic on port 1900 (SSDP).

Open settings? [Y/n]
```

**Example (Linux):**
```
No speakers found.

hint: Ensure your firewall allows UDP multicast on port 1900 (SSDP).
      For ufw: sudo ufw allow proto udp from any to 239.255.255.250 port 1900
```

## Why This Approach

- **Platform detection is trivial in Rust** — `cfg!(target_os = "...")` at compile time or `std::env::consts::OS` at runtime
- **Keeps the SDK clean** — diagnostic hints and settings-opening are CLI UX concerns, not SDK concerns. The SDK already returns empty speaker lists or `DiscoveryFailed` errors; the CLI interprets those for the user
- **Auto-open where possible** — reduces friction on macOS and Windows where a single settings pane can fix the problem. Linux gets text-only since there's no universal settings app
- **Ask before opening** — prompting `Open settings? [Y/n]` is polite and expected for a CLI tool that launches external apps

## Key Decisions

1. **This lives in sonos-cli, not sonos-sdk.** The SDK reports discovery results. The CLI decides how to present failures to the user. Platform-specific UX hints are a CLI concern.

2. **Hint on zero speakers only.** Don't show the hint when speakers are found but a specific speaker/group isn't — that's a different problem (typo, speaker offline).

3. **Platform-specific hints, not generic.** A generic "check your network" message isn't actionable. Each OS has a specific, common cause and a specific place to fix it.

4. **Prompt before auto-open.** The CLI asks `Open settings? [Y/n]` rather than silently launching a settings pane. Default is Yes for low friction.

5. **stderr for all diagnostic output.** Hints and prompts go to stderr, consistent with the project's convention that stdout is for command output only.

## Open Questions

None — scope and approach are clear.

## Platform Details

| Platform | Likely cause | Hint text | Auto-open target |
|----------|-------------|-----------|-----------------|
| macOS | Local Network privacy permission | "your terminal needs Local Network access" | `open "x-apple.systempreferences:com.apple.preference.security?Privacy_LocalNetwork"` |
| Windows | Firewall / Network Discovery | "ensure Network Discovery is enabled and firewall allows UDP 1900" | `start ms-settings:network` or equivalent |
| Linux | Firewall (ufw/iptables/nftables) | "ensure your firewall allows UDP multicast on port 1900" | Text-only (no auto-open) |
