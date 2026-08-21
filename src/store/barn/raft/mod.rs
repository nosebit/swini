mod config;
mod network;
mod storage;
mod types;

pub use config::*;
pub use types::*;

use network::NetworkFactory;
use openraft::{storage::RaftStateMachine, Raft, RaftTypeConfig};
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use storage::LogStorage;

/// This function creates a new openraft instance that can be directly used by
/// clients.
pub async fn create<C, SM>(
  node_id: C::NodeId,
  config: &Config,
  db_path: PathBuf,
  state: SM,
) -> Result<Raft<C>, Box<dyn Error>>
where
  C: RaftTypeConfig<Node = Node>,
  SM: RaftStateMachine<C>,
{
  let raft_config = Arc::new(
    openraft::Config {
      cluster_name: "swini".to_string(),
      heartbeat_interval: config.heartbeat_interval,
      election_timeout_min: config.election_timeout_min,
      election_timeout_max: config.election_timeout_max,
      ..Default::default()
    }
    .validate()?,
  );

  let raft_db_path = db_path.with_extension("raft");
  let db = Arc::new(redb::Database::create(raft_db_path)?);

  let log_storage = LogStorage::<C>::new(db.clone())?;
  let network = NetworkFactory::<C>::new();

  let raft =
    Raft::new(node_id, raft_config, network, log_storage, state).await?;

  Ok(raft)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::store::barn::storage::Storage;
  use crate::store::barn::types::TypeConfig;
  use redb::backends::InMemoryBackend;
  use redb::Builder;

  fn test_config() -> Config {
    Config {
      heartbeat_interval: 50,
      election_timeout_min: 150,
      election_timeout_max: 300,
      join_addresses: Vec::new(),
    }
  }

  fn test_state() -> Storage {
    let db = Builder::new()
      .create_with_backend(InMemoryBackend::new())
      .expect("create in-memory redb database");
    Storage::new(Arc::new(db)).expect("create storage")
  }

  #[tokio::test]
  async fn create_builds_a_usable_raft_instance() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let db_path = dir.path().join("swini");

    let raft =
      create::<TypeConfig, Storage>(1, &test_config(), db_path, test_state())
        .await
        .expect("create raft instance");

    let metrics = raft.metrics().borrow().clone();
    assert_eq!(metrics.id, 1);
    assert_eq!(metrics.current_leader, None);

    raft.shutdown().await.expect("shutdown raft instance");
  }
}
