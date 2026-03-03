# Design Document: Milestone 1 - Project Foundation

## Overview

This design establishes the foundational architecture for `sonos-cli`, a Rust CLI/TUI application for controlling Sonos speakers. The core architectural principle is **Action dispatch**: both CLI and TUI emit `Action` enum values, and a single `executor` module translates these into SDK calls. This separation ensures consistent behavior across interfaces and centralizes all SDK interaction.

The system is sync-first (no async/await) to match the underlying `sonos-sdk`. Discovery results are cached locally to avoid repeated SSDP network scans on every command.

### Key Design Decisions

1. **Single dispatch point**: The `executor.rs` module is the only place SDK methods are called. CLI and TUI code never import or call SDK types directly.

2. **Target resolution hierarchy**: `--group` flag wins over `--speaker`. When neither is specified, `Target::Default` resolves to `config.default_group` if set, otherwise the first discovered group.

3. **Atomic cache writes**: Cache updates use write-to-temp + rename to prevent corruption from interrupted writes.

4. **Exit code semantics**: 0 = success, 1 = runtime error (network, SDK), 2 = validation/usage error.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         main.rs                                  │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────┐  │
│  │ No args +   │    │ Subcommand  │    │ Config::load()      │  │
│  │ is_terminal │    │ provided    │    │ CachedSystem::load()│  │
│  └──────┬──────┘    └──────┬──────┘    └─────────────────────┘  │
│         │                  │                                     │
│         ▼                  ▼                                     │
│  ┌─────────────┐    ┌─────────────┐                             │
│  │ Launch TUI  │    │ CLI::parse()│                             │
│  │ (stub)      │    │ → Action    │                             │
│  └─────────────┘    └──────┬──────┘                             │
│                            │                                     │
└────────────────────────────┼─────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                       executor.rs                                │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │ execute(action: Action, system: &SonosSystem)            │    │
│  │   → Result<String, CliError>                             │    │
│  │                                                          │    │
│  │ resolve_target(target: Target, system, config)           │    │
│  │   → Result<ResolvedTarget, CliError>                     │    │
│  └─────────────────────────────────────────────────────────┘    │
│                            │                                     │
│                            ▼                                     │
│                    ┌───────────────┐                            │
│                    │  sonos-sdk    │                            │
│                    │  SonosSystem  │                            │
│                    │  Speaker      │                            │
│                    │  Group        │                            │
│                    └───────────────┘                            │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                    Supporting Modules                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │ actions.rs  │  │ cache.rs    │  │ config.rs               │  │
│  │ Action enum │  │ CachedSystem│  │ Config struct           │  │
│  │ Target enum │  │ load/save   │  │ load + env overrides    │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
│                                                                  │
│  ┌─────────────┐                                                │
│  │ errors.rs   │                                                │
│  │ CliError    │                                                │
│  └─────────────┘                                                │
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow

1. **Startup**: `main.rs` loads `Config` from `~/.config/sonos-cli/config.toml` and attempts to load `CachedSystem` from `~/.config/sonos-cli/cache.json`.

2. **CLI path**: When a subcommand is provided, `cli/mod.rs` parses arguments into an `Action` value. The action is passed to `executor::execute()` along with the `SonosSystem`.

3. **TUI path** (future): When no args and stdout is a terminal, the TUI launches. User interactions generate `Action` values that flow through the same executor.

4. **Execution**: The executor matches on the `Action` variant, resolves any `Target` to a concrete `Speaker` or `Group` handle, calls the appropriate SDK method, and returns a human-readable success message or `CliError`.

## Components and Interfaces

### actions.rs

Defines the `Action` and `Target` enums that form the command vocabulary.

```rust
/// Target specifies which speaker or group an action applies to.
pub enum Target {
    /// A specific speaker by friendly name
    Speaker(String),
    /// A specific group by coordinator name
    Group(String),
    /// Use config.default_group or first discovered group
    Default,
}

/// All operations the CLI/TUI can perform.
pub enum Action {
    // Discovery
    Discover,
    
    // Queries
    ListSpeakers,
    ListGroups,
    Status { target: Target },
    
    // Playback
    Play { target: Target },
    Pause { target: Target },
    Stop { target: Target },
    Next { target: Target },
    Previous { target: Target },
    Seek { position: String, target: Target },
    SetPlayMode { mode: PlayMode, target: Target },
    
    // Volume
    SetVolume { level: u8, target: Target },
    Mute { target: Target },
    Unmute { target: Target },
    
    // EQ (speaker-only)
    SetBass { level: i8, speaker: String },
    SetTreble { level: i8, speaker: String },
    SetLoudness { enabled: bool, speaker: String },
    
    // Queue
    ShowQueue { target: Target },
    AddToQueue { uri: String, target: Target },
    ClearQueue { target: Target },
    
    // Grouping
    JoinGroup { speaker: String, group: String },
    LeaveGroup { speaker: String },
    
    // Sleep timer
    SetSleepTimer { duration: String, target: Target },
    CancelSleepTimer { target: Target },
}
```

### executor.rs

The single point of SDK interaction.

```rust
use crate::actions::{Action, Target};
use crate::config::Config;
use crate::errors::CliError;
use sonos_sdk::{SonosSystem, Speaker, Group};

/// Resolved target after looking up speaker/group handles.
pub enum ResolvedTarget {
    Speaker(Speaker),
    Group(Group),
}

/// Resolve a Target to a concrete Speaker or Group handle.
pub fn resolve_target(
    target: Target,
    system: &SonosSystem,
    config: &Config,
) -> Result<ResolvedTarget, CliError>;

/// Execute an action against the Sonos system.
/// Returns a human-readable success message.
pub fn execute(
    action: Action,
    system: &SonosSystem,
    config: &Config,
) -> Result<String, CliError>;
```

**Resolution logic for `Target::Default`**:
1. If `config.default_group` is `Some(name)`, resolve as `Target::Group(name)`
2. Otherwise, use `system.groups().first()` and resolve to that group
3. If no groups exist, return `CliError::GroupNotFound("no groups discovered")`

### errors.rs

Domain error type with recovery hints and exit codes.

```rust
use std::process::ExitCode;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("speaker \"{0}\" not found")]
    SpeakerNotFound(String),
    
    #[error("group \"{0}\" not found")]
    GroupNotFound(String),
    
    #[error("SDK error: {0}")]
    Sdk(#[from] sonos_sdk::SdkError),
    
    #[error("configuration error: {0}")]
    Config(String),
    
    #[error("cache error: {0}")]
    Cache(String),
    
    #[error("validation error: {0}")]
    Validation(String),
}

impl CliError {
    /// Returns actionable follow-up text for the user.
    pub fn recovery_hint(&self) -> Option<&str> {
        match self {
            Self::SpeakerNotFound(_) | Self::GroupNotFound(_) => {
                Some("Run 'sonos discover' to refresh the speaker list.")
            }
            Self::Sdk(_) => Some("Check network connectivity and speaker power."),
            Self::Cache(_) => Some("Try 'sonos discover' to rebuild the cache."),
            _ => None,
        }
    }
    
    /// Returns the appropriate exit code.
    pub fn exit_code(&self) -> ExitCode {
        match self {
            Self::Validation(_) => ExitCode::from(2), // usage error
            _ => ExitCode::from(1),                   // runtime error
        }
    }
}
```

### cache.rs

Persistent storage for discovered speakers and groups.

```rust
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Serialize, Deserialize)]
pub struct CachedSpeaker {
    pub name: String,
    pub id: String,
    pub ip: String,
    pub model_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct CachedGroup {
    pub id: String,
    pub coordinator_id: String,
    pub member_ids: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CachedSystem {
    pub speakers: Vec<CachedSpeaker>,
    pub groups: Vec<CachedGroup>,
    pub cached_at: SystemTime,
}

impl CachedSystem {
    /// Load from ~/.config/sonos-cli/cache.json.
    /// Returns None if file is missing or unparseable.
    pub fn load() -> Option<Self>;
    
    /// Save to ~/.config/sonos-cli/cache.json atomically.
    /// Writes to temp file first, then renames.
    pub fn save(system: &SonosSystem) -> Result<(), std::io::Error>;
    
    /// Check if cache has exceeded TTL.
    pub fn is_stale(&self, ttl_hours: u64) -> bool;
}
```

**Atomic write implementation**:
```rust
pub fn save(system: &SonosSystem) -> Result<(), std::io::Error> {
    let cache_dir = dirs::config_dir()
        .map(|p| p.join("sonos"))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "config dir not found"))?;
    
    fs::create_dir_all(&cache_dir)?;
    
    let cache_path = cache_dir.join("cache.json");
    let temp_path = cache_dir.join("cache.json.tmp");
    
    let cached = CachedSystem::from_system(system);
    let json = serde_json::to_string_pretty(&cached)?;
    
    fs::write(&temp_path, json)?;
    fs::rename(&temp_path, &cache_path)?;
    
    Ok(())
}
```

### config.rs

User preferences with environment variable overrides.

```rust
use serde::Deserialize;

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub default_group: Option<String>,
    pub cache_ttl_hours: u64,
    pub theme: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_group: None,
            cache_ttl_hours: 24,
            theme: "dark".to_string(),
        }
    }
}

impl Config {
    /// Load from config file with environment variable overrides.
    pub fn load() -> Self {
        let config_dir = std::env::var("SONOS_CONFIG_DIR")
            .map(PathBuf::from)
            .or_else(|_| dirs::config_dir().map(|p| p.join("sonos")))
            .unwrap_or_else(|| PathBuf::from("."));
        
        let config_path = config_dir.join("config.toml");
        
        let mut config: Config = fs::read_to_string(&config_path)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default();
        
        // Environment variable overrides
        if let Ok(group) = std::env::var("SONOS_DEFAULT_GROUP") {
            config.default_group = Some(group);
        }
        
        config
    }
}
```

### main.rs

Entry point with mode detection.

```rust
use clap::Parser;
use std::io::IsTerminal;
use std::process::ExitCode;

mod actions;
mod cache;
mod cli;
mod config;
mod errors;
mod executor;

#[derive(Parser)]
#[command(name = "sonos", about = "Control Sonos speakers")]
struct Cli {
    #[command(subcommand)]
    command: Option<cli::Commands>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let config = config::Config::load();
    
    match cli.command {
        None => {
            if std::io::stdout().is_terminal() {
                // Launch TUI (future milestone)
                eprintln!("TUI not yet implemented");
                ExitCode::from(1)
            } else {
                eprintln!("error: no command specified and stdout is not a terminal");
                ExitCode::from(1)
            }
        }
        Some(cmd) => {
            let action = cmd.into_action();
            
            // Load or create SonosSystem
            let system = match load_or_discover(&config) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: {}", e);
                    if let Some(hint) = e.recovery_hint() {
                        eprintln!("{}", hint);
                    }
                    return e.exit_code();
                }
            };
            
            match executor::execute(action, &system, &config) {
                Ok(msg) => {
                    println!("{}", msg);
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("error: {}", e);
                    if let Some(hint) = e.recovery_hint() {
                        eprintln!("{}", hint);
                    }
                    e.exit_code()
                }
            }
        }
    }
}
```

### cli/mod.rs

Clap-derived command parsing that maps to `Action` values.

```rust
use clap::Subcommand;
use crate::actions::{Action, Target};
use sonos_sdk::PlayMode;

#[derive(Subcommand)]
pub enum Commands {
    /// Refresh speaker discovery cache
    Discover,
    
    /// List all speakers
    Speakers,
    
    /// List all groups
    Groups,
    
    /// Show current playback status
    Status {
        #[arg(long)]
        speaker: Option<String>,
        #[arg(long)]
        group: Option<String>,
    },
    
    /// Start playback
    Play {
        #[arg(long)]
        speaker: Option<String>,
        #[arg(long)]
        group: Option<String>,
    },
    
    /// Pause playback
    Pause {
        #[arg(long)]
        speaker: Option<String>,
        #[arg(long)]
        group: Option<String>,
    },
    
    // ... additional commands follow same pattern
}

impl Commands {
    pub fn into_action(self) -> Action {
        match self {
            Self::Discover => Action::Discover,
            Self::Speakers => Action::ListSpeakers,
            Self::Groups => Action::ListGroups,
            Self::Status { speaker, group } => Action::Status {
                target: resolve_target_args(speaker, group),
            },
            Self::Play { speaker, group } => Action::Play {
                target: resolve_target_args(speaker, group),
            },
            // ... etc
        }
    }
}

/// Convert CLI args to Target. --group wins over --speaker.
fn resolve_target_args(speaker: Option<String>, group: Option<String>) -> Target {
    match (group, speaker) {
        (Some(g), _) => Target::Group(g),
        (None, Some(s)) => Target::Speaker(s),
        (None, None) => Target::Default,
    }
}
```

## Data Models

### Cache File Format

Location: `~/.config/sonos-cli/cache.json` (or `$SONOS_CONFIG_DIR/cache.json`)

```json
{
  "speakers": [
    {
      "name": "Living Room",
      "id": "RINCON_000E58A0123456",
      "ip": "192.168.1.100",
      "model_name": "Sonos One"
    },
    {
      "name": "Kitchen",
      "id": "RINCON_000E58A0789012",
      "ip": "192.168.1.101",
      "model_name": "Sonos Play:1"
    }
  ],
  "groups": [
    {
      "id": "RINCON_000E58A0123456:1",
      "coordinator_id": "RINCON_000E58A0123456",
      "member_ids": ["RINCON_000E58A0123456", "RINCON_000E58A0789012"]
    }
  ],
  "cached_at": {
    "secs_since_epoch": 1709251200,
    "nanos_since_epoch": 0
  }
}
```

### Config File Format

Location: `~/.config/sonos-cli/config.toml` (or `$SONOS_CONFIG_DIR/config.toml`)

```toml
# Default group to target when --speaker/--group not specified
default_group = "Living Room"

# Hours before cache is considered stale (default: 24)
cache_ttl_hours = 24

# TUI color theme: "dark" or "light" (default: "dark")
theme = "dark"
```

### Environment Variables

| Variable | Effect |
|----------|--------|
| `SONOS_CONFIG_DIR` | Override config/cache directory (default: `~/.config/sonos-cli/`) |
| `SONOS_DEFAULT_GROUP` | Override `default_group` from config file |

### Exit Codes

| Code | Meaning | Examples |
|------|---------|----------|
| 0 | Success | Command completed |
| 1 | Runtime error | Network failure, speaker offline, SDK error |
| 2 | Usage/validation error | Invalid volume level, missing required arg |



## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property 1: CLI Dispatch Consistency

*For any* valid CLI subcommand, parsing the command and converting it to an Action should produce a deterministic Action value that correctly represents the user's intent, including all provided flags and arguments.

**Validates: Requirements 2.5**

### Property 2: Exit Code Correctness

*For any* execution result (success or failure), the returned exit code should be 0 for success, 1 for runtime errors (network, SDK, cache), and 2 for validation/usage errors. The exit code category should be consistent with the error type.

**Validates: Requirements 2.6, 2.7, 6.8**

### Property 3: Target Default Resolution

*For any* `Target::Default` value, resolution should return the group specified in `config.default_group` if set, otherwise the first group from `system.groups()`. If no groups exist, resolution should fail with `GroupNotFound`.

**Validates: Requirements 4.4**

### Property 4: Target Resolution Correctness

*For any* `Target::Speaker(name)` or `Target::Group(name)`, resolution should return the matching handle if it exists in the system, or return `SpeakerNotFound`/`GroupNotFound` respectively if no match exists. Speaker lookup uses `system.get_speaker_by_name()`, group lookup matches by coordinator name.

**Validates: Requirements 5.5, 5.6, 5.7, 5.8**

### Property 5: Executor Success Messages

*For any* successfully executed Action, the returned message string should be non-empty and describe the completed operation in human-readable form.

**Validates: Requirements 5.3**

### Property 6: Recovery Hint Coverage

*For any* `CliError` variant that represents a recoverable condition (SpeakerNotFound, GroupNotFound, Sdk, Cache), `recovery_hint()` should return `Some` with actionable guidance. For non-recoverable errors, it may return `None`.

**Validates: Requirements 6.7**

### Property 7: Cache Staleness Check

*For any* `CachedSystem` with `cached_at` timestamp and any `ttl_hours` value, `is_stale()` should return `true` if and only if the current time exceeds `cached_at + ttl_hours`. The check should be monotonic (once stale, always stale for the same inputs).

**Validates: Requirements 7.8**

### Property 8: Cache Round-Trip

*For any* valid `CachedSystem` value, serializing to JSON via `save()` and then deserializing via `load()` should produce an equivalent `CachedSystem` with the same speakers, groups, and `cached_at` timestamp.

**Validates: Requirements 7.10**

### Property 9: Config Environment Override

*For any* configuration where `SONOS_DEFAULT_GROUP` environment variable is set, `Config::load()` should return a Config with `default_group` equal to the environment variable value, regardless of what the config file contains. Similarly, `SONOS_CONFIG_DIR` should override the config file location.

**Validates: Requirements 8.7, 8.8**

### Property 10: Config Missing File Defaults

*For any* missing or unparseable config file, `Config::load()` should return a Config with `default_group = None`, `cache_ttl_hours = 24`, and `theme = "dark"`.

**Validates: Requirements 8.6**

## Error Handling

### Error Categories

| Category | Exit Code | Examples |
|----------|-----------|----------|
| Success | 0 | Command completed successfully |
| Runtime Error | 1 | Network failure, speaker offline, SDK error, cache I/O error |
| Validation Error | 2 | Invalid volume (>100), malformed duration, unknown subcommand |

### Error Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                        Error Sources                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │ SDK calls   │  │ Target      │  │ Config/Cache            │  │
│  │ SdkError    │  │ resolution  │  │ I/O errors              │  │
│  └──────┬──────┘  └──────┬──────┘  └───────────┬─────────────┘  │
│         │                │                     │                 │
│         ▼                ▼                     ▼                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                      CliError                                ││
│  │  - SpeakerNotFound(String)                                   ││
│  │  - GroupNotFound(String)                                     ││
│  │  - Sdk(SdkError)                                             ││
│  │  - Config(String)                                            ││
│  │  - Cache(String)                                             ││
│  │  - Validation(String)                                        ││
│  └──────────────────────────┬──────────────────────────────────┘│
│                             │                                    │
└─────────────────────────────┼────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                        main.rs                                   │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ eprintln!("error: {}", e);                                   ││
│  │ if let Some(hint) = e.recovery_hint() {                      ││
│  │     eprintln!("{}", hint);                                   ││
│  │ }                                                            ││
│  │ return e.exit_code();                                        ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

### Recovery Hints

| Error Type | Recovery Hint |
|------------|---------------|
| `SpeakerNotFound` | "Run 'sonos discover' to refresh the speaker list." |
| `GroupNotFound` | "Run 'sonos discover' to refresh the speaker list." |
| `Sdk` | "Check network connectivity and speaker power." |
| `Cache` | "Try 'sonos discover' to rebuild the cache." |
| `Config` | None (user should fix config file manually) |
| `Validation` | None (error message should explain the issue) |

### Error Message Format

All errors follow the format specified in `docs/references/cli-guidelines.md`:

```
error: <description>
<recovery hint if available>
```

Example:
```
error: speaker "Bedroom" not found
Run 'sonos discover' to refresh the speaker list.
```

## Testing Strategy

### Dual Testing Approach

This milestone uses both unit tests and property-based tests:

- **Unit tests**: Verify specific examples, edge cases, and integration points
- **Property tests**: Verify universal properties across randomized inputs

Both are complementary—unit tests catch concrete bugs while property tests verify general correctness.

### Property-Based Testing Configuration

- **Library**: `proptest` crate for Rust property-based testing
- **Iterations**: Minimum 100 iterations per property test
- **Tagging**: Each test includes a comment referencing the design property

Tag format: `// Feature: milestone-1-project-foundation, Property {number}: {property_text}`

### Test Organization

```
tests/
  cache_tests.rs      ← Cache round-trip, staleness checks
  config_tests.rs     ← Config loading, env overrides, defaults
  executor_tests.rs   ← Target resolution, success messages
  errors_tests.rs     ← Exit codes, recovery hints
  cli_tests.rs        ← CLI dispatch consistency
```

### Property Test Implementations

#### Property 8: Cache Round-Trip

```rust
// Feature: milestone-1-project-foundation, Property 8: Cache Round-Trip
proptest! {
    #[test]
    fn cache_roundtrip(
        speakers in prop::collection::vec(arbitrary_cached_speaker(), 0..10),
        groups in prop::collection::vec(arbitrary_cached_group(), 0..5),
    ) {
        let original = CachedSystem {
            speakers,
            groups,
            cached_at: SystemTime::now(),
        };
        
        // Save to temp file
        let temp_dir = tempfile::tempdir()?;
        let cache_path = temp_dir.path().join("cache.json");
        save_to_path(&original, &cache_path)?;
        
        // Load back
        let loaded = load_from_path(&cache_path)?;
        
        prop_assert_eq!(original.speakers, loaded.speakers);
        prop_assert_eq!(original.groups, loaded.groups);
    }
}
```

#### Property 7: Cache Staleness Check

```rust
// Feature: milestone-1-project-foundation, Property 7: Cache Staleness Check
proptest! {
    #[test]
    fn cache_staleness_monotonic(
        hours_ago in 0u64..1000,
        ttl_hours in 1u64..100,
    ) {
        let cached_at = SystemTime::now() - Duration::from_secs(hours_ago * 3600);
        let cache = CachedSystem {
            speakers: vec![],
            groups: vec![],
            cached_at,
        };
        
        let is_stale = cache.is_stale(ttl_hours);
        let expected_stale = hours_ago >= ttl_hours;
        
        prop_assert_eq!(is_stale, expected_stale);
    }
}
```

#### Property 4: Target Resolution Correctness

```rust
// Feature: milestone-1-project-foundation, Property 4: Target Resolution Correctness
proptest! {
    #[test]
    fn target_resolution_speaker_not_found(
        speaker_name in "[a-zA-Z ]{1,20}",
    ) {
        let system = mock_system_with_speakers(&["Living Room", "Kitchen"]);
        let config = Config::default();
        
        if speaker_name != "Living Room" && speaker_name != "Kitchen" {
            let result = resolve_target(
                Target::Speaker(speaker_name.clone()),
                &system,
                &config,
            );
            prop_assert!(matches!(result, Err(CliError::SpeakerNotFound(_))));
        }
    }
}
```

### Unit Test Coverage

| Module | Unit Test Focus |
|--------|-----------------|
| `cache.rs` | Missing file returns None, malformed JSON returns None, atomic write creates temp file |
| `config.rs` | Default values correct, TOML parsing, env var precedence |
| `executor.rs` | Each Action variant calls correct SDK method, error propagation |
| `errors.rs` | Error message formatting, Display impl correctness |
| `cli/mod.rs` | Flag parsing, --group wins over --speaker |

### Integration Tests

Integration tests verify end-to-end behavior with a mock SDK:

1. **CLI dispatch**: Parse args → Action → execute → verify output
2. **Cache lifecycle**: Discover → save → load → verify equivalence
3. **Config precedence**: File + env vars → verify merged config

### Test Helpers

```rust
// Mock SonosSystem for testing without network
fn mock_system_with_speakers(names: &[&str]) -> MockSonosSystem;

// Arbitrary generators for proptest
fn arbitrary_cached_speaker() -> impl Strategy<Value = CachedSpeaker>;
fn arbitrary_cached_group() -> impl Strategy<Value = CachedGroup>;
fn arbitrary_target() -> impl Strategy<Value = Target>;
```

## Future Considerations

The following interfaces are intentionally not defined in this milestone to avoid premature design:

- **Query Interface for TUI**: The executor will be extended with query functions (`query_speaker()`, `query_group()`, `list_all_speakers()`, `list_all_groups()`) for TUI state access in a future milestone. These functions will return plain data structs (`SpeakerInfo`, `GroupInfo`, `SpeakerState`, `GroupState`) rather than SDK handles.

- **Property Watching Interface**: Property watching (`watch_property()`, `unwatch_property()`, `change_events()`) will be added for reactive TUI updates in a future milestone. This will enable the TUI to re-render when speaker/group state changes without polling.

These interfaces will be designed when the TUI milestone begins, informed by the actual TUI implementation needs.
