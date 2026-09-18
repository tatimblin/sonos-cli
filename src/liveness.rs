//! Did this command actually reach a speaker?
//!
//! `SonosSystem::new()` can hand back a full speaker list without a single
//! packet leaving the machine: discovery is cached on disk for 24h, and on a
//! cache hit no SSDP sweep is attempted at all. Commands that only *read*
//! state — `speakers`, `groups`, `status` — then discard every failed fetch
//! with `.ok()` and print whatever they still have, which on a machine with
//! broken SSDP is a list of names from a weeks-old cache. stdout looked
//! exactly like success and the exit code was 0.
//!
//! The fix does not need new data. Those commands already make the calls that
//! answer the question; they just threw the answers away. This module is the
//! bookkeeping ([`Probes`]) plus the two pure decisions — did anything come
//! back ([`verdict`]), and what should `main` do about it ([`disposition`]) —
//! kept free of I/O so both are unit-testable with no network.

/// Whether a command established contact with at least one speaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Liveness {
    /// At least one probe reached a speaker and came back.
    Live,
    /// Every probe this command attempted failed. Anything printed came out of
    /// the discovery cache and was never confirmed against a speaker.
    NothingContacted,
}

/// Decide liveness from the probes a command attempted and the ones that
/// came back.
///
/// Pure: no network, no clock, no globals.
///
/// **One success is enough.** A partly reachable household is a real
/// household, and a speaker that is merely powered off must not turn a
/// correct listing into a failure. The condition being guarded is *total*
/// silence rendered as success.
///
/// **`attempted == 0` is [`Liveness::Live`].** A command that probed nothing
/// made no claim about any speaker's state, so there is nothing unvalidated to
/// flag — `sonos speakers` on an empty system prints "No speakers found" and
/// emits its own discovery hint. Returning `NothingContacted` there would
/// staple a "this data may be wrong" banner onto output that contains no data.
pub fn verdict(attempted: usize, succeeded: usize) -> Liveness {
    if attempted == 0 || succeeded > 0 {
        Liveness::Live
    } else {
        Liveness::NothingContacted
    }
}

/// Running tally of probes attempted versus probes that returned.
///
/// [`Probes::record`] is a drop-in for the `.ok()` that used to discard each
/// outcome: it still yields `Option<T>`, so the formatting code below it is
/// unchanged, but the outcome is counted on the way past.
#[derive(Debug, Default, Clone, Copy)]
pub struct Probes {
    attempted: usize,
    succeeded: usize,
}

impl Probes {
    /// Count one probe and convert it to the `Option` the callers already use.
    pub fn record<T, E>(&mut self, outcome: Result<T, E>) -> Option<T> {
        self.attempted += 1;
        match outcome {
            Ok(value) => {
                self.succeeded += 1;
                Some(value)
            }
            Err(_) => None,
        }
    }

    /// The verdict for everything recorded so far.
    pub fn verdict(&self) -> Liveness {
        verdict(self.attempted, self.succeeded)
    }
}

/// A command's stdout together with whether it was validated against a live
/// speaker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    /// Exactly what the command would have printed before this change. The
    /// plain-text format is the de-facto machine contract (there is no `--json`
    /// mode), so nothing is annotated here — the verdict travels beside the
    /// text, not inside it.
    pub stdout: String,
    /// Whether any speaker was actually reached while producing `stdout`.
    pub liveness: Liveness,
}

impl CommandOutput {
    /// Output from a command that cannot silently degrade.
    ///
    /// Every write command (`play`, `volume`, `join`, …) propagates its SOAP
    /// error with `?`, so reaching the `Ok` arm at all proves a speaker
    /// answered. Only the read commands that swallow failures per-field need
    /// to count.
    pub fn live(stdout: impl Into<String>) -> Self {
        Self {
            stdout: stdout.into(),
            liveness: Liveness::Live,
        }
    }

    /// Output whose liveness was measured by the handler.
    pub fn new(stdout: impl Into<String>, liveness: Liveness) -> Self {
        Self {
            stdout: stdout.into(),
            liveness,
        }
    }
}

/// What `main` should do with a finished command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    /// Print stdout, say nothing on stderr, exit 0.
    Success,
    /// Print stdout, print a one-line note on stderr, exit 0. (`--offline`)
    AcceptedUnvalidated,
    /// Print stdout, print the banner on stderr, exit 3. (default)
    WarnUnvalidated,
    /// Suppress stdout, print the banner on stderr, exit 1. (`--require-live`)
    RejectUnvalidated,
}

impl Disposition {
    /// Whether the command's stdout should be printed at all.
    ///
    /// Only `--require-live` withholds it: the user asked for verified data, so
    /// handing them unverified data would be the same bug in a quieter form.
    pub fn prints_stdout(self) -> bool {
        !matches!(self, Self::RejectUnvalidated)
    }

    /// Whether the framed stderr banner should be emitted.
    pub fn prints_banner(self) -> bool {
        matches!(self, Self::WarnUnvalidated | Self::RejectUnvalidated)
    }

    /// Whether the short `--offline` acknowledgement should be emitted.
    pub fn prints_note(self) -> bool {
        matches!(self, Self::AcceptedUnvalidated)
    }

    /// Process exit status.
    ///
    /// 3 is a new code, deliberately outside the documented 0/1/2 range rather
    /// than folded into it: the command *did* run and its stdout *is* printed,
    /// so it is not the runtime failure 1 means, and it is certainly not a
    /// usage error. A script that only tests `-ne 0` treats it as failure
    /// (which is the point); one that wants the old behaviour opts in with
    /// `--offline`.
    pub fn exit_code(self) -> u8 {
        match self {
            Self::Success | Self::AcceptedUnvalidated => 0,
            Self::RejectUnvalidated => 1,
            Self::WarnUnvalidated => 3,
        }
    }
}

/// Map a verdict plus the two opt-in flags onto what `main` does.
///
/// Pure, so the whole exit-code matrix is testable without spawning the binary.
///
/// `--offline` and `--require-live` are declared as conflicting in the clap
/// definition, so the both-set row is unreachable through the CLI. It is still
/// defined here rather than left to panic: `require_live` wins, because it is
/// the stricter request and the safer thing to do with an ambiguous demand is
/// to withhold unverified data.
pub fn disposition(liveness: Liveness, offline: bool, require_live: bool) -> Disposition {
    match liveness {
        Liveness::Live => Disposition::Success,
        Liveness::NothingContacted => {
            if require_live {
                Disposition::RejectUnvalidated
            } else if offline {
                Disposition::AcceptedUnvalidated
            } else {
                Disposition::WarnUnvalidated
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_failure_is_not_live() {
        assert_eq!(verdict(4, 0), Liveness::NothingContacted);
    }

    #[test]
    fn one_success_out_of_four_is_live() {
        assert_eq!(verdict(4, 1), Liveness::Live);
    }

    #[test]
    fn all_successes_are_live() {
        assert_eq!(verdict(4, 4), Liveness::Live);
    }

    /// Probing nothing is not evidence of a dead network — see `verdict`.
    #[test]
    fn nothing_attempted_is_live() {
        assert_eq!(verdict(0, 0), Liveness::Live);
    }

    #[test]
    fn probes_counts_both_arms() {
        let mut probes = Probes::default();
        assert_eq!(probes.record::<u8, ()>(Ok(7)), Some(7));
        assert_eq!(probes.record::<u8, ()>(Err(())), None);
        // `record` must stay a drop-in for `.ok()`: same Option, counted.
        assert_eq!(probes.verdict(), Liveness::Live);
    }

    #[test]
    fn probes_all_failing_is_nothing_contacted() {
        let mut probes = Probes::default();
        for _ in 0..3 {
            assert_eq!(probes.record::<u8, ()>(Err(())), None);
        }
        assert_eq!(probes.verdict(), Liveness::NothingContacted);
    }

    #[test]
    fn empty_probes_is_live() {
        assert_eq!(Probes::default().verdict(), Liveness::Live);
    }

    #[test]
    fn command_output_live_helper() {
        let out = CommandOutput::live("Playing (Kitchen)");
        assert_eq!(out.stdout, "Playing (Kitchen)");
        assert_eq!(out.liveness, Liveness::Live);
    }

    // -- the full exit matrix ------------------------------------------------

    #[test]
    fn live_prints_and_exits_zero() {
        let d = disposition(Liveness::Live, false, false);
        assert_eq!(d, Disposition::Success);
        assert!(d.prints_stdout());
        assert!(!d.prints_banner());
        assert!(!d.prints_note());
        assert_eq!(d.exit_code(), 0);
    }

    #[test]
    fn live_ignores_both_flags() {
        for (offline, require_live) in [(true, false), (false, true)] {
            let d = disposition(Liveness::Live, offline, require_live);
            assert_eq!(
                d,
                Disposition::Success,
                "offline={offline} require_live={require_live}"
            );
            assert_eq!(d.exit_code(), 0);
        }
    }

    #[test]
    fn nothing_contacted_with_offline_prints_and_exits_zero() {
        let d = disposition(Liveness::NothingContacted, true, false);
        assert_eq!(d, Disposition::AcceptedUnvalidated);
        assert!(d.prints_stdout());
        assert!(!d.prints_banner());
        assert!(d.prints_note());
        assert_eq!(d.exit_code(), 0);
    }

    #[test]
    fn nothing_contacted_by_default_prints_and_exits_three() {
        let d = disposition(Liveness::NothingContacted, false, false);
        assert_eq!(d, Disposition::WarnUnvalidated);
        assert!(d.prints_stdout());
        assert!(d.prints_banner());
        assert!(!d.prints_note());
        assert_eq!(d.exit_code(), 3);
    }

    #[test]
    fn nothing_contacted_with_require_live_suppresses_stdout_and_exits_one() {
        let d = disposition(Liveness::NothingContacted, false, true);
        assert_eq!(d, Disposition::RejectUnvalidated);
        assert!(!d.prints_stdout());
        assert!(d.prints_banner());
        assert_eq!(d.exit_code(), 1);
    }

    /// clap rejects this combination, but the function must still be total.
    #[test]
    fn require_live_wins_over_offline() {
        assert_eq!(
            disposition(Liveness::NothingContacted, true, true),
            Disposition::RejectUnvalidated
        );
    }

    /// The whole point of the change: total failure must never exit 0 unless
    /// the user opted in.
    #[test]
    fn total_failure_never_exits_zero_without_opt_in() {
        assert_ne!(
            disposition(verdict(10, 0), false, false).exit_code(),
            0,
            "ten failed probes reported as success"
        );
    }
}
