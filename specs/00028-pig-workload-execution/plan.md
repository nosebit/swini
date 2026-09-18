---
id: 0028
goal: specs/00028-pig-workload-execution/goal.md
author: @brunomacf
created: 2026-09-16
---

# [Plan] Pig Workload Execution and Process Driver

This technical plan details the architecture and implementation for deploying,
scheduling, executing, and supervising workloads in Swini. Workload definitions
are loaded from declarative YAML configurations into domain `Pig` entities, persisted
to consensus storage (**Barn**), and reconciled by the **Drover** running on the
Primary Croft. Workload replicas (**Piglets**) are scheduled onto target Crofts
based on available Yard capacity (**CroftResources**) and supervised locally on
each Croft by a **Supervisor** actor using an `exec` task driver that manages
native OS processes.

See [goal.md](./goal.md) for the product requirements and operational scenarios
this design satisfies.

## Architecture

```mermaid
graph TD
    CLI["CLI (swini pig run / stop)"] -->|gRPC PigApi| Gate["Croft Gate"]
    PigClerk["Pig Clerk (src/pig/clerk)"] -->|writes pig/{space}/{name}| Barn[("Barn Consensus Store")]
    Gate --> PigClerk

    subgraph "Primary Croft (Leader)"
        Drover["Drover (src/drover)"] -->|reconciliation loop| Barn
        Drover -->|schedule function| Drover
        Drover -->|farrows piglet/{croft_id}/...| Barn
    end

    subgraph "All Crofts (Local Node Execution)"
        Supervisor["Supervisor (src/supervisor)"] -->|watches piglet/{self_id}/...| Barn
        Supervisor --> ExecEngine["ExecEngine (src/supervisor/task/engines/exec)"]
        ExecEngine --> SysAdapter["Sys Adapter (src/core/sys)"]
        SysAdapter --> OSProc["Native OS Process (PID)"]
        Supervisor -->|updates task & piglet status| Barn
    end
```

The workload execution subsystem is composed of five primary components:
1. **`src/core/sys/`**: Host system abstraction and adapter layer
   ([`Sys`](#srccoresystypesrs) trait in `types.rs`, and concrete platform
   adapters in `variants/`: [`UnixSys`](#srccoresysvariantsunixrs),
   [`MacSys`](#srccoresysvariantsmacosrs), [`LinuxSys`](#srccoresysvariantslinuxmodrs),
   [`UbuntuSys`](#srccoresysvariantslinuxubunturs), and [`sys::select()`](#srccoresysmodrs)).
   Provides unified, cross-platform host process spawning, signal delivery
   (`SIGTERM`/`SIGKILL`), liveness polling, and CPU/memory sampling.
2. **`src/pig/`**: Pure domain data models, configuration loading, and the
   front-office clerk. Houses `Pig`, `Piglet`, `Task`, `Placement`, and `Yard`
   data structures ([`types.rs`](#pigtypes)), YAML configuration parsing with herd
   defaults ([`config.rs`](#pigconfig)), and the [`PigClerk`](#pigclerk) service
   mounting `PigApi` onto the Croft `Gate`.
3. **`src/drover/`**: Cluster workload orchestrator running **exclusively on the
   Primary Croft**. Houses the cluster reconciliation loop and the `schedule`
   function that evaluates `CroftResources` and placement constraints across the
   Ranch to farrow and scale Piglets.
4. **`src/supervisor/`**: Local node supervisor actor running on **every Croft**
   (spawned by the Regent). Watches local Yard assignments in the Barn
   (`piglet/{self_croft_id}/...`), houses the task execution engine registry
   ([`ExecEngine`](#supervisortaskenginesexec), extensible for future drivers),
   delegates host operations to `src/core/sys`, tracks PIDs, samples CPU/mem,
   handles service task restarts, and updates status in the Barn.
5. **`src/cli/pig.rs`**: CLI commands implementing `swini pig run` and
   `swini pig stop`.

---

## Implementation Details

### `src/pig/types.rs`

Domain entities representing workload configurations, runtime states, and
resource allocations.

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Default logical space for workloads when omitted.
pub const DEFAULT_SPACE: &str = "main";

/// Operational status of a Pig across the cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PigStatus {
  /// Configuration accepted and stored in the Barn; awaiting initial scheduling.
  Received,
  /// Actively adjusting Piglet count (scaling up or down) to meet desired size.
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pig {
  /// Canonical hierarchical identifier of the Pig in `<space>/<name>` format.
  pub path: String,
  /// Desired workload configuration (replica size, placement, tasks).
  pub config: PigConfig,
  /// Current aggregated operational status across the Ranch.
  pub status: PigStatus,
  /// Consecutive Piglet replacement counter used for crashloop/thrashing protection.
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PigConfig {
  /// Logical namespace for isolation (defaults to `"main"`).
  #[serde(default = "default_space")]
  pub space: String,
  /// Unique name of this Pig within its namespace.
  pub name: String,
  /// Optional herd name this Pig belongs to.
  #[serde(default)]
  pub herd: Option<String>,
  /// Informational grouping and routing tags associated with this Pig.
  #[serde(default)]
  pub tags: Vec<String>,
  /// Desired number of identical replica Piglets to maintain across the cluster.
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PigletConfig {
  /// Scheduling policy, Croft tag requirements, and Yard resource bounds.
  #[serde(default)]
  pub placement: Placement,
  /// List of tasks executed inside this Piglet's Yard.
  #[serde(default)]
  pub tasks: Vec<TaskConfig>,
}

/// Placement constraints and scheduling strategy for a Piglet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Placement {
  /// Croft-level constraints (e.g. required Croft tags).
  #[serde(default)]
  pub croft: CroftPlacement,
  /// Placement algorithm (`spread` vs `packed`).
  #[serde(default)]
  pub method: PlacementMethod,
  /// When `true`, guarantees no two Piglets from the same Pig share the same Croft.
  #[serde(default)]
  pub exclusive: bool,
  /// Resource capacity required to carve out the Piglet's Yard.
  #[serde(default)]
  pub yard: YardPlacement,
}

/// Croft-level constraints for candidate filtering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CroftPlacement {
  /// List of tags that candidate Crofts must possess to be eligible for placement.
  #[serde(default)]
  pub tags: Vec<String>,
}

/// Placement ranking strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PlacementMethod {
  /// Distributes Piglets across the least populated Crofts.
  #[default]
  Spread,
  /// Consolidates Piglets onto the most utilized Crofts to minimize footprint.
  Packed,
}

/// Resource requirements reserved inside a Croft's Yardable capacity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct YardPlacement {
  /// Required CPU clock capacity in Hertz (e.g. 1_000_000_000 for 1 GHz).
  #[serde(default)]
  pub cpu: u64,
  /// Required RAM capacity in bytes (e.g. 536_870_912 for 512 MB).
  #[serde(default)]
  pub mem: u64,
  /// Optional maximum CPU burst limit in Hertz.
  pub cpu_max: Option<u64>,
  /// Optional maximum RAM burst limit in bytes.
  pub mem_max: Option<u64>,
}

/// Operational status of an individual Piglet replica on a Croft.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PigletStatus {
  /// Yard allocated and reserved on the target Croft; waiting for tasks to launch.
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Piglet {
  /// Canonical hierarchical identifier in `<space>/<pig_name>/<piglet_id>` format.
  pub path: String,
  /// Current operational state of this Piglet on its assigned Croft.
  pub status: PigletStatus,
  /// Hardware resource parcel reserved for this Piglet on the host Croft.
  pub yard: Yard,
  /// Map of task name to active runtime task status records.
  #[serde(default)]
  pub tasks: HashMap<String, Task>,
  /// Aggregated live CPU consumption across all tasks in Hertz.
  pub cpu_usage: Option<u64>,
  /// Aggregated live RAM consumption across all tasks in bytes.
  pub mem_usage: Option<u64>,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Yard {
  /// 64-bit Croft identifier where this Yard is hosted.
  pub croft_id: u64,
  /// Reserved CPU capacity in Hertz.
  pub cpu: u64,
  /// Reserved RAM capacity in bytes.
  pub mem: u64,
  /// Optional maximum CPU burst limit in Hertz.
  pub cpu_max: Option<u64>,
  /// Optional maximum RAM burst limit in bytes.
  pub mem_max: Option<u64>,
}

/// Operational status of an individual Task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TaskMode {
  /// Persistent process; automatically restarted on unexpected exit.
  #[default]
  Persistent,
  /// Transient process; runs once and transitions to `Stopped` upon exit code 0.
  Transient,
}

/// Active task execution state within a Piglet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Task {
  /// Canonical identifier in `<space>/<pig_name>/<piglet_id>/<task_name>` format.
  pub path: String,
  /// Current execution state of this task.
  pub status: TaskStatus,
  /// Engine runtime identifier (OS PID string for native exec processes, or container ID for container tasks).
  pub run_id: Option<String>,
  /// Total number of restarts attempted after unexpected crashes.
  pub restart_count: u32,
  /// Measured CPU utilization of the task process in Hertz.
  pub cpu_usage: Option<u64>,
  /// Measured RAM utilization of the task process in bytes.
  pub mem_usage: Option<u64>,
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
  /// Returns the execution engine name inferred from task parameters (e.g. `"exec"`).
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
  /// Executable binary path and command-line arguments (e.g. `["/bin/sleep", "10"]`).
  pub command: Vec<String>,
}
```

---

### `src/pig/config.rs`

Parses YAML configurations supporting both top-level `pigs: [...]` and
`herds: [ { pigs: [...] } ]`, validating names and normalizing paths into
[`PigRunConfig`].

```rust
use crate::pig::types::{default_space, PigConfig, Placement, DEFAULT_SPACE};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::error::Error;
use std::path::Path;

/// Top-level declarative workload configuration parsed from `pigs.yml`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PigRunConfig {
  /// Default namespace applied to all contained pigs unless overridden.
  #[serde(default = "default_space")]
  pub space: String,
  /// Direct top-level list of Pig configurations.
  #[serde(default)]
  pub pigs: Vec<PigConfig>,
  /// Optional herd groupings organizing pigs into logical sets.
  #[serde(default)]
  pub herds: Vec<HerdConfig>,
}

/// Weak organizational grouping of pigs within a configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HerdConfig {
  /// Name of the herd grouping.
  pub name: String,
  /// Default configuration inherited by pigs within this herd.
  #[serde(default)]
  pub defaults: Option<PigDefaultsConfig>,
  /// List of Pig configurations belonging to this herd.
  #[serde(default)]
  pub pigs: Vec<PigConfig>,
}

/// Default values inherited by pigs in a herd when not explicitly specified on the pig.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PigDefaultsConfig {
  /// Default tags inherited by pigs in this herd.
  #[serde(default)]
  pub tags: Vec<String>,
  /// Optional default replica size inherited by pigs in this herd.
  pub size: Option<u32>,
  /// Optional default placement constraints and Yard allocations.
  pub placement: Option<Placement>,
}

impl PigRunConfig {
  /// Loads and parses a configuration YAML file from disk.
  pub fn load(path: &Path) -> Result<Self, Box<dyn Error>> {
    let content = std::fs::read_to_string(path)?;
    Self::from_yaml(&content)
  }

  /// Parses YAML content and normalizes pigs across herds.
  pub fn from_yaml(yaml_str: &str) -> Result<Self, Box<dyn Error>> {
    let mut config: PigRunConfig = serde_yaml::from_str(yaml_str)?;
    config.normalize()?;
    Ok(config)
  }

  /// Flattens herds into `self.pigs`, applies inherited space and defaults,
  /// and validates that pig names are unique across all herds.
  pub fn normalize(&mut self) -> Result<(), Box<dyn Error>> {
    let mut all_pigs = Vec::new();
    let mut seen_names = HashSet::new();

    for mut pig in self.pigs.drain(..) {
      if pig.space.is_empty() {
        pig.space = if self.space.is_empty() {
          DEFAULT_SPACE.to_string()
        } else {
          self.space.clone()
        };
      }
      Self::pig_validate(&pig)?;
      if !seen_names.insert(pig.name.clone()) {
        return Err(format!("Duplicate pig name '{}' in configuration", pig.name).into());
      }
      all_pigs.push(pig);
    }

    for herd in &self.herds {
      for mut pig in herd.pigs.clone() {
        if pig.space.is_empty() {
          pig.space = if self.space.is_empty() {
            DEFAULT_SPACE.to_string()
          } else {
            self.space.clone()
          };
        }
        pig.herd = Some(herd.name.clone());

        // Apply herd defaults if defined
        if let Some(ref defaults) = herd.defaults {
          for tag in &defaults.tags {
            if !pig.tags.contains(tag) {
              pig.tags.push(tag.clone());
            }
          }
          if let Some(size) = defaults.size {
            if pig.size == 1 {
              pig.size = size;
            }
          }
          if let Some(ref placement) = defaults.placement {
            if pig.piglet.placement == Placement::default() {
              pig.piglet.placement = placement.clone();
            }
          }
        }

        Self::pig_validate(&pig)?;
        if !seen_names.insert(pig.name.clone()) {
          return Err(format!("Duplicate pig name '{}' in herd '{}'", pig.name, herd.name).into());
        }
        all_pigs.push(pig);
      }
    }

    self.pigs = all_pigs;
    Ok(())
  }

  fn pig_validate(pig: &PigConfig) -> Result<(), Box<dyn Error>> {
    if pig.name.trim().is_empty() {
      return Err("Pig name cannot be empty".into());
    }
    for task in &pig.piglet.tasks {
      if task.name.trim().is_empty() {
        return Err(format!("Task name in pig '{}' cannot be empty", pig.name).into());
      }
      if task.exec.is_none() {
        return Err(format!("Task '{}' in pig '{}' must specify an exec command", task.name, pig.name).into());
      }
    }
    Ok(())
  }
}
```

---

### `src/pig/clerk/mod.rs` & `src/pig/clerk/api.rs`

Domain Clerk stationed on `LiveCroft` managing `PigApi` gRPC service.

```rust
use crate::croft::LiveCroft;
use crate::pig::clerk::api::Api;
use crate::pig::config::PigRunConfig;
use crate::pig::types::{Pig, PigConfig, PigStatus};
use crate::pig::PIG_PREFIX;
use crate::store::ItemStore;
use futures::future::try_join_all;
use std::error::Error;
use std::sync::Arc;

pub mod api;

#[derive(Clone)]
pub struct PigClerk {
  pub croft: Arc<LiveCroft>,
}

impl PigClerk {
  /// Spawns the PigClerk staff member and registers its `PigApi` service on Gate.
  pub fn spawn(croft: Arc<LiveCroft>) -> Result<Self, Box<dyn Error>> {
    let clerk = Self { croft: croft.clone() };
    croft.gate.add(api::Api::new(clerk.clone()).into_server());
    Ok(clerk)
  }

  /// Submits and saves desired Pig configurations from a run configuration into the Barn concurrently.
  pub async fn pig_run(
    &self,
    config: PigRunConfig,
  ) -> Result<Vec<Pig>, Box<dyn Error>> {
    let mut created = Vec::new();
    let mut write_futs = Vec::new();
    let now = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)?
      .as_secs();

    for pig_config in config.pigs {
      let path = format!("{}/{}", pig_config.space, pig_config.name);
      let pig = Pig {
        path: path.clone(),
        config: pig_config,
        status: PigStatus::Received,
        replace_count: 0,
        replaced_at: None,
        created_at: now,
      };

      let val = serde_json::to_vec(&pig)?;
      write_futs.push(self.croft.barn.set(&pig.key(), val));
      created.push(pig);
    }
    try_join_all(write_futs).await?;
    Ok(created)
  }

  /// Sets the target Pig status to `Stopping` in the Barn.
  pub async fn pig_stop(
    &self,
    space: &str,
    name: &str,
  ) -> Result<Pig, Box<dyn Error>> {
    let path = format!("{}/{}", space, name);
    let key = format!("{}{}", PIG_PREFIX, path);
    let val = self.croft.barn.get(&key).await?
      .ok_or_else(|| format!("Pig '{}' not found", path))?;

    let mut pig: Pig = serde_json::from_slice(&val)?;
    pig.status = PigStatus::Stopping;
    self.croft.barn.set(&key, serde_json::to_vec(&pig)?).await?;
    Ok(pig)
  }
}
```

```rust
// src/pig/clerk/api.rs
use crate::core::proto::pig::pig_api_server::{PigApi, PigApiServer};
use crate::core::proto::pig::{PigRunReq, PigRunRes, PigStopReq, PigStopRes};
use crate::pig::clerk::PigClerk;
use crate::pig::config::PigRunConfig;
use tonic::{Request, Response, Status};

#[derive(Clone)]
pub struct Api {
  clerk: PigClerk,
}

impl Api {
  pub fn new(clerk: PigClerk) -> Self {
    Self { clerk }
  }

  pub fn into_server(self) -> PigApiServer<Self> {
    PigApiServer::new(self)
  }
}

#[tonic::async_trait]
impl PigApi for Api {
  async fn run(&self, request: Request<PigRunReq>) -> Result<Response<PigRunRes>, Status> {
    let req = request.into_inner();
    let config = PigRunConfig::from_yaml(&req.config_yaml)
      .map_err(|e| Status::invalid_argument(e.to_string()))?;
    let pigs = self.clerk.pig_run(config).await
      .map_err(|e| Status::internal(e.to_string()))?;
    Ok(Response::new(PigRunRes {
      status: "ok".to_string(),
      started_pigs: pigs.into_iter().map(|p| p.path).collect(),
    }))
  }

  async fn stop(&self, request: Request<PigStopReq>) -> Result<Response<PigStopRes>, Status> {
    let req = request.into_inner();
    self.clerk.pig_stop(&req.space, &req.name).await
      .map_err(|e| Status::internal(e.to_string()))?;
    Ok(Response::new(PigStopRes {
      status: "ok".to_string(),
    }))
  }
}
```

---

### `src/drover/mod.rs`

Cluster-level orchestrator running **exclusively on the Primary Croft**.
Discovers all `Pig` and `Piglet` records in the Barn, executes the
reconciliation loop, and schedules new Piglets via its `schedule` function.

```rust
use crate::croft::{Croft, LiveCroft, CROFT_PREFIX};
use crate::pig::types::{
  Pig, PigStatus, Piglet, PigletStatus, PlacementMethod, Task, TaskStatus, Yard,
  PIGLET_PREFIX, PIG_PREFIX,
};
use crate::store::ItemStore;
use futures::future::try_join_all;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// Cluster workload orchestrator operating on the Primary Croft.
pub struct Drover {
  pub croft: Arc<LiveCroft>,
}

impl Drover {
  pub fn spawn(croft: Arc<LiveCroft>) -> Arc<Self> {
    let drover = Arc::new(Self { croft });

    let d_clone = drover.clone();
    tokio::spawn(async move {
      d_clone.run().await;
    });

    drover
  }

  async fn run(&self) {
    loop {
      // Reconcile cluster state only when this Croft is the Raft leader (Primary)
      if self.croft.barn.is_leader().await {
        if let Err(e) = self.reconcile().await {
          tracing::error!(error = %e, "Drover reconciliation error");
        }
      }
      sleep(Duration::from_millis(500)).await;
    }
  }

  /// Reconciles all active Pig records against existing Piglets in the Barn.
  pub async fn reconcile(&self) -> Result<(), Box<dyn Error>> {
    let pig_pairs = self.croft.barn.list(Some(PIG_PREFIX)).await?;
    let piglet_pairs = self.croft.barn.list(Some(PIGLET_PREFIX)).await?;

    // Group all active piglets by pig path ("<space>/<pig_name>")
    let mut piglets_by_pig: HashMap<String, Vec<(String, Piglet)>> = HashMap::new();
    for (key, bytes) in piglet_pairs {
      if let Ok(piglet) = serde_json::from_slice::<Piglet>(&bytes) {
        let pig_path = format!("{}/{}", piglet.space(), piglet.pig_name());
        piglets_by_pig.entry(pig_path).or_default().push((key, piglet));
      }
    }

    for (_, bytes) in pig_pairs {
      let mut pig: Pig = match serde_json::from_slice(&bytes) {
        Ok(p) => p,
        Err(_) => continue,
      };

      let my_piglets = piglets_by_pig.remove(&pig.path).unwrap_or_default();
      self.pig_reconcile(&mut pig, my_piglets).await?;
    }

    Ok(())
  }

  async fn pig_reconcile(
    &self,
    pig: &mut Pig,
    piglets: Vec<(String, Piglet)>,
  ) -> Result<(), Box<dyn Error>> {
    if pig.status == PigStatus::Stopping {
      if piglets.is_empty() {
        pig.status = PigStatus::Stopped;
        self.croft.barn.set(&pig.key(), serde_json::to_vec(pig)?).await?;
        return Ok(());
      }

      let mut stop_futs = Vec::new();
      for (key, mut piglet) in piglets {
        if piglet.status == PigletStatus::Stopped || piglet.status == PigletStatus::Failed {
          stop_futs.push(self.croft.barn.delete(&key));
        } else if piglet.status != PigletStatus::Stopping {
          piglet.status = PigletStatus::Stopping;
          let val = serde_json::to_vec(&piglet)?;
          stop_futs.push(self.croft.barn.set(&key, val));
        }
      }
      try_join_all(stop_futs).await?;
      return Ok(());
    }

    // Filter alive piglets, prune dead ones, and release Croft Yard capacity
    let mut alive_piglets = Vec::new();
    let mut prune_futs = Vec::new();
    let mut released_by_croft: HashMap<u64, (u64, u64)> = HashMap::new();
    for (key, piglet) in piglets {
      if piglet.status == PigletStatus::Failed || piglet.status == PigletStatus::Stopped {
        prune_futs.push(self.croft.barn.delete(&key));
        let entry = released_by_croft.entry(piglet.yard.croft_id).or_default();
        entry.0 += piglet.yard.cpu;
        entry.1 += piglet.yard.mem;
      } else {
        alive_piglets.push((key, piglet));
      }
    }
    if !prune_futs.is_empty() {
      try_join_all(prune_futs).await?;
      let mut release_futs = Vec::new();
      for (croft_id, (cpu, mem)) in released_by_croft {
        let key = format!("{}{}", CROFT_PREFIX, croft_id);
        if let Ok(Some(bytes)) = self.croft.barn.get(&key).await {
          if let Ok(mut croft) = serde_json::from_slice::<Croft>(&bytes) {
            croft.resources.cpu_reserved = croft.resources.cpu_reserved.saturating_sub(cpu);
            croft.resources.mem_reserved = croft.resources.mem_reserved.saturating_sub(mem);
            if let Ok(val) = serde_json::to_vec(&croft) {
              release_futs.push(self.croft.barn.set(&key, val));
            }
          }
        }
      }
      if !release_futs.is_empty() {
        try_join_all(release_futs).await?;
      }
    }

    let active_count = alive_piglets.len() as u32;
    if active_count < pig.config.size {
      pig.status = PigStatus::Updating;
      let needed = (pig.config.size - active_count) as usize;
      let yards = self.schedule(pig, needed).await?;
      let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

      let mut create_futs = Vec::new();
      for yard in yards {
        let id = format!("{:x}", rand::random::<u32>());
        let piglet_path = format!("{}/{}", pig.path, id);
        let new_piglet = Piglet {
          path: piglet_path.clone(),
          status: PigletStatus::Yarded,
          yard,
          tasks: pig.config.piglet.tasks.iter().map(|t| {
            (t.name.clone(), Task {
              path: format!("{}/{}", piglet_path, t.name),
              status: TaskStatus::Starting,
              ..Default::default()
            })
          }).collect(),
          cpu_usage: None,
          mem_usage: None,
          created_at: now,
        };

        let val = serde_json::to_vec(&new_piglet)?;
        create_futs.push(self.croft.barn.set(&new_piglet.key(), val));
      }
      if !create_futs.is_empty() {
        try_join_all(create_futs).await?;
      }
      self.croft.barn.set(&pig.key(), serde_json::to_vec(pig)?).await?;
    } else if active_count > pig.config.size {
      // Scale down excess piglets concurrently
      pig.status = PigStatus::Updating;
      let excess = (active_count - pig.config.size) as usize;
      let mut scale_down_futs = Vec::new();
      for (key, mut piglet) in alive_piglets.into_iter().take(excess) {
        if piglet.status != PigletStatus::Stopping {
          piglet.status = PigletStatus::Stopping;
          let val = serde_json::to_vec(&piglet)?;
          scale_down_futs.push(self.croft.barn.set(&key, val));
        }
      }
      if !scale_down_futs.is_empty() {
        try_join_all(scale_down_futs).await?;
      }
      self.croft.barn.set(&pig.key(), serde_json::to_vec(pig)?).await?;
    } else {
      let all_running = alive_piglets.iter().all(|(_, p)| p.status == PigletStatus::Running);
      let target_status = if all_running { PigStatus::Running } else { PigStatus::Updating };
      if pig.status != target_status {
        pig.status = target_status;
        self.croft.barn.set(&pig.key(), serde_json::to_vec(pig)?).await?;
      }
    }

    Ok(())
  }

  /// Evaluates cluster Crofts and returns carved-out `Yard` allocations for `count` Piglet replicas.
  pub async fn schedule(
    &self,
    pig: &Pig,
    count: usize,
  ) -> Result<Vec<Yard>, Box<dyn Error>> {
    if count == 0 {
      return Ok(Vec::new());
    }

    let placement = &pig.config.piglet.placement;
    let req_cpu = placement.yard.cpu;
    let req_mem = placement.yard.mem;

    // 1. Fetch and filter eligible worker Crofts matching tags once
    let croft_pairs = self.croft.barn.list(Some(CROFT_PREFIX)).await?;
    let mut crofts: Vec<Croft> = croft_pairs
      .into_iter()
      .filter_map(|(_, bytes)| serde_json::from_slice::<Croft>(&bytes).ok())
      .filter(|c| c.is_worker() && placement.croft.tags.iter().all(|t| c.tags.contains(t)))
      .collect();

    let mut yards = Vec::new();

    // 2. Allocate Yards up to `count` replicas
    for _ in 0..count {
      let chosen = crofts
        .iter_mut()
        .filter(|c| c.resources.cpu_available() >= req_cpu && c.resources.mem_available() >= req_mem)
        .min_by_key(|c| match placement.method {
          PlacementMethod::Spread => c.resources.mem_reserved,
          PlacementMethod::Packed => c.resources.mem_available(),
        });

      let Some(croft) = chosen else {
        break; // No more Crofts with sufficient capacity
      };

      yards.push(Yard {
        croft_id: croft.id,
        cpu: req_cpu,
        mem: req_mem,
        cpu_max: placement.yard.cpu_max,
        mem_max: placement.yard.mem_max,
      });

      croft.resources.cpu_reserved += req_cpu;
      croft.resources.mem_reserved += req_mem;
    }

    // 3. Persist modified Crofts to the Barn concurrently
    let modified_ids: HashSet<u64> = yards.iter().map(|y| y.croft_id).collect();
    let mut update_futs = Vec::new();
    for croft in &crofts {
      if modified_ids.contains(&croft.id) {
        let key = format!("{}{}", CROFT_PREFIX, croft.id);
        let val = serde_json::to_vec(croft)?;
        update_futs.push(self.croft.barn.set(&key, val));
      }
    }
    if !update_futs.is_empty() {
      try_join_all(update_futs).await?;
    }

    Ok(yards)
  }
}
```

---

### `src/core/sys/types.rs`

Host system trait and process execution metric structures.

```rust
use async_trait::async_trait;
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;

/// Instantaneous resource utilization sampled for a host process.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProcessStats {
  /// Clock frequency utilization in Hertz / percentage.
  pub cpu_usage: u64,
  /// Physical Resident Set Size (RSS) memory utilization in bytes.
  pub mem_usage: u64,
}

/// Host system level abstraction for process execution, signaling, and metrics.
pub trait Sys: Send + Sync {
  /// Canonical identifier of the host system (e.g. "macos", "ubuntu", "linux").
  fn name(&self) -> &'static str;

  /// Spawns a host process command with arguments, optional working directory, and environment variables.
  fn spawn(
    &self,
    command: &[String],
    cwd: Option<&Path>,
    env: &HashMap<String, String>,
  ) -> Result<u32, Box<dyn Error>>;

  /// Sends a graceful termination signal (SIGTERM) to the process.
  fn terminate(&self, pid: u32) -> Result<(), Box<dyn Error>>;

  /// Sends a forceful kill signal (SIGKILL) to the process.
  fn kill(&self, pid: u32) -> Result<(), Box<dyn Error>>;

  /// Checks whether a process with the given PID is currently active and alive.
  fn is_alive(&self, pid: u32) -> bool;

  /// Samples instantaneous CPU and memory metrics for the process.
  fn stats(&self, pid: u32) -> Result<ProcessStats, Box<dyn Error>>;
}
```

---

### `src/core/sys/variants/unix.rs`

Base Unix/POSIX implementation leveraging standard `libc` and `std::process` primitives.

```rust
use crate::core::sys::types::{ProcessStats, Sys};
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;

/// Base Unix/POSIX implementation leveraging standard libc and std::process primitives.
pub struct UnixSys;

impl UnixSys {
  pub fn new() -> Self {
    Self
  }

  pub fn spawn_process(
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

  pub fn terminate_process(&self, pid: u32) -> Result<(), Box<dyn Error>> {
    unsafe {
      libc::kill(pid as i32, libc::SIGTERM);
    }
    Ok(())
  }

  pub fn kill_process(&self, pid: u32) -> Result<(), Box<dyn Error>> {
    unsafe {
      libc::kill(pid as i32, libc::SIGKILL);
    }
    Ok(())
  }

  pub fn check_alive(&self, pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
  }

  pub fn sample_stats(&self, pid: u32) -> Result<ProcessStats, Box<dyn Error>> {
    let mut sys = sysinfo::System::new();
    sys.refresh_processes();
    if let Some(process) = sys.process(sysinfo::Pid::from(pid as usize)) {
      return Ok(ProcessStats {
        cpu_usage: process.cpu_usage() as u64,
        mem_usage: process.memory(),
      });
    }
    Ok(ProcessStats::default())
  }
}

impl Default for UnixSys {
  fn default() -> Self {
    Self::new()
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
    self.spawn_process(command, cwd, env)
  }

  fn terminate(&self, pid: u32) -> Result<(), Box<dyn Error>> {
    self.terminate_process(pid)
  }

  fn kill(&self, pid: u32) -> Result<(), Box<dyn Error>> {
    self.kill_process(pid)
  }

  fn is_alive(&self, pid: u32) -> bool {
    self.check_alive(pid)
  }

  fn stats(&self, pid: u32) -> Result<ProcessStats, Box<dyn Error>> {
    self.sample_stats(pid)
  }
}
```

---

### `src/core/sys/variants/macos.rs`

macOS (Darwin) host system adapter composing Unix primitives.

```rust
use crate::core::sys::types::{ProcessStats, Sys};
use crate::core::sys::variants::unix::UnixSys;
use std::collections::HashMap;
use std::error::Error;
use std::ops::Deref;
use std::path::Path;

/// macOS (Darwin) host system adapter composing standard Unix primitives.
pub struct MacSys {
  base: UnixSys,
}

impl MacSys {
  pub fn new() -> Self {
    Self {
      base: UnixSys::new(),
    }
  }
}

impl Default for MacSys {
  fn default() -> Self {
    Self::new()
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
```

---

### `src/core/sys/variants/linux/mod.rs` & `src/core/sys/variants/linux/ubuntu.rs`

Generic Linux and Ubuntu-specific host system adapters.

```rust
// src/core/sys/variants/linux/mod.rs
pub mod ubuntu;

use crate::core::sys::types::{ProcessStats, Sys};
use crate::core::sys::variants::unix::UnixSys;
pub use ubuntu::UbuntuSys;
use std::collections::HashMap;
use std::error::Error;
use std::ops::Deref;
use std::path::Path;

/// Generic Linux host system adapter composing standard Unix primitives.
pub struct LinuxSys {
  base: UnixSys,
}

impl LinuxSys {
  pub fn new() -> Self {
    Self {
      base: UnixSys::new(),
    }
  }
}

impl Default for LinuxSys {
  fn default() -> Self {
    Self::new()
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
```

```rust
// src/core/sys/variants/linux/ubuntu.rs
use super::LinuxSys;
use crate::core::sys::types::{ProcessStats, Sys};
use std::collections::HashMap;
use std::error::Error;
use std::ops::Deref;
use std::path::Path;

/// Ubuntu Linux host system adapter extending Linux base functionality.
pub struct UbuntuSys {
  base: LinuxSys,
}

impl UbuntuSys {
  pub fn new() -> Self {
    Self {
      base: LinuxSys::new(),
    }
  }
}

impl Default for UbuntuSys {
  fn default() -> Self {
    Self::new()
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
```

```rust
// src/core/sys/variants/mod.rs
pub mod linux;
pub mod macos;
pub mod unix;

pub use linux::{LinuxSys, UbuntuSys};
pub use macos::MacSys;
pub use unix::UnixSys;
```

---

### `src/core/sys/mod.rs`

System selection factory and root module exports.

```rust
pub mod types;
pub mod variants;

pub use types::{ProcessStats, Sys};
pub use variants::{LinuxSys, MacSys, UbuntuSys, UnixSys};

use std::sync::Arc;

/// Selects and instantiates the appropriate Sys adapter matching the current runtime host environment.
pub fn select() -> Arc<dyn Sys> {
  match std::env::consts::OS {
    "macos" => Arc::new(MacSys::new()),
    "linux" => {
      // Check for Ubuntu or specific Linux distribution via /etc/os-release
      if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        if content.lines().any(|l| l.starts_with("ID=") && l.contains("ubuntu")) {
          return Arc::new(UbuntuSys::new());
        }
      }
      Arc::new(LinuxSys::new())
    }
    _ => Arc::new(UnixSys::new()),
  }
}
```

---

### `src/supervisor/task/types.rs`

Task execution engine abstractions and runtime metrics definitions.

```rust
use crate::pig::types::{Pig, Piglet, TaskConfig, TaskStatus};
use async_trait::async_trait;
use std::error::Error;

/// Measured runtime resource utilization of a task process.
#[derive(Debug, Clone, Default)]
pub struct TaskStats {
  /// Clock frequency utilization in Hertz.
  pub cpu_usage: u64,
  /// Physical RAM utilization in bytes.
  pub mem_usage: u64,
}

/// Abstract task execution engine for spawning and managing workloads on the host.
#[async_trait]
pub trait TaskEngine: Send + Sync {
  /// Spawns a task process and returns its unique runtime identifier (e.g. PID).
  async fn start(&self, pig: &Pig, piglet: &Piglet, task: &TaskConfig) -> Result<String, Box<dyn Error>>;
  /// Sends a graceful termination signal to the task process.
  async fn stop(&self, run_id: &str) -> Result<(), Box<dyn Error>>;
  /// Queries the current execution status of the task process.
  async fn status(&self, run_id: &str) -> Result<TaskStatus, Box<dyn Error>>;
  /// Samples instantaneous CPU and memory utilization for the process.
  async fn stats(&self, run_id: &str) -> Result<TaskStats, Box<dyn Error>>;
}
```

---

### `src/supervisor/task/engines/exec.rs` & `src/supervisor/task/engines/mod.rs`

Native OS child process execution engine delegating to `src/core/sys`.

```rust
use crate::core::sys::{self, Sys};
use crate::pig::types::{Pig, Piglet, TaskConfig, TaskStatus};
use crate::supervisor::task::types::{TaskEngine, TaskStats};
use async_trait::async_trait;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

/// Native OS process execution engine managing child processes via host system adapter.
pub struct ExecEngine {
  pub sys: Arc<dyn Sys>,
}

impl ExecEngine {
  pub fn new() -> Self {
    Self {
      sys: sys::select(),
    }
  }

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
  async fn start(&self, _pig: &Pig, _piglet: &Piglet, task: &TaskConfig) -> Result<String, Box<dyn Error>> {
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
```

```rust
// src/supervisor/task/engines/mod.rs
pub mod exec;
pub use exec::ExecEngine;
```

```rust
// src/supervisor/task/mod.rs
pub mod engines;
pub mod types;

pub use engines::*;
pub use types::*;
```

---

### `src/supervisor/mod.rs`

Local node supervisor running on **every Croft**, monitoring assigned Piglets
under `piglet/{self_croft_id}/...`, driving process execution via registered
`TaskEngine`s, and updating task/piglet states.

```rust
pub mod task;

use crate::croft::LiveCroft;
use crate::pig::types::{
  Pig, PigStatus, Piglet, PigletStatus, TaskMode, TaskStatus,
  PIGLET_PREFIX, PIG_PREFIX,
};
use crate::store::ItemStore;
use crate::supervisor::task::engines::ExecEngine;
use crate::supervisor::task::types::TaskEngine;
use futures::future::try_join_all;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// Local node supervisor actor managing Yard workloads on the local Croft.
pub struct Supervisor {
  /// Reference to the local operational Croft compound.
  pub croft: Arc<LiveCroft>,
  /// Registry of task execution engines keyed by engine name ("exec", "container", etc.).
  pub engines: HashMap<String, Arc<dyn TaskEngine>>,
}

impl Supervisor {
  /// Spawns the Supervisor actor with default supported task engines and starts the background monitoring loop.
  pub fn spawn(croft: Arc<LiveCroft>) -> Arc<Self> {
    let mut engines: HashMap<String, Arc<dyn TaskEngine>> = HashMap::new();
    engines.insert("exec".to_string(), Arc::new(ExecEngine::new()));

    Self::with_engines(croft, engines)
  }

  /// Spawns the Supervisor actor with a customized engine registry.
  pub fn with_engines(croft: Arc<LiveCroft>, engines: HashMap<String, Arc<dyn TaskEngine>>) -> Arc<Self> {
    let supervisor = Arc::new(Self { croft, engines });

    let s_clone = supervisor.clone();
    tokio::spawn(async move {
      s_clone.run().await;
    });

    supervisor
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

  /// Inspects all locally assigned Piglets, starts new tasks, checks process liveness,
  /// restarts failing service tasks, and updates actual states in the Barn.
  pub async fn reconcile(&self) -> Result<(), Box<dyn Error>> {
    let prefix = format!("{}{}/", PIGLET_PREFIX, self.croft.id);
    let pairs = self.croft.barn.list(Some(&prefix)).await?;
    let mut update_futs = Vec::new();

    for (key, bytes) in pairs {
      let mut piglet: Piglet = match serde_json::from_slice(&bytes) {
        Ok(p) => p,
        Err(_) => continue,
      };

      let pig_key = format!("{}{}/{}", PIG_PREFIX, piglet.space(), piglet.pig_name());
      let pig: Pig = match self.croft.barn.get(&pig_key).await? {
        Some(v) => serde_json::from_slice(&v)?,
        None => continue,
      };

      if piglet.status == PigletStatus::Stopped || piglet.status == PigletStatus::Failed {
        continue;
      }

      let is_stopping = piglet.status == PigletStatus::Stopping || pig.status == PigStatus::Stopping;
      let mut all_tasks_running = true;
      let mut all_tasks_stopped = true;
      let mut any_task_failed = false;
      let mut total_cpu = 0;
      let mut total_mem = 0;

      for task_config in &pig.config.piglet.tasks {
        let task = piglet.tasks.entry(task_config.name.clone()).or_default();

        let engine_name = task_config.engine_name();
        let engine = match self.engine(engine_name) {
          Some(e) => e,
          None => {
            task.status = TaskStatus::Failed;
            task.error_msg = Some(format!("Unsupported task engine '{}'", engine_name));
            all_tasks_running = false;
            any_task_failed = true;
            continue;
          }
        };

        if is_stopping {
          if task.status != TaskStatus::Stopped && task.status != TaskStatus::Failed {
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
                let st = engine.status(run_id).await.unwrap_or(TaskStatus::Failed);
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
      update_futs.push(self.croft.barn.set(&key, val));
    }

    if !update_futs.is_empty() {
      try_join_all(update_futs).await?;
    }

    Ok(())
  }
}
```

---

### `src/cli/pig.rs` & `src/cli/mod.rs`

CLI command dispatch and gRPC client connection for `swini pig run` and `swini pig stop`.

```rust
use crate::cli::resolve_api_url;
use crate::core::proto::pig::pig_api_client::PigApiClient;
use crate::core::proto::pig::{PigRunReq, PigStopReq};
use crate::pig::config::PigRunConfig;
use clap::Subcommand;
use std::error::Error;
use std::path::PathBuf;

#[derive(Subcommand, Debug, PartialEq, Eq)]
pub enum Command {
  /// Deploys and starts pigs defined in a configuration file
  Run {
    /// Path to configuration file (defaults to pigs.yml)
    #[arg(short, long)]
    config: Option<PathBuf>,
    /// Optional specific pig names to run
    names: Vec<String>,
  },
  /// Stops a running pig
  Stop {
    /// Path to configuration file (optional, to infer space)
    #[arg(short, long)]
    config: Option<PathBuf>,
    /// Name or path of the pig to stop
    name: String,
  },
}

pub async fn run(cmd: Command) -> Result<(), Box<dyn Error>> {
  let api_url = resolve_api_url(None)?;
  let mut client = PigApiClient::connect(api_url).await?;

  match cmd {
    Command::Run { config: config_path, names } => {
      let path = config_path.unwrap_or_else(|| PathBuf::from("pigs.yml"));
      let mut config = PigRunConfig::load(&path)?;

      if !names.is_empty() {
        let name_set: std::collections::HashSet<_> = names.iter().collect();
        // Validate that all specified pig names exist in the configuration
        for req_name in &names {
          if !config.pigs.iter().any(|p| &p.name == req_name) {
            return Err(format!("Pig '{}' not found in configuration", req_name).into());
          }
        }
        config.pigs.retain(|p| name_set.contains(&p.name));
      }

      let req = PigRunReq {
        config_yaml: serde_yaml::to_string(&config)?,
      };
      let res = client.run(req).await?.into_inner();
      println!("{}", res.status);
    }
    Command::Stop { config: config_path, name } => {
      let space = if let Some(path) = config_path {
        PigRunConfig::load(&path).map(|m| m.space).unwrap_or_else(|_| "main".to_string())
      } else {
        "main".to_string()
      };

      let req = PigStopReq { space, name };
      let res = client.stop(req).await?.into_inner();
      println!("{}", res.status);
    }
  }
  Ok(())
}
```

---

## Data Model / API Changes

### Protobuf Service: `proto/pig.proto`

```protobuf
syntax = "proto3";

package swini.pig;

service PigApi {
  rpc Run(PigRunReq) returns (PigRunRes);
  rpc Stop(PigStopReq) returns (PigStopRes);
}

message PigRunReq {
  string config_yaml = 1;
}

message PigRunRes {
  string status = 1;
  repeated string started_pigs = 2;
}

message PigStopReq {
  string space = 1;
  string name = 2;
}

message PigStopRes {
  string status = 1;
}
```

### Barn Storage Layout

| Key Pattern | Value Format | Description |
| :--- | :--- | :--- |
| `pig/{space}/{name}` | JSON (`Pig`) | Authoritative cluster workload definition & state |
| `piglet/{croft_id}/{space}/{name}/{piglet_id}` | JSON (`Piglet`) | Placement, yard allocation, and task execution state |

---

## Testing Strategy

- **Unit Tests**:
  - `src/core/sys/`: Host system detection (`sys::select()`), process spawning, signal termination (`SIGTERM`/`SIGKILL`), liveness checks, and metric sampling across `UnixSys`, `MacSys`, `LinuxSys`, and `UbuntuSys`.
  - `src/pig/config.rs`: Configuration parsing covering top-level `pigs`, `herds`, `defaults` inheritance (tags, size, placement), name collision rejection, and default space `"main"`.
  - `src/pig/types.rs`: Serialization and round-trip conversions of `Pig`, `PigConfig`, `Piglet`, `Task`, `Placement`, and `Yard`.
  - `src/drover/`: `Drover::schedule` evaluating candidate Crofts (tags, available vs. reserved CPU/RAM, exclusivity) and ranking (`spread` vs. `packed`).
  - `src/supervisor/task/engines/exec.rs`: `ExecEngine` process spawning (`/bin/echo` or `sleep`), PID extraction, exit status checking, and `SIGTERM` stopping via injected mock/concrete `Sys`.
  - `src/supervisor/`: `Supervisor` engine registry resolution, unsupported engine rejection, reconciliation loop, task starting, status updating, and crash recovery.
  - `src/pig/clerk/`: `PigClerk` `pig_run` and `pig_stop` against an in-memory Barn.
- **Integration & E2E Tests (`tests/`)**:
  - E2e test with multi-node cluster running `swini pig run -c sandbox/pigs/exec.yml`, verifying that Piglets are scheduled to worker Crofts, tasks spawn OS processes, and `swini pig stop` cleanly terminates processes.

---

## Risks & Open Questions

- **Process Cleanup on Abnormal Croft Crash**: If a Croft process crashes abruptly, orphaned child OS processes could linger until supervisor restarts and re-claims or terminates them. The supervisor startup scan cleans up stale PIDs.
- **Raft Leadership Transition**: When a new Raft leader takes over as Primary, `Drover` automatically assumes reconciliation without losing state because all `Pig` and `Piglet` records reside in consensus storage.

---

## Out of Scope

- Container and Docker task drivers (deferred to a follow-up spec).
- Active protocol health checks (HTTP/TCP probes and custom exec probes).
- Dynamic resource cgroup enforcement (resource allocation is accounted for in Barn scheduling).
