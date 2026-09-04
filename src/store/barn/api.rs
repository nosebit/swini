//! gRPC service implementation for the `BarnApi` consensus transport.
//!
//! Provides [`Api`], forwarding Raft consensus messages, forwarded
//! reads/writes, and membership change requests directly to the [`Barn`]
//! storage engine.

use crate::core::proto::barn::barn::barn_api_server::{BarnApi, BarnApiServer};
use crate::core::proto::barn::barn::{BarnMessage, NodeAddReq, NodeAddRes};
use crate::store::barn::{Action, Barn, Node, ReadAction};
use crate::store::{SpreadNode, SpreadStore};
use tonic::{Request, Response, Status};

/// Decodes a `BarnMessage`'s opaque JSON payload back into `T`.
fn decode<T: serde::de::DeserializeOwned>(
  message: BarnMessage,
) -> Result<T, Status> {
  serde_json::from_slice(&message.payload)
    .map_err(|err| Status::invalid_argument(err.to_string()))
}

/// Encodes `value` into a `BarnMessage` response.
fn encode<T: serde::Serialize>(
  value: &T,
) -> Result<Response<BarnMessage>, Status> {
  let payload = serde_json::to_vec(value)
    .map_err(|err| Status::internal(err.to_string()))?;
  Ok(Response::new(BarnMessage { payload }))
}

fn to_status<E: std::fmt::Display>(err: E) -> Status {
  Status::internal(err.to_string())
}

/// gRPC service handler for Barn Raft transport and node operations.
#[derive(Clone)]
pub struct Api {
  barn: Barn,
}

impl Api {
  /// Creates a new [`Api`] handler wrapping the provided [`Barn`] instance.
  pub fn new(barn: Barn) -> Self {
    Self { barn }
  }

  /// Converts this handler into a tonic [`BarnApiServer`].
  pub fn into_server(self) -> BarnApiServer<Self> {
    BarnApiServer::new(self)
  }
}

#[tonic::async_trait]
impl BarnApi for Api {
  async fn append_entries(
    &self,
    request: Request<BarnMessage>,
  ) -> Result<Response<BarnMessage>, Status> {
    let req = decode(request.into_inner())?;
    let response = self
      .barn
      .raft()
      .append_entries(req)
      .await
      .map_err(to_status)?;
    encode(&response)
  }

  async fn vote(
    &self,
    request: Request<BarnMessage>,
  ) -> Result<Response<BarnMessage>, Status> {
    let req = decode(request.into_inner())?;
    let response = self.barn.raft().vote(req).await.map_err(to_status)?;
    encode(&response)
  }

  async fn install_snapshot(
    &self,
    request: Request<BarnMessage>,
  ) -> Result<Response<BarnMessage>, Status> {
    let req = decode(request.into_inner())?;
    let response = self
      .barn
      .raft()
      .install_snapshot(req)
      .await
      .map_err(to_status)?;
    encode(&response)
  }

  async fn forward_write(
    &self,
    request: Request<BarnMessage>,
  ) -> Result<Response<BarnMessage>, Status> {
    let action: Action = decode(request.into_inner())?;
    encode(&self.barn.local_write(action).await)
  }

  async fn forward_read(
    &self,
    request: Request<BarnMessage>,
  ) -> Result<Response<BarnMessage>, Status> {
    let action: ReadAction = decode(request.into_inner())?;
    encode(&self.barn.local_read(action).await)
  }

  async fn node_add(
    &self,
    request: Request<NodeAddReq>,
  ) -> Result<Response<NodeAddRes>, Status> {
    let req = request.into_inner();
    let node = Node::new(req.id, req.api_addr);
    self.barn.node_add(node).await.map_err(to_status)?;
    Ok(Response::new(NodeAddRes {}))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use openraft::raft::VoteRequest;
  use openraft::Vote;

  #[test]
  fn encode_then_decode_round_trips_an_action() {
    let action = Action::Set {
      key: "foo".to_string(),
      value: b"bar".to_vec(),
    };

    let message = encode(&action).expect("encode").into_inner();
    let decoded: Action = decode(message).expect("decode");

    assert_eq!(decoded, action);
  }

  #[test]
  fn encode_then_decode_round_trips_a_raft_request() {
    let req = VoteRequest::new(Vote::new(1, 1u64), None);

    let message = encode(&req).expect("encode").into_inner();
    let decoded: VoteRequest<u64> = decode(message).expect("decode");

    assert_eq!(decoded, req);
  }

  #[test]
  fn decode_surfaces_malformed_payload_as_invalid_argument() {
    let message = BarnMessage {
      payload: b"not valid json".to_vec(),
    };

    let result: Result<Action, Status> = decode(message);
    assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
  }
}
