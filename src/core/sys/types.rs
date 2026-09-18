//! Host system driver abstraction and process execution metric types.
//!
//! Exposes [`Sys`], the fundamental trait providing cross-platform process
//! spawning, signal dispatching, liveness probing, and resource accounting,
//! alongside [`ProcessStats`] for instantaneous CPU and memory metrics.

use std::collections::HashMap;
use std::error::Error;
use std::path::Path;

/// Instantaneous resource utilization sampled for a host OS process.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProcessStats {
  /// CPU utilization in percentage or MHz.
  pub cpu_usage: f64,
  /// Physical Resident Set Size (RSS) memory utilization in MB.
  pub mem_usage: f64,
}

/// Host system level abstraction for process execution, signaling, and metrics.
pub trait Sys: Send + Sync {
  /// Returns the canonical platform identifier of the host system (e.g.
  /// "macos", "ubuntu", "linux", "unix").
  fn name(&self) -> &'static str;

  /// Spawns a native OS process command with arguments, optional working
  /// directory, and environment variables.
  ///
  /// # Errors
  /// Returns an error if the command is empty or if the host operating system
  /// fails to spawn the process.
  fn spawn(
    &self,
    command: &[String],
    cwd: Option<&Path>,
    env: &HashMap<String, String>,
  ) -> Result<u32, Box<dyn Error>>;

  /// Sends a graceful termination signal (`SIGTERM`) to the process.
  ///
  /// # Errors
  /// Returns an error if the signal cannot be dispatched to the given process
  /// identifier.
  fn terminate(&self, pid: u32) -> Result<(), Box<dyn Error>>;

  /// Sends a forceful kill signal (`SIGKILL`) to the process.
  ///
  /// # Errors
  /// Returns an error if the signal cannot be dispatched to the given process
  /// identifier.
  fn kill(&self, pid: u32) -> Result<(), Box<dyn Error>>;

  /// Checks whether a process with the given PID is currently active and alive
  /// on the host.
  fn is_alive(&self, pid: u32) -> bool;

  /// Samples instantaneous CPU and memory metrics for the target process.
  ///
  /// # Errors
  /// Returns an error if process information cannot be queried from the host
  /// system.
  fn stats(&self, pid: u32) -> Result<ProcessStats, Box<dyn Error>>;
}
