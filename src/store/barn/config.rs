//! Configuration parameters for a Barn storage node.

use std::path::PathBuf;

/// Pure data entity representing configuration for an instantiated
/// [`Barn`](super::Barn) node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
  /// Unique 64-bit identifier for this Barn node (matches Plot ID).
  pub id: u64,
  /// Network address (e.g., `127.0.0.1:7440`) where the Barn gRPC service
  /// listens.
  pub addr: String,
  /// Local directory where database state and Raft logs are persisted.
  pub data_dir: PathBuf,
  /// Raft leader heartbeat interval in milliseconds.
  pub heartbeat_interval: u64,
  /// Minimum Raft election timeout in milliseconds.
  pub election_timeout_min: u64,
  /// Maximum Raft election timeout in milliseconds.
  pub election_timeout_max: u64,
}
