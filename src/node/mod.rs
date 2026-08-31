pub mod types;

pub use types::*;

use crate::core::proto::cluster::cluster::cluster_api_client::ClusterApiClient;
use crate::core::proto::cluster::cluster::JoinReq;
use crate::daemon::Config;
use crate::store::barn::{Barn, Node as BarnNode};
use crate::store::{SpreadNode, SpreadStore};
use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct NodeBackend {
  config: Config,
  self_node: Arc<RwLock<Node>>,
  barn: Arc<Barn>,
}

impl NodeBackend {
  /// Spawns the NodeBackend, generating/persisting the local node ID and
  /// initializing self state.
  pub async fn spawn(
    config: Config,
    barn: Arc<Barn>,
  ) -> Result<Self, Box<dyn Error>> {
    let node_id = Self::provide_node_id(&config.data_dir)?;
    let name = config
      .name
      .clone()
      .unwrap_or_else(|| format!("node-{}", node_id));

    let self_node = Node {
      id: node_id,
      name,
      tags: config.tags.clone(),
      roles: config.roles.clone(),
      api_addr: config.api.bind_address.to_string(),
      joined_at: String::new(),
      ..Default::default()
    };

    Ok(Self {
      config,
      self_node: Arc::new(RwLock::new(self_node)),
      barn,
    })
  }

  /// Returns a handle to the self node state.
  pub fn self_node(&self) -> Arc<RwLock<Node>> {
    self.self_node.clone()
  }

  /// Initiates join against configured peer addresses, trying each address
  /// sequentially and failing over to the next if connection fails.
  pub async fn join(&self) -> Result<(), Box<dyn Error>> {
    if self.config.join_addresses.is_empty() {
      return Ok(());
    }

    let node = self.self_node.read().await.clone();
    for peer in &self.config.join_addresses {
      match self.dial_join(peer, &node).await {
        Ok(server_nodes) => {
          // For worker nodes, populate the Barn node cache with the received
          // server nodes
          if !node.roles.contains(&NodeRole::Server) {
            let barn_nodes: Vec<BarnNode> = server_nodes
              .into_iter()
              .map(|n| BarnNode::new(n.id, n.api_addr))
              .collect();
            self.barn.node_cache_set(barn_nodes).await?;
          }
          return Ok(());
        }
        Err(e) => {
          tracing::warn!(peer = %peer, error = %e, "failed to join peer, attempting next if available");
        }
      }
    }

    Err("Failed to join cluster from any configured peer address".into())
  }

  /// Performs the remote gRPC join RPC call to a peer.
  async fn dial_join(
    &self,
    peer_addr: &str,
    node: &Node,
  ) -> Result<Vec<Node>, Box<dyn Error>> {
    let endpoint = if peer_addr.starts_with("http://")
      || peer_addr.starts_with("https://")
    {
      peer_addr.to_string()
    } else {
      format!("http://{}", peer_addr)
    };

    let mut client = ClusterApiClient::connect(endpoint).await?;
    let req = JoinReq {
      id: node.id,
      name: node.name.clone(),
      api_addr: node.api_addr.clone(),
      roles: node
        .roles
        .iter()
        .map(|r| format!("{:?}", r).to_lowercase())
        .collect(),
      tags: node.tags.clone(),
    };

    let res = client.join(req).await?.into_inner();
    let server_nodes = res
      .server_nodes
      .into_iter()
      .map(|n| Node {
        id: n.id,
        name: n.name,
        api_addr: n.api_addr,
        roles: n
          .roles
          .into_iter()
          .map(|r| match r.to_lowercase().as_str() {
            "server" => NodeRole::Server,
            _ => NodeRole::Worker,
          })
          .collect(),
        tags: n.tags,
        joined_at: n.joined_at,
        ..Default::default()
      })
      .collect();

    Ok(server_nodes)
  }

  /// Helper to persist or retrieve the deterministic node ID from disk
  /// (`data_dir/node.id`).
  pub fn provide_node_id(data_dir: &Path) -> Result<u64, Box<dyn Error>> {
    let id_file = data_dir.join("node.id");
    if id_file.exists() {
      let content = std::fs::read_to_string(&id_file)?;
      let id = content.trim().parse::<u64>()?;
      return Ok(id);
    }

    let id = rand::random::<u32>() as u64;
    std::fs::create_dir_all(data_dir)?;
    std::fs::write(&id_file, id.to_string())?;
    Ok(id)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::daemon::ApiConfig;
  use tempfile::tempdir;

  fn test_raft_config() -> crate::store::barn::RaftConfig {
    crate::store::barn::RaftConfig {
      heartbeat_interval: 100,
      election_timeout_min: 200,
      election_timeout_max: 400,
    }
  }

  async fn spawn_test_barn(node_id: u64, port: u16) -> Barn {
    let dir = tempdir().expect("create temp dir");
    let config = crate::store::barn::Config {
      node_id,
      api_addr: format!("127.0.0.1:{port}"),
      data_dir: dir.path().to_path_buf(),
      raft: test_raft_config(),
    };
    std::mem::forget(dir);
    Barn::spawn(config).await.expect("spawn barn")
  }

  #[test]
  fn provide_node_id_generates_and_persists() {
    let dir = tempdir().unwrap();
    let id1 = NodeBackend::provide_node_id(dir.path()).unwrap();
    assert!(id1 > 0);

    // Reading again returns identical ID
    let id2 = NodeBackend::provide_node_id(dir.path()).unwrap();
    assert_eq!(id1, id2);
  }

  #[tokio::test]
  async fn node_backend_spawn_initializes_node() {
    let dir = tempdir().unwrap();
    let barn = Arc::new(spawn_test_barn(1, 7440).await);

    let config = Config {
      name: Some("test-node".to_string()),
      api: ApiConfig::default(),
      data_dir: dir.path().to_path_buf(),
      roles: vec![NodeRole::Server, NodeRole::Worker],
      tags: vec!["alpha".to_string()],
      join_addresses: vec![],
    };

    let backend = NodeBackend::spawn(config, barn).await.unwrap();
    let node = backend.self_node.read().await.clone();

    assert_eq!(node.name, "test-node");
    assert_eq!(node.roles, vec![NodeRole::Server, NodeRole::Worker]);
    assert_eq!(node.tags, vec!["alpha"]);
    assert_eq!(node.api_addr, "127.0.0.1:7440");
  }

  #[tokio::test]
  async fn node_backend_join_failover() {
    let dir = tempdir().unwrap();
    let barn = Arc::new(spawn_test_barn(2, 7442).await);

    // When all join addresses are unreachable, join returns error
    let config = Config {
      name: Some("worker-fail".to_string()),
      api: ApiConfig::default(),
      data_dir: dir.path().to_path_buf(),
      roles: vec![NodeRole::Worker],
      tags: vec![],
      join_addresses: vec![
        "127.0.0.1:59998".to_string(),
        "127.0.0.1:59999".to_string(),
      ],
    };

    let backend = NodeBackend::spawn(config, barn).await.unwrap();
    let res = backend.join().await;
    assert!(res.is_err());
  }
}
