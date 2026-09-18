//! Native OS child process execution engine delegating to `src/core/sys`.
//!
//! Implements [`TaskEngine`] by spawning native processes directly on the host
//! operating system via [`Sys`], tracking operating system PIDs, and delegating
//! signals (`SIGTERM`, `SIGKILL`) to [`Sys`].

use crate::core::sys::{self, Sys};
use crate::pig::types::{Pig, Piglet, TaskConfig, TaskStatus};
use crate::supervisor::task::types::{TaskEngine, TaskStats};
use async_trait::async_trait;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

/// Native OS process execution engine managing child processes via host system
/// adapter.
pub struct ExecEngine {
  /// Platform host system driver used for low-level process operations.
  pub sys: Arc<dyn Sys>,
}

impl ExecEngine {
  /// Instantiates an `ExecEngine` using the default runtime system driver
  /// ([`sys::select()`]).
  pub fn new() -> Self {
    Self { sys: sys::select() }
  }

  /// Instantiates an `ExecEngine` backed by the specified host system driver.
  pub fn with_sys(sys: Arc<dyn Sys>) -> Self {
    Self { sys }
  }
}

impl Default for ExecEngine {
  fn default() -> Self {
    Self::new()
  }
}

#[async_trait]
impl TaskEngine for ExecEngine {
  async fn start(
    &self,
    _pig: &Pig,
    _piglet: &Piglet,
    task: &TaskConfig,
  ) -> Result<String, Box<dyn Error>> {
    let exec = task.exec.as_ref().ok_or("Missing exec configuration")?;
    if exec.command.is_empty() {
      return Err("Exec command is empty".into());
    }

    let env = HashMap::new();
    let pid = self.sys.spawn(&exec.command, None, &env)?;
    tracing::info!(pid = pid, task = %task.name, sys = %self.sys.name(), "Spawned exec process");
    Ok(pid.to_string())
  }

  async fn stop(&self, run_id: &str) -> Result<(), Box<dyn Error>> {
    if let Ok(pid) = run_id.trim().parse::<u32>() {
      self.sys.terminate(pid)?;
    }
    Ok(())
  }

  async fn status(&self, run_id: &str) -> Result<TaskStatus, Box<dyn Error>> {
    if let Ok(pid) = run_id.trim().parse::<u32>() {
      if self.sys.is_alive(pid) {
        return Ok(TaskStatus::Running);
      } else {
        return Ok(TaskStatus::Stopped);
      }
    }
    Ok(TaskStatus::Failed)
  }

  async fn stats(&self, run_id: &str) -> Result<TaskStats, Box<dyn Error>> {
    if let Ok(pid) = run_id.trim().parse::<u32>() {
      let stats = self.sys.stats(pid)?;
      return Ok(TaskStats {
        cpu_usage: stats.cpu_usage,
        mem_usage: stats.mem_usage,
      });
    }
    Ok(TaskStats::default())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::pig::types::{ExecConfig, PigConfig, PigletConfig, TaskMode};

  #[tokio::test]
  async fn test_exec_engine_lifecycle() {
    let engine = ExecEngine::new();
    let task = TaskConfig {
      name: "sleep-task".to_string(),
      mode: TaskMode::Persistent,
      exec: Some(ExecConfig {
        command: vec!["sleep".to_string(), "5".to_string()],
      }),
    };

    let pig = Pig {
      path: "test/pig".to_string(),
      config: PigConfig {
        space: "test".to_string(),
        name: "pig".to_string(),
        herd: None,
        tags: vec![],
        size: 1,
        piglet: PigletConfig::default(),
      },
      status: crate::pig::types::PigStatus::Running,
      replace_count: 0,
      replaced_at: None,
      created_at: 0,
    };

    let piglet = Piglet {
      path: "test/pig/1".to_string(),
      status: crate::pig::types::PigletStatus::Running,
      yard: Default::default(),
      tasks: Default::default(),
      cpu_usage: None,
      mem_usage: None,
      created_at: 0,
    };

    let run_id = engine.start(&pig, &piglet, &task).await.unwrap();
    assert!(!run_id.is_empty());

    let status = engine.status(&run_id).await.unwrap();
    assert_eq!(status, TaskStatus::Running);

    let stats = engine.stats(&run_id).await.unwrap();
    let _ = stats;

    assert!(engine.stop(&run_id).await.is_ok());
  }

  #[tokio::test]
  async fn test_exec_engine_missing_exec_config() {
    let engine = ExecEngine::new();
    let task = TaskConfig {
      name: "no-exec".to_string(),
      mode: TaskMode::Persistent,
      exec: None,
    };
    let pig = Pig {
      path: "test/pig".to_string(),
      config: PigConfig {
        space: "test".to_string(),
        name: "pig".to_string(),
        herd: None,
        tags: vec![],
        size: 1,
        piglet: PigletConfig::default(),
      },
      status: crate::pig::types::PigStatus::Running,
      replace_count: 0,
      replaced_at: None,
      created_at: 0,
    };

    let piglet = Piglet {
      path: "test/pig/1".to_string(),
      status: crate::pig::types::PigletStatus::Running,
      yard: Default::default(),
      tasks: Default::default(),
      cpu_usage: None,
      mem_usage: None,
      created_at: 0,
    };

    assert!(engine.start(&pig, &piglet, &task).await.is_err());
  }
}
