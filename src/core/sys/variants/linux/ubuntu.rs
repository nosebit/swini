//! Ubuntu Linux host system adapter.
//!
//! Extends base Linux host system driver functionality for Ubuntu environments.

use super::LinuxSys;
use crate::core::sys::types::{ProcessStats, Sys};
use std::collections::HashMap;
use std::error::Error;
use std::ops::Deref;
use std::path::Path;

/// Ubuntu Linux host system adapter extending Linux base functionality.
#[derive(Debug, Default, Clone)]
pub struct UbuntuSys {
  base: LinuxSys,
}

impl UbuntuSys {
  /// Creates a new `UbuntuSys` driver instance.
  pub fn new() -> Self {
    Self {
      base: LinuxSys::new(),
    }
  }
}

impl Deref for UbuntuSys {
  type Target = LinuxSys;

  fn deref(&self) -> &Self::Target {
    &self.base
  }
}

impl Sys for UbuntuSys {
  fn name(&self) -> &'static str {
    "ubuntu"
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
  fn test_ubuntu_sys_name() {
    let sys = UbuntuSys::new();
    assert_eq!(sys.name(), "ubuntu");
  }
}
