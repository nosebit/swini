//! Operational compound and canonical domain entity representing an individual
//! host machine on the Ranch.
//!
//! Exposes [`Croft`], the persistent domain entity, and [`LiveCroft`], its
//! active runtime specialization uniting machine identity ([`Croft`]),
//! consensus storage ([`Barn`]), and network gateway ([`Gate`]).
//!
//! Submodules:
//! - [`clerk`]: Domain clerk managing cluster-wide Croft discovery and
//!   registration.
//! - [`config`]: Croft configuration loading, YAML merging, and path
//!   resolution.
//! - [`gate`]: Network gRPC gateway for service registration and listening.
//! - [`types`]: Pure domain data types, role definitions, and entity metadata.

pub mod clerk;
pub mod config;
pub mod gate;
pub mod resources;
pub mod types;

pub use clerk::Clerk;
pub use config::Config;
pub use gate::Gate;
pub use resources::{CroftResources, CroftTelemetry};
pub use types::{Croft, CroftRole};

use crate::store::barn::{Barn, Config as BarnConfig};
use crate::store::ItemStore;
use std::error::Error;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

/// Key prefix in Barn consensus storage where Croft records are stored.
pub const CROFT_PREFIX: &str = "croft/";

/// Active local runtime compound of a host machine on the Ranch.
///
/// Extends [`Croft`] via [`Deref`], embedding required local consensus storage
/// engine ([`Barn`]) and network gRPC gateway ([`Gate`]). All base `Croft`
/// fields and methods are accessible transparently.
#[derive(Clone)]
pub struct LiveCroft {
  base: Croft,
  /// Consensus replicated key-value storage engine.
  pub barn: Arc<Barn>,
  /// Network gRPC gateway for service registration and network entry.
  pub gate: Gate,
}

impl Deref for LiveCroft {
  type Target = Croft;

  fn deref(&self) -> &Self::Target {
    &self.base
  }
}

impl std::fmt::Debug for LiveCroft {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("LiveCroft")
      .field("id", &self.id)
      .field("name", &self.name)
      .field("addr", &self.addr)
      .field("roles", &self.roles)
      .field("tags", &self.tags)
      .field("joined_at", &self.joined_at)
      .finish()
  }
}

impl LiveCroft {
  /// Spawns a local [`LiveCroft`] compound, initializing its persistent ID,
  /// Barn storage, network Gate, and base [`Croft`] identity.
  ///
  /// # Errors
  /// Returns an error if reading/writing `croft.id` fails or if the Barn
  /// storage engine fails to initialize.
  pub async fn spawn(config: &Config) -> Result<Self, Box<dyn Error>> {
    let id = Self::id_provide(&config.data_dir)?;
    let gate = Gate::new();
    let barn = Arc::new(
      Barn::spawn(
        &gate,
        BarnConfig {
          id,
          addr: config.addr.to_string(),
          data_dir: config.data_dir.join("data"),
          heartbeat_interval: 100,
          election_timeout_min: 300,
          election_timeout_max: 600,
        },
      )
      .await?,
    );

    let base = Croft {
      id,
      name: config.name.clone(),
      addr: config.addr.to_string(),
      roles: config.roles.clone(),
      tags: config.tags.clone(),
      joined_at: chrono::Utc::now().to_rfc3339(),
      resources: CroftResources::probe(),
    };

    Ok(Self { base, barn, gate })
  }

  /// Provides a unique and persistent 64-bit ID for this Croft.
  /// Reads `{data_dir}/croft.id` or generates and persists a new random ID.
  fn id_provide(data_dir: &Path) -> Result<u64, Box<dyn Error>> {
    std::fs::create_dir_all(data_dir)?;
    let id_path = data_dir.join("croft.id");
    if id_path.exists() {
      let content = std::fs::read_to_string(&id_path)?;
      if let Ok(id) = content.trim().parse::<u64>() {
        return Ok(id);
      }
    }

    let new_id = rand::random::<u64>() & 0x7FFF_FFFF_FFFF_FFFF;
    std::fs::write(&id_path, new_id.to_string())?;
    Ok(new_id)
  }

  /// Persists this Croft's base data directly into the local Barn storage under
  /// `croft/{id}`.
  ///
  /// Retries write attempts briefly to allow leader election to complete
  /// when bootstrapping a fresh cluster.
  ///
  /// # Errors
  /// Returns an error if serialization fails or Barn write fails after all retries.
  pub async fn persist(&self) -> Result<(), Box<dyn Error>> {
    let key = format!("{}{}", CROFT_PREFIX, self.id);
    let payload = serde_json::to_vec(&self.base)?;
    let mut last_err = None;
    for _ in 0..30 {
      match self.barn.set(&key, payload.clone()).await {
        Ok(()) => return Ok(()),
        Err(e) => {
          last_err = Some(e);
          tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
      }
    }
    if let Some(e) = last_err {
      return Err(e);
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::net::SocketAddr;
  use tempfile::tempdir;

  #[tokio::test]
  async fn live_croft_spawns_cleanly_and_derefs_to_croft() {
    let dir = tempdir().unwrap();
    let addr: SocketAddr = "127.0.0.1:7440".parse().unwrap();
    let config = Config {
      name: "test-node".to_string(),
      addr,
      data_dir: dir.path().to_path_buf(),
      roles: vec![CroftRole::Server, CroftRole::Worker],
      ..Default::default()
    };

    let live_croft = LiveCroft::spawn(&config).await.unwrap();
    // Direct field and method access via Deref
    assert_eq!(live_croft.name, "test-node");
    assert!(live_croft.id > 0);
    assert_eq!(live_croft.addr, "127.0.0.1:7440");
    assert!(live_croft.is_server());
    assert!(live_croft.is_worker());
    assert_eq!(live_croft.roles.len(), 2);
  }

  #[tokio::test]
  async fn live_croft_persist_stores_croft_in_barn() {
    use crate::store::barn::Node as BarnNode;
    use crate::store::SpreadNode;
    use std::collections::BTreeMap;

    let dir = tempdir().unwrap();
    let addr: SocketAddr = "127.0.0.1:7445".parse().unwrap();
    let config = Config {
      name: "persist-node".to_string(),
      addr,
      data_dir: dir.path().to_path_buf(),
      ..Default::default()
    };

    let live_croft = LiveCroft::spawn(&config).await.unwrap();
    let self_node = BarnNode::new(live_croft.id, live_croft.addr.clone());
    let mut members = BTreeMap::new();
    members.insert(self_node.id, self_node);
    live_croft.barn.raft().initialize(members).await.unwrap();

    live_croft.persist().await.expect("persist live croft");

    let key = format!("{}{}", CROFT_PREFIX, live_croft.id);
    let raw = live_croft.barn.get(&key).await.unwrap();
    assert!(raw.is_some());

    let croft: Croft = serde_json::from_slice(&raw.unwrap()).unwrap();
    assert_eq!(croft.name, "persist-node");
    assert_eq!(croft.id, live_croft.id);
  }
}
