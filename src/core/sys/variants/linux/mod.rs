//! Generic Linux host system adapter.
//!
//! Provides the generic Linux system driver and exports specialized
//! distribution variants.

pub mod ubuntu;

use crate::core::sys::types::{ProcessStats, Sys};
use crate::core::sys::variants::unix::UnixSys;
use std::collections::HashMap;
use std::error::Error;
use std::ops::Deref;
use std::path::Path;
pub use ubuntu::UbuntuSys;

/// Generic Linux host system adapter composing standard Unix primitives.
#[derive(Debug, Default, Clone)]
pub struct LinuxSys {
  base: UnixSys,
}

impl LinuxSys {
  /// Creates a new `LinuxSys` driver instance.
  pub fn new() -> Self {
    Self {
      base: UnixSys::new(),
    }
  }
}

impl Deref for LinuxSys {
  type Target = UnixSys;

  fn deref(&self) -> &Self::Target {
    &self.base
  }
}

impl Sys for LinuxSys {
  fn name(&self) -> &'static str {
    "linux"
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
  fn test_linux_sys_name() {
    let sys = LinuxSys::new();
    assert_eq!(sys.name(), "linux");
  }
}
