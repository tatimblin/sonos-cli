//! Logging setup using `tracing` + `tracing-subscriber`.
//!
//! Initializes a global tracing subscriber with two layers:
//! - **File layer** (always): writes to `~/.local/share/sonos/sonos.log`, truncated per session.
//! - **Stderr layer** (CLI only): writes to stderr. Disabled in TUI mode to avoid corrupting the display.

use std::fs::{self, File};
use std::sync::Mutex;

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialize the global tracing subscriber.
///
/// Must be called early in `main()`, before any SDK calls (which emit tracing events).
///
/// - `verbosity`: from the `-v` flag count. 0 = warn (or RUST_LOG fallback), 1 = info, 2 = debug, 3+ = trace.
/// - `is_tui`: when true, omits the stderr layer (ratatui owns the terminal).
pub fn init_logging(verbosity: u8, is_tui: bool) {
    let filter = build_filter(verbosity);

    let file_layer =
        create_log_file().map(|file| fmt::layer().with_writer(Mutex::new(file)).with_ansi(false));

    let stderr_layer = if !is_tui {
        Some(fmt::layer().with_writer(std::io::stderr).without_time())
    } else {
        None
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(stderr_layer)
        .init();
}

/// Build an `EnvFilter` from the verbosity count.
///
/// When verbosity is 0, falls back to `RUST_LOG` env var if set, otherwise defaults to `warn`.
/// When verbosity > 0, the `-v` flag takes precedence over `RUST_LOG`.
fn build_filter(verbosity: u8) -> EnvFilter {
    if verbosity == 0 {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"))
    } else {
        let level = match verbosity {
            1 => "info",
            2 => "debug",
            _ => "trace",
        };
        EnvFilter::new(level)
    }
}

/// Create (or truncate) the log file, ensuring the parent directory exists.
///
/// Returns `None` with a warning on stderr if the file cannot be created.
fn create_log_file() -> Option<File> {
    let data_dir = dirs::data_local_dir()?;
    let log_dir = data_dir.join("sonos");

    if let Err(e) = fs::create_dir_all(&log_dir) {
        eprintln!(
            "warning: could not create log directory {}: {e}",
            log_dir.display()
        );
        return None;
    }

    let log_path = log_dir.join("sonos.log");
    match File::create(&log_path) {
        Ok(file) => Some(file),
        Err(e) => {
            eprintln!(
                "warning: could not create log file {}: {e}",
                log_path.display()
            );
            None
        }
    }
}
