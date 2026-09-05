//! gRPC API handler and protobuf conversion helpers for the `CroftApi` service.
//!
//! Exposes [`Api`], which implements the tonic-generated [`CroftApi`]
//! trait, delegating join registrations and status inquiries to the underlying
//! [`Clerk`].

use crate::core::proto::croft::croft_api_server::{CroftApi, CroftApiServer};
use crate::core::proto::croft::{
  Croft as ProtoCroft, JoinReq, JoinRes, StatusReq, StatusRes,
};
use crate::croft::clerk::Clerk;
use crate::croft::{Croft, CroftRole};
use tonic::{Request, Response, Status};

/// Validates and converts an incoming [`JoinReq`] Protobuf message into a
/// domain [`Croft`].
///
/// # Errors
/// Returns an error string if `name` or `addr` are blank, or if any role string
/// is unrecognized.
pub fn croft_from_join_req(req: JoinReq) -> Result<Croft, String> {
  let name = req.name.trim();
  if name.is_empty() {
    return Err("Croft name cannot be empty".to_string());
  }

  let addr = req.addr.trim();
  if addr.is_empty() {
    return Err("Croft addr cannot be empty".to_string());
  }

  let roles: Result<Vec<CroftRole>, String> = req
    .roles
    .into_iter()
    .map(|r| r.parse::<CroftRole>())
    .collect();
  let roles = roles?;

  if roles.is_empty() {
    return Err("Croft must have at least one role".to_string());
  }

  Ok(Croft {
    id: req.id,
    name: name.to_string(),
    addr: addr.to_string(),
    roles,
    tags: req.tags,
    joined_at: String::new(),
    barn: None,
    gate: None,
  })
}

/// Converts an internal domain [`Croft`] entity into its Protobuf
/// [`ProtoCroft`] wire format.
pub fn proto_croft_from_croft(croft: Croft) -> ProtoCroft {
  ProtoCroft {
    id: croft.id,
    name: croft.name,
    addr: croft.addr,
    roles: croft.roles.into_iter().map(|r| r.to_string()).collect(),
    tags: croft.tags,
    joined_at: croft.joined_at,
    is_primary: false,
  }
}

/// gRPC service implementation for [`CroftApi`].
#[derive(Clone)]
pub struct Api {
  clerk: Clerk,
}

impl Api {
  /// Wraps a [`Clerk`] instance into a gRPC service handler.
  pub fn new(clerk: Clerk) -> Self {
    Self { clerk }
  }

  /// Converts this handler into a tonic [`CroftApiServer`].
  pub fn into_server(self) -> CroftApiServer<Self> {
    CroftApiServer::new(self)
  }
}

#[tonic::async_trait]
impl CroftApi for Api {
  async fn join(
    &self,
    request: Request<JoinReq>,
  ) -> Result<Response<JoinRes>, Status> {
    let incoming = croft_from_join_req(request.into_inner())
      .map_err(Status::invalid_argument)?;

    let server_crofts = self
      .clerk
      .join(incoming)
      .await
      .map_err(|e| Status::internal(e.to_string()))?;

    let proto_servers = server_crofts
      .into_iter()
      .map(proto_croft_from_croft)
      .collect();
    Ok(Response::new(JoinRes {
      server_crofts: proto_servers,
    }))
  }

  async fn status(
    &self,
    _request: Request<StatusReq>,
  ) -> Result<Response<StatusRes>, Status> {
    let registered_crofts = self
      .clerk
      .list()
      .await
      .map_err(|e| Status::internal(e.to_string()))?;

    let crofts = registered_crofts
      .into_iter()
      .map(proto_croft_from_croft)
      .collect();
    Ok(Response::new(StatusRes {
      crofts,
      primary_id: 0,
    }))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn croft_from_join_req_validation() {
    let valid_req = JoinReq {
      id: 42,
      name: "worker-01".to_string(),
      addr: "10.0.0.1:7440".to_string(),
      roles: vec!["server".to_string(), "worker".to_string()],
      tags: vec!["zone-a".to_string()],
    };
    let croft = croft_from_join_req(valid_req).unwrap();
    assert_eq!(croft.id, 42);
    assert_eq!(croft.name, "worker-01");
    assert_eq!(croft.addr, "10.0.0.1:7440");
    assert_eq!(croft.roles, vec![CroftRole::Server, CroftRole::Worker]);
    assert_eq!(croft.tags, vec!["zone-a"]);

    let empty_name = JoinReq {
      name: "   ".to_string(),
      ..Default::default()
    };
    assert!(croft_from_join_req(empty_name).is_err());

    let empty_addr = JoinReq {
      name: "test".to_string(),
      addr: "   ".to_string(),
      roles: vec!["server".to_string()],
      tags: vec![],
      id: 1,
    };
    assert!(croft_from_join_req(empty_addr).is_err());

    let empty_roles = JoinReq {
      name: "test".to_string(),
      addr: "127.0.0.1:7440".to_string(),
      roles: vec![],
      tags: vec![],
      id: 1,
    };
    assert!(croft_from_join_req(empty_roles).is_err());

    let invalid_role = JoinReq {
      name: "test".to_string(),
      addr: "127.0.0.1:7440".to_string(),
      roles: vec!["invalid_role".to_string()],
      tags: vec![],
      id: 1,
    };
    assert!(croft_from_join_req(invalid_role).is_err());
  }

  #[test]
  fn proto_croft_conversion() {
    let domain_croft = Croft {
      id: 99,
      name: "croft-99".to_string(),
      addr: "127.0.0.1:7440".to_string(),
      roles: vec![CroftRole::Server],
      tags: vec!["fast".to_string()],
      joined_at: "2026-09-05T12:00:00Z".to_string(),
      barn: None,
      gate: None,
    };

    let proto = proto_croft_from_croft(domain_croft.clone());
    assert_eq!(proto.id, 99);
    assert_eq!(proto.name, "croft-99");
    assert_eq!(proto.roles, vec!["server".to_string()]);
    assert_eq!(proto.joined_at, "2026-09-05T12:00:00Z");
  }

  #[tokio::test]
  async fn api_handler_status_and_join_rpcs() {
    use crate::store::barn::Node as BarnNode;
    use crate::store::SpreadNode;
    use std::collections::BTreeMap;

    let dir = tempfile::tempdir().unwrap();
    let config = crate::croft::Config {
      addr: "127.0.0.1:0".parse().unwrap(),
      data_dir: dir.path().to_path_buf(),
      ..Default::default()
    };
    let croft =
      std::sync::Arc::new(crate::croft::Croft::spawn(&config).await.unwrap());
    let barn = croft.barn.as_ref().unwrap();
    let self_node = BarnNode::new(croft.id, croft.addr.clone());
    let mut members = BTreeMap::new();
    members.insert(self_node.id, self_node);
    barn.raft().initialize(members).await.unwrap();

    let clerk = Clerk::spawn(croft).unwrap();
    let api = Api::new(clerk);
    let _server = api.clone().into_server();

    let status_res = api.status(Request::new(StatusReq {})).await;
    assert!(status_res.is_ok());

    let join_res = api
      .join(Request::new(JoinReq {
        id: 100,
        name: "node-100".to_string(),
        addr: "127.0.0.1:7440".to_string(),
        roles: vec!["worker".to_string()],
        tags: vec!["zone-a".to_string()],
      }))
      .await;
    assert!(join_res.is_ok());
  }
}
