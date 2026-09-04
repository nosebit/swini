//! gRPC API handler and protobuf conversion helpers for the `PlotApi` service.
//!
//! Exposes [`Api`], which implements the tonic-generated [`PlotApi`]
//! trait, delegating join registrations and status inquiries to the underlying
//! [`Clerk`].

use crate::core::proto::plot::plot_api_server::{PlotApi, PlotApiServer};
use crate::core::proto::plot::{
  JoinReq, JoinRes, Plot as ProtoPlot, StatusReq, StatusRes,
};
use crate::plot::clerk::Clerk;
use crate::plot::{Plot, PlotRole};
use tonic::{Request, Response, Status};

/// Validates and converts an incoming [`JoinReq`] Protobuf message into a
/// domain [`Plot`].
///
/// # Errors
/// Returns an error string if `name` or `addr` are blank, or if any role string
/// is unrecognized.
pub fn plot_from_join_req(req: JoinReq) -> Result<Plot, String> {
  let name = req.name.trim();
  if name.is_empty() {
    return Err("Plot name cannot be empty".to_string());
  }

  let addr = req.addr.trim();
  if addr.is_empty() {
    return Err("Plot addr cannot be empty".to_string());
  }

  let roles: Result<Vec<PlotRole>, String> = req
    .roles
    .into_iter()
    .map(|r| r.parse::<PlotRole>())
    .collect();
  let roles = roles?;

  if roles.is_empty() {
    return Err("Plot must have at least one role".to_string());
  }

  Ok(Plot {
    id: req.id,
    name: name.to_string(),
    addr: addr.to_string(),
    roles,
    tags: req.tags,
    joined_at: String::new(),
  })
}

/// Converts an internal domain [`Plot`] entity into its Protobuf [`ProtoPlot`]
/// wire format.
pub fn proto_plot_from_plot(plot: Plot) -> ProtoPlot {
  ProtoPlot {
    id: plot.id,
    name: plot.name,
    addr: plot.addr,
    roles: plot.roles.into_iter().map(|r| r.to_string()).collect(),
    tags: plot.tags,
    joined_at: plot.joined_at,
    is_primary: false,
  }
}

/// gRPC service implementation for [`PlotApi`].
#[derive(Clone)]
pub struct Api {
  clerk: Clerk,
}

impl Api {
  /// Wraps a [`Clerk`] instance into a gRPC service handler.
  pub fn new(clerk: Clerk) -> Self {
    Self { clerk }
  }

  /// Converts this handler into a tonic [`PlotApiServer`].
  pub fn into_server(self) -> PlotApiServer<Self> {
    PlotApiServer::new(self)
  }
}

#[tonic::async_trait]
impl PlotApi for Api {
  async fn join(
    &self,
    request: Request<JoinReq>,
  ) -> Result<Response<JoinRes>, Status> {
    let incoming = plot_from_join_req(request.into_inner())
      .map_err(Status::invalid_argument)?;

    let server_plots = self
      .clerk
      .join(incoming)
      .await
      .map_err(|e| Status::internal(e.to_string()))?;

    let proto_servers =
      server_plots.into_iter().map(proto_plot_from_plot).collect();
    Ok(Response::new(JoinRes {
      server_plots: proto_servers,
    }))
  }

  async fn status(
    &self,
    _request: Request<StatusReq>,
  ) -> Result<Response<StatusRes>, Status> {
    let registered_plots = self
      .clerk
      .list()
      .await
      .map_err(|e| Status::internal(e.to_string()))?;

    let plots = registered_plots
      .into_iter()
      .map(proto_plot_from_plot)
      .collect();
    Ok(Response::new(StatusRes {
      plots,
      primary_id: 0,
    }))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn plot_from_join_req_validation() {
    let valid_req = JoinReq {
      id: 42,
      name: "worker-01".to_string(),
      addr: "10.0.0.1:7440".to_string(),
      roles: vec!["server".to_string(), "worker".to_string()],
      tags: vec!["zone-a".to_string()],
    };
    let plot = plot_from_join_req(valid_req).unwrap();
    assert_eq!(plot.id, 42);
    assert_eq!(plot.name, "worker-01");
    assert_eq!(plot.addr, "10.0.0.1:7440");
    assert_eq!(plot.roles, vec![PlotRole::Server, PlotRole::Worker]);
    assert_eq!(plot.tags, vec!["zone-a"]);

    let empty_name = JoinReq {
      name: "   ".to_string(),
      ..Default::default()
    };
    assert!(plot_from_join_req(empty_name).is_err());

    let empty_roles = JoinReq {
      name: "test".to_string(),
      addr: "127.0.0.1:7440".to_string(),
      roles: vec![],
      tags: vec![],
      id: 1,
    };
    assert!(plot_from_join_req(empty_roles).is_err());

    let invalid_role = JoinReq {
      name: "test".to_string(),
      addr: "127.0.0.1:7440".to_string(),
      roles: vec!["invalid_role".to_string()],
      tags: vec![],
      id: 1,
    };
    assert!(plot_from_join_req(invalid_role).is_err());
  }

  #[test]
  fn proto_plot_conversion() {
    let domain_plot = Plot {
      id: 99,
      name: "plot-99".to_string(),
      addr: "127.0.0.1:7440".to_string(),
      roles: vec![PlotRole::Server],
      tags: vec!["fast".to_string()],
      joined_at: "2026-09-03T12:00:00Z".to_string(),
    };

    let proto = proto_plot_from_plot(domain_plot.clone());
    assert_eq!(proto.id, 99);
    assert_eq!(proto.name, "plot-99");
    assert_eq!(proto.roles, vec!["server".to_string()]);
    assert_eq!(proto.joined_at, "2026-09-03T12:00:00Z");
  }
}
