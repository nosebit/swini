//! Operational compound and canonical domain entity representing an individual
//! host machine on the Ranch.
//!
//! Exposes [`Croft`], which unites machine identity (`id`, `name`, `addr`,
//! `roles`, `tags`), persistent consensus storage ([`Barn`]), and network
//! gateway ([`Gate`]).
//!
//! Submodules:
//! - [`clerk`]: Domain clerk managing cluster-wide Croft discovery and
//!   registration.
//! - [`config`]: Croft configuration loading, YAML merging, and path
//!   resolution.
//! - [`gate`]: Network gRPC gateway for service registration and listening.
//! - [`types`]: Pure domain data types and cluster role definitions.

pub mod clerk;
pub mod config;
pub mod gate;
pub mod types;

pub use clerk::Clerk;
pub use config::Config;
pub use gate::Gate;
pub use types::CroftRole;

use crate::store::barn::{Barn, Config as BarnConfig};
use crate::store::ItemStore;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::Path;
use std::sync::Arc;

/// Key prefix in Barn consensus storage where Croft records are stored.
pub const CROFT_PREFIX: &str = "croft/";

/// Domain representation and operational compound of a host machine on the
/// Ranch.
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Croft {
  /// Unique 64-bit identifier for this Croft.
  pub id: u64,
  /// Human-readable node name (e.g., "worker-01").
  pub name: String,
  /// Canonical network address (e.g., "127.0.0.1:7440") where Gate listens.
  pub addr: String,
  /// Assigned roles in the cluster (Server, Worker).
  pub roles: Vec<CroftRole>,
  /// Informational grouping tags associated with the Croft.
  pub tags: Vec<String>,
  /// ISO-8601 UTC timestamp when the Croft joined the Ranch.
  pub joined_at: String,

  /// Consensus replicated key-value storage engine (present on local Croft).
  #[serde(skip)]
  pub barn: Option<Arc<Barn>>,
  /// Network gRPC gateway for service registration (present on local Croft).
  #[serde(skip)]
  pub gate: Option<Gate>,
}

impl std::fmt::Debug for Croft {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Croft")
      .field("id", &self.id)
      .field("name", &self.name)
      .field("addr", &self.addr)
      .field("roles", &self.roles)
      .field("tags", &self.tags)
      .field("joined_at", &self.joined_at)
      .finish()
  }
}

impl Croft {
  /// Spawns a local [`Croft`] compound, initializing its persistent ID, Barn
  /// storage, and network Gate.
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

    Ok(Self {
      id,
      name: config.name.clone(),
      addr: config.addr.to_string(),
      roles: config.roles.clone(),
      tags: config.tags.clone(),
      joined_at: chrono::Utc::now().to_rfc3339(),
      barn: Some(barn),
      gate: Some(gate),
    })
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

  /// Persists this Croft's state directly into the local Barn storage under
  /// `croft/{id}`.
  ///
  /// # Errors
  /// Returns an error if serialization fails or Barn write fails.
  pub async fn persist(&self) -> Result<(), Box<dyn Error>> {
    if let Some(ref barn) = self.barn {
      let key = format!("{}{}", CROFT_PREFIX, self.id);
      let payload = serde_json::to_vec(self)?;
      barn.set(&key, payload).await?;
    }
    Ok(())
  }

  /// Returns `true` if this Croft has the [`CroftRole::Server`] role.
  pub fn is_server(&self) -> bool {
    self.roles.contains(&CroftRole::Server)
  }

  /// Returns `true` if this Croft has the [`CroftRole::Worker`] role.
  pub fn is_worker(&self) -> bool {
    self.roles.contains(&CroftRole::Worker)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::net::SocketAddr;
  use tempfile::tempdir;

  #[tokio::test]
  async fn croft_spawns_cleanly() {
    let dir = tempdir().unwrap();
    let addr: SocketAddr = "127.0.0.1:7440".parse().unwrap();
    let config = Config {
      name: "test-node".to_string(),
      addr,
      data_dir: dir.path().to_path_buf(),
      ..Default::default()
    };

    let croft = Croft::spawn(&config).await.unwrap();
    assert_eq!(croft.name, "test-node");
    assert!(croft.id > 0);
    assert!(croft.is_server());
    assert!(croft.is_worker());
    assert!(croft.barn.is_some());
    assert!(croft.gate.is_some());
  }

  #[test]
  fn croft_serialization_roundtrip_skips_infrastructure() {
    let croft = Croft {
      id: 12345,
      name: "worker-01".to_string(),
      addr: "127.0.0.1:7440".to_string(),
      roles: vec![CroftRole::Server],
      tags: vec!["zone-a".to_string()],
      joined_at: "2026-09-05T12:00:00Z".to_string(),
      barn: None,
      gate: None,
    };

    let json = serde_json::to_string(&croft).unwrap();
    assert!(!json.contains("barn"));
    assert!(!json.contains("gate"));

    let deserialized: Croft = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.id, 12345);
    assert_eq!(deserialized.name, "worker-01");
    assert_eq!(deserialized.addr, "127.0.0.1:7440");
    assert_eq!(deserialized.roles, vec![CroftRole::Server]);
    assert_eq!(deserialized.tags, vec!["zone-a".to_string()]);
    assert_eq!(deserialized.joined_at, "2026-09-05T12:00:00Z");
    assert!(deserialized.barn.is_none());
    assert!(deserialized.gate.is_none());
  }
}
