//! gRPC API handler and protobuf conversion helpers for the `CroftApi` service.
//!
//! Exposes [`Api`], which implements the tonic-generated [`CroftApi`]
//! trait, delegating join registrations, cluster discovery listings, and node
//! status inquiries to the underlying [`Clerk`].

use crate::core::proto::croft::croft_api_server::{CroftApi, CroftApiServer};
use crate::core::proto::croft::{
  Croft as ProtoCroft, CroftResources as ProtoCroftResources,
  CroftTelemetry as ProtoCroftTelemetry, JoinReq, JoinRes, ListReq, ListRes,
  StatusReq, StatusRes,
};
use crate::croft::clerk::Clerk;
use crate::croft::resources::{CroftResources, CroftTelemetry};
use crate::croft::{Croft, CroftRole};
use tonic::{Request, Response, Status};

impl From<CroftResources> for ProtoCroftResources {
  /// Converts a domain [`CroftResources`] entity into its Protobuf
  /// [`ProtoCroftResources`] wire format.
  fn from(resources: CroftResources) -> Self {
    ProtoCroftResources {
      cpu_total: resources.cpu_total,
      cpu_yardable: resources.cpu_yardable,
      cpu_reserved: resources.cpu_reserved,
      mem_total: resources.mem_total,
      mem_yardable: resources.mem_yardable,
      mem_reserved: resources.mem_reserved,
    }
  }
}

impl From<ProtoCroftResources> for CroftResources {
  /// Converts a Protobuf [`ProtoCroftResources`] wire format into a domain
  /// [`CroftResources`].
  fn from(proto: ProtoCroftResources) -> Self {
    CroftResources {
      cpu_total: proto.cpu_total,
      cpu_yardable: proto.cpu_yardable,
      cpu_reserved: proto.cpu_reserved,
      mem_total: proto.mem_total,
      mem_yardable: proto.mem_yardable,
      mem_reserved: proto.mem_reserved,
    }
  }
}

impl From<CroftTelemetry> for ProtoCroftTelemetry {
  /// Converts a domain [`CroftTelemetry`] snapshot into its Protobuf
  /// [`ProtoCroftTelemetry`] wire format.
  fn from(telemetry: CroftTelemetry) -> Self {
    ProtoCroftTelemetry {
      mem_used: telemetry.mem_used,
      cpu_used: telemetry.cpu_used,
    }
  }
}

impl From<ProtoCroftTelemetry> for CroftTelemetry {
  /// Converts a Protobuf [`ProtoCroftTelemetry`] wire format into a domain
  /// [`CroftTelemetry`].
  fn from(proto: ProtoCroftTelemetry) -> Self {
    CroftTelemetry {
      mem_used: proto.mem_used,
      cpu_used: proto.cpu_used,
    }
  }
}

impl TryFrom<JoinReq> for Croft {
  type Error = String;

  /// Validates and converts an incoming [`JoinReq`] Protobuf message into a
  /// domain [`Croft`].
  ///
  /// # Errors
  /// Returns an error string if `name` or `addr` are blank, or if any role
  /// string is unrecognized.
  fn try_from(req: JoinReq) -> Result<Self, Self::Error> {
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

    let resources = req.resources.map(CroftResources::from).unwrap_or_default();

    Ok(Croft {
      id: req.id,
      name: name.to_string(),
      addr: addr.to_string(),
      roles,
      tags: req.tags,
      joined_at: String::new(),
      resources,
    })
  }
}

impl TryFrom<ProtoCroft> for Croft {
  type Error = String;

  /// Converts a wire format [`ProtoCroft`] message into a domain [`Croft`].
  ///
  /// # Errors
  /// Returns an error string if any role string is unrecognized.
  fn try_from(proto: ProtoCroft) -> Result<Self, Self::Error> {
    let roles: Result<Vec<CroftRole>, String> = proto
      .roles
      .into_iter()
      .map(|r| r.parse::<CroftRole>())
      .collect();

    let resources = proto
      .resources
      .map(CroftResources::from)
      .unwrap_or_default();

    Ok(Croft {
      id: proto.id,
      name: proto.name,
      addr: proto.addr,
      roles: roles?,
      tags: proto.tags,
      joined_at: proto.joined_at,
      resources,
    })
  }
}

impl From<Croft> for ProtoCroft {
  /// Converts an internal domain [`Croft`] entity into its Protobuf
  /// [`ProtoCroft`] wire format.
  fn from(croft: Croft) -> Self {
    ProtoCroft {
      id: croft.id,
      name: croft.name,
      addr: croft.addr,
      roles: croft.roles.into_iter().map(|r| r.to_string()).collect(),
      tags: croft.tags,
      joined_at: croft.joined_at,
      is_primary: false,
      resources: Some(ProtoCroftResources::from(croft.resources)),
    }
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
    let incoming = Croft::try_from(request.into_inner())
      .map_err(Status::invalid_argument)?;

    let server_crofts = self
      .clerk
      .join(incoming)
      .await
      .map_err(|e| Status::internal(e.to_string()))?;

    let proto_servers =
      server_crofts.into_iter().map(ProtoCroft::from).collect();
    Ok(Response::new(JoinRes {
      server_crofts: proto_servers,
    }))
  }

  async fn list(
    &self,
    _request: Request<ListReq>,
  ) -> Result<Response<ListRes>, Status> {
    let registered_crofts = self
      .clerk
      .list()
      .await
      .map_err(|e| Status::internal(e.to_string()))?;

    let crofts = registered_crofts
      .into_iter()
      .map(ProtoCroft::from)
      .collect();
    Ok(Response::new(ListRes {
      crofts,
      primary_id: 0,
    }))
  }

  async fn status(
    &self,
    request: Request<StatusReq>,
  ) -> Result<Response<StatusRes>, Status> {
    let req = request.into_inner();
    let name = req.name.trim();
    if name.is_empty() {
      return Err(Status::invalid_argument("Croft name cannot be empty"));
    }

    match self.clerk.status(name, req.live).await {
      Ok(Some((croft, telemetry))) => Ok(Response::new(StatusRes {
        croft: Some(ProtoCroft::from(croft)),
        telemetry: telemetry.map(ProtoCroftTelemetry::from),
      })),
      Ok(None) => Err(Status::not_found(format!("Croft '{}' not found", name))),
      Err(e) => Err(Status::internal(e.to_string())),
    }
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
      resources: Some(ProtoCroftResources {
        cpu_total: 10_000_000_000,
        cpu_yardable: 9_000_000_000,
        cpu_reserved: 0,
        mem_total: 16_000_000_000,
        mem_yardable: 14_000_000_000,
        mem_reserved: 0,
      }),
    };
    let croft = Croft::try_from(valid_req).unwrap();
    assert_eq!(croft.id, 42);
    assert_eq!(croft.name, "worker-01");
    assert_eq!(croft.addr, "10.0.0.1:7440");
    assert_eq!(croft.roles, vec![CroftRole::Server, CroftRole::Worker]);
    assert_eq!(croft.tags, vec!["zone-a"]);
    assert_eq!(croft.resources.cpu_total, 10_000_000_000);

    let empty_name = JoinReq {
      name: "   ".to_string(),
      ..Default::default()
    };
    assert!(Croft::try_from(empty_name).is_err());

    let empty_addr = JoinReq {
      name: "test".to_string(),
      addr: "   ".to_string(),
      roles: vec!["server".to_string()],
      tags: vec![],
      id: 1,
      resources: None,
    };
    assert!(Croft::try_from(empty_addr).is_err());

    let empty_roles = JoinReq {
      name: "test".to_string(),
      addr: "127.0.0.1:7440".to_string(),
      roles: vec![],
      tags: vec![],
      id: 1,
      resources: None,
    };
    assert!(Croft::try_from(empty_roles).is_err());

    let invalid_role = JoinReq {
      name: "test".to_string(),
      addr: "127.0.0.1:7440".to_string(),
      roles: vec!["invalid_role".to_string()],
      tags: vec![],
      id: 1,
      resources: None,
    };
    assert!(Croft::try_from(invalid_role).is_err());
  }

  #[test]
  fn proto_croft_roundtrip() {
    let domain_croft = Croft {
      id: 99,
      name: "croft-99".to_string(),
      addr: "127.0.0.1:7440".to_string(),
      roles: vec![CroftRole::Server],
      tags: vec!["fast".to_string()],
      joined_at: "2026-09-05T12:00:00Z".to_string(),
      resources: CroftResources {
        cpu_total: 25_600_000_000,
        cpu_yardable: 23_040_000_000,
        cpu_reserved: 1_000_000_000,
        mem_total: 32_000_000_000,
        mem_yardable: 28_800_000_000,
        mem_reserved: 2_000_000_000,
      },
    };

    let proto = ProtoCroft::from(domain_croft.clone());
    assert_eq!(proto.id, 99);
    assert_eq!(proto.name, "croft-99");
    assert_eq!(proto.roles, vec!["server".to_string()]);
    assert_eq!(proto.joined_at, "2026-09-05T12:00:00Z");

    let restored = Croft::try_from(proto).unwrap();
    assert_eq!(restored, domain_croft);
  }

  #[tokio::test]
  async fn api_handler_list_status_and_join_rpcs() {
    use crate::store::barn::Node as BarnNode;
    use crate::store::SpreadNode;
    use std::collections::BTreeMap;

    let dir = tempfile::tempdir().unwrap();
    let config = crate::croft::Config {
      addr: "127.0.0.1:0".parse().unwrap(),
      data_dir: dir.path().to_path_buf(),
      ..Default::default()
    };
    let live_croft = std::sync::Arc::new(
      crate::croft::LiveCroft::spawn(&config).await.unwrap(),
    );
    let self_node = BarnNode::new(live_croft.id, live_croft.addr.clone());
    let mut members = BTreeMap::new();
    members.insert(self_node.id, self_node);
    live_croft.barn.raft().initialize(members).await.unwrap();

    let clerk = Clerk::spawn(live_croft).unwrap();
    let api = Api::new(clerk);
    let _server = api.clone().into_server();

    // List RPC on empty barn returns 0 crofts
    let list_res = api.list(Request::new(ListReq {})).await;
    assert!(list_res.is_ok());

    // Join a worker croft
    let join_res = api
      .join(Request::new(JoinReq {
        id: 100,
        name: "node-100".to_string(),
        addr: "127.0.0.1:7440".to_string(),
        roles: vec!["worker".to_string()],
        tags: vec!["zone-a".to_string()],
        resources: Some(ProtoCroftResources {
          cpu_total: 10_000_000_000,
          cpu_yardable: 9_000_000_000,
          cpu_reserved: 0,
          mem_total: 16_000_000_000,
          mem_yardable: 14_000_000_000,
          mem_reserved: 0,
        }),
      }))
      .await;
    assert!(join_res.is_ok());

    // Status query without live
    let status_res = api
      .status(Request::new(StatusReq {
        name: "node-100".to_string(),
        live: false,
      }))
      .await;
    assert!(status_res.is_ok());
    let status_res = status_res.unwrap().into_inner();
    assert_eq!(status_res.croft.unwrap().name, "node-100");
    assert!(status_res.telemetry.is_none());

    // Status query with live
    let status_live_res = api
      .status(Request::new(StatusReq {
        name: "node-100".to_string(),
        live: true,
      }))
      .await;
    assert!(status_live_res.is_ok());
    let status_live_res = status_live_res.unwrap().into_inner();
    assert!(status_live_res.telemetry.is_some());

    // Status query for nonexistent node returns NOT_FOUND
    let not_found = api
      .status(Request::new(StatusReq {
        name: "ghost".to_string(),
        live: false,
      }))
      .await;
    assert_eq!(not_found.unwrap_err().code(), tonic::Code::NotFound);
  }
}
