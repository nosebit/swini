//! Pig Clerk domain component managing workload submissions and lifecycle
//! commands.
//!
//! Exposes [`PigClerk`], the staff member operating on a [`LiveCroft`] that
//! accepts workload configurations, stores desired [`Pig`] states in Barn, and
//! dispatches stop requests across the Ranch.
//!
//! Submodules:
//! - [`api`]: Tonic gRPC service handler and protobuf conversions for `PigApi`.

pub mod api;

#[allow(unused_imports)]
pub use api::Api;

use crate::croft::LiveCroft;
use crate::pig::config::PigRunConfig;
use crate::pig::types::{Pig, PigStatus, PIG_PREFIX};
use crate::store::ItemStore;
use std::error::Error;
use std::sync::Arc;

/// Domain clerk governing workload submissions and operations across the Ranch.
#[derive(Clone)]
pub struct PigClerk {
  /// Reference to the operational Croft compound this clerk serves.
  pub croft: Arc<LiveCroft>,
}

impl PigClerk {
  /// Spawns a new [`PigClerk`] staff member for the given [`LiveCroft`],
  /// registering its gRPC [`Api`] handler onto the Croft's [`Gate`].
  ///
  /// # Errors
  /// Returns an error if registering to the gate fails.
  pub fn spawn(croft: Arc<LiveCroft>) -> Result<Self, Box<dyn Error>> {
    let clerk = Self {
      croft: croft.clone(),
    };
    croft.gate.add(api::Api::new(clerk.clone()).into_server());
    Ok(clerk)
  }

  /// Instantiates a [`PigClerk`] without registering to the Gate (useful for
  /// tests).
  pub fn new(croft: Arc<LiveCroft>) -> Self {
    Self { croft }
  }

  /// Submits and saves desired Pig configurations from a run configuration into
  /// the Barn concurrently.
  ///
  /// # Errors
  /// Returns an error if saving to Barn fails.
  pub async fn pig_run(
    &self,
    config: PigRunConfig,
  ) -> Result<Vec<Pig>, Box<dyn Error>> {
    let now = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)?
      .as_secs();

    let pigs = config.to_pigs(now);
    let mut write_futs = Vec::new();

    for pig in &pigs {
      let barn = self.croft.barn.clone();
      let key = pig.key();
      let val = serde_json::to_vec(pig)?;
      write_futs.push(async move {
        barn.set(&key, val).await.map_err(|e| e.to_string())
      });
    }

    futures::future::try_join_all(write_futs).await?;
    Ok(pigs)
  }

  /// Sets the target Pig status to `Stopping` in the Barn.
  ///
  /// # Errors
  /// Returns an error if the Pig is not found or if persisting to Barn fails.
  pub async fn pig_stop(
    &self,
    space: &str,
    name: &str,
  ) -> Result<Pig, Box<dyn Error>> {
    let path = format!("{}/{}", space, name);
    let key = format!("{}{}", PIG_PREFIX, path);
    let val = self
      .croft
      .barn
      .get(&key)
      .await?
      .ok_or_else(|| format!("Pig '{}' not found", path))?;

    let mut pig: Pig = serde_json::from_slice(&val)?;
    pig.status = PigStatus::Stopping;
    self.croft.barn.set(&key, serde_json::to_vec(&pig)?).await?;
    Ok(pig)
  }

  /// Retrieves a [`Pig`] by space and name from Barn consensus storage.
  ///
  /// # Errors
  /// Returns an error if querying Barn fails.
  pub async fn get(
    &self,
    space: &str,
    name: &str,
  ) -> Result<Option<Pig>, Box<dyn Error>> {
    let path = format!("{}/{}", space, name);
    let key = format!("{}{}", PIG_PREFIX, path);
    if let Some(bytes) = self.croft.barn.get(&key).await? {
      let pig: Pig = serde_json::from_slice(&bytes)?;
      return Ok(Some(pig));
    }
    Ok(None)
  }

  /// Lists all [`Pig`] entities currently registered in Barn storage,
  /// optionally filtered by space.
  ///
  /// # Errors
  /// Returns an error if scanning Barn fails.
  pub async fn list(
    &self,
    space: Option<&str>,
  ) -> Result<Vec<Pig>, Box<dyn Error>> {
    let prefix = if let Some(sp) = space {
      format!("{}{}/", PIG_PREFIX, sp)
    } else {
      PIG_PREFIX.to_string()
    };

    let mut pigs = Vec::new();
    let pairs = self.croft.barn.list(Some(&prefix)).await?;
    for (_key, bytes) in pairs {
      if let Ok(pig) = serde_json::from_slice::<Pig>(&bytes) {
        pigs.push(pig);
      }
    }
    Ok(pigs)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::croft::Config;
  use crate::pig::types::{
    ExecConfig, PigConfig, PigletConfig, TaskConfig, TaskMode,
  };
  use crate::store::barn::Node as BarnNode;
  use crate::store::SpreadNode;
  use std::collections::BTreeMap;
  use std::net::SocketAddr;
  use tempfile::tempdir;

  #[tokio::test]
  async fn test_pig_clerk_run_get_list_stop() {
    let dir = tempdir().unwrap();
    let addr: SocketAddr = "127.0.0.1:7448".parse().unwrap();
    let config = Config {
      name: "pig-clerk-croft".to_string(),
      addr,
      data_dir: dir.path().to_path_buf(),
      ..Default::default()
    };
    let live_croft = Arc::new(LiveCroft::spawn(&config).await.unwrap());
    let self_node = BarnNode::new(live_croft.id, live_croft.addr.clone());
    let mut members = BTreeMap::new();
    members.insert(self_node.id, self_node);
    live_croft.barn.raft().initialize(members).await.unwrap();

    let clerk = PigClerk::spawn(live_croft.clone()).unwrap();

    let run_config = PigRunConfig {
      space: "production".to_string(),
      pigs: vec![PigConfig {
        space: "production".to_string(),
        name: "web-server".to_string(),
        herd: None,
        tags: vec!["frontend".to_string()],
        size: 1,
        piglet: PigletConfig {
          placement: Default::default(),
          tasks: vec![TaskConfig {
            name: "web".to_string(),
            mode: TaskMode::Persistent,
            exec: Some(ExecConfig {
              command: vec!["echo".to_string(), "running".to_string()],
            }),
          }],
        },
      }],
      herds: vec![],
    };

    let started = clerk.pig_run(run_config).await.unwrap();
    assert_eq!(started.len(), 1);
    assert_eq!(started[0].path, "production/web-server");

    let fetched = clerk.get("production", "web-server").await.unwrap();
    assert!(fetched.is_some());
    let pig = fetched.unwrap();
    assert_eq!(pig.status, PigStatus::Received);

    let all = clerk.list(Some("production")).await.unwrap();
    assert_eq!(all.len(), 1);

    let stopped = clerk.pig_stop("production", "web-server").await.unwrap();
    assert_eq!(stopped.status, PigStatus::Stopping);

    let after_stop = clerk.get("production", "web-server").await.unwrap();
    assert_eq!(after_stop.unwrap().status, PigStatus::Stopping);
  }
}
