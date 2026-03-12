---
date: 2026-03-08
topic: sdk-caching
---

# SDK-Level Caching & Discovery

## What We're Building

Move discovery caching from `sonos-cli` into `sonos-sdk` so that `SonosSystem::new()` is the only API consumers need. The SDK handles SSDP discovery, caching results to disk, loading from cache when fresh, and auto-rediscovering on miss — all transparently. Consumers never interact with `sonos-discovery` directly.

## Why This Approach

The current design forces every SDK consumer to:
1. Depend on `sonos-discovery` for the `Device` type and discovery functions
2. Implement their own caching layer (serialize devices, manage TTL, reconstruct on load)
3. Handle the cache-or-discover decision in their own code

This is unnecessary complexity. The SDK already knows about speakers, IPs, and IDs. Caching is a natural SDK responsibility — it's the same data the SDK already manages in memory.

**Alternative rejected:** Re-exporting `Device` and discovery functions from the SDK. This still forces consumers to manage caching themselves. The problem isn't missing re-exports — it's that caching shouldn't be the consumer's job.

## Key Decisions

- **`SonosSystem::new()` is the only entry point.** It loads from cache if fresh, discovers if stale/missing, and saves after discovery. No configuration needed.
- **`sonos discover` CLI command is removed.** There's no user-facing cache management. The SDK handles freshness transparently.
- **Auto-rediscover on miss.** If `system.get_speaker_by_name("X")` returns `None`, the SDK runs a fresh SSDP scan once before giving up. Transparent to the consumer — just slower on miss.
- **`Device` stays internal.** Consumers use `Speaker` and `Group`. `Device` is an implementation detail of discovery/caching.
- **`sonos-cli/src/cache.rs` is deleted.** Cache is fully an SDK concern. The CLI has no caching code.
- **`sonos-discovery` is not a CLI dependency.** Only the SDK depends on it.
- **Cache location:** SDK uses `dirs::config_dir()` → `~/.config/sonos/cache.json` (or platform equivalent).
- **Cache TTL:** 24 hours (hardcoded reasonable default). SDK decides — not exposed to consumers.

## Impact on sonos-cli

### Removed
- `src/cache.rs` — deleted entirely
- `Action::Discover` variant — removed from `src/actions.rs`
- `Commands::Discover` — removed from `src/cli/mod.rs`
- `sonos-discovery` dependency — removed from `Cargo.toml`
- All cache-related logic in `main.rs`

### Simplified
- `main.rs` — just `SonosSystem::new()?` and pass to executor
- `executor.rs` — no cache loading/saving, no discovery orchestration
- Error messages — "speaker not found" is all the CLI needs to show (SDK handles retry internally)

### Unchanged
- `sonos speakers` — still lists speakers, just gets them from `system.speakers()`
- `sonos groups` — still lists groups from `system.groups()`
- `sonos status` — unchanged

## Impact on sonos-sdk

### New behavior in `SonosSystem::new()`
1. Check for cache file at `~/.config/sonos/cache.json`
2. If cache exists and is fresh (< 24h): load devices from cache, call `from_discovered_devices()`
3. If cache is missing or stale: run `sonos_discovery::get()`, save to cache, call `from_discovered_devices()`
4. Return `SonosSystem`

### New behavior in speaker/group lookups
- `get_speaker_by_name()`: if returns `None`, trigger one auto-rediscovery, retry lookup, then return `None` if still not found
- Same pattern for group lookups

### New internal module
- `src/cache.rs` (in the SDK) — `CachedSystem` struct, `load()`, `save()`, `is_stale()`. Same logic as current `sonos-cli/src/cache.rs` but operating on `Device` directly.

## Open Questions

None — all resolved during brainstorming.

## Next Steps

1. Implement caching in `sonos-sdk` (new `cache` module)
2. Update `SonosSystem::new()` to use cache
3. Add auto-rediscovery to speaker/group lookups
4. Remove `cache.rs`, `Action::Discover`, and `sonos-discovery` dependency from `sonos-cli`
5. Update the plan and roadmap to reflect the simplified architecture
