//! Task execution engine abstractions and runtime metrics definitions.
//!
//! Exposes [`TaskEngine`], the foundational trait for driver adapters that
//! execute and supervise workload tasks on a host node, alongside
//! [`TaskStats`].

use crate::pig::types::{Pig, Piglet, TaskConfig, TaskStatus};
use async_trait::async_trait;
use std::error::Error;

/// Measured runtime resource utilization of a task process in MHz and MB.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TaskStats {
  /// CPU utilization in percentage or MHz.
  pub cpu_usage: f64,
  /// Physical RAM utilization in MB.
  pub mem_usage: f64,
}

/// Abstract task execution engine for spawning and managing workloads on the
/// host.
#[async_trait]
pub trait TaskEngine: Send + Sync {
  /// Spawns a task process and returns its unique runtime identifier (e.g. PID
  /// string or container ID).
  ///
  /// # Errors
  /// Returns an error if task configuration is invalid or process spawning
  /// fails.
  async fn start(
    &self,
    pig: &Pig,
    piglet: &Piglet,
    task: &TaskConfig,
  ) -> Result<String, Box<dyn Error>>;

  /// Sends a graceful termination signal to the task process.
  ///
  /// # Errors
  /// Returns an error if process signaling fails.
  async fn stop(&self, run_id: &str) -> Result<(), Box<dyn Error>>;

  /// Queries the current execution status of the task process.
  ///
  /// # Errors
  /// Returns an error if process status cannot be determined.
  async fn status(&self, run_id: &str) -> Result<TaskStatus, Box<dyn Error>>;

  /// Samples instantaneous CPU and memory utilization for the process.
  ///
  /// # Errors
  /// Returns an error if metrics cannot be sampled.
  async fn stats(&self, run_id: &str) -> Result<TaskStats, Box<dyn Error>>;
}
