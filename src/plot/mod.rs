//! Swini Plot domain model, role definitions, and staff clerk operations.
//!
//! Submodules:
//! - [`clerk`]: Domain [`Clerk`] and gRPC API handler managing Plot registry
//!   and cluster operations.

pub mod clerk;

pub use clerk::Clerk;

use crate::core::Config;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Operational role assigned to a Plot within the Ranch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlotRole {
  /// Server node: participates in Raft consensus, metadata storage, and
  /// control plane.
  Server,
  /// Worker node: executes scheduled workloads (Pigs) under Drover
  /// supervision.
  Worker,
}

impl fmt::Display for PlotRole {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      PlotRole::Server => write!(f, "server"),
      PlotRole::Worker => write!(f, "worker"),
    }
  }
}

impl FromStr for PlotRole {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.trim().to_lowercase().as_str() {
      "server" => Ok(PlotRole::Server),
      "worker" => Ok(PlotRole::Worker),
      other => Err(format!(
        "Unknown Plot role '{}'. Expected 'server' or 'worker'.",
        other
      )),
    }
  }
}

/// Domain representation of a computational parcel on the Ranch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Plot {
  /// Unique 64-bit identifier for this Plot.
  pub id: u64,
  /// Human-readable node name (e.g., "worker-01").
  pub name: String,
  /// Canonical network address (e.g., "127.0.0.1:7440") where the Plot's
  /// Gate listens.
  pub addr: String,
  /// Assigned roles in the cluster (e.g., Server, Worker).
  pub roles: Vec<PlotRole>,
  /// Informational or grouping tags associated with the Plot (e.g., "gpu",
  /// "zone-a").
  pub tags: Vec<String>,
  /// ISO-8601 UTC timestamp when the Plot joined the Ranch.
  pub joined_at: String,
}

impl Plot {
  /// Instantiates a new [`Plot`] domain model from the runtime configuration,
  /// reading or generating a persistent 64-bit Plot identifier.
  pub fn new(config: &Config) -> Result<Self, Box<dyn std::error::Error>> {
    let plot_id = Self::id_provide(&config.data_dir)?;
    Ok(Self {
      id: plot_id,
      name: config.name.clone(),
      addr: config.addr.to_string(),
      roles: config.roles.clone(),
      tags: config.tags.clone(),
      joined_at: chrono::Utc::now().to_rfc3339(),
    })
  }

  /// Provides a unique and persistent 64-bit ID for this Plot.
  /// Reads `{data_dir}/plot.id` or generates and persists a new random ID.
  pub fn id_provide(
    data_dir: &std::path::Path,
  ) -> Result<u64, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(data_dir)?;
    let id_path = data_dir.join("plot.id");
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

  /// Returns `true` if this Plot has the [`PlotRole::Server`] role.
  pub fn is_server(&self) -> bool {
    self.roles.contains(&PlotRole::Server)
  }

  /// Returns `true` if this Plot has the [`PlotRole::Worker`] role.
  pub fn is_worker(&self) -> bool {
    self.roles.contains(&PlotRole::Worker)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn plot_role_parsing() {
    assert_eq!("server".parse::<PlotRole>().unwrap(), PlotRole::Server);
    assert_eq!("worker".parse::<PlotRole>().unwrap(), PlotRole::Worker);
    assert_eq!("SERVER".parse::<PlotRole>().unwrap(), PlotRole::Server);
    assert_eq!("Worker ".parse::<PlotRole>().unwrap(), PlotRole::Worker);
    assert!("invalid".parse::<PlotRole>().is_err());
  }

  #[test]
  fn plot_role_serialization_roundtrip() {
    let server = PlotRole::Server;
    let json = serde_json::to_string(&server).unwrap();
    assert_eq!(json, "\"server\"");
    let parsed: PlotRole = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, server);
  }

  #[test]
  fn plot_entity_serialization_roundtrip() {
    let plot = Plot {
      id: 12345,
      name: "node-1".to_string(),
      addr: "127.0.0.1:7440".to_string(),
      roles: vec![PlotRole::Server, PlotRole::Worker],
      tags: vec!["gpu".to_string(), "rack-a".to_string()],
      joined_at: "2026-09-03T15:00:00Z".to_string(),
    };

    assert!(plot.is_server());
    assert!(plot.is_worker());

    let json = serde_json::to_string(&plot).unwrap();
    let parsed: Plot = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, plot);
  }
}
