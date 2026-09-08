//! Croft Clerk domain component managing cluster Croft registration and queries
//! across the Ranch.
//!
//! Submodules:
//! - [`api`]: Tonic gRPC service handler and protobuf conversions for
//!   `CroftApi`.

pub mod api;

pub use api::Api;

use crate::croft::{Croft, LiveCroft, CROFT_PREFIX};
use crate::store::barn::Node as BarnNode;
use crate::store::{ItemStore, SpreadNode, SpreadStore};
use std::error::Error;
use std::sync::Arc;

/// Domain clerk governing cluster Croft registration and Croft queries across
/// the Ranch. Staff member operating inside a [`LiveCroft`].
#[derive(Clone)]
pub struct Clerk {
  /// Reference to the operational Croft compound this clerk serves.
  pub croft: Arc<LiveCroft>,
}

impl Clerk {
  /// Spawns a new [`Clerk`] staff member for the given [`LiveCroft`],
  /// registering its gRPC [`Api`] handler onto the Croft's [`Gate`].
  ///
  /// # Errors
  /// Returns an error if registering to the gate fails.
  pub fn spawn(croft: Arc<LiveCroft>) -> Result<Self, Box<dyn Error>> {
    let clerk = Self {
      croft: croft.clone(),
    };
    croft.gate.add(Api::new(clerk.clone()).into_server());
    Ok(clerk)
  }

  /// Instantiates a [`Clerk`] without registering to the Gate (useful for
  /// tests).
  pub fn new(croft: Arc<LiveCroft>) -> Self {
    Self { croft }
  }

  /// Retrieves a registered [`Croft`] by its 64-bit ID from Barn consensus
  /// storage.
  ///
  /// # Errors
  /// Returns an error if Barn storage retrieval or deserialization fails.
  pub async fn get(&self, id: u64) -> Result<Option<Croft>, Box<dyn Error>> {
    let key = format!("{}{id}", CROFT_PREFIX);
    if let Some(bytes) = self.croft.barn.get(&key).await? {
      let croft: Croft = serde_json::from_slice(&bytes)?;
      return Ok(Some(croft));
    }

    Ok(None)
  }

  /// Returns all registered Crofts in the Ranch by scanning Barn storage under
  /// `croft/`.
  ///
  /// # Errors
  /// Returns an error if Barn scan fails.
  pub async fn list(&self) -> Result<Vec<Croft>, Box<dyn Error>> {
    let mut crofts = Vec::new();
    let prefix = CROFT_PREFIX.to_string();
    let pairs = self.croft.barn.list(Some(&prefix)).await?;
    for (_key, bytes) in pairs {
      if let Ok(croft) = serde_json::from_slice::<Croft>(&bytes) {
        crofts.push(croft);
      }
    }

    Ok(crofts)
  }

  /// Registers an incoming Croft: updates Barn Raft membership if a Server,
  /// stamps `joined_at`, persists the Croft in `croft/{id}`, and returns
  /// all active Server crofts.
  ///
  /// # Errors
  /// Returns an error if Raft membership change or storage write fails.
  pub async fn join(
    &self,
    mut incoming: Croft,
  ) -> Result<Vec<Croft>, Box<dyn Error>> {
    if incoming.is_server() {
      let barn_node = BarnNode::new(incoming.id, incoming.addr.clone());
      self.croft.barn.node_add(barn_node).await?;
    }

    if incoming.joined_at.is_empty() {
      incoming.joined_at = chrono::Utc::now().to_rfc3339();
    }

    let key = format!("{}{}", CROFT_PREFIX, incoming.id);
    let payload = serde_json::to_vec(&incoming)?;
    self.croft.barn.set(&key, payload).await?;

    let all_crofts = self.list().await?;
    let server_crofts =
      all_crofts.into_iter().filter(|c| c.is_server()).collect();
    Ok(server_crofts)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::croft::{Config, CroftRole};
  use std::collections::BTreeMap;
  use std::net::SocketAddr;
  use tempfile::tempdir;

  #[tokio::test]
  async fn clerk_spawn_and_join() {
    let dir = tempdir().unwrap();
    let addr: SocketAddr = "127.0.0.1:7441".parse().unwrap();
    let config = Config {
      name: "clerk-croft".to_string(),
      addr,
      data_dir: dir.path().to_path_buf(),
      ..Default::default()
    };
    let live_croft = Arc::new(LiveCroft::spawn(&config).await.unwrap());
    let self_node = BarnNode::new(live_croft.id, live_croft.addr.clone());
    let mut members = BTreeMap::new();
    members.insert(self_node.id, self_node);
    live_croft.barn.raft().initialize(members).await.unwrap();

    let clerk = Clerk::spawn(live_croft.clone()).unwrap();

    let test_croft = Croft {
      id: 200,
      name: "worker-200".to_string(),
      addr: "127.0.0.1:7442".to_string(),
      roles: vec![CroftRole::Worker],
      tags: vec![],
      joined_at: String::new(),
    };

    let _ = clerk.join(test_croft.clone()).await.unwrap();

    let fetched = clerk.get(200).await.unwrap();
    assert_eq!(fetched.unwrap().name, "worker-200");

    let all = clerk.list().await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].name, "worker-200");
  }
}
