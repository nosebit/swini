//! Local node supervisor actor and task engine coordinator.
//!
//! Exposes [`Supervisor`], which runs on every Croft, watches assigned workload
//! replicas ([`Piglet`]) stored under `piglet/{self_croft_id}/...`, drives task
//! execution across registered [`TaskEngine`](task::TaskEngine) implementations
//! (e.g. [`ExecEngine`](task::ExecEngine)), tracks process health/telemetry,
//! and synchronizes status back to Barn consensus storage.
//!
//! Submodules:
//! - [`task`]: Task engine abstractions, execution drivers, and metric types.

pub mod task;

use crate::croft::LiveCroft;
use crate::pig::types::{
  Pig, PigStatus, Piglet, PigletStatus, TaskMode, TaskStatus, PIGLET_PREFIX,
  PIG_PREFIX,
};
use crate::store::ItemStore;
use crate::supervisor::task::engines::ExecEngine;
use crate::supervisor::task::types::TaskEngine;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// Local node supervisor actor managing Yard workloads on the local Croft.
pub struct Supervisor {
  /// Reference to the local operational Croft compound.
  pub croft: Arc<LiveCroft>,
  /// Registry of task execution engines keyed by engine name ("exec",
  /// "container", etc.).
  pub engines: HashMap<String, Arc<dyn TaskEngine>>,
}

impl Supervisor {
  /// Spawns the Supervisor actor with default supported task engines and starts
  /// the background monitoring loop.
  pub fn spawn(croft: Arc<LiveCroft>) -> Arc<Self> {
    let mut engines: HashMap<String, Arc<dyn TaskEngine>> = HashMap::new();
    engines.insert("exec".to_string(), Arc::new(ExecEngine::new()));

    Self::with_engines(croft, engines)
  }

  /// Spawns the Supervisor actor with a customized engine registry.
  pub fn with_engines(
    croft: Arc<LiveCroft>,
    engines: HashMap<String, Arc<dyn TaskEngine>>,
  ) -> Arc<Self> {
    let supervisor = Arc::new(Self { croft, engines });

    let s_clone = supervisor.clone();
    tokio::spawn(async move {
      s_clone.run().await;
    });

    supervisor
  }

  /// Instantiates a [`Supervisor`] without launching the background monitoring
  /// loop (useful for testing).
  pub fn new(croft: Arc<LiveCroft>) -> Self {
    let mut engines: HashMap<String, Arc<dyn TaskEngine>> = HashMap::new();
    engines.insert("exec".to_string(), Arc::new(ExecEngine::new()));
    Self { croft, engines }
  }

  /// Resolves the task engine corresponding to an engine name.
  pub fn engine(&self, name: &str) -> Option<&Arc<dyn TaskEngine>> {
    self.engines.get(name)
  }

  async fn run(&self) {
    loop {
      if let Err(e) = self.reconcile().await {
        tracing::error!(error = %e, "Supervisor reconciliation error");
      }
      sleep(Duration::from_millis(500)).await;
    }
  }

  /// Inspects all locally assigned Piglets, starts new tasks, checks process
  /// liveness, restarts failing service tasks, and updates actual states in
  /// the Barn.
  ///
  /// # Errors
  /// Returns an error if reading from or updating Barn fails.
  pub async fn reconcile(&self) -> Result<(), Box<dyn Error>> {
    let prefix = format!("{}{}/", PIGLET_PREFIX, self.croft.id);
    let pairs = self.croft.barn.list(Some(&prefix)).await?;
    let mut update_futs = Vec::new();

    for (key, bytes) in pairs {
      let mut piglet: Piglet = match serde_json::from_slice(&bytes) {
        Ok(p) => p,
        Err(_) => continue,
      };

      let pig_key =
        format!("{}{}/{}", PIG_PREFIX, piglet.space(), piglet.pig_name());
      let pig: Pig = match self.croft.barn.get(&pig_key).await? {
        Some(v) => serde_json::from_slice(&v)?,
        None => continue,
      };

      if piglet.status == PigletStatus::Stopped
        || piglet.status == PigletStatus::Failed
      {
        continue;
      }

      let is_stopping = piglet.status == PigletStatus::Stopping
        || pig.status == PigStatus::Stopping;
      let mut all_tasks_running = true;
      let mut all_tasks_stopped = true;
      let mut any_task_failed = false;
      let mut total_cpu = 0.0;
      let mut total_mem = 0.0;

      for task_config in &pig.config.piglet.tasks {
        let mut task =
          piglet.tasks.remove(&task_config.name).unwrap_or_default();

        let engine_name = task_config.engine_name();
        let engine = match self.engine(engine_name) {
          Some(e) => e,
          None => {
            task.status = TaskStatus::Failed;
            task.error_msg =
              Some(format!("Unsupported task engine '{}'", engine_name));
            all_tasks_running = false;
            any_task_failed = true;
            piglet.tasks.insert(task_config.name.clone(), task);
            continue;
          }
        };

        if is_stopping {
          if task.status != TaskStatus::Stopped
            && task.status != TaskStatus::Failed
          {
            if let Some(run_id) = task.run_id.as_deref() {
              let _ = engine.stop(run_id).await;
            }
            task.status = TaskStatus::Stopped;
          }
        } else {
          match task.status {
            TaskStatus::Starting => {
              match engine.start(&pig, &piglet, task_config).await {
                Ok(run_id) => {
                  task.run_id = Some(run_id);
                  task.status = TaskStatus::Running;
                }
                Err(e) => {
                  task.restart_count += 1;
                  task.error_msg = Some(e.to_string());
                  if task.restart_count > 3 {
                    task.status = TaskStatus::Failed;
                  }
                }
              }
            }
            TaskStatus::Running => {
              if let Some(run_id) = task.run_id.as_deref() {
                let st =
                  engine.status(run_id).await.unwrap_or(TaskStatus::Failed);
                if st == TaskStatus::Running {
                  let stats = engine.stats(run_id).await.unwrap_or_default();
                  task.cpu_usage = Some(stats.cpu_usage);
                  task.mem_usage = Some(stats.mem_usage);
                  total_cpu += stats.cpu_usage;
                  total_mem += stats.mem_usage;
                } else {
                  // Process exited unexpectedly
                  if task_config.mode == TaskMode::Transient {
                    task.status = TaskStatus::Stopped;
                  } else {
                    task.restart_count += 1;
                    if task.restart_count > 3 {
                      task.status = TaskStatus::Failed;
                    } else {
                      task.status = TaskStatus::Starting;
                    }
                  }
                }
              }
            }
            _ => {}
          }
        }

        if task.status != TaskStatus::Running {
          all_tasks_running = false;
        }
        if task.status != TaskStatus::Stopped {
          all_tasks_stopped = false;
        }
        if task.status == TaskStatus::Failed {
          any_task_failed = true;
        }

        piglet.tasks.insert(task_config.name.clone(), task);
      }

      if is_stopping && all_tasks_stopped {
        piglet.status = PigletStatus::Stopped;
      } else if any_task_failed {
        piglet.status = PigletStatus::Failed;
      } else if all_tasks_running {
        piglet.status = PigletStatus::Running;
      }

      piglet.cpu_usage = Some(total_cpu);
      piglet.mem_usage = Some(total_mem);
      let val = serde_json::to_vec(&piglet)?;
      let barn = self.croft.barn.clone();
      update_futs.push(async move {
        barn.set(&key, val).await.map_err(|e| e.to_string())
      });
    }

    if !update_futs.is_empty() {
      futures::future::try_join_all(update_futs).await?;
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::croft::Config;
  use crate::pig::types::{
    ExecConfig, PigConfig, PigletConfig, Task, TaskConfig, Yard,
  };
  use crate::store::barn::Node as BarnNode;
  use crate::store::SpreadNode;
  use std::collections::BTreeMap;
  use std::net::SocketAddr;
  use tempfile::tempdir;

  #[tokio::test]
  async fn test_supervisor_reconcile_spawns_process() {
    let dir = tempdir().unwrap();
    let addr: SocketAddr = "127.0.0.1:7455".parse().unwrap();
    let config = Config {
      name: "supervisor-node".to_string(),
      addr,
      data_dir: dir.path().to_path_buf(),
      ..Default::default()
    };
    let live_croft = Arc::new(LiveCroft::spawn(&config).await.unwrap());
    let self_node = BarnNode::new(live_croft.id, live_croft.addr.clone());
    let mut members = BTreeMap::new();
    members.insert(self_node.id, self_node);
    live_croft.barn.raft().initialize(members).await.unwrap();

    let supervisor = Supervisor::new(live_croft.clone());

    let pig = Pig {
      path: "main/worker".to_string(),
      config: PigConfig {
        space: "main".to_string(),
        name: "worker".to_string(),
        herd: None,
        tags: vec![],
        size: 1,
        piglet: PigletConfig {
          placement: Default::default(),
          tasks: vec![TaskConfig {
            name: "sleeper".to_string(),
            mode: TaskMode::Persistent,
            exec: Some(ExecConfig {
              command: vec!["sleep".to_string(), "10".to_string()],
            }),
          }],
        },
      },
      status: PigStatus::Updating,
      replace_count: 0,
      replaced_at: None,
      created_at: 1726500000,
    };

    let piglet = Piglet {
      path: "main/worker/1".to_string(),
      status: PigletStatus::Yarded,
      yard: Yard {
        croft_id: live_croft.id,
        cpu: 0.1,
        mem: 0.1,
        cpu_max: None,
        mem_max: None,
      },
      tasks: [(
        "sleeper".to_string(),
        Task {
          path: "main/worker/1/sleeper".to_string(),
          status: TaskStatus::Starting,
          ..Default::default()
        },
      )]
      .into_iter()
      .collect(),
      cpu_usage: None,
      mem_usage: None,
      created_at: 1726500000,
    };

    live_croft
      .barn
      .set(&pig.key(), serde_json::to_vec(&pig).unwrap())
      .await
      .unwrap();

    live_croft
      .barn
      .set(&piglet.key(), serde_json::to_vec(&piglet).unwrap())
      .await
      .unwrap();

    supervisor.reconcile().await.unwrap();

    let raw = live_croft.barn.get(&piglet.key()).await.unwrap().unwrap();
    let updated: Piglet = serde_json::from_slice(&raw).unwrap();
    assert_eq!(updated.status, PigletStatus::Running);
    assert_eq!(
      updated.tasks.get("sleeper").unwrap().status,
      TaskStatus::Running
    );

    // Stop workload
    let mut stopping_pig = pig.clone();
    stopping_pig.status = PigStatus::Stopping;
    live_croft
      .barn
      .set(
        &stopping_pig.key(),
        serde_json::to_vec(&stopping_pig).unwrap(),
      )
      .await
      .unwrap();

    supervisor.reconcile().await.unwrap();

    let raw_stopped =
      live_croft.barn.get(&piglet.key()).await.unwrap().unwrap();
    let stopped: Piglet = serde_json::from_slice(&raw_stopped).unwrap();
    assert_eq!(stopped.status, PigletStatus::Stopped);
  }
}
