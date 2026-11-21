//! Logging setup using `tracing` + `tracing-subscriber`.
//!
//! Initializes a global tracing subscriber with two layers:
//! - **File layer** (always): writes to `~/.local/share/sonos/sonos.log`, appended to by
//!   every run and rotated to `sonos.log.1` once it passes 1 MB.
//! - **Stderr layer** (CLI only): writes to stderr. Disabled in TUI mode to avoid corrupting the display.

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::Path;
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

/// Rotate the log once it grows past this size at startup.
const MAX_LOG_BYTES: u64 = 1024 * 1024;

/// Open the log file for appending, ensuring the parent directory exists.
///
/// Appends rather than truncates: the TUI and one-off CLI commands share the
/// file, so truncating on startup would wipe a running TUI session's log.
/// Returns `None` with a warning on stderr if the file cannot be opened.
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
    match open_log_file(&log_path, MAX_LOG_BYTES) {
        Ok(file) => Some(file),
        Err(e) => {
            eprintln!(
                "warning: could not open log file {}: {e}",
                log_path.display()
            );
            None
        }
    }
}

/// Rotate `path` if it exceeds `max_bytes`, then open it for appending.
///
/// Rotation is best-effort: a failed rename just keeps appending.
fn open_log_file(path: &Path, max_bytes: u64) -> io::Result<File> {
    if let Err(e) = rotate_if_larger_than(path, max_bytes) {
        eprintln!("warning: could not rotate log file {}: {e}", path.display());
    }
    OpenOptions::new().create(true).append(true).open(path)
}

/// Move `path` to `<path>.1` (replacing any previous one) if it is larger than
/// `max_bytes`. A missing file is left alone.
fn rotate_if_larger_than(path: &Path, max_bytes: u64) -> io::Result<()> {
    match fs::metadata(path) {
        Ok(meta) if meta.len() > max_bytes => {}
        Ok(_) => return Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    }
    let mut rotated = path.as_os_str().to_owned();
    rotated.push(".1");
    // `rename` does not replace an existing target on Windows.
    match fs::remove_file(&rotated) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
        _ => {}
    }
    fs::rename(path, rotated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn rotated(path: &Path) -> std::path::PathBuf {
        let mut p = path.as_os_str().to_owned();
        p.push(".1");
        p.into()
    }

    #[test]
    fn appends_instead_of_truncating() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sonos.log");
        fs::write(&path, "first\n").unwrap();
        open_log_file(&path, 1024)
            .unwrap()
            .write_all(b"second\n")
            .unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "first\nsecond\n");
        assert!(!rotated(&path).exists());
    }

    #[test]
    fn creates_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sonos.log");
        open_log_file(&path, 1024).unwrap();
        assert!(path.exists());
        assert!(!rotated(&path).exists());
    }

    #[test]
    fn rotates_oversized_file_replacing_previous_rotation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sonos.log");
        fs::write(rotated(&path), "ancient\n").unwrap();
        fs::write(&path, "x".repeat(11)).unwrap();

        open_log_file(&path, 10)
            .unwrap()
            .write_all(b"fresh\n")
            .unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "fresh\n");
        assert_eq!(fs::read_to_string(rotated(&path)).unwrap(), "x".repeat(11));
    }

    #[test]
    fn file_at_the_limit_is_not_rotated() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sonos.log");
        fs::write(&path, "x".repeat(10)).unwrap();
        rotate_if_larger_than(&path, 10).unwrap();
        assert!(path.exists());
        assert!(!rotated(&path).exists());
    }
}
