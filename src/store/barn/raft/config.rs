//! Configuration parameters for Raft consensus election and heartbeat timers.

/// Pure data entity representing timer parameters for Raft consensus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
  /// Raft leader heartbeat interval in milliseconds.
  pub heartbeat_interval: u64,
  /// Minimum election timeout in milliseconds.
  pub election_timeout_min: u64,
  /// Maximum election timeout in milliseconds.
  pub election_timeout_max: u64,
}
