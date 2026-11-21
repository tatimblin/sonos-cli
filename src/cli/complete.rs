use std::collections::HashMap;
use std::ffi::OsStr;

use clap_complete::engine::CompletionCandidate;

use crate::config::Config;

/// Candidates for `--speaker`, filtered by what has been typed so far.
pub fn speaker_candidates(current: &OsStr) -> Vec<CompletionCandidate> {
    let mut candidates = Vec::new();

    // Quiet: a warning on stderr would garble the shell's completion output.
    let config = Config::try_load().unwrap_or_default();
    for (name, alias) in &config.aliases {
        candidates.push(CompletionCandidate::new(alias).help(Some(name.into())));
    }

    if let Ok(system) = sonos_sdk::SonosSystem::new() {
        for spk in system.speakers() {
            candidates.push(CompletionCandidate::new(&spk.name));
        }
    }

    matching_prefix(candidates, current)
}

/// Candidates for `--group`, which resolves an alias and then matches a group
/// by its **coordinator's** name (`SonosSystem::group`). Non-coordinator
/// members are not accepted, so they are not suggested.
///
/// Group membership is not in the discovery cache, so `groups()` costs one
/// topology request to a speaker. If that fails, falls back to every speaker
/// name: each standalone speaker coordinates its own group, so this is the
/// best offline guess.
pub fn group_candidates(current: &OsStr) -> Vec<CompletionCandidate> {
    // Quiet: a warning on stderr would garble the shell's completion output.
    let config = Config::try_load().unwrap_or_default();

    let coordinators = sonos_sdk::SonosSystem::new().ok().map(|system| {
        let names: Vec<String> = system
            .groups()
            .iter()
            .filter_map(|g| g.coordinator())
            .map(|c| c.name)
            .collect();
        if names.is_empty() {
            system.speakers().into_iter().map(|s| s.name).collect()
        } else {
            names
        }
    });

    matching_prefix(
        group_candidates_from(&config.aliases, coordinators.as_deref()),
        current,
    )
}

/// Keep candidates whose value starts with `current`, ignoring case.
///
/// Name resolution is case-insensitive (`--group office` finds "Office"), so
/// completion is too. clap's built-in filter for `ArgValueCandidates` is
/// case-sensitive, which is why these flags use `ArgValueCompleter` and filter
/// here instead.
fn matching_prefix(
    candidates: Vec<CompletionCandidate>,
    current: &OsStr,
) -> Vec<CompletionCandidate> {
    let current = current.to_string_lossy().to_lowercase();
    candidates
        .into_iter()
        .filter(|c| {
            c.get_value()
                .to_string_lossy()
                .to_lowercase()
                .starts_with(&current)
        })
        .collect()
}

/// Build `--group` candidates from coordinator names and the configured
/// aliases (key = full name, value = alias).
///
/// An alias is offered only when its target is a coordinator, compared
/// case-insensitively like `--group` resolution. With `coordinators` unknown
/// (`None`), every alias is offered rather than none.
fn group_candidates_from(
    aliases: &HashMap<String, String>,
    coordinators: Option<&[String]>,
) -> Vec<CompletionCandidate> {
    let is_coordinator =
        |name: &str| coordinators.is_none_or(|cs| cs.iter().any(|c| c.eq_ignore_ascii_case(name)));

    let mut candidates: Vec<CompletionCandidate> = aliases
        .iter()
        .filter(|(name, _)| is_coordinator(name))
        .map(|(name, alias)| CompletionCandidate::new(alias).help(Some(name.into())))
        .collect();
    candidates.extend(
        coordinators
            .unwrap_or_default()
            .iter()
            .map(CompletionCandidate::new),
    );
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(candidates: &[CompletionCandidate]) -> Vec<String> {
        let mut v: Vec<String> = candidates
            .iter()
            .map(|c| c.get_value().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }

    fn aliases(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(n, a)| (n.to_string(), a.to_string()))
            .collect()
    }

    #[test]
    fn offers_coordinators_and_their_aliases_only() {
        let aliases = aliases(&[("living room", "lr"), ("Bathroom", "bath")]);
        let coordinators = vec!["Living Room".to_string(), "Kitchen".to_string()];
        let candidates = group_candidates_from(&aliases, Some(&coordinators));
        // "bath" targets a grouped non-coordinator, which `--group` rejects.
        assert_eq!(values(&candidates), ["Kitchen", "Living Room", "lr"]);
    }

    #[test]
    fn alias_candidate_shows_its_target_as_help() {
        let aliases = aliases(&[("Kitchen", "kit")]);
        let coordinators = vec!["Kitchen".to_string()];
        let candidates = group_candidates_from(&aliases, Some(&coordinators));
        let kit = candidates
            .iter()
            .find(|c| c.get_value() == "kit")
            .expect("alias offered");
        assert_eq!(
            kit.get_help().map(|h| h.to_string()).as_deref(),
            Some("Kitchen")
        );
    }

    #[test]
    fn prefix_match_ignores_case() {
        let filtered = |current: &str| {
            let candidates = ["Office / Roam", "roam", "Kitchen"]
                .into_iter()
                .map(CompletionCandidate::new)
                .collect();
            values(&matching_prefix(candidates, OsStr::new(current)))
        };
        assert_eq!(filtered("o"), ["Office / Roam"]);
        assert_eq!(filtered("ROA"), ["roam"]);
        assert_eq!(filtered("").len(), 3);
    }

    #[test]
    fn unknown_topology_offers_every_alias() {
        let aliases = aliases(&[("Kitchen", "kit"), ("Bathroom", "bath")]);
        let candidates = group_candidates_from(&aliases, None);
        assert_eq!(values(&candidates), ["bath", "kit"]);
    }
}
