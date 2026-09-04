//! Telemetry, distributed tracing, and non-blocking rolling file logging for
//! Swini.
//!
//! Configures a unified `tracing` subscriber registry that supports:
//! - Daily rolling file logs writing to `{data_dir}/logs/regent.log`.
//! - Non-blocking asynchronous writes via `tracing-appender`.
//! - Conditional stdout console logging when running in the foreground.
//! - Dynamic log level filtering controlled by `SWINI_LOG_LEVEL` (fallback
//!   `RUST_LOG`).

use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter, Layer};

/// Guard maintaining the lifecycle of the non-blocking background logging
/// worker thread.
///
/// Ensures buffered log entries are flushed to disk before the application
/// terminates.
pub struct TelemetryGuard {
  _file_guard: Option<WorkerGuard>,
}

/// Initializes global telemetry and logging according to execution mode.
///
/// # Arguments
/// - `console_enabled`: Set to `true` when running in the foreground to stream
///   formatted logs to stdout.
/// - `log_dir`: Optional directory path where `regent.log` daily files will be
///   stored.
/// - `_log_prefix`: Optional prefix name for log files (defaults to
///   `regent.log`).
pub fn init(
  console_enabled: bool,
  log_dir: Option<&Path>,
  _log_prefix: Option<&str>,
) -> TelemetryGuard {
  let filter_str = std::env::var("SWINI_LOG_LEVEL")
    .or_else(|_| std::env::var("RUST_LOG"))
    .unwrap_or_else(|_| "info,openraft=warn".to_string());

  let env_filter = EnvFilter::new(filter_str);

  let mut file_guard = None;
  let file_layer = if let Some(dir) = log_dir {
    let _ = std::fs::create_dir_all(dir);
    let file_appender = tracing_appender::rolling::daily(dir, "regent.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    file_guard = Some(guard);

    Some(
      fmt::layer()
        .with_ansi(false)
        .with_writer(non_blocking)
        .with_target(true)
        .with_filter(EnvFilter::new("info,openraft=warn")),
    )
  } else {
    None
  };

  let console_layer = if console_enabled {
    Some(fmt::layer().with_ansi(true).with_target(false))
  } else {
    None
  };

  let _ = tracing_subscriber::registry()
    .with(env_filter)
    .with(file_layer)
    .with(console_layer)
    .try_init();

  TelemetryGuard {
    _file_guard: file_guard,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::tempdir;

  #[test]
  fn test_telemetry_initialization() {
    let _guard = init(true, None, None);
    tracing::info!("test info message");
  }

  #[test]
  fn test_file_logging_initialization() {
    let dir = tempdir().unwrap();
    let _guard = init(false, Some(dir.path()), Some("regent"));
    tracing::info!("test file info message");
  }
}
