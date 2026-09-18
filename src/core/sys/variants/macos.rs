//! macOS (Darwin) host system adapter.
//!
//! Extends base Unix driver functionality with macOS-specific behaviors.

use crate::core::sys::types::{ProcessStats, Sys};
use crate::core::sys::variants::unix::UnixSys;
use std::collections::HashMap;
use std::error::Error;
use std::ops::Deref;
use std::path::Path;

/// macOS (Darwin) host system adapter composing standard Unix primitives.
#[derive(Debug, Default, Clone)]
pub struct MacSys {
  base: UnixSys,
}

impl MacSys {
  /// Creates a new `MacSys` driver instance.
  pub fn new() -> Self {
    Self {
      base: UnixSys::new(),
    }
  }
}

impl Deref for MacSys {
  type Target = UnixSys;

  fn deref(&self) -> &Self::Target {
    &self.base
  }
}

impl Sys for MacSys {
  fn name(&self) -> &'static str {
    "macos"
  }

  fn spawn(
    &self,
    command: &[String],
    cwd: Option<&Path>,
    env: &HashMap<String, String>,
  ) -> Result<u32, Box<dyn Error>> {
    self.base.spawn(command, cwd, env)
  }

  fn terminate(&self, pid: u32) -> Result<(), Box<dyn Error>> {
    self.base.terminate(pid)
  }

  fn kill(&self, pid: u32) -> Result<(), Box<dyn Error>> {
    self.base.kill(pid)
  }

  fn is_alive(&self, pid: u32) -> bool {
    self.base.is_alive(pid)
  }

  fn stats(&self, pid: u32) -> Result<ProcessStats, Box<dyn Error>> {
    self.base.stats(pid)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_mac_sys_name() {
    let sys = MacSys::new();
    assert_eq!(sys.name(), "macos");
  }

  #[test]
  fn test_mac_sys_lifecycle() {
    let sys = MacSys::new();
    let cmd = vec!["echo".to_string(), "hello".to_string()];
    let pid = sys.spawn(&cmd, None, &HashMap::new()).unwrap();
    assert!(pid > 0);
  }
}
