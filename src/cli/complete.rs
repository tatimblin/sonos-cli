use clap_complete::engine::CompletionCandidate;

use crate::config::Config;

pub fn speaker_candidates() -> Vec<CompletionCandidate> {
    let mut candidates = Vec::new();

    let config = Config::load();
    for (name, alias) in &config.aliases {
        candidates.push(CompletionCandidate::new(alias).help(Some(name.into())));
    }

    if let Ok(system) = sonos_sdk::SonosSystem::new() {
        for spk in system.speakers() {
            candidates.push(CompletionCandidate::new(&spk.name));
        }
    }

    candidates
}

pub fn group_candidates() -> Vec<CompletionCandidate> {
    let mut candidates = Vec::new();

    let config = Config::load();
    for (name, alias) in &config.aliases {
        candidates.push(CompletionCandidate::new(alias).help(Some(name.into())));
    }

    if let Ok(system) = sonos_sdk::SonosSystem::new() {
        for grp in system.groups() {
            if let Some(coord) = grp.coordinator() {
                candidates.push(CompletionCandidate::new(&coord.name));
            }
        }
    }

    candidates
}
