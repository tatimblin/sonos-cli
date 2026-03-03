# Implementation Plan: Milestone 1 - Project Foundation

## Overview

This plan establishes the foundational architecture for `sonos-cli` in Rust. The focus is on creating compilable type definitions, module structure, and infrastructure (cache, config, errors) without implementing actual SDK calls. The executor will be a stub that compiles but returns placeholder messages.

## Tasks

- [x] 1. Set up project structure and dependencies
  - [x] 1.1 Create Cargo.toml with all required dependencies
    - Add `sonos-sdk` as path dependency to `"../sonos-sdk/sonos-sdk"`
    - Add `clap` v4 with `derive` feature
    - Add `ratatui` and `crossterm` for TUI
    - Add `serde`, `serde_json`, `toml` for serialization
    - Add `dirs` for config directory location
    - Add `anyhow` and `thiserror` for error handling
    - Add `ratatui-image` and `image` for album art
    - Add `proptest` as dev-dependency for property tests
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8_

  - [x] 1.2 Create module structure with empty files
    - Create `src/main.rs`, `src/actions.rs`, `src/errors.rs`
    - Create `src/config.rs`, `src/cache.rs`, `src/executor.rs`
    - Create `src/cli/mod.rs`
    - _Requirements: 2.1, 2.2_

- [x] 2. Implement core type definitions
  - [x] 2.1 Implement Action and Target enums in `src/actions.rs`
    - Define `Target` enum with `Speaker(String)`, `Group(String)`, `Default` variants
    - Define `Action` enum with all variants: Discover, ListSpeakers, ListGroups, Status, Play, Pause, Stop, Next, Previous, Seek, SetPlayMode, SetVolume, Mute, Unmute, SetBass, SetTreble, SetLoudness, ShowQueue, AddToQueue, ClearQueue, JoinGroup, LeaveGroup, SetSleepTimer, CancelSleepTimer
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 3.9, 3.10, 3.11, 4.1, 4.2, 4.3_

  - [x] 2.2 Implement CliError enum in `src/errors.rs`
    - Define error variants: SpeakerNotFound, GroupNotFound, Sdk, Config, Cache, Validation
    - Use `thiserror` derive macro with appropriate error messages
    - Implement `recovery_hint(&self) -> Option<&str>` method
    - Implement `exit_code(&self) -> ExitCode` method (1 for runtime, 2 for validation)
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7, 6.8_

- [x] 3. Checkpoint - Verify core types compile
  - Ensure `cargo build` succeeds with Action, Target, and CliError types
  - Ask the user if questions arise

- [x] 4. Implement configuration system
  - [x] 4.1 Implement Config struct and load() in `src/config.rs`
    - Define `Config` struct with `default_group: Option<String>`, `cache_ttl_hours: u64`, `theme: String`
    - Use `#[serde(default)]` for optional fields
    - Implement `Default` trait with `cache_ttl_hours = 24`, `theme = "dark"`
    - Implement `Config::load()` reading from `~/.config/sonos-cli/config.toml`
    - Support `SONOS_CONFIG_DIR` env var override for config directory
    - Support `SONOS_DEFAULT_GROUP` env var override for default_group
    - Return defaults when file is missing or unparseable
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5, 8.6, 8.7, 8.8_

  - [ ]* 4.2 Write property test for Config environment override
    - **Property 9: Config Environment Override**
    - Test that `SONOS_DEFAULT_GROUP` env var overrides config file value
    - Test that `SONOS_CONFIG_DIR` env var changes config file location
    - **Validates: Requirements 8.7, 8.8**

  - [ ]* 4.3 Write property test for Config missing file defaults
    - **Property 10: Config Missing File Defaults**
    - Test that missing/unparseable config returns correct defaults
    - Verify `default_group = None`, `cache_ttl_hours = 24`, `theme = "dark"`
    - **Validates: Requirements 8.6**

- [x] 5. Implement cache system
  - [x] 5.1 Implement cache structs and functions in `src/cache.rs`
    - Define `CachedSpeaker` struct with `name`, `id`, `ip`, `model_name` fields
    - Define `CachedGroup` struct with `id`, `coordinator_id`, `member_ids` fields
    - Define `CachedSystem` struct with `speakers`, `groups`, `cached_at: SystemTime`
    - Implement `CachedSystem::load() -> Option<Self>` reading from `~/.config/sonos-cli/cache.json`
    - Implement `CachedSystem::save(&self) -> Result<()>` with atomic write (temp file + rename)
    - Implement `CachedSystem::is_stale(&self, ttl_hours: u64) -> bool`
    - Use `dirs::config_dir()` for cache location
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 7.7, 7.8, 7.9_

  - [x] 5.2 Write property test for cache staleness check
    - **Property 7: Cache Staleness Check**
    - Test that `is_stale()` returns true iff current time exceeds `cached_at + ttl_hours`
    - Verify monotonicity (once stale, always stale for same inputs)
    - **Validates: Requirements 7.8**

  - [ ]* 5.3 Write property test for cache round-trip
    - **Property 8: Cache Round-Trip**
    - Test that save() then load() produces equivalent CachedSystem
    - Verify speakers, groups, and cached_at are preserved
    - **Validates: Requirements 7.10**

- [x] 6. Checkpoint - Verify config and cache compile and test
  - Ensure `cargo build` succeeds
  - Ensure `cargo test` passes for cache round-trip
  - Ask the user if questions arise

- [x] 7. Implement executor stub
  - [x] 7.1 Implement executor stub in `src/executor.rs`
    - Define `ResolvedTarget` enum with `Speaker` and `Group` variants (using placeholder types for now)
    - Implement `resolve_target(target: Target, config: &Config) -> Result<ResolvedTarget, CliError>` stub
    - Implement `execute(action: Action, config: &Config) -> Result<String, CliError>` stub
    - Stub should match on Action variants and return placeholder success messages
    - Return appropriate errors for Target::Speaker/Group when name doesn't match (stub behavior)
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8_

- [x] 8. Implement CLI parsing
  - [x] 8.1 Implement Clap Commands enum in `src/cli/mod.rs`
    - Define `Commands` enum with all subcommands using clap derive
    - Add `--speaker` and `--group` flags to relevant commands
    - Implement `Commands::into_action(self) -> Action` conversion
    - Implement `resolve_target_args()` helper (--group wins over --speaker)
    - _Requirements: 2.3, 2.5_

  - [x] 8.2 Implement main.rs entry point
    - Define `Cli` struct with optional `Commands` subcommand
    - Implement mode detection: TUI when no args + is_terminal, CLI otherwise
    - Print error and return exit code 1 when no args and not a terminal
    - Convert subcommand to Action and call executor
    - Print success message or error with recovery hint
    - Return appropriate ExitCode (SUCCESS, or from CliError)
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7_

- [x] 9. Final checkpoint - Verify complete build
  - Ensure `cargo build` succeeds
  - Ensure `cargo test` passes
  - Verify Action enum and executor stub compile cleanly
  - Verify cache round-trips correctly in unit test
  - Verify config loads defaults when no file exists
  - Ask the user if questions arise

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- The executor is intentionally a stub - actual SDK integration is Milestone 2
- Property tests use the `proptest` crate for randomized input generation
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation of the build
