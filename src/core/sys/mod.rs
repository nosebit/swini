//! Host system driver abstraction, platform detection, and process management.
//!
//! Exposes the [`Sys`] trait and runtime [`select()`] factory that instantiates
//! the appropriate driver adapter based on target host OS detection.

pub mod types;
pub mod variants;

#[allow(unused_imports)]
pub use types::{ProcessStats, Sys};
#[allow(unused_imports)]
pub use variants::{LinuxSys, MacSys, UbuntuSys, UnixSys};

use std::sync::Arc;

/// Selects and instantiates the appropriate [`Sys`] adapter matching the
/// current runtime host environment.
pub fn select() -> Arc<dyn Sys> {
  match std::env::consts::OS {
    "macos" => Arc::new(MacSys::new()),
    "linux" => {
      // Check for Ubuntu or specific Linux distribution via /etc/os-release
      if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        if content
          .lines()
          .any(|l| l.starts_with("ID=") && l.contains("ubuntu"))
        {
          return Arc::new(UbuntuSys::new());
        }
      }
      Arc::new(LinuxSys::new())
    }
    _ => Arc::new(UnixSys::new()),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_select_returns_valid_sys() {
    let sys = select();
    assert!(!sys.name().is_empty());
  }
}
