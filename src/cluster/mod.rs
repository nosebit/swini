use crate::node::{Node, NodeRole};
use crate::store::barn::{Barn, Node as BarnNode};
use crate::store::{ItemStore, SpreadNode, SpreadStore};
use std::error::Error;
use std::sync::Arc;

#[derive(Clone)]
pub struct ClusterBackend {
  barn: Arc<Barn>,
  node_store: Barn,
}

impl ClusterBackend {
  /// Creates a new ClusterBackend wrapping the node slice of the Barn.
  pub fn new(barn: Arc<Barn>) -> Self {
    let node_store = barn.slice("node/");
    Self { barn, node_store }
  }

  /// Registers a joining node into the cluster, stamping joined_at, and returns
  /// all server nodes.
  pub async fn join(
    &self,
    mut node: Node,
  ) -> Result<Vec<Node>, Box<dyn Error>> {
    // 1. If joining node has Server role, add to Barn Raft membership
    if node.roles.contains(&NodeRole::Server) {
      let barn_node = BarnNode::new(node.id, node.api_addr.clone());
      self.barn.node_add(barn_node).await?;
    }

    // 2. Set authoritative join timestamp
    node.joined_at = chrono::Utc::now().to_rfc3339();

    // 3. Persist the full node record into Barn under "node/{node_id}"
    let payload = serde_json::to_vec(&node)?;
    self.node_store.set(&node.id.to_string(), payload).await?;

    // 4. Return the list of all registered Server nodes (for worker node
    //    routing cache)
    let all_nodes = self.status().await?;
    let server_nodes = all_nodes
      .into_iter()
      .filter(|n| n.roles.contains(&NodeRole::Server))
      .collect();
    Ok(server_nodes)
  }

  /// Queries all registered nodes from the cluster store.
  pub async fn status(&self) -> Result<Vec<Node>, Box<dyn Error>> {
    let records = self.node_store.list(None).await?;
    let mut nodes = Vec::new();
    for (_, value) in records {
      let node: Node = serde_json::from_slice(&value)?;
      nodes.push(node);
    }
    Ok(nodes)
  }

  /// Returns the node ID of the current cluster primary (Barn Raft leader).
  pub async fn primary_node_id(&self) -> Option<u64> {
    self
      .barn
      .node_list()
      .await
      .ok()
      .and_then(|nodes| nodes.into_iter().find(|n| n.is_leader).map(|n| n.id))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
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

  #[tokio::test]
  async fn cluster_backend_join_and_status() {
    let barn = Arc::new(spawn_test_barn(1, 7440).await);
    let cluster = ClusterBackend::new(barn.clone());

    let server_node = Node {
      id: 1,
      name: "seed-server".to_string(),
      tags: vec!["primary".to_string()],
      roles: vec![NodeRole::Server],
      api_addr: "127.0.0.1:7440".to_string(),
      ..Default::default()
    };

    let worker_node = Node {
      id: 2,
      name: "worker-1".to_string(),
      tags: vec!["gpu".to_string()],
      roles: vec![NodeRole::Worker],
      api_addr: "127.0.0.1:7441".to_string(),
      ..Default::default()
    };

    // Join server
    let server_nodes = cluster.join(server_node.clone()).await.unwrap();
    assert_eq!(server_nodes.len(), 1);
    assert_eq!(server_nodes[0].id, 1);

    // Join worker
    let server_nodes_after_worker =
      cluster.join(worker_node.clone()).await.unwrap();
    assert_eq!(server_nodes_after_worker.len(), 1);

    // Status queries both nodes
    let all_nodes = cluster.status().await.unwrap();
    assert_eq!(all_nodes.len(), 2);

    let found_server = all_nodes.iter().find(|n| n.id == 1).unwrap();
    assert_eq!(found_server.name, "seed-server");
    assert!(!found_server.joined_at.is_empty());

    let found_worker = all_nodes.iter().find(|n| n.id == 2).unwrap();
    assert_eq!(found_worker.name, "worker-1");
    assert!(!found_worker.joined_at.is_empty());

    let primary_id = cluster.primary_node_id().await;
    assert_eq!(primary_id, Some(1));
  }

  fn reserve_local_addr() -> std::net::SocketAddr {
    let listener =
      std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    let addr = listener.local_addr().expect("local addr");
    drop(listener);
    addr
  }

  #[tokio::test]
  async fn multi_node_cluster_join_over_grpc() {
    // 1. Seed server (Node 1)
    let addr1 = reserve_local_addr();
    let dir1 = tempdir().unwrap();
    let barn_cfg1 = crate::store::barn::Config {
      node_id: 1,
      api_addr: addr1.to_string(),
      data_dir: dir1.path().to_path_buf(),
      raft: test_raft_config(),
    };
    std::mem::forget(dir1);
    let barn1 = Arc::new(Barn::spawn(barn_cfg1).await.unwrap());
    let cluster1 = ClusterBackend::new(barn1.clone());

    // Register node 1 in cluster state
    let server1_node = Node {
      id: 1,
      name: "seed-server".to_string(),
      roles: vec![NodeRole::Server],
      api_addr: addr1.to_string(),
      ..Default::default()
    };
    cluster1.join(server1_node).await.unwrap();

    // Start API server on addr1
    let api1 = crate::api::Api::new(barn1.clone());
    tokio::spawn(async move {
      let _ = api1.listen(addr1).await;
    });

    // Wait for node 1 to establish raft leadership
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
      while cluster1.primary_node_id().await != Some(1) {
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
      }
    })
    .await
    .expect("node 1 becomes leader");

    // 2. Joining Server (Node 2)
    let addr2 = reserve_local_addr();
    let dir2 = tempdir().unwrap();
    let node2_dir = dir2.path().to_path_buf();
    std::mem::forget(dir2);
    let node2_id =
      crate::node::NodeBackend::provide_node_id(&node2_dir).unwrap();
    let barn_cfg2 = crate::store::barn::Config {
      node_id: node2_id,
      api_addr: addr2.to_string(),
      data_dir: node2_dir.clone(),
      raft: test_raft_config(),
    };
    let barn2 = Arc::new(Barn::spawn(barn_cfg2).await.unwrap());

    let api2 = crate::api::Api::new(barn2.clone());
    tokio::spawn(async move {
      let _ = api2.listen(addr2).await;
    });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let node2_backend = crate::node::NodeBackend::spawn(
      crate::daemon::Config {
        name: Some("server-2".to_string()),
        api: crate::daemon::ApiConfig {
          bind_address: addr2,
        },
        data_dir: node2_dir,
        roles: vec![NodeRole::Server],
        tags: vec!["backup".to_string()],
        join_addresses: vec![addr1.to_string()],
      },
      barn2.clone(),
    )
    .await
    .unwrap();

    node2_backend.join().await.expect("server 2 join");

    // Verify both nodes are in cluster state
    let nodes = cluster1.status().await.unwrap();
    assert_eq!(nodes.len(), 2);

    // 3. Joining Worker (Node 3)
    let addr3 = reserve_local_addr();
    let dir3 = tempdir().unwrap();
    let node3_dir = dir3.path().to_path_buf();
    std::mem::forget(dir3);
    let node3_id =
      crate::node::NodeBackend::provide_node_id(&node3_dir).unwrap();
    let barn_cfg3 = crate::store::barn::Config {
      node_id: node3_id,
      api_addr: addr3.to_string(),
      data_dir: node3_dir.clone(),
      raft: test_raft_config(),
    };
    let barn3 = Arc::new(Barn::spawn(barn_cfg3).await.unwrap());

    let node3_backend = crate::node::NodeBackend::spawn(
      crate::daemon::Config {
        name: Some("worker-3".to_string()),
        api: crate::daemon::ApiConfig {
          bind_address: addr3,
        },
        data_dir: node3_dir,
        roles: vec![NodeRole::Worker],
        tags: vec!["compute".to_string()],
        join_addresses: vec![addr1.to_string()],
      },
      barn3.clone(),
    )
    .await
    .unwrap();

    node3_backend.join().await.expect("worker 3 join");

    // Verify all 3 nodes are registered in cluster
    let all_nodes = cluster1.status().await.unwrap();
    assert_eq!(all_nodes.len(), 3);

    // Verify worker node cache has received server topology
    let cached_servers = barn3.node_list().await.unwrap();
    assert_eq!(cached_servers.len(), 2);
  }
}
