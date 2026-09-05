//! Plot Clerk domain component managing cluster Plot registration and queries
//! across the Ranch.
//!
//! Submodules:
//! - [`api`]: Tonic gRPC service handler and protobuf conversions for
//!   `PlotApi`.

pub mod api;

pub use api::Api;

use crate::croft::Croft;
use crate::plot::{Plot, PlotRole};
use crate::store::barn::Node as BarnNode;
use crate::store::{ItemStore, SpreadNode, SpreadStore};
use std::error::Error;
use std::sync::Arc;

/// Key prefix in Barn storage where Plot records are stored.
const PLOT_PREFIX: &str = "plot/";

/// Domain clerk governing cluster Plot registration and Plot queries across the
/// Ranch. Staff member operating inside a [`Croft`].
#[derive(Clone)]
pub struct Clerk {
  /// Reference to the operational Croft compound this clerk serves.
  pub croft: Arc<Croft>,
}

impl Clerk {
  /// Spawns a new [`Clerk`] staff member for the given [`Croft`], registering
  /// its gRPC [`Api`] handler onto the Croft's [`Gate`].
  pub fn spawn(croft: Arc<Croft>) -> Result<Self, Box<dyn Error>> {
    let clerk = Self {
      croft: croft.clone(),
    };
    croft.gate.add(Api::new(clerk.clone()).into_server());
    Ok(clerk)
  }

  /// Instantiates a [`Clerk`] without registering to the Gate (useful for
  /// tests).
  pub fn new(croft: Arc<Croft>) -> Self {
    Self { croft }
  }

  /// Retrieves a plot by ID by querying the Barn store.
  pub async fn get(
    &self,
    plot_id: u64,
  ) -> Result<Option<Plot>, Box<dyn Error>> {
    let key = format!("{}{plot_id}", PLOT_PREFIX);
    if let Some(bytes) = self.croft.barn.get(&key).await? {
      let plot: Plot = serde_json::from_slice(&bytes)?;
      return Ok(Some(plot));
    }

    Ok(None)
  }

  /// Returns all registered Plots in the Ranch by scanning Barn storage under
  /// `plot/`.
  pub async fn list(&self) -> Result<Vec<Plot>, Box<dyn Error>> {
    let mut plots = Vec::new();
    let prefix = PLOT_PREFIX.to_string();
    let pairs = self.croft.barn.list(Some(&prefix)).await?;
    for (_key, bytes) in pairs {
      if let Ok(plot) = serde_json::from_slice::<Plot>(&bytes) {
        plots.push(plot);
      }
    }

    Ok(plots)
  }

  /// Registers an incoming Plot: updates Barn Raft membership if a Server,
  /// stamps `joined_at`, persists the Plot record in `plot/{id}`, and returns
  /// all active Server plots.
  pub async fn join(
    &self,
    mut incoming: Plot,
  ) -> Result<Vec<Plot>, Box<dyn Error>> {
    if incoming.roles.contains(&PlotRole::Server) {
      let barn_node = BarnNode::new(incoming.id, incoming.addr.clone());
      self.croft.barn.node_add(barn_node).await?;
    }

    if incoming.joined_at.is_empty() {
      incoming.joined_at = chrono::Utc::now().to_rfc3339();
    }

    let key = format!("{}{}", PLOT_PREFIX, incoming.id);
    let payload = serde_json::to_vec(&incoming)?;
    self.croft.barn.set(&key, payload).await?;

    let all_plots = self.list().await?;
    let server_plots =
      all_plots.into_iter().filter(|p| p.is_server()).collect();
    Ok(server_plots)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::Config;
  use std::collections::BTreeMap;
  use std::net::SocketAddr;
  use tempfile::tempdir;

  #[tokio::test]
  async fn clerk_spawn_and_join() {
    let dir = tempdir().unwrap();
    let addr: SocketAddr = "127.0.0.1:7441".parse().unwrap();
    let config = Config {
      name: "clerk-plot".to_string(),
      addr,
      data_dir: dir.path().to_path_buf(),
      ..Default::default()
    };
    let croft = Arc::new(Croft::spawn(&config).await.unwrap());
    let self_node = BarnNode::new(croft.plot.id, croft.config.addr.to_string());
    let mut members = BTreeMap::new();
    members.insert(self_node.id, self_node);
    croft.barn.raft().initialize(members).await.unwrap();

    let clerk = Clerk::spawn(croft.clone()).unwrap();

    let test_plot = Plot {
      id: 200,
      name: "worker-200".to_string(),
      addr: "127.0.0.1:7442".to_string(),
      roles: vec![PlotRole::Worker],
      tags: vec![],
      joined_at: String::new(),
    };

    let _ = clerk.join(test_plot.clone()).await.unwrap();

    let fetched = clerk.get(200).await.unwrap();
    assert_eq!(fetched.unwrap().name, "worker-200");

    let all = clerk.list().await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].name, "worker-200");
  }
}
