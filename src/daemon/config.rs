use crate::node::NodeRole;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;

pub const DEFAULT_API_PORT: u16 = 7440;

/// API server configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiConfig {
  /// Local socket address the gRPC server binds to.
  pub bind_address: SocketAddr,
}

impl Default for ApiConfig {
  fn default() -> Self {
    Self {
      bind_address: SocketAddr::from(([127, 0, 0, 1], DEFAULT_API_PORT)),
    }
  }
}

/// Configuration passed to a Swini daemon on startup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
  /// Optional human-readable daemon name (also used as node name).
  pub name: Option<String>,
  /// API configuration for gRPC listener and peer reachability.
  pub api: ApiConfig,
  /// Directory for persistent storage (Barn DB, raft logs, node identity).
  pub data_dir: PathBuf,
  /// The roles this node fulfills (Server, Worker, or both).
  pub roles: Vec<NodeRole>,
  /// Custom tags assigned to this node.
  pub tags: Vec<String>,
  /// List of peer API addresses to join on boot.
  pub join_addresses: Vec<String>,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn default_api_config_binds_to_7440() {
    let api = ApiConfig::default();
    assert_eq!(
      api.bind_address,
      SocketAddr::from(([127, 0, 0, 1], DEFAULT_API_PORT))
    );
  }

  #[test]
  fn config_serialization_roundtrip() {
    let config = Config {
      name: Some("node-primary".to_string()),
      api: ApiConfig::default(),
      data_dir: PathBuf::from("/tmp/swini/data"),
      roles: vec![NodeRole::Server, NodeRole::Worker],
      tags: vec!["us-east".to_string()],
      join_addresses: vec!["127.0.0.1:7440".to_string()],
    };

    let serialized = serde_json::to_string(&config).expect("serialize");
    let deserialized: Config =
      serde_json::from_str(&serialized).expect("deserialize");

    assert_eq!(deserialized.name.as_deref(), Some("node-primary"));
    assert_eq!(deserialized.api, ApiConfig::default());
    assert_eq!(deserialized.roles, vec![NodeRole::Server, NodeRole::Worker]);
    assert_eq!(deserialized.join_addresses, vec!["127.0.0.1:7440"]);
  }
}
