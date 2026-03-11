---
topic: CLI Architecture Simplification
date: 2026-03-10
status: complete
---

# CLI Architecture Simplification

## What We're Building

Remove the Action enum and executor indirection layer from sonos-cli. The SDK has a clean DOM-like API (`system.get_speaker_by_name("Kitchen")?.play()`) — the CLI should call it directly from clap command handlers instead of going through an intermediate Action enum + execute() dispatch function.

Also: remove `from_discovered_devices()` from the SDK's public API, keep `Device` internal to the SDK, and make `SonosSystem::new()` the only constructor.

## Why This Approach

The golden path analogy is the web DOM API: `document.getElementById("foo")` just works — you don't discover elements first, you don't wrap calls in an action dispatcher. The SDK should work the same way.

**The Action enum adds zero value today:**
- Maps 1:1 to SDK methods (no aggregation, no cross-cutting concerns)
- Forces every new command through 4 files instead of 2
- Duplicates types already in the SDK (`PlayMode`)
- Returns `Result<String>` which flattens structured SDK responses the TUI will need
- The TUI doesn't exist yet (Milestone 6+) — premature abstraction

**What well-regarded Rust CLIs do:** ripgrep, cargo, bat, fd all dispatch directly from clap-parsed structs to library calls. None use an intermediate "action enum" repackaging layer.

## Key Decisions

1. **Delete `actions.rs` and `executor.rs` entirely.** No Action enum, no execute() dispatch function. The SDK is the shared API layer — both CLI and TUI call SDK methods directly.

2. **CLI commands call SDK directly from clap handlers.** Each `Commands` variant resolves its target (speaker or group) and calls the SDK method inline. Output formatting happens at the call site.

3. **TUI calls SDK directly too.** When the TUI arrives, keypress handlers call `speaker.play()`, `speaker.set_volume()` etc. directly on the speaker/group handles they already hold. No shared dispatch layer needed — the SDK IS the shared layer.

4. **`SonosSystem::new()` is the only constructor.** Remove `from_discovered_devices()` from the public API. `Device` stays internal to the SDK. Consumers never see discovery internals.

5. **Remove `pub use sonos_discovery` from SDK.** The discovery crate is an implementation detail. SDK consumers don't need it.

6. **Target resolution stays in the CLI.** The `--speaker` / `--group` flag logic is CLI-specific. A `resolve_speaker()` or `resolve_group()` helper in the CLI module handles this — it's not SDK responsibility.

7. **Update CLAUDE.md Rule 1.** Replace "Action dispatch only" with: "SDK methods are called directly from CLI command handlers and TUI event handlers. No intermediate dispatch layer."

## SDK Interface Improvements

The SDK should be ergonomic enough that the CLI needs almost no glue code. These changes make the SDK a better fit for CLI (and future TUI) consumption:

### 1. `SonosSystem::new()` is the only public constructor

Remove `from_discovered_devices()` from the public API (keep as `pub(crate)` for internal use). Remove `pub use sonos_discovery;` — the `Device` type stays internal. Consumers call `SonosSystem::new()` and everything works.

### 2. Add `get_group_by_name(&str) -> Option<Group>`

Matches the `get_speaker_by_name()` pattern. The CLI needs to resolve `--group "Living Room"` to a `Group` handle. Without this, every consumer has to iterate and filter `system.groups()`.

### 3. Test support via feature flag

The SDK provides a test constructor behind `#[cfg(feature = "test-support")]`:

```rust
impl SonosSystem {
    #[cfg(feature = "test-support")]
    pub fn with_speakers(names: &[&str]) -> Self { ... }
}
```

This creates an in-memory system with fake speakers — no network, no discovery. CLI tests use `sonos-sdk = { ..., features = ["test-support"] }` in `[dev-dependencies]`.

### 4. Speaker has public fields and Display

Speaker already exposes `name`, `id`, `ip`, `model_name` as public fields. The CLI formats them directly — no summary methods needed. SDK types like `Volume`, `PlaybackState` should implement `Display` for clean terminal output.

## What Changes

### SDK (`sonos-sdk`)
- Remove `pub use sonos_discovery;` from `lib.rs`
- Make `from_discovered_devices()` `pub(crate)` (still needed internally)
- Add `get_group_by_name(&str) -> Option<Group>` to `SonosSystem`
- Add `#[cfg(feature = "test-support")] pub fn with_speakers(names: &[&str]) -> Self`
- Ensure SDK value types implement `Display` where they don't already

### CLI (`sonos-cli`)
- Delete `src/actions.rs`
- Delete `src/executor.rs`
- Move target resolution helpers into `src/cli/mod.rs`
- Each `Commands` variant handles its own SDK call and output formatting
- `main.rs` creates `SonosSystem::new()`, passes to CLI dispatch
- Update CLAUDE.md to reflect new architecture
- Tests use `sonos-sdk/test-support` feature

## Resolved Questions

**Q: How do CLI and TUI share SDK interaction logic?**
A: They don't need a shared layer. The SDK is the shared layer. Both call `speaker.play()` directly.

**Q: Where does target resolution live?**
A: In the CLI module. It's CLI-specific logic (`--speaker` / `--group` flags). The TUI has its own target selection (focused speaker/group in the UI).

**Q: How do you test CLI logic without network access?**
A: SDK provides `SonosSystem::with_speakers(&["Kitchen", "Bedroom"])` behind a `test-support` feature flag. Creates in-memory system with fake speakers.

**Q: How does the CLI look up groups by name?**
A: SDK adds `system.get_group_by_name("Living Room")` matching the speaker pattern.

**Q: How does the CLI format SDK data for terminal output?**
A: Speaker has public fields. CLI formats them directly. SDK types implement `Display`.

## Out of Scope

- TUI implementation (Milestone 6+)
- New CLI commands beyond what's already stubbed
- Streaming/watch features (TUI-only concern)
