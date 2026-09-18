//! End-to-end liveness reporting for the read-only commands.
//!
//! The regression these guard against: on a machine where SSDP is completely
//! broken, `sonos speakers` printed five speaker names and exited 0. The names
//! came from a three-week-old disk cache and no speaker was contacted. The
//! assertion that matters below is therefore *both* halves at once — the names
//! are still printed (the output contract is unchanged) *and* the result is
//! marked `NothingContacted`.
//!
//! Every speaker here points at a loopback port with no listener, so each
//! `fetch()` fails with an instant ECONNREFUSED instead of waiting out
//! soap-client's 5s connect timeout. That is the observed machine's state,
//! minus the wait. Using the SDK's `with_speakers` helper instead would
//! hardcode `192.168.1.100+`, which may be routable on the developer's LAN and
//! would hang.
#![cfg(feature = "test-helpers")]

use std::net::TcpListener;

use sonos_sdk::sonos_discovery::Device;
use sonos_sdk::{GroupId, SonosSystem, SpeakerId};

use sonos_cli::cli::{run_command, Commands, GlobalFlags};
use sonos_cli::config::Config;
use sonos_cli::liveness::{disposition, Liveness};

/// A loopback port with nothing listening on it.
///
/// Bound and immediately dropped so the port is known to be closed rather than
/// merely assumed to be — connecting to it refuses instantly.
fn closed_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback bind");
    listener.local_addr().expect("local addr").port()
}

/// Speakers that will never answer.
///
/// `Device::port` is recorded but not used: the SDK addresses every speaker at
/// a hardcoded 1400, so what makes these unreachable is that nothing is
/// listening on loopback at all.
fn devices_at_localhost(names: &[&str]) -> Vec<Device> {
    let port = closed_port();
    names
        .iter()
        .enumerate()
        .map(|(i, name)| Device {
            id: format!("RINCON_{i:03}"),
            name: (*name).to_string(),
            room_name: (*name).to_string(),
            ip_address: "127.0.0.1".to_string(),
            port,
            model_name: "Sonos One".to_string(),
        })
        .collect()
}

// -- the defect ---------------------------------------------------------------

/// Fails against the old code, which returned a bare `String` and gave `main`
/// no way to tell this apart from a verified listing.
#[test]
fn speakers_with_nothing_reachable_prints_names_but_is_not_live() {
    let system = SonosSystem::from_devices_offline(devices_at_localhost(&[
        "Living Room",
        "Kitchen",
        "Bedroom",
    ]))
    .expect("offline construction cannot fail");

    let out = run_command(
        Commands::Speakers,
        &system,
        &Config::default(),
        &GlobalFlags::default(),
    )
    .expect("listing cached speakers is not an error");

    // The output contract is untouched: no suffix, no header, no annotation —
    // each line is the bare cached name, so `sonos speakers | fzf` and
    // `awk '{print $1}'` still work. (Order is not asserted: `speakers()` is
    // backed by a HashMap.)
    assert!(out.stdout.contains("Living Room"), "got: {:?}", out.stdout);
    assert_eq!(out.stdout.lines().count(), 3, "got: {:?}", out.stdout);
    let mut lines: Vec<&str> = out.stdout.lines().collect();
    lines.sort_unstable();
    assert_eq!(lines, ["Bedroom", "Kitchen", "Living Room"]);

    assert_eq!(
        out.liveness,
        Liveness::NothingContacted,
        "three speakers, six failed probes, reported as live"
    );
    assert_eq!(
        disposition(out.liveness, false, false).exit_code(),
        3,
        "total discovery failure must not exit 0"
    );
}

/// `status` had the same bug one row wide: it printed `<name> unknown`, exit 0.
#[test]
fn status_with_nothing_reachable_is_not_live() {
    let system = SonosSystem::from_devices_offline(devices_at_localhost(&["Living Room"]))
        .expect("offline construction cannot fail");

    let out = run_command(
        Commands::Status,
        &system,
        &Config::default(),
        &GlobalFlags::default(),
    )
    .expect("status of a cached speaker is not an error");

    assert!(out.stdout.contains("Living Room"));
    assert!(out.stdout.contains("unknown"));
    assert_eq!(out.liveness, Liveness::NothingContacted);
}

#[test]
fn groups_with_nothing_reachable_is_not_live() {
    let system = SonosSystem::from_devices_offline_with_groups(
        devices_at_localhost(&["Living Room", "Kitchen"]),
        vec![(
            GroupId::new("RINCON_000:1"),
            SpeakerId::new("RINCON_000"),
            vec![SpeakerId::new("RINCON_000"), SpeakerId::new("RINCON_001")],
        )],
    )
    .expect("offline construction cannot fail");

    let out = run_command(
        Commands::Groups,
        &system,
        &Config::default(),
        &GlobalFlags::default(),
    )
    .expect("listing cached groups is not an error");

    assert!(out.stdout.contains("Living Room"), "got: {:?}", out.stdout);
    assert_eq!(out.liveness, Liveness::NothingContacted);
}

/// An empty system claims nothing about any speaker, so it must not be flagged
/// — it already prints its own discovery hint. See `liveness::verdict`.
#[test]
fn empty_system_is_live_not_unvalidated() {
    let system =
        SonosSystem::from_devices_offline(vec![]).expect("offline construction cannot fail");

    let out = run_command(
        Commands::Speakers,
        &system,
        &Config::default(),
        &GlobalFlags::default(),
    )
    .expect("an empty system is not an error");

    assert_eq!(out.stdout, "No speakers found");
    assert_eq!(out.liveness, Liveness::Live);
    assert_eq!(disposition(out.liveness, false, false).exit_code(), 0);
}

// -- no false positives -------------------------------------------------------

/// A write command against an unreachable system must *fail*, not come back
/// marked live.
///
/// Every write arm ends in a SOAP call propagated with `?`, which is the whole
/// justification for `CommandOutput::live` on those arms. If that assumption
/// ever stopped holding, `play` would start reporting "Playing (Living Room)"
/// at exit 0 on a dead network — the same defect in a worse place, since a
/// write command claims it changed something.
#[test]
fn write_command_against_dead_system_errors_rather_than_claiming_live() {
    let system = SonosSystem::from_devices_offline(devices_at_localhost(&["Living Room"]))
        .expect("offline construction cannot fail");

    let result = run_command(
        Commands::Play,
        &system,
        &Config::default(),
        &GlobalFlags::default(),
    );

    assert!(
        result.is_err(),
        "play reported success without reaching a speaker: {:?}",
        result.map(|o| o.stdout)
    );
}

// Partial liveness — one speaker answering while others do not — is covered by
// the pure tests in `src/liveness.rs` (`verdict(4, 1)`, `Probes`) rather than
// here on purpose. The SDK addresses every speaker at a hardcoded port 1400,
// so a stub responder is a process-global resource: standing one up to make a
// single speaker reachable would also make every "nothing is reachable" test
// above reachable, and no second loopback address fails fast enough to play
// the dead speaker (127.0.0.2 and friends hang until the connect timeout on
// macOS rather than refusing).
