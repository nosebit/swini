//! Tonic gRPC service handler and protobuf conversions for `PigApi`.
//!
//! Implements [`PigApi`] to handle incoming workload run and stop requests over
//! gRPC channels, dispatching them to [`PigClerk`].

use crate::core::proto::pig::pig_api_server::{PigApi, PigApiServer};
use crate::core::proto::pig::{PigRunReq, PigRunRes, PigStopReq, PigStopRes};
use crate::pig::clerk::PigClerk;
use crate::pig::config::PigRunConfig;
use tonic::{Request, Response, Status};

/// Tonic gRPC service handler for the `PigApi` service.
#[derive(Clone)]
pub struct Api {
  clerk: PigClerk,
}

impl Api {
  /// Creates a new [`Api`] handler backed by the given [`PigClerk`].
  pub fn new(clerk: PigClerk) -> Self {
    Self { clerk }
  }

  /// Wraps this handler into a tonic [`PigApiServer`] ready for registration on
  /// Gate.
  pub fn into_server(self) -> PigApiServer<Self> {
    PigApiServer::new(self)
  }
}

#[tonic::async_trait]
impl PigApi for Api {
  /// Submits and accepts declarative workload definitions for scheduling.
  async fn run(
    &self,
    request: Request<PigRunReq>,
  ) -> Result<Response<PigRunRes>, Status> {
    let req = request.into_inner();
    let config = PigRunConfig::from_yaml(&req.config_yaml)
      .map_err(|e| Status::invalid_argument(e.to_string()))?;
    let pigs = self
      .clerk
      .pig_run(config)
      .await
      .map_err(|e| Status::internal(e.to_string()))?;

    Ok(Response::new(PigRunRes {
      status: "ok".to_string(),
      started_pigs: pigs.into_iter().map(|p| p.path).collect(),
    }))
  }

  /// Stops an active workload across the Ranch.
  async fn stop(
    &self,
    request: Request<PigStopReq>,
  ) -> Result<Response<PigStopRes>, Status> {
    let req = request.into_inner();
    self
      .clerk
      .pig_stop(&req.space, &req.name)
      .await
      .map_err(|e| Status::internal(e.to_string()))?;

    Ok(Response::new(PigStopRes {
      status: "ok".to_string(),
    }))
  }
}
