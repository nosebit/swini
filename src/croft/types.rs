//! Domain types, role definitions, and cluster operational designations for
//! Crofts.
//!
//! Exposes [`CroftRole`], which designates the functional responsibilities
//! assigned to a Croft within the Ranch cluster (Server consensus vs. Worker
//! execution), and [`Croft`], the persistent domain entity holding a Croft's
//! identity and metadata.

use crate::croft::resources::CroftResources;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Operational role assigned to a Croft within the Ranch.
///
/// Determines whether the Croft participates in Raft consensus and control
/// plane storage ([`CroftRole::Server`]) or acts strictly as a workload
/// executor ([`CroftRole::Worker`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CroftRole {
  /// Server node: participates in Raft consensus, metadata storage, and
  /// control plane.
  Server,
  /// Worker node: executes scheduled workloads under supervision.
  Worker,
}

impl fmt::Display for CroftRole {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      CroftRole::Server => write!(f, "server"),
      CroftRole::Worker => write!(f, "worker"),
    }
  }
}

impl FromStr for CroftRole {
  type Err = String;

  /// Parses a string slice into a [`CroftRole`].
  ///
  /// # Errors
  /// Returns an error string if the role is neither "server" nor "worker"
  /// (case-insensitive).
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.trim().to_lowercase().as_str() {
      "server" => Ok(CroftRole::Server),
      "worker" => Ok(CroftRole::Worker),
      other => Err(format!(
        "Unknown Croft role '{}'. Expected 'server' or 'worker'.",
        other
      )),
    }
  }
}

/// Persistent domain entity representing a Croft's identity and metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
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
  /// Hardware resource capacity and active workload allocations.
  pub resources: CroftResources,
}

impl Croft {
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

  #[test]
  fn croft_role_parsing() {
    assert_eq!("server".parse::<CroftRole>().unwrap(), CroftRole::Server);
    assert_eq!("worker".parse::<CroftRole>().unwrap(), CroftRole::Worker);
    assert_eq!("SERVER".parse::<CroftRole>().unwrap(), CroftRole::Server);
    assert_eq!("Worker ".parse::<CroftRole>().unwrap(), CroftRole::Worker);
    assert!("invalid".parse::<CroftRole>().is_err());
  }

  #[test]
  fn croft_role_serialization_roundtrip() {
    let server = CroftRole::Server;
    let json = serde_json::to_string(&server).unwrap();
    assert_eq!(json, "\"server\"");
    let parsed: CroftRole = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, server);

    let worker = CroftRole::Worker;
    let json = serde_json::to_string(&worker).unwrap();
    assert_eq!(json, "\"worker\"");
    let parsed: CroftRole = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, worker);
  }

  #[test]
  fn croft_role_checks_and_serialization() {
    let croft = Croft {
      id: 42,
      name: "worker-01".to_string(),
      addr: "127.0.0.1:7440".to_string(),
      roles: vec![CroftRole::Worker],
      tags: vec!["zone-a".to_string()],
      joined_at: "2026-09-05T12:00:00Z".to_string(),
      resources: CroftResources {
        cpu_total: 16_000_000_000,
        cpu_yardable: 14_400_000_000,
        cpu_reserved: 0,
        mem_total: 16_000_000_000,
        mem_yardable: 14_400_000_000,
        mem_reserved: 0,
      },
    };

    assert!(!croft.is_server());
    assert!(croft.is_worker());
    assert_eq!(croft.resources.cpu_available(), 14_400_000_000);

    let json = serde_json::to_string(&croft).unwrap();
    let deserialized: Croft = serde_json::from_str(&json).unwrap();
    assert_eq!(croft, deserialized);
  }
}
