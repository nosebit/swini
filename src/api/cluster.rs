use crate::cluster::ClusterBackend;
use crate::core::proto::cluster::cluster::cluster_api_server::ClusterApi;
use crate::core::proto::cluster::cluster::{
  JoinReq, JoinRes, Node as ProtoNode, StatusReq, StatusRes,
};
use crate::node::{Node, NodeRole};
use std::sync::Arc;
use tonic::{Request, Response, Status};

impl TryFrom<JoinReq> for Node {
  type Error = Status;

  fn try_from(req: JoinReq) -> Result<Self, Self::Error> {
    if req.id == 0 {
      return Err(Status::invalid_argument("node id must be greater than 0"));
    }
    if req.api_addr.trim().is_empty() {
      return Err(Status::invalid_argument("api_addr cannot be empty"));
    }
    let roles = req
      .roles
      .into_iter()
      .map(|r| match r.to_lowercase().as_str() {
        "server" => Ok(NodeRole::Server),
        "worker" => Ok(NodeRole::Worker),
        other => {
          Err(Status::invalid_argument(format!("unknown role: {other}")))
        }
      })
      .collect::<Result<Vec<_>, _>>()?;

    if roles.is_empty() {
      return Err(Status::invalid_argument(
        "node must specify at least one role",
      ));
    }

    Ok(Node {
      id: req.id,
      name: if req.name.is_empty() {
        format!("node-{}", req.id)
      } else {
        req.name
      },
      tags: req.tags,
      roles,
      api_addr: req.api_addr,
      joined_at: String::new(),
      ..Default::default()
    })
  }
}

/// Converts a domain `Node` into its protobuf representation.
fn to_proto_node(node: Node, primary_id: Option<u64>) -> ProtoNode {
  ProtoNode {
    id: node.id,
    name: node.name,
    api_addr: node.api_addr,
    roles: node
      .roles
      .into_iter()
      .map(|r| format!("{:?}", r).to_lowercase())
      .collect(),
    tags: node.tags,
    joined_at: node.joined_at,
    is_primary: Some(node.id) == primary_id,
  }
}

/// Maps a vector of domain `Node`s to protobuf `Node`s.
fn to_proto_nodes(nodes: Vec<Node>, primary_id: Option<u64>) -> Vec<ProtoNode> {
  nodes
    .into_iter()
    .map(|n| to_proto_node(n, primary_id))
    .collect()
}

use crate::store::barn::Barn;

fn to_status<E: std::fmt::Display>(err: E) -> Status {
  Status::internal(err.to_string())
}

#[derive(Clone)]
pub struct ClusterApiHandler {
  cluster: ClusterBackend,
}

impl ClusterApiHandler {
  pub fn new(barn: Arc<Barn>) -> Self {
    Self {
      cluster: ClusterBackend::new(barn),
    }
  }
}

#[tonic::async_trait]
impl ClusterApi for ClusterApiHandler {
  async fn join(
    &self,
    request: Request<JoinReq>,
  ) -> Result<Response<JoinRes>, Status> {
    let node: Node = request.into_inner().try_into()?;
    let server_nodes = self.cluster.join(node).await.map_err(to_status)?;
    let primary_id = self.cluster.primary_node_id().await;

    Ok(Response::new(JoinRes {
      server_nodes: to_proto_nodes(server_nodes, primary_id),
    }))
  }

  async fn status(
    &self,
    _request: Request<StatusReq>,
  ) -> Result<Response<StatusRes>, Status> {
    let nodes = self.cluster.status().await.map_err(to_status)?;
    let primary_id = self.cluster.primary_node_id().await;

    Ok(Response::new(StatusRes {
      nodes: to_proto_nodes(nodes, primary_id),
      primary_id: primary_id.unwrap_or(0),
    }))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn join_req_try_into_valid_node() {
    let req = JoinReq {
      id: 10,
      name: "node-10".to_string(),
      api_addr: "127.0.0.1:7440".to_string(),
      roles: vec!["server".to_string(), "WORKER".to_string()],
      tags: vec!["tag1".to_string()],
    };

    let node: Node = req.try_into().expect("valid node");
    assert_eq!(node.id, 10);
    assert_eq!(node.name, "node-10");
    assert_eq!(node.api_addr, "127.0.0.1:7440");
    assert_eq!(node.roles, vec![NodeRole::Server, NodeRole::Worker]);
    assert_eq!(node.tags, vec!["tag1"]);
  }

  #[test]
  fn join_req_invalid_id_returns_error() {
    let req = JoinReq {
      id: 0,
      name: "zero".to_string(),
      api_addr: "127.0.0.1:7440".to_string(),
      roles: vec!["server".to_string()],
      tags: vec![],
    };

    let result: Result<Node, Status> = req.try_into();
    assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
  }

  #[test]
  fn join_req_empty_api_addr_returns_error() {
    let req = JoinReq {
      id: 1,
      name: "node".to_string(),
      api_addr: "   ".to_string(),
      roles: vec!["server".to_string()],
      tags: vec![],
    };

    let result: Result<Node, Status> = req.try_into();
    assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
  }

  #[test]
  fn join_req_unknown_role_returns_error() {
    let req = JoinReq {
      id: 1,
      name: "node".to_string(),
      api_addr: "127.0.0.1:7440".to_string(),
      roles: vec!["coordinator".to_string()],
      tags: vec![],
    };

    let result: Result<Node, Status> = req.try_into();
    assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
  }

  #[test]
  fn join_req_empty_roles_returns_error() {
    let req = JoinReq {
      id: 1,
      name: "node".to_string(),
      api_addr: "127.0.0.1:7440".to_string(),
      roles: vec![],
      tags: vec![],
    };

    let result: Result<Node, Status> = req.try_into();
    assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
  }

  #[test]
  fn to_proto_nodes_maps_primary_flag() {
    let nodes = vec![
      Node {
        id: 1,
        name: "node-1".to_string(),
        roles: vec![NodeRole::Server],
        api_addr: "127.0.0.1:7440".to_string(),
        joined_at: "2026-08-31T00:00:00Z".to_string(),
        ..Default::default()
      },
      Node {
        id: 2,
        name: "node-2".to_string(),
        roles: vec![NodeRole::Worker],
        api_addr: "127.0.0.1:7441".to_string(),
        joined_at: "2026-08-31T00:01:00Z".to_string(),
        ..Default::default()
      },
    ];

    let proto_nodes = to_proto_nodes(nodes, Some(1));
    assert_eq!(proto_nodes.len(), 2);
    assert!(proto_nodes[0].is_primary);
    assert!(!proto_nodes[1].is_primary);
  }
}
