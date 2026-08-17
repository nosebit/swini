//! This module is responsible for setting up the telemetry used throughout the
//! code. We currently use the tracing crate to allow logging and execution
//! tracing.
use tracing_subscriber::EnvFilter;

/// This function initializes the telemetry.
pub fn init() {
  // Build a subscriber configured for formatting text to the console
  tracing_subscriber::fmt()
    // Read the RUST_LOG environment variable to determine the log level.
    // If the variable isn't set, default to the "info" level.
    .with_env_filter(
      EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info")),
    )
    // Install this subscriber as the global default for the entire app.
    // We use `try_init().ok()` instead of `init()` so that if this is called
    // multiple times (e.g., by different unit tests running in parallel),
    // it won't panic.
    .try_init()
    .ok();
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_telemetry_initialization() {
    // Calling it once should succeed and set up the telemetry.
    init();

    // Calling it a second time should gracefully do nothing because we used
    // `try_init().ok()`, ensuring our unit tests won't panic if they share
    // the same process!
    init();
  }
}
