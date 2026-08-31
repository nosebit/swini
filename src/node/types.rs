use serde::{Deserialize, Serialize};

/// This represents the multiple roles a swini node can have.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeRole {
  /// This role is used for nodes that keeps a local copy of the entire cluster
  /// state and participate in the pig scheduling.
  Server,

  /// This role is used for nodes that are capable of running pigs.
  Worker,
}

/// By default the NodeRole is set to worker.
impl Default for NodeRole {
  fn default() -> Self {
    Self::Worker
  }
}

/// This represents a node (the node state) in the cluster.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Node {
  /// The unique node id.
  pub id: u64,

  /// A custom name the user can associated with this node for easy debugging
  /// and logs.
  pub name: String,

  /// A set of tags the user can associate with this node. This tags can be
  /// used to target specific nodes when scheduling pigs.
  pub tags: Vec<String>,

  /// The total number of CPU MHz available in the node.
  pub cpu_total: u32,

  /// The total number of Memory Mb available in the node.
  pub memory_total: u32,

  /// The total number of CPU MHz reserved for piglets running in this node.
  pub cpu_reserved: u32,

  /// The total number of Memory Mbs reserved for piglets running in this node.
  pub memory_reserved: u32,

  /// The available (free) number of CPU MHz we can use in this node for
  /// piglets.
  pub yardable_cpu: u32,

  /// The available (free) number of Memory Mb we can use in this node for
  /// piglets.
  pub yardable_memory: u32,

  /// The list of piglets currently running in this node.
  pub running_piglets: Vec<String>,

  /// The list of roles played by this node.
  pub roles: Vec<NodeRole>,

  /// The daemon api address on this node.
  pub api_addr: String,

  /// ISO 8601 / RFC 3339 timestamp when the node joined the cluster.
  pub joined_at: String,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn default_node_role_is_worker() {
    assert_eq!(NodeRole::default(), NodeRole::Worker);
  }

  #[test]
  fn node_serialization_roundtrip() {
    let node = Node {
      id: 42,
      name: "worker-1".to_string(),
      tags: vec!["zone-a".to_string()],
      cpu_total: 8000,
      memory_total: 16000,
      cpu_reserved: 1000,
      memory_reserved: 2000,
      yardable_cpu: 7000,
      yardable_memory: 14000,
      running_piglets: vec!["pig-1".to_string()],
      roles: vec![NodeRole::Server, NodeRole::Worker],
      api_addr: "127.0.0.1:7440".to_string(),
      joined_at: "2026-08-31T12:00:00Z".to_string(),
    };

    let serialized = serde_json::to_string(&node).expect("serialize");
    let deserialized: Node =
      serde_json::from_str(&serialized).expect("deserialize");

    assert_eq!(deserialized.id, 42);
    assert_eq!(deserialized.name, "worker-1");
    assert_eq!(deserialized.roles, vec![NodeRole::Server, NodeRole::Worker]);
    assert_eq!(deserialized.api_addr, "127.0.0.1:7440");
    assert_eq!(deserialized.joined_at, "2026-08-31T12:00:00Z");
  }
}
