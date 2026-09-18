//! Cluster workload orchestrator operating on the Primary Croft.
//!
//! Exposes [`Drover`], which executes the cluster-level reconciliation loop,
//! monitoring workload declarations ([`Pig`]) against active workload replicas
//! ([`Piglet`]) in Barn storage, placing Yard resource reservations, and
//! balancing replica counts across eligible Crofts on the Ranch.

use crate::croft::{Croft, LiveCroft, CROFT_PREFIX};
use crate::pig::types::{
  Pig, PigStatus, Piglet, PigletStatus, PlacementMethod, Task, TaskStatus,
  Yard, PIGLET_PREFIX, PIG_PREFIX,
};
use crate::store::ItemStore;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// Cluster workload orchestrator operating on the Primary Croft.
pub struct Drover {
  /// Reference to the operational Croft compound this orchestrator runs on.
  pub croft: Arc<LiveCroft>,
}

impl Drover {
  /// Instantiates a [`Drover`] cluster orchestrator on the given [`LiveCroft`].
  pub fn new(croft: Arc<LiveCroft>) -> Self {
    Self { croft }
  }

  /// Runs the Drover reconciliation loop continuously until cancelled.
  ///
  /// Executed exclusively on the Primary Croft (Raft leader) under Regent
  /// supervision.
  pub async fn run(&self) {
    loop {
      if let Err(e) = self.reconcile().await {
        tracing::error!(error = %e, "Drover reconciliation error");
      }
      sleep(Duration::from_millis(500)).await;
    }
  }

  /// Reconciles all active Pig records against existing Piglets in the Barn.
  ///
  /// # Errors
  /// Returns an error if reading or writing to Barn fails.
  pub async fn reconcile(&self) -> Result<(), Box<dyn Error>> {
    let pig_pairs = self.croft.barn.list(Some(&PIG_PREFIX.to_string())).await?;
    let piglet_pairs = self
      .croft
      .barn
      .list(Some(&PIGLET_PREFIX.to_string()))
      .await?;

    // Group all active piglets by pig path ("<space>/<pig_name>")
    let mut piglets_by_pig: HashMap<String, Vec<(String, Piglet)>> =
      HashMap::new();
    for (key, bytes) in piglet_pairs {
      if let Ok(piglet) = serde_json::from_slice::<Piglet>(&bytes) {
        let pig_path = format!("{}/{}", piglet.space(), piglet.pig_name());
        piglets_by_pig
          .entry(pig_path)
          .or_default()
          .push((key, piglet));
      }
    }

    let items = self
      .croft
      .barn
      .list(Some(&CROFT_PREFIX.to_string()))
      .await?;
    let active_croft_ids: HashSet<u64> = items
      .into_iter()
      .filter_map(|(_, bytes)| serde_json::from_slice::<Croft>(&bytes).ok())
      .map(|c| c.id)
      .collect();

    for (_, bytes) in pig_pairs {
      let mut pig: Pig = match serde_json::from_slice(&bytes) {
        Ok(p) => p,
        Err(_) => continue,
      };

      let my_piglets = piglets_by_pig.remove(&pig.path).unwrap_or_default();
      self
        .pig_reconcile(&mut pig, my_piglets, &active_croft_ids)
        .await?;
    }

    Ok(())
  }

  async fn pig_reconcile(
    &self,
    pig: &mut Pig,
    piglets: Vec<(String, Piglet)>,
    active_croft_ids: &HashSet<u64>,
  ) -> Result<(), Box<dyn Error>> {
    if pig.status == PigStatus::Stopping {
      if piglets.is_empty() {
        pig.status = PigStatus::Stopped;
        self
          .croft
          .barn
          .set(&pig.key(), serde_json::to_vec(pig)?)
          .await?;
        return Ok(());
      }

      let mut stop_futs: Vec<
        futures::future::BoxFuture<'static, Result<(), String>>,
      > = Vec::new();
      for (key, mut piglet) in piglets {
        let barn = self.croft.barn.clone();
        if piglet.status == PigletStatus::Stopped
          || piglet.status == PigletStatus::Failed
        {
          stop_futs.push(Box::pin(async move {
            barn.delete(&key).await.map_err(|e| e.to_string())
          }));
        } else if piglet.status != PigletStatus::Stopping {
          piglet.status = PigletStatus::Stopping;
          let val = serde_json::to_vec(&piglet)?;
          stop_futs.push(Box::pin(async move {
            barn.set(&key, val).await.map_err(|e| e.to_string())
          }));
        }
      }
      futures::future::try_join_all(stop_futs).await?;
      return Ok(());
    }

    // Filter alive piglets, prune dead ones (or those on removed Crofts), and
    // release Croft Yard capacity
    let mut alive_piglets = Vec::new();
    let mut prune_futs = Vec::new();
    let mut released_by_croft: HashMap<u64, (f64, f64)> = HashMap::new();
    for (key, piglet) in piglets {
      let croft_alive = active_croft_ids.contains(&piglet.yard.croft_id);
      if !croft_alive
        || piglet.status == PigletStatus::Failed
        || piglet.status == PigletStatus::Stopped
      {
        let barn = self.croft.barn.clone();
        prune_futs.push(async move {
          barn.delete(&key).await.map_err(|e| e.to_string())
        });
        if croft_alive {
          let entry =
            released_by_croft.entry(piglet.yard.croft_id).or_default();
          entry.0 += piglet.yard.cpu;
          entry.1 += piglet.yard.mem;
        }
      } else {
        alive_piglets.push((key, piglet));
      }
    }
    if !prune_futs.is_empty() {
      futures::future::try_join_all(prune_futs).await?;
      let mut release_futs = Vec::new();
      for (croft_id, (cpu, mem)) in released_by_croft {
        let key = format!("{}{}", CROFT_PREFIX, croft_id);
        if let Ok(Some(bytes)) = self.croft.barn.get(&key).await {
          if let Ok(mut croft) = serde_json::from_slice::<Croft>(&bytes) {
            croft.resources.cpu_reserved =
              (croft.resources.cpu_reserved - cpu).max(0.0);
            croft.resources.mem_reserved =
              (croft.resources.mem_reserved - mem).max(0.0);
            if let Ok(val) = serde_json::to_vec(&croft) {
              let barn = self.croft.barn.clone();
              release_futs.push(async move {
                barn.set(&key, val).await.map_err(|e| e.to_string())
              });
            }
          }
        }
      }
      if !release_futs.is_empty() {
        futures::future::try_join_all(release_futs).await?;
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
          tasks: pig
            .config
            .piglet
            .tasks
            .iter()
            .map(|t| {
              (
                t.name.clone(),
                Task {
                  path: format!("{}/{}", piglet_path, t.name),
                  status: TaskStatus::Starting,
                  ..Default::default()
                },
              )
            })
            .collect(),
          cpu_usage: None,
          mem_usage: None,
          created_at: now,
        };

        let barn = self.croft.barn.clone();
        let key = new_piglet.key();
        let val = serde_json::to_vec(&new_piglet)?;
        create_futs.push(async move {
          barn.set(&key, val).await.map_err(|e| e.to_string())
        });
      }
      if !create_futs.is_empty() {
        futures::future::try_join_all(create_futs).await?;
      }
      self
        .croft
        .barn
        .set(&pig.key(), serde_json::to_vec(pig)?)
        .await?;
    } else if active_count > pig.config.size {
      // Scale down excess piglets concurrently
      pig.status = PigStatus::Updating;
      let excess = (active_count - pig.config.size) as usize;
      let mut scale_down_futs = Vec::new();
      for (key, mut piglet) in alive_piglets.into_iter().take(excess) {
        if piglet.status != PigletStatus::Stopping {
          piglet.status = PigletStatus::Stopping;
          let val = serde_json::to_vec(&piglet)?;
          let barn = self.croft.barn.clone();
          scale_down_futs.push(async move {
            barn.set(&key, val).await.map_err(|e| e.to_string())
          });
        }
      }
      if !scale_down_futs.is_empty() {
        futures::future::try_join_all(scale_down_futs).await?;
      }
      self
        .croft
        .barn
        .set(&pig.key(), serde_json::to_vec(pig)?)
        .await?;
    } else {
      let all_running = alive_piglets
        .iter()
        .all(|(_, p)| p.status == PigletStatus::Running);
      let target_status = if all_running {
        PigStatus::Running
      } else {
        PigStatus::Updating
      };
      if pig.status != target_status {
        pig.status = target_status;
        self
          .croft
          .barn
          .set(&pig.key(), serde_json::to_vec(pig)?)
          .await?;
      }
    }

    Ok(())
  }

  /// Evaluates cluster Crofts and returns carved-out [`Yard`] allocations for
  /// `count` Piglet replicas.
  ///
  /// # Errors
  /// Returns an error if querying or persisting Croft resource allocations
  /// fails.
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
    let croft_pairs = self
      .croft
      .barn
      .list(Some(&CROFT_PREFIX.to_string()))
      .await?;
    let mut crofts: Vec<Croft> = croft_pairs
      .into_iter()
      .filter_map(|(_, bytes)| serde_json::from_slice::<Croft>(&bytes).ok())
      .filter(|c| {
        c.is_worker() && placement.croft.tags.iter().all(|t| c.tags.contains(t))
      })
      .collect();

    let mut yards = Vec::new();

    // 2. Allocate Yards up to `count` replicas
    for _ in 0..count {
      let chosen = crofts
        .iter_mut()
        .filter(|c| {
          c.resources.cpu_available() >= req_cpu
            && c.resources.mem_available() >= req_mem
        })
        .min_by(|a, b| match placement.method {
          PlacementMethod::Spread => a
            .resources
            .mem_reserved
            .total_cmp(&b.resources.mem_reserved),
          PlacementMethod::Packed => a
            .resources
            .mem_available()
            .total_cmp(&b.resources.mem_available()),
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
        let barn = self.croft.barn.clone();
        update_futs.push(async move {
          barn.set(&key, val).await.map_err(|e| e.to_string())
        });
      }
    }
    if !update_futs.is_empty() {
      futures::future::try_join_all(update_futs).await?;
    }

    Ok(yards)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::croft::{Config, CroftResources, CroftRole};
  use crate::pig::types::{
    CroftPlacement, ExecConfig, PigConfig, PigletConfig, TaskConfig, TaskMode,
    YardPlacement,
  };
  use crate::store::barn::Node as BarnNode;
  use crate::store::SpreadNode;
  use std::collections::BTreeMap;
  use std::net::SocketAddr;
  use tempfile::tempdir;

  #[tokio::test]
  async fn test_drover_schedule_and_reconcile() {
    let dir = tempdir().unwrap();
    let addr: SocketAddr = "127.0.0.1:7450".parse().unwrap();
    let config = Config {
      name: "drover-primary".to_string(),
      addr,
      data_dir: dir.path().to_path_buf(),
      ..Default::default()
    };
    let live_croft = Arc::new(LiveCroft::spawn(&config).await.unwrap());
    let self_node = BarnNode::new(live_croft.id, live_croft.addr.clone());
    let mut members = BTreeMap::new();
    members.insert(self_node.id, self_node);
    live_croft.barn.raft().initialize(members).await.unwrap();

    // Register a worker croft in Barn
    let worker_croft = Croft {
      id: 501,
      name: "worker-501".to_string(),
      addr: "127.0.0.1:7451".to_string(),
      roles: vec![CroftRole::Worker],
      tags: vec!["fast".to_string()],
      joined_at: "2026-09-16T12:00:00Z".to_string(),
      resources: CroftResources {
        cpu_total: 4000.0,
        cpu_yardable: 4000.0,
        cpu_reserved: 0.0,
        mem_total: 4000.0,
        mem_yardable: 4000.0,
        mem_reserved: 0.0,
      },
    };
    live_croft
      .barn
      .set(
        &format!("{}{}", CROFT_PREFIX, worker_croft.id),
        serde_json::to_vec(&worker_croft).unwrap(),
      )
      .await
      .unwrap();

    let drover = Drover::new(live_croft.clone());

    let pig = Pig {
      path: "production/api".to_string(),
      config: PigConfig {
        space: "production".to_string(),
        name: "api".to_string(),
        herd: None,
        tags: vec![],
        size: 2,
        piglet: PigletConfig {
          placement: crate::pig::types::Placement {
            croft: CroftPlacement {
              tags: vec!["fast".to_string()],
            },
            method: PlacementMethod::Spread,
            exclusive: false,
            yard: YardPlacement {
              cpu: 1000.0,
              mem: 500.0,
              cpu_max: None,
              mem_max: None,
            },
          },
          tasks: vec![TaskConfig {
            name: "web".to_string(),
            mode: TaskMode::Persistent,
            exec: Some(ExecConfig {
              command: vec!["echo".to_string(), "hi".to_string()],
            }),
          }],
        },
      },
      status: PigStatus::Received,
      replace_count: 0,
      replaced_at: None,
      created_at: 1726500000,
    };

    // Store pig in Barn
    live_croft
      .barn
      .set(&pig.key(), serde_json::to_vec(&pig).unwrap())
      .await
      .unwrap();

    // Reconcile
    drover.reconcile().await.unwrap();

    // Check that piglets were farrowed
    let piglet_pairs = live_croft
      .barn
      .list(Some(&PIGLET_PREFIX.to_string()))
      .await
      .unwrap();
    assert_eq!(piglet_pairs.len(), 2);

    let pig_raw = live_croft.barn.get(&pig.key()).await.unwrap().unwrap();
    let updated_pig: Pig = serde_json::from_slice(&pig_raw).unwrap();
    assert_eq!(updated_pig.status, PigStatus::Updating);
  }
}
