//! Workload domain models, execution state, and placement constraints.
//!
//! Exposes domain entities representing high-level workloads ([`Pig`]),
//! workload replicas ([`Piglet`]), individual supervised processes ([`Task`]),
//! and resource reservations ([`Yard`], [`Placement`]).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Storage prefix for Pig records in Barn.
pub const PIG_PREFIX: &str = "pig/";

/// Storage prefix for Piglet records in Barn.
pub const PIGLET_PREFIX: &str = "piglet/";

/// Default logical space for workloads when omitted.
pub const DEFAULT_SPACE: &str = "main";

/// Operational status of a Pig across the cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PigStatus {
  /// Configuration accepted and stored in the Barn; awaiting initial
  /// scheduling.
  Received,
  /// Actively adjusting Piglet count (scaling up or down) to meet desired
  /// size.
  Updating,
  /// All desired Piglets are successfully yarded and running across the Ranch.
  Running,
  /// Shutdown initiated; gracefully stopping all active Piglets.
  Stopping,
  /// All Piglets have been terminated and pruned; workload is fully stopped.
  Stopped,
  /// Consecutive Piglet crashes detected (thrashing protection tripped).
  Failing,
  /// Workload permanently failed and unrecoverable without operator action.
  Failed,
}

/// Persistent domain entity representing a cluster-wide workload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pig {
  /// Canonical hierarchical identifier of the Pig in `<space>/<name>` format.
  pub path: String,
  /// Desired workload configuration (replica size, placement, tasks).
  pub config: PigConfig,
  /// Current aggregated operational status across the Ranch.
  pub status: PigStatus,
  /// Consecutive Piglet replacement counter used for crashloop/thrashing
  /// protection.
  #[serde(default)]
  pub replace_count: u32,
  /// Unix timestamp of the most recent Piglet failure and replacement attempt.
  #[serde(default)]
  pub replaced_at: Option<u64>,
  /// Unix timestamp when this Pig was originally submitted to the Barn.
  #[serde(default)]
  pub created_at: u64,
}

impl Pig {
  /// Returns the Barn storage key for this Pig.
  pub fn key(&self) -> String {
    format!("{}{}", PIG_PREFIX, self.path)
  }

  /// Returns the namespace segment of the Pig path.
  pub fn space(&self) -> &str {
    self.path.split('/').next().unwrap_or("")
  }

  /// Returns the Pig name segment of the Pig path.
  pub fn name(&self) -> &str {
    self.path.split('/').nth(1).unwrap_or("")
  }
}

/// User-defined configuration for a Pig workload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PigConfig {
  /// Logical namespace for isolation (defaults to top-level space or
  /// `"main"`).
  #[serde(default)]
  pub space: String,
  /// Unique name of this Pig within its namespace.
  pub name: String,
  /// Optional herd name this Pig belongs to.
  #[serde(default)]
  pub herd: Option<String>,
  /// Informational grouping and routing tags associated with this Pig.
  #[serde(default)]
  pub tags: Vec<String>,
  /// Desired number of identical replica Piglets to maintain across the
  /// cluster.
  #[serde(default = "default_size")]
  pub size: u32,
  /// Template defining placement constraints and tasks for each replica.
  pub piglet: PigletConfig,
}

fn default_space() -> String {
  DEFAULT_SPACE.to_string()
}

fn default_size() -> u32 {
  1
}

/// Template defining placement constraints and tasks for a Piglet replica.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PigletConfig {
  /// Scheduling policy, Croft tag requirements, and Yard resource bounds.
  #[serde(default)]
  pub placement: Placement,
  /// List of tasks executed inside this Piglet's Yard.
  #[serde(default)]
  pub tasks: Vec<TaskConfig>,
}

/// Placement constraints and scheduling strategy for a Piglet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Placement {
  /// Croft-level constraints (e.g. required Croft tags).
  #[serde(default)]
  pub croft: CroftPlacement,
  /// Placement algorithm (`spread` vs `packed`).
  #[serde(default)]
  pub method: PlacementMethod,
  /// When `true`, guarantees no two Piglets from the same Pig share the same
  /// Croft.
  #[serde(default)]
  pub exclusive: bool,
  /// Resource capacity required to carve out the Piglet's Yard in MHz (CPU)
  /// and MB (RAM).
  #[serde(default)]
  pub yard: YardPlacement,
}

/// Croft-level constraints for candidate filtering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CroftPlacement {
  /// List of tags that candidate Crofts must possess to be eligible for
  /// placement.
  #[serde(default)]
  pub tags: Vec<String>,
}

/// Placement ranking strategy.
#[derive(
  Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "lowercase")]
pub enum PlacementMethod {
  /// Distributes Piglets across the least populated Crofts.
  #[default]
  Spread,
  /// Consolidates Piglets onto the most utilized Crofts to minimize footprint.
  Packed,
}

/// Resource requirements reserved inside a Croft's Yardable capacity in MHz and
/// MB.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct YardPlacement {
  /// Required CPU clock capacity in megahertz (MHz) (or parsed from unit
  /// strings like `"1 GHz"`, `"500 MHz"`).
  #[serde(
    default,
    deserialize_with = "crate::core::format::deserialize_cpu_mhz"
  )]
  pub cpu: f64,
  /// Required RAM capacity in megabytes (MB) (or parsed from unit strings like
  /// `"512 MB"`, `"2 GB"`).
  #[serde(
    default,
    deserialize_with = "crate::core::format::deserialize_memory_mb"
  )]
  pub mem: f64,
  /// Optional maximum CPU burst limit in megahertz (MHz).
  #[serde(
    default,
    deserialize_with = "crate::core::format::deserialize_opt_cpu_mhz"
  )]
  pub cpu_max: Option<f64>,
  /// Optional maximum RAM burst limit in megabytes (MB).
  #[serde(
    default,
    deserialize_with = "crate::core::format::deserialize_opt_memory_mb"
  )]
  pub mem_max: Option<f64>,
}

/// Operational status of an individual Piglet replica on a Croft.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PigletStatus {
  /// Yard allocated and reserved on the target Croft; waiting for tasks to
  /// launch.
  Yarded,
  /// Tasks are actively launching on the local host.
  Starting,
  /// All tasks inside the Yard are running healthily.
  Running,
  /// Piglet is shutting down; signals sent to terminate child task processes.
  Stopping,
  /// All child processes have exited cleanly; Yard is released.
  Stopped,
  /// One or more tasks failed unexpectedly; awaiting supervisor recovery.
  Failing,
  /// Unrecoverable task failure or maximum restart retries exceeded.
  Failed,
}

/// Active running workload replica on a specific Croft.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Piglet {
  /// Canonical hierarchical identifier in `<space>/<pig_name>/<piglet_id>`
  /// format.
  pub path: String,
  /// Current operational state of this Piglet on its assigned Croft.
  pub status: PigletStatus,
  /// Hardware resource parcel reserved for this Piglet on the host Croft.
  pub yard: Yard,
  /// Map of task name to active runtime task status records.
  #[serde(default)]
  pub tasks: HashMap<String, Task>,
  /// Aggregated live CPU consumption across all tasks in MHz.
  pub cpu_usage: Option<f64>,
  /// Aggregated live RAM consumption across all tasks in MB.
  pub mem_usage: Option<f64>,
  /// Unix timestamp when this Piglet was farrowed and stored.
  #[serde(default)]
  pub created_at: u64,
}

impl Piglet {
  /// Returns the Barn storage key for this Piglet on its assigned Croft.
  pub fn key(&self) -> String {
    format!("{}{}/{}", PIGLET_PREFIX, self.yard.croft_id, self.path)
  }

  /// Returns the namespace segment of the Piglet path.
  pub fn space(&self) -> &str {
    self.path.split('/').next().unwrap_or("")
  }

  /// Returns the parent Pig name segment of the Piglet path.
  pub fn pig_name(&self) -> &str {
    self.path.split('/').nth(1).unwrap_or("")
  }

  /// Returns the unique Piglet ID segment of the Piglet path.
  pub fn id(&self) -> &str {
    self.path.split('/').nth(2).unwrap_or("")
  }
}

/// Hardware resource reservation carved out on a specific Croft for a Piglet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Yard {
  /// 64-bit Croft identifier where this Yard is hosted.
  pub croft_id: u64,
  /// Reserved CPU capacity in MHz.
  #[serde(
    default,
    deserialize_with = "crate::core::format::deserialize_cpu_mhz"
  )]
  pub cpu: f64,
  /// Reserved RAM capacity in MB.
  #[serde(
    default,
    deserialize_with = "crate::core::format::deserialize_memory_mb"
  )]
  pub mem: f64,
  /// Optional maximum CPU burst limit in MHz.
  #[serde(
    default,
    deserialize_with = "crate::core::format::deserialize_opt_cpu_mhz"
  )]
  pub cpu_max: Option<f64>,
  /// Optional maximum RAM burst limit in MB.
  #[serde(
    default,
    deserialize_with = "crate::core::format::deserialize_opt_memory_mb"
  )]
  pub mem_max: Option<f64>,
}

/// Operational status of an individual Task.
#[derive(
  Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
  /// Process is in the process of being spawned.
  #[default]
  Starting,
  /// Process is running actively on the host OS.
  Running,
  /// Termination signal (`SIGTERM`) sent to the process.
  Stopping,
  /// Process has terminated cleanly (or completed batch execution).
  Stopped,
  /// Process exited with a non-zero code or failed to spawn.
  Failed,
}

/// Operational lifecycle mode of a Task.
#[derive(
  Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "lowercase")]
pub enum TaskMode {
  /// Persistent process; automatically restarted on unexpected exit.
  #[default]
  Persistent,
  /// Transient process; runs once and transitions to `Stopped` upon exit code
  /// 0.
  Transient,
}

/// Active task execution state within a Piglet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Task {
  /// Canonical identifier in `<space>/<pig_name>/<piglet_id>/<task_name>`
  /// format.
  pub path: String,
  /// Current execution state of this task.
  pub status: TaskStatus,
  /// Engine runtime identifier (OS PID string for native exec processes, or
  /// container ID for container tasks).
  pub run_id: Option<String>,
  /// Total number of restarts attempted after unexpected crashes.
  pub restart_count: u32,
  /// Measured CPU utilization of the task process in GHz.
  pub cpu_usage: Option<f64>,
  /// Measured RAM utilization of the task process in GB.
  pub mem_usage: Option<f64>,
  /// Last captured exit code when the process terminated.
  pub exit_code: Option<i32>,
  /// Error details if process execution or monitoring encountered failures.
  pub error_msg: Option<String>,
}

/// Task configuration within a Piglet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskConfig {
  /// Unique task name within the Piglet.
  pub name: String,
  /// Operational mode (`Persistent` vs `Transient`).
  #[serde(default)]
  pub mode: TaskMode,
  /// Native OS process execution parameters.
  pub exec: Option<ExecConfig>,
}

impl TaskConfig {
  /// Returns the execution engine name inferred from task parameters (e.g.
  /// `"exec"`).
  pub fn engine_name(&self) -> &str {
    if self.exec.is_some() {
      "exec"
    } else {
      "unknown"
    }
  }
}

/// Native OS execution parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecConfig {
  /// Executable binary path and command-line arguments (e.g. `["/bin/sleep",
  /// "10"]`).
  pub command: Vec<String>,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_pig_key_and_path_helpers() {
    let pig = Pig {
      path: "production/api-gateway".to_string(),
      config: PigConfig {
        space: "production".to_string(),
        name: "api-gateway".to_string(),
        herd: None,
        tags: vec!["web".to_string()],
        size: 3,
        piglet: PigletConfig::default(),
      },
      status: PigStatus::Received,
      replace_count: 0,
      replaced_at: None,
      created_at: 1726500000,
    };

    assert_eq!(pig.key(), "pig/production/api-gateway");
    assert_eq!(pig.space(), "production");
    assert_eq!(pig.name(), "api-gateway");
  }

  #[test]
  fn test_piglet_key_and_path_helpers() {
    let piglet = Piglet {
      path: "production/api-gateway/1".to_string(),
      status: PigletStatus::Running,
      yard: Yard {
        croft_id: 101,
        cpu: 1000.0,
        mem: 512.0,
        cpu_max: None,
        mem_max: None,
      },
      tasks: HashMap::new(),
      cpu_usage: None,
      mem_usage: None,
      created_at: 1726500000,
    };

    assert_eq!(piglet.key(), "piglet/101/production/api-gateway/1");
    assert_eq!(piglet.space(), "production");
    assert_eq!(piglet.pig_name(), "api-gateway");
    assert_eq!(piglet.id(), "1");
  }

  #[test]
  fn test_task_config_engine_name() {
    let exec_task = TaskConfig {
      name: "web".to_string(),
      mode: TaskMode::Persistent,
      exec: Some(ExecConfig {
        command: vec!["server".to_string()],
      }),
    };
    assert_eq!(exec_task.engine_name(), "exec");

    let unknown_task = TaskConfig {
      name: "web".to_string(),
      mode: TaskMode::Persistent,
      exec: None,
    };
    assert_eq!(unknown_task.engine_name(), "unknown");
  }

  #[test]
  fn test_serialization_roundtrip() {
    let pig = Pig {
      path: "main/worker".to_string(),
      config: PigConfig {
        space: "main".to_string(),
        name: "worker".to_string(),
        herd: Some("backend".to_string()),
        tags: vec!["queue".to_string()],
        size: 2,
        piglet: PigletConfig {
          placement: Placement {
            croft: CroftPlacement {
              tags: vec!["fast-disk".to_string()],
            },
            method: PlacementMethod::Spread,
            exclusive: true,
            yard: YardPlacement {
              cpu: 500.0,
              mem: 256.0,
              cpu_max: None,
              mem_max: None,
            },
          },
          tasks: vec![TaskConfig {
            name: "consumer".to_string(),
            mode: TaskMode::Persistent,
            exec: Some(ExecConfig {
              command: vec![
                "./consumer".to_string(),
                "--threads=4".to_string(),
              ],
            }),
          }],
        },
      },
      status: PigStatus::Running,
      replace_count: 1,
      replaced_at: Some(1726500100),
      created_at: 1726500000,
    };

    let serialized = serde_json::to_string(&pig).unwrap();
    let deserialized: Pig = serde_json::from_str(&serialized).unwrap();
    assert_eq!(pig, deserialized);
  }
}
