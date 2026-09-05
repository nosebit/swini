//! Domain types, role definitions, and cluster operational designations for
//! Crofts.
//!
//! Exposes [`CroftRole`], which designates the functional responsibilities
//! assigned to a Croft within the Ranch cluster (Server consensus vs. Worker
//! execution).

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
}
