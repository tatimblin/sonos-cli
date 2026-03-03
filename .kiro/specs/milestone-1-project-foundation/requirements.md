# Requirements Document

## Introduction

Milestone 1 establishes the foundational architecture for `sonos-cli`, a Rust CLI/TUI application for controlling Sonos speakers. This milestone creates the project scaffolding, core types, and infrastructure that all subsequent features build upon. No user-visible features are delivered — the goal is a clean, compilable foundation with the Action dispatch pattern, error handling, caching, and configuration systems in place.

## Glossary

- **CLI**: Command-line interface for one-off commands (e.g., `sonos play`)
- **TUI**: Terminal user interface for interactive control (launched when no args given)
- **Action**: An enum representing all SDK operations; the single dispatch point for both CLI and TUI
- **Target**: An enum specifying which speaker or group an action applies to
- **Executor**: The module that receives Action values and calls SDK methods
- **SonosSystem**: The SDK entry point providing access to speakers and groups
- **Speaker**: An SDK handle representing a single Sonos device
- **Group**: An SDK handle representing a logical grouping of speakers
- **Cache**: A JSON file storing discovered speakers/groups to avoid repeated SSDP discovery
- **Config**: A TOML file storing user preferences (default group, cache TTL, theme)
- **CliError**: The application's domain error type with recovery hints and exit codes

## Requirements

### Requirement 1: Project Dependencies

**User Story:** As a developer, I want all required dependencies declared in Cargo.toml, so that the project compiles and has access to all necessary crates.

#### Acceptance Criteria

1. THE Cargo_Manifest SHALL declare `sonos-sdk` as a path dependency pointing to `"../sonos-sdk/sonos-sdk"`
2. THE Cargo_Manifest SHALL declare `clap` version 4 with the `derive` feature enabled
3. THE Cargo_Manifest SHALL declare `ratatui` and `crossterm` for TUI rendering
4. THE Cargo_Manifest SHALL declare `serde`, `serde_json`, and `toml` for serialization
5. THE Cargo_Manifest SHALL declare `dirs` for locating the user config directory
6. THE Cargo_Manifest SHALL declare `anyhow` for error propagation
7. THE Cargo_Manifest SHALL declare `thiserror` for domain error types
8. THE Cargo_Manifest SHALL declare `ratatui-image` and `image` for album art rendering

### Requirement 2: Application Entry Point

**User Story:** As a user, I want the application to launch the TUI when run without arguments and execute commands when given subcommands, so that I can use either mode seamlessly.

#### Acceptance Criteria

1. THE Main_Function SHALL return `ExitCode` for proper exit code control
2. THE Main_Function SHALL parse command-line arguments using the clap-derived `Cli` struct
3. WHEN no subcommand is provided AND stdout is a terminal, THE Main_Function SHALL launch the TUI
4. WHEN no subcommand is provided AND stdout is not a terminal, THE Main_Function SHALL print an error to stderr and return exit code 1
5. WHEN a subcommand is provided, THE Main_Function SHALL convert it to an Action and execute it via the executor
6. WHEN execution succeeds, THE Main_Function SHALL return `ExitCode::SUCCESS`
7. WHEN execution fails, THE Main_Function SHALL print the error to stderr and return the appropriate exit code

### Requirement 3: Action Enum Definition

**User Story:** As a developer, I want a comprehensive Action enum covering all SDK operations, so that both CLI and TUI can dispatch commands through a single unified interface.

#### Acceptance Criteria

1. THE Action_Enum SHALL define a `Discover` variant for running SSDP discovery and writing cache
2. THE Action_Enum SHALL define `ListSpeakers` and `ListGroups` variants for system queries
3. THE Action_Enum SHALL define a `Status` variant with a `target: Target` field
4. THE Action_Enum SHALL define playback variants: `Play`, `Pause`, `Stop`, `Next`, `Previous` each with a `target: Target` field
5. THE Action_Enum SHALL define a `Seek` variant with `position: String` and `target: Target` fields
6. THE Action_Enum SHALL define a `SetPlayMode` variant with `mode: PlayMode` and `target: Target` fields
7. THE Action_Enum SHALL define volume variants: `SetVolume` with `level: u8` and `target: Target`, `Mute` and `Unmute` with `target: Target`
8. THE Action_Enum SHALL define EQ variants: `SetBass` and `SetTreble` with `level: i8` and `speaker: String`, `SetLoudness` with `enabled: bool` and `speaker: String`
9. THE Action_Enum SHALL define queue variants: `ShowQueue`, `AddToQueue` with `uri: String`, and `ClearQueue` each with `target: Target`
10. THE Action_Enum SHALL define grouping variants: `JoinGroup` with `speaker: String` and `group: String`, `LeaveGroup` with `speaker: String`
11. THE Action_Enum SHALL define sleep timer variants: `SetSleepTimer` with `duration: String` and `target: Target`, `CancelSleepTimer` with `target: Target`

### Requirement 4: Target Enum Definition

**User Story:** As a developer, I want a Target enum to specify command targets, so that actions can be directed at specific speakers, groups, or use sensible defaults.

#### Acceptance Criteria

1. THE Target_Enum SHALL define a `Speaker(String)` variant containing the speaker's friendly name
2. THE Target_Enum SHALL define a `Group(String)` variant containing the group name
3. THE Target_Enum SHALL define a `Default` variant for when no explicit target is specified
4. WHEN resolving `Target::Default`, THE Executor SHALL use `config.default_group` if set, otherwise the first discovered group

### Requirement 5: Executor Module

**User Story:** As a developer, I want an executor module that translates Action values into SDK calls, so that SDK interaction is centralized in one place.

#### Acceptance Criteria

1. THE Executor SHALL expose a function `execute(action: Action, system: &SonosSystem) -> Result<String, CliError>`
2. THE Executor SHALL match on Action variants and call the corresponding SDK methods
3. THE Executor SHALL return human-readable success messages describing the completed operation
4. THE Executor SHALL expose a `resolve_target()` helper function that converts `Target` to a `Speaker` or `Group` handle
5. WHEN `Target::Speaker(name)` is provided, THE Resolver SHALL call `system.get_speaker_by_name(&name)`
6. WHEN `Target::Group(name)` is provided, THE Resolver SHALL find the group by coordinator name in `system.groups()`
7. IF a targeted speaker is not found, THEN THE Executor SHALL return `CliError::SpeakerNotFound`
8. IF a targeted group is not found, THEN THE Executor SHALL return `CliError::GroupNotFound`

### Requirement 6: Error Types

**User Story:** As a developer, I want a domain error type with recovery hints and exit codes, so that users receive actionable error messages and scripts can detect failure modes.

#### Acceptance Criteria

1. THE CliError_Enum SHALL use `thiserror` for derive-based error implementation
2. THE CliError_Enum SHALL define a `SpeakerNotFound(String)` variant with message `"speaker \"{0}\" not found"`
3. THE CliError_Enum SHALL define a `GroupNotFound(String)` variant with message `"group \"{0}\" not found"`
4. THE CliError_Enum SHALL define an `Sdk` variant that wraps SDK errors via `#[from]`
5. THE CliError_Enum SHALL define `Config(String)` and `Cache(String)` variants for configuration and cache errors
6. THE CliError_Enum SHALL define a `Validation(String)` variant for input validation errors
7. THE CliError_Enum SHALL implement `recovery_hint(&self) -> Option<&str>` returning actionable follow-up text
8. THE CliError_Enum SHALL implement `exit_code(&self) -> ExitCode` returning 1 for runtime errors and 2 for validation/usage errors

### Requirement 7: Cache System

**User Story:** As a user, I want discovered speakers and groups cached locally, so that commands execute quickly without repeated network discovery.

#### Acceptance Criteria

1. THE Cache_Module SHALL define a `CachedSystem` struct containing speakers list, groups list, and `cached_at: SystemTime`
2. THE Cache_Module SHALL define a `CachedSpeaker` struct with fields: `name`, `id`, `ip`, `model_name`
3. THE Cache_Module SHALL define a `CachedGroup` struct with fields: `id`, `coordinator_id`, `member_ids`
4. THE Cache_Module SHALL expose `load() -> Option<CachedSystem>` that reads from `~/.config/sonos/cache.json`
5. WHEN the cache file is missing or unparseable, THE Load_Function SHALL return `None`
6. THE Cache_Module SHALL expose `save(system: &SonosSystem) -> Result<()>` that writes the cache atomically
7. THE Save_Function SHALL write to a temporary file first, then use `fs::rename` for atomic replacement
8. THE Cache_Module SHALL expose `is_stale(cache: &CachedSystem, ttl_hours: u64) -> bool` that checks `cached_at + ttl`
9. THE Cache_Module SHALL use `dirs::config_dir()` to locate `~/.config/sonos/`
10. FOR ALL valid CachedSystem values, saving then loading SHALL produce an equivalent CachedSystem (round-trip property)

### Requirement 8: Configuration System

**User Story:** As a user, I want to configure default settings in a config file, so that I don't have to specify common options on every command.

#### Acceptance Criteria

1. THE Config_Struct SHALL use serde with `#[serde(default)]` for optional fields
2. THE Config_Struct SHALL define `default_group: Option<String>` for the default target group
3. THE Config_Struct SHALL define `cache_ttl_hours: u64` with a default value of 24
4. THE Config_Struct SHALL define `theme: String` with a default value of `"dark"`
5. THE Config_Module SHALL expose `Config::load() -> Config` that reads from `~/.config/sonos/config.toml`
6. WHEN the config file is missing or unparseable, THE Load_Function SHALL return a Config with all default values
7. THE Config_Module SHALL support environment variable overrides: `SONOS_DEFAULT_GROUP` and `SONOS_CONFIG_DIR`
8. WHEN `SONOS_CONFIG_DIR` is set, THE Config_Module SHALL use that directory instead of `~/.config/sonos/`


