---
title: "feat: SDK-Level Discovery Caching"
type: feat
status: completed
date: 2026-03-08
origin: docs/brainstorms/2026-03-08-sdk-caching-brainstorm.md
milestone: "Milestone 2: CLI — Discovery & System Commands"
deepened: 2026-03-08
---

# feat: SDK-Level Discovery Caching

## Enhancement Summary

**Deepened on:** 2026-03-08
**Sections enhanced:** 7
**Research agents used:** architecture-strategist, performance-oracle, security-sentinel, code-simplicity-reviewer, pattern-recognition-specialist, spec-flow-analyzer, repo-research-analyst, best-practices-researcher, SDK explorer

### Key Improvements

1. **Cache directory fix:** Changed from `config_dir()` to `cache_dir()` per XDG conventions — cache is disposable data, not configuration
2. **Transparent auto-rediscovery:** `get_speaker_by_name()` auto-rediscovers on miss (one-shot per session). SDK consumers never deal with discovery — it just works.
3. **Stale cache fallback:** Added graceful degradation when cache is stale AND network unavailable — use stale data with warning instead of failing
4. **New error variants:** Added `SdkError::DiscoveryFailed` and `SdkError::LockPoisoned`
5. **Portable timestamps:** Changed `SystemTime` to `u64` epoch seconds for debuggable, cross-platform cache files
6. **No CWD fallback:** Removed `PathBuf::from(".")` fallback in config.rs. Validated `SONOS_CACHE_DIR` env var (reject empty/relative paths).
7. **Phase consolidation:** Collapsed 6 phases to 3 focused phases; removed system commands (belongs in companion plan)

### New Considerations Discovered

- Rediscovery must call `state_manager.add_devices()` or new speakers have no state tracking
- `RwLock` write lock during rediscovery must be held briefly (map swap only), not during 3s SSDP scan
- Error messages referencing `sonos discover` must be updated since that command is removed
- Companion plan (`2026-03-08-feat-discovery-and-system-commands-plan.md`) conflicts on caching approach — its caching sections are superseded by this plan
- `tracing` is already an SDK dependency — available for cache warning logs
- `is_stale()` must reject future `cached_at` timestamps (treat as stale)

---

## Overview

Move discovery caching from `sonos-cli` into `sonos-sdk` so that `SonosSystem::new()` is the only API consumers need. The SDK handles SSDP discovery, disk caching, TTL, and rediscovery — all transparently. This eliminates the need for CLI consumers to depend on `sonos-discovery` or implement their own cache.

This is a cross-repo change: new code in `sonos-sdk`, removals in `sonos-cli`.

**Estimated scope:** ~80-100 lines new SDK code (cache module + system.rs changes), ~50 lines CLI deletions.

## Problem Statement / Motivation

Today, every SDK consumer must:
1. Depend on `sonos-discovery` directly for the `Device` type and discovery functions
2. Implement their own caching layer (serialize devices, manage TTL, reconstruct on load)
3. Orchestrate the cache-or-discover decision in their own code

The SDK already knows about speakers, IPs, and IDs. Caching is a natural SDK responsibility. (see brainstorm: `docs/brainstorms/2026-03-08-sdk-caching-brainstorm.md`)

## Proposed Solution

**Two workstreams:**

1. **SDK side** — Add a `cache` module to `sonos-sdk`. Update `SonosSystem::new()` to check cache before running SSDP. Add transparent auto-rediscovery on speaker miss.
2. **CLI side** — Delete `cache.rs`, remove `Action::Discover` and `Commands::Discover`, remove `sonos-discovery` dependency, simplify `main.rs` to just `SonosSystem::new()`.

### SDK: New `SonosSystem::new()` Flow

```
SonosSystem::new():
  1. Try cache::load() from ~/.cache/sonos-sdk/cache.json
  2. If cache exists and fresh (< 24h):
     → from_discovered_devices(cached_devices)
  3. If cache missing or corrupt:
     → sonos_discovery::get() (3s SSDP)
     → cache::save(devices) (log warning on save failure, non-fatal)
     → from_discovered_devices(devices)
  4. If cache stale:
     → sonos_discovery::get() (3s SSDP)
     → If SSDP returns devices: cache::save(devices) → from_discovered_devices(devices)
     → If SSDP returns empty: fall back to stale cache data (log warning)
  5. If SSDP finds nothing and no cache exists (fresh or stale):
     → return Err(SdkError::DiscoveryFailed)
```

### Research Insights: Stale Cache Fallback

**Gap identified by spec-flow-analyzer:** The original plan had no behavior for "stale cache + no network." This is a real scenario (laptop moved off home network, brief WiFi outage). The enhanced flow (step 4) degrades gracefully: stale cached IPs may still work if speakers haven't moved, and the CLI can show a warning. This is strictly better than hard failure.

### SDK: Transparent Auto-Rediscovery on Miss

**Design principle:** SDK consumers should never deal with discovery or refresh. `SonosSystem::new()` and `get_speaker_by_name()` should just work.

When `get_speaker_by_name("X")` returns `None`:
1. Run fresh SSDP discovery (3s)
2. Register new devices with `state_manager.add_devices()`
3. Rebuild internal speaker map (write lock held briefly for swap only)
4. Save updated cache (log warning on failure, non-fatal)
5. Retry lookup
6. Return `None` if still not found

A `has_rediscovered: bool` flag (per-session) prevents infinite rediscovery loops. The flag is checked before running SSDP — if already rediscovered once this session, skip straight to returning `None`.

```rust
// SDK API — consumer never thinks about discovery
impl SonosSystem {
    pub fn get_speaker_by_name(&self, name: &str) -> Option<Speaker> {
        if let Some(speaker) = self.speakers.read().ok()?.get(name).cloned() {
            return Some(speaker);
        }
        // Not found — try rediscovery once per session
        self.try_rediscover();
        self.speakers.read().ok()?.get(name).cloned()
    }
}

// CLI usage — simple, no discovery orchestration
fn resolve_speaker(system: &SonosSystem, name: &str) -> Result<Speaker, CliError> {
    system.get_speaker_by_name(name)
        .ok_or_else(|| CliError::SpeakerNotFound(name.to_string()))
}
```

**Tradeoff accepted:** The 3s SSDP delay is hidden inside a "lookup" method on first miss. This is intentional — the SDK's job is to make things just work. The delay only happens once per session (one-shot flag) and only when a speaker genuinely isn't in the map.

### CLI: Simplification

```
main.rs (before):
  parse args → load config → load cache → build system → execute

main.rs (after):
  parse args → load config → SonosSystem::new()? → execute
```

## Technical Considerations

### `Device` Serialization

`sonos_discovery::Device` currently derives only `Debug, Clone`. It needs `Serialize, Deserialize` added for JSON caching. The `sonos-discovery` crate already has `serde = { version = "1.0", features = ["derive"] }` in its `Cargo.toml`, so this is just adding derives to the struct.

**Device fields (all serializable primitives):**
```rust
pub struct Device {
    pub id: String,          // "RINCON_XXX"
    pub name: String,        // "Kitchen"
    pub room_name: String,   // "Kitchen"
    pub ip_address: String,  // "192.168.1.10"
    pub port: u16,           // 1400
    pub model_name: String,  // "Sonos One"
}
```

### Cache Directory: `cache_dir()` Not `config_dir()`

**Research insight (Context7 + best-practices-researcher):** Per XDG Base Directory conventions, cached/disposable data belongs in `cache_dir()` (`~/.cache/` on Linux, `~/Library/Caches/` on macOS), not `config_dir()` (`~/.config/`). The cache file can be deleted at any time and the system recovers via SSDP — it is not configuration.

- Cache path: `dirs::cache_dir()` → `~/.cache/sonos/cache.json`
- Config path stays: `dirs::config_dir()` → `~/.config/sonos/config.toml`
- Env override: `SONOS_CACHE_DIR` (separate from `SONOS_CONFIG_DIR`)

### Cache Timestamp: `u64` Epoch Seconds

**Research insight (best-practices-researcher):** `SystemTime` serde is platform-dependent and produces `{"secs_since_epoch": N, "nanos_since_epoch": N}`. Using `u64` epoch seconds instead is:
- Portable across platforms
- Human-readable in the JSON (`"cached_at": 1741392000`)
- Trivially debuggable (`date -d @1741392000`)
- Nanosecond precision is unnecessary for a 24-hour TTL

### Cache Stores Devices Only, Not Groups

Group topology comes from the ZoneGroupTopology UPnP subscription, not from SSDP discovery. The cache stores `Vec<Device>` + timestamp. Groups are populated after `from_discovered_devices()` when a speaker's `group_membership` property is watched or topology events arrive.

### Interior Mutability for Rediscovery

`SonosSystem` already uses `RwLock<HashMap<String, Speaker>>` for its speaker map. The `try_rediscover()` method can update this map in-place without requiring `&mut self`.

**Write lock timing:** The write lock must be held briefly — only for the map swap. SSDP discovery (3s) runs before acquiring the lock. Pattern:

```rust
fn try_rediscover(&self) {
    if self.has_rediscovered.get() { return; }
    self.has_rediscovered.set(true);

    // 1. SSDP runs WITHOUT holding any lock (3s)
    let devices = sonos_discovery::get_with_timeout(Duration::from_secs(3));
    if devices.is_empty() { return; }

    // 2. Register devices with state manager (required for property tracking)
    if let Err(e) = self.state_manager.add_devices(devices.clone()) {
        tracing::warn!("Failed to register rediscovered devices: {}", e);
        return;
    }

    // 3. Build new Speaker handles from devices (no lock needed)
    let new_speakers = self.build_speakers(&devices);

    // 4. Acquire write lock BRIEFLY for map swap only
    if let Ok(mut map) = self.speakers.write() {
        *map = new_speakers;
    }

    // 5. Save cache (non-fatal on failure)
    if let Err(e) = cache::save(&devices) {
        tracing::warn!("Failed to save discovery cache: {}", e);
    }
}
```

**Key implementation details:**
- `has_rediscovered: Cell<bool>` — simple boolean flag, no atomics needed (SDK is single-threaded for CLI use)
- `state_manager.add_devices()` **must** be called before building Speaker handles, or property `fetch()`/`watch()` calls will fail
- The `build_speakers()` helper needs to be extracted from `from_discovered_devices()` — it constructs Speaker handles using `api_client` and `state_manager`
- The system needs the `SonosClient` field (currently `_api_client`) renamed to `api_client` for constructing new `Speaker` handles during rediscovery

### New SDK Dependencies

The SDK needs three new dependencies for caching:
- `serde` + `serde_json` — serialize `Device` list to JSON
- `dirs` — locate `~/.cache/sonos/` cross-platform

Note: `tracing` is already an SDK dependency (`tracing = "0.1"`) — available for cache warning logs without additional additions.

### Cache Robustness

- **Corrupt cache:** `load()` returns `None` on parse error → falls through to SSDP
- **Permission errors on save:** Log warning via `tracing::warn!`, continue without caching (non-fatal)
- **Concurrent access:** Atomic write (temp file + rename) prevents partial reads
- **First run:** No cache exists → SSDP discovery → save cache → subsequent runs are fast
- **File size sanity:** Reject cache files over 1MB on load (defense against corruption)

### Path Safety

1. **No CWD fallback:** If `dirs::cache_dir()` returns `None` and no env override, return `None` from cache — don't fall back to current working directory. Same fix needed in `config.rs` (remove `PathBuf::from(".")` fallback).

2. **Validate `SONOS_CACHE_DIR`:** Reject empty strings and relative paths. Require absolute path to prevent reintroducing CWD fallback through the env var.

### CLI `config.cache_ttl_hours` Becomes Unused

The CLI config has `cache_ttl_hours: u64` (default 24). Since the SDK now owns TTL with a hardcoded 24h, this config field becomes dead code. Remove it from `Config`.

### New Error Variants

`SdkError` needs two new variants:

```rust
// sonos-sdk/src/error.rs
pub enum SdkError {
    // ... existing variants ...
    #[error("discovery failed: {0}")]
    DiscoveryFailed(String),

    #[error("internal lock poisoned")]
    LockPoisoned,
}
```

### Updated Error Messages

**Gap identified by spec-flow-analyzer:** With `sonos discover` removed, CLI error recovery hints must stop referencing it. Update `errors.rs` recovery hints:

- Before: `"Run 'sonos discover' to refresh the speaker list."`
- After: `"Check that your speakers are on the same network, then retry."`

### Plan Conflict Resolution

**Identified by multiple agents:** The companion plan `docs/plans/2026-03-08-feat-discovery-and-system-commands-plan.md` has overlapping scope. Its caching and discovery sections (Phases 1, 2, 6, 7) are **superseded** by this plan. Its executor/command sections (Phases 3, 4, 5) remain valid and should be implemented after this plan completes. That plan's `Action::Discover` and cache orchestration sections should be marked as superseded.

## Acceptance Criteria

### SDK Changes (`sonos-sdk`)

- [x] `Device` in `sonos-discovery` derives `Serialize, Deserialize`
- [x] New `SdkError::DiscoveryFailed(String)` and `SdkError::LockPoisoned` variants in `sonos-sdk/src/error.rs`
- [x] New `sonos-sdk/src/cache.rs` module with:
  - [x] `CachedDevices` struct: `devices: Vec<Device>`, `cached_at: u64` (epoch seconds)
  - [x] `load() -> Option<CachedDevices>` — reads from `~/.cache/sonos/cache.json`, rejects files >1MB
  - [x] `save(devices: &[Device]) -> Result<()>` — atomic write (temp + rename), log warning on failure via `tracing::warn!`
  - [x] `is_stale(cached: &CachedDevices) -> bool` — 24h TTL, rejects future timestamps
  - [x] `cache_dir()` uses `dirs::cache_dir()` with `SONOS_CACHE_DIR` env override (validated: non-empty, absolute), no CWD fallback
- [x] `SonosSystem::new()` uses cache-first strategy:
  - [x] Fresh cache → `from_discovered_devices(cached)`
  - [x] Stale cache → SSDP → if empty, fall back to stale cache with `tracing::warn!`
  - [x] Missing/corrupt cache → SSDP → save → `from_discovered_devices()`
  - [x] No SSDP results and no cache → `Err(SdkError::DiscoveryFailed(...))`
- [x] Transparent auto-rediscovery on speaker miss:
  - [x] `get_speaker_by_name()` triggers one SSDP scan if initial lookup returns `None`
  - [x] `has_rediscovered: bool` flag prevents repeated SSDP within the same session
  - [x] Rediscovery calls `state_manager.add_devices()` before rebuilding speaker map
  - [x] New speakers added to internal map + cache saved (log warning on save failure)
- [x] `_api_client` field renamed to `api_client` for use during rediscovery
- [x] `build_speakers()` helper extracted from `from_discovered_devices()` for reuse during rediscovery
- [x] `serde`, `serde_json`, `dirs` added to `sonos-sdk/Cargo.toml`

### CLI Changes (`sonos-cli`)

- [x] `src/cache.rs` deleted
- [x] `mod cache;` removed from `main.rs`
- [x] `Action::Discover` removed from `src/actions.rs`
- [x] `Commands::Discover` removed from `src/cli/mod.rs`
- [x] `sonos-discovery` removed from `Cargo.toml`
- [x] `config.cache_ttl_hours` removed from `Config` struct
- [x] `CliError::Cache` variant removed
- [x] `main.rs` simplified: `SonosSystem::new()?` then pass to executor
- [x] `executor.rs` signature updated to `execute(action, &SonosSystem, &Config)`
- [x] `resolve_target()` uses `system.get_speaker_by_name()` — SDK handles rediscovery transparently
- [x] Error recovery hints updated — no references to `sonos discover`
- [x] Error test assertions in `errors.rs` updated to match new recovery hint text
- [x] `config.rs` CWD fallback (`PathBuf::from(".")`) removed — return `Config::default()` if no config dir

### Post-Implementation Documentation

- [x] `CLAUDE.md` updated: remove `cache.rs` from module structure, update cache path to `~/.cache/`, remove `sonos discover` references
- [x] `docs/product/roadmap.md`: mark `sonos discover` task as superseded (not just checked off)
- [x] Companion plan `2026-03-08-feat-discovery-and-system-commands-plan.md`: mark caching/discovery sections as superseded

## Implementation Plan

### Phase 1: SDK Changes

**Repo:** `sonos-sdk`

1. **Add Serialize/Deserialize to Device** (`sonos-discovery/src/lib.rs`)
   - Add `Serialize, Deserialize` derives to `Device` struct (serde already in crate deps)

2. **Add DiscoveryFailed error variant** (`sonos-sdk/src/error.rs`)
   - Add `DiscoveryFailed(String)` to `SdkError` enum

3. **Create cache module** (`sonos-sdk/src/cache.rs`)

```rust
// sonos-sdk/src/cache.rs
use sonos_discovery::Device;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{fs, io};

const CACHE_TTL_SECS: u64 = 24 * 3600;
const MAX_CACHE_SIZE: u64 = 1_048_576; // 1MB

#[derive(Serialize, Deserialize)]
pub(crate) struct CachedDevices {
    pub devices: Vec<Device>,
    pub cached_at: u64, // seconds since UNIX_EPOCH
}

pub(crate) fn cache_dir() -> Option<PathBuf> {
    std::env::var("SONOS_CACHE_DIR")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| dirs::cache_dir().map(|p| p.join("sonos")))
}

pub(crate) fn load() -> Option<CachedDevices> {
    let path = cache_dir()?.join("cache.json");
    let meta = fs::metadata(&path).ok()?;
    if meta.len() > MAX_CACHE_SIZE { return None; }
    let contents = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&contents).ok()
}

pub(crate) fn save(devices: &[Device]) -> Result<(), io::Error> {
    let dir = cache_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "cache dir not found"))?;
    fs::create_dir_all(&dir)?;

    let cached = CachedDevices {
        devices: devices.to_vec(),
        cached_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };
    let json = serde_json::to_string(&cached)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let temp_path = dir.join("cache.json.tmp");
    fs::write(&temp_path, &json)?;
    fs::rename(&temp_path, dir.join("cache.json"))?;
    Ok(())
}

pub(crate) fn is_stale(cached: &CachedDevices) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Reject future timestamps — treat as stale (forces rediscovery)
    if cached.cached_at > now {
        return true;
    }
    now - cached.cached_at >= CACHE_TTL_SECS
}

```

4. **Update `SonosSystem`** (`sonos-sdk/src/system.rs`)
   - Update `new()` with cache-first logic (see flow above)
   - Add `has_rediscovered: Cell<bool>` field
   - Add `fn try_rediscover(&self)` — runs SSDP, calls `state_manager.add_devices()`, rebuilds map, saves cache
   - Extract `build_speakers()` helper from `from_discovered_devices()` for reuse
   - Update `get_speaker_by_name()` to call `try_rediscover()` on miss
   - Rename `_api_client` → `api_client`
   - Add `serde`, `serde_json`, `dirs` to `Cargo.toml`

### Phase 2: CLI Removals

**Repo:** `sonos-cli`

1. Delete `src/cache.rs`
2. Remove `mod cache;` from `main.rs`
3. Remove `Action::Discover` from `actions.rs`
4. Remove `Commands::Discover` from `cli/mod.rs` and `into_action()` match
5. Remove `sonos-discovery` from `Cargo.toml`
6. Remove `cache_ttl_hours` from `Config`
7. Remove `CliError::Cache` variant from `errors.rs`
8. Update error recovery hints — replace `"Run 'sonos discover'"` messages
9. Remove `PathBuf::from(".")` fallback in `config.rs` — use `Config::default()` when no config dir

### Phase 3: CLI Wiring

**Repo:** `sonos-cli`

1. Update `main.rs`: `SonosSystem::new()?` then pass to executor
2. Update `executor::execute()` signature to `(action, &SonosSystem, &Config)`
3. Update `resolve_target()` to use `system.get_speaker_by_name()` — SDK handles rediscovery transparently, CLI just checks the result

## Dependencies & Risks

**Dependencies:**
- `sonos-sdk` repo must accept the cache module and new dependencies
- `sonos-discovery::Device` must gain `Serialize/Deserialize` derives

**Risks:**
- **SDK API mismatch:** The SDK reference docs may not perfectly match the actual crate. May need to adjust during implementation.
- **Group topology timing:** After loading from cache, `system.groups()` returns empty until a topology subscription fires. Commands like `sonos groups` need to watch `group_membership` on at least one speaker first to trigger this. (This is a known SDK behavior, not solved by this plan.)
- **First-run latency:** First run always takes ~3s for SSDP. No way around this — users just need to wait once.
- **Auto-rediscovery latency:** First speaker miss per session triggers a hidden 3s SSDP scan inside `get_speaker_by_name()`. Accepted tradeoff — SDK should just work without consumers managing discovery.

## Sources & References

### Origin

- **Brainstorm document:** [docs/brainstorms/2026-03-08-sdk-caching-brainstorm.md](../brainstorms/2026-03-08-sdk-caching-brainstorm.md) — Key decisions: caching moves to SDK, `sonos discover` removed, auto-rediscovery on miss, `Device` stays internal.

### Internal References

- SDK system.rs: `../sonos-sdk/sonos-sdk/src/system.rs` — `SonosSystem::new()`, `from_discovered_devices()`, `RwLock<HashMap>` speaker map
- SDK discovery: `../sonos-sdk/sonos-discovery/src/lib.rs` — `Device` struct, `get()`, `get_with_timeout()`
- SDK error: `../sonos-sdk/sonos-sdk/src/error.rs` — `SdkError` enum (10 variants, needs `DiscoveryFailed`)
- CLI cache (to delete): `src/cache.rs` — current `CachedSystem` implementation
- CLI executor (to update): `src/executor.rs` — stub implementations
- CLI main (to simplify): `src/main.rs` — dispatch flow
- CLI errors (to update): `src/errors.rs` — recovery hints reference `sonos discover`
- CLI config (to fix): `src/config.rs` — CWD fallback on line 35
- SDK API reference: `docs/references/sonos-sdk.md`
- Roadmap: `docs/product/roadmap.md` — Milestone 2
- Companion plan (partially superseded): `docs/plans/2026-03-08-feat-discovery-and-system-commands-plan.md` — caching/discovery sections superseded; executor/command sections still valid

### Research References

- XDG Base Directory Specification: cache_dir for disposable data, config_dir for settings
- Rust `dirs` crate v5: `cache_dir()` returns `~/.cache` (Linux), `~/Library/Caches` (macOS), `%LOCALAPPDATA%` (Windows)
- serde derive: `sonos-discovery` already has `serde = { version = "1.0", features = ["derive"] }` — just add derives to Device
- Atomic file writes: temp file + `fs::rename()` (POSIX atomic on same filesystem)
