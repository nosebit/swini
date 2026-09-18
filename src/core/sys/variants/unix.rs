//! Base Unix/POSIX host system driver implementation.
//!
//! Provides the foundational Unix process driver using standard `libc`
//! signaling, `std::process::Command` process spawning, and `sysinfo` process
//! metrics inspection.

use crate::core::sys::types::{ProcessStats, Sys};
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use sysinfo::{Pid, ProcessesToUpdate, System};

/// Base Unix/POSIX implementation leveraging standard libc and process
/// primitives.
#[derive(Debug, Default, Clone)]
pub struct UnixSys;

impl UnixSys {
  /// Creates a new `UnixSys` driver instance.
  pub fn new() -> Self {
    Self
  }

  /// Spawns a process command using `std::process::Command`.
  ///
  /// # Errors
  /// Returns an error if the command argument vector is empty or process
  /// spawning fails.
  pub fn process_spawn(
    &self,
    command: &[String],
    cwd: Option<&Path>,
    env: &HashMap<String, String>,
  ) -> Result<u32, Box<dyn Error>> {
    if command.is_empty() {
      return Err("Command is empty".into());
    }
    let cmd = &command[0];
    let args = &command[1..];

    let mut builder = std::process::Command::new(cmd);
    builder.args(args);
    if let Some(dir) = cwd {
      builder.current_dir(dir);
    }
    for (k, v) in env {
      builder.env(k, v);
    }

    let child = builder.spawn()?;
    Ok(child.id())
  }

  /// Sends `SIGTERM` to the specified process.
  ///
  /// # Errors
  /// Returns an error if signaling fails.
  pub fn process_terminate(&self, pid: u32) -> Result<(), Box<dyn Error>> {
    let ret = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
    if ret != 0 {
      let err = std::io::Error::last_os_error();
      // If process already exited (ESRCH), treat as success
      if err.raw_os_error() != Some(libc::ESRCH) {
        return Err(Box::new(err));
      }
    }
    Ok(())
  }

  /// Sends `SIGKILL` to the specified process.
  ///
  /// # Errors
  /// Returns an error if signaling fails.
  pub fn process_kill(&self, pid: u32) -> Result<(), Box<dyn Error>> {
    let ret = unsafe { libc::kill(pid as i32, libc::SIGKILL) };
    if ret != 0 {
      let err = std::io::Error::last_os_error();
      if err.raw_os_error() != Some(libc::ESRCH) {
        return Err(Box::new(err));
      }
    }
    Ok(())
  }

  /// Checks if the process is alive by sending signal 0.
  pub fn process_is_alive(&self, pid: u32) -> bool {
    let ret = unsafe { libc::kill(pid as i32, 0) };
    ret == 0
  }

  /// Samples CPU and memory usage for the given PID.
  ///
  /// # Errors
  /// Returns an error if process monitoring fails.
  pub fn process_stats(
    &self,
    pid: u32,
  ) -> Result<ProcessStats, Box<dyn Error>> {
    let mut sys = System::new();
    let sysinfo_pid = Pid::from_u32(pid);
    sys.refresh_processes(ProcessesToUpdate::Some(&[sysinfo_pid]), true);
    if let Some(process) = sys.process(sysinfo_pid) {
      let cpu_usage = (process.cpu_usage() as f64 / 100.0).max(0.0);
      let mem_usage = process.memory() as f64 / 1_000_000.0;
      return Ok(ProcessStats {
        cpu_usage,
        mem_usage,
      });
    }
    Ok(ProcessStats::default())
  }
}

impl Sys for UnixSys {
  fn name(&self) -> &'static str {
    "unix"
  }

  fn spawn(
    &self,
    command: &[String],
    cwd: Option<&Path>,
    env: &HashMap<String, String>,
  ) -> Result<u32, Box<dyn Error>> {
    self.process_spawn(command, cwd, env)
  }

  fn terminate(&self, pid: u32) -> Result<(), Box<dyn Error>> {
    self.process_terminate(pid)
  }

  fn kill(&self, pid: u32) -> Result<(), Box<dyn Error>> {
    self.process_kill(pid)
  }

  fn is_alive(&self, pid: u32) -> bool {
    self.process_is_alive(pid)
  }

  fn stats(&self, pid: u32) -> Result<ProcessStats, Box<dyn Error>> {
    self.process_stats(pid)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_unix_sys_lifecycle() {
    let sys = UnixSys::new();
    assert_eq!(sys.name(), "unix");

    let cmd = vec!["sleep".to_string(), "5".to_string()];
    let pid = sys.spawn(&cmd, None, &HashMap::new()).unwrap();
    assert!(pid > 0);
    assert!(sys.is_alive(pid));

    let stats = sys.stats(pid).unwrap();
    let _ = stats;

    assert!(sys.terminate(pid).is_ok());
    // Give OS time to reap process
    std::thread::sleep(std::time::Duration::from_millis(50));
    let _ = sys.kill(pid);
  }

  #[test]
  fn test_unix_sys_empty_command() {
    let sys = UnixSys::new();
    assert!(sys.spawn(&[], None, &HashMap::new()).is_err());
  }
}
