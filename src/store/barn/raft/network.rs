use crate::core::proto::barn::barn::barn_api_client::BarnApiClient;
use crate::core::proto::barn::barn::BarnMessage;
use openraft::error::{
  InstallSnapshotError, NetworkError, RPCError, RaftError,
};
use openraft::network::RPCOption;
use openraft::raft::{
  AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest,
  InstallSnapshotResponse, VoteRequest, VoteResponse,
};
use openraft::{RaftNetwork, RaftNetworkFactory, RaftTypeConfig};
use std::marker::PhantomData;

use super::Node;

/// Builds a [`NetworkConnection`] for talking to one other cluster member
/// over the `BarnApi` gRPC service (`proto/barn.proto`).
///
/// Generic over `C: RaftTypeConfig` like `LogStorage` — this module has no
/// idea `barn` exists. The one thing it does need concretely is `Node`
/// (for `node.api_addr`, to actually dial a peer), and `Node` is defined
/// right here in `raft`, not in `barn`, so pinning `C::Node = Node` via
/// the trait bound doesn't reach outside this module either.
pub struct NetworkFactory<C: RaftTypeConfig<Node = Node>> {
  _data: PhantomData<C>,
}

impl<C: RaftTypeConfig<Node = Node>> NetworkFactory<C> {
  pub fn new() -> Self {
    Self { _data: PhantomData }
  }
}

/// A gRPC client for one other cluster member, implementing the three RPCs
/// openraft's engine needs to replicate the log and run elections.
pub struct NetworkConnection<C: RaftTypeConfig<Node = Node>> {
  client: BarnApiClient<tonic::transport::Channel>,
  _data: PhantomData<C>,
}

impl<C: RaftTypeConfig<Node = Node>> NetworkConnection<C> {
  /// Serializes `req` the same way
  /// [`LogStorage`](crate::store::barn::raft::storage::LogStorage) and
  /// [`Storage`](crate::store::barn::storage::Storage) serialize
  /// everything they persist, and wraps it in the `BarnMessage { payload }`
  /// envelope every `BarnApi` raft RPC uses — see `proto/barn.proto` for
  /// why it's an opaque envelope rather than hand-mirrored fields.
  fn encode<Req, E>(
    req: &Req,
  ) -> Result<BarnMessage, RPCError<C::NodeId, Node, E>>
  where
    Req: serde::Serialize,
    E: std::error::Error,
  {
    let payload = serde_json::to_vec(req)
      .map_err(|err| RPCError::Network(NetworkError::new(&err)))?;

    Ok(BarnMessage { payload })
  }

  /// The other half of [`Self::encode`]: unwraps a `BarnMessage` response
  /// and deserializes its payload back into whatever response type the
  /// caller expects.
  fn decode<Resp, E>(
    response: tonic::Response<BarnMessage>,
  ) -> Result<Resp, RPCError<C::NodeId, Node, E>>
  where
    Resp: serde::de::DeserializeOwned,
    E: std::error::Error,
  {
    let message = response.into_inner();

    serde_json::from_slice(&message.payload)
      .map_err(|err| RPCError::Network(NetworkError::new(&err)))
  }
}

/// See documentation in
/// https://docs.rs/openraft/0.9.25/openraft/network/trait.RaftNetworkFactory.html
impl<C: RaftTypeConfig<Node = Node>> RaftNetworkFactory<C>
  for NetworkFactory<C>
{
  type Network = NetworkConnection<C>;

  /// Builds a client for `node`, without actually connecting yet.
  ///
  /// Per the trait's own contract, this must not fail and must not block
  /// on establishing a real connection — the target might be unreachable
  /// right now, or its address might be permanently misconfigured, and
  /// openraft has no way to act on an error returned from here regardless.
  /// `connect_lazy` gives exactly that: it builds a `Channel` that only
  /// resolves the endpoint and attempts the connection when the first RPC
  /// actually goes out (and transparently reconnects later if it drops),
  /// so any real connectivity failure surfaces where openraft can actually
  /// do something about it — as an `Err` from `append_entries`/`vote`/
  /// `install_snapshot` below.
  async fn new_client(
    &mut self,
    _target: C::NodeId,
    node: &Node,
  ) -> NetworkConnection<C> {
    let target_addr = format!("http://{}", node.api_addr);
    let channel = tonic::transport::Endpoint::from_shared(target_addr)
      .expect("node api_addr must be a valid URI")
      .connect_lazy();

    NetworkConnection {
      client: BarnApiClient::new(channel),
      _data: PhantomData,
    }
  }
}

/// See documentation in
/// https://docs.rs/openraft/0.9.25/openraft/network/trait.RaftNetwork.html
impl<C: RaftTypeConfig<Node = Node>> RaftNetwork<C> for NetworkConnection<C> {
  async fn append_entries(
    &mut self,
    req: AppendEntriesRequest<C>,
    _option: RPCOption,
  ) -> Result<
    AppendEntriesResponse<C::NodeId>,
    RPCError<C::NodeId, Node, RaftError<C::NodeId>>,
  > {
    let message = Self::encode(&req)?;
    let response = self
      .client
      .append_entries(message)
      .await
      .map_err(|status| RPCError::Network(NetworkError::new(&status)))?;

    Self::decode(response)
  }

  async fn vote(
    &mut self,
    req: VoteRequest<C::NodeId>,
    _option: RPCOption,
  ) -> Result<
    VoteResponse<C::NodeId>,
    RPCError<C::NodeId, Node, RaftError<C::NodeId>>,
  > {
    let message = Self::encode(&req)?;
    let response = self
      .client
      .vote(message)
      .await
      .map_err(|status| RPCError::Network(NetworkError::new(&status)))?;

    Self::decode(response)
  }

  async fn install_snapshot(
    &mut self,
    req: InstallSnapshotRequest<C>,
    _option: RPCOption,
  ) -> Result<
    InstallSnapshotResponse<C::NodeId>,
    RPCError<C::NodeId, Node, RaftError<C::NodeId, InstallSnapshotError>>,
  > {
    let message = Self::encode(&req)?;
    let response = self
      .client
      .install_snapshot(message)
      .await
      .map_err(|status| RPCError::Network(NetworkError::new(&status)))?;

    Self::decode(response)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::proto::barn::barn::barn_api_server::{
    BarnApi, BarnApiServer,
  };
  use crate::core::proto::barn::barn::{NodeAddReq, NodeAddRes};
  use openraft::{SnapshotMeta, StoredMembership, Vote};
  use std::convert::Infallible;
  use std::io::Cursor;
  use std::time::Duration;
  use tokio::net::TcpListener;
  use tokio_stream::wrappers::TcpListenerStream;

  openraft::declare_raft_types!(
    pub TestTypeConfig:
      D = (),
      R = (),
      Node = Node,
  );

  /// A `BarnApi` implementation that either echoes back a canned success
  /// response or fails every raft RPC with `status`, so tests can drive
  /// `NetworkConnection` against a real gRPC transport without needing the
  /// real (not-yet-implemented) `BarnApi` server.
  struct MockBarnApi {
    status: Option<tonic::Status>,
  }

  #[tonic::async_trait]
  impl BarnApi for MockBarnApi {
    async fn append_entries(
      &self,
      _request: tonic::Request<BarnMessage>,
    ) -> Result<tonic::Response<BarnMessage>, tonic::Status> {
      if let Some(status) = &self.status {
        return Err(status.clone());
      }
      let payload = serde_json::to_vec(&AppendEntriesResponse::<u64>::Success)
        .expect("encode response");
      Ok(tonic::Response::new(BarnMessage { payload }))
    }

    async fn vote(
      &self,
      _request: tonic::Request<BarnMessage>,
    ) -> Result<tonic::Response<BarnMessage>, tonic::Status> {
      if let Some(status) = &self.status {
        return Err(status.clone());
      }
      let response = VoteResponse {
        vote: Vote::new(1, 1u64),
        vote_granted: true,
        last_log_id: None,
      };
      let payload = serde_json::to_vec(&response).expect("encode response");
      Ok(tonic::Response::new(BarnMessage { payload }))
    }

    async fn install_snapshot(
      &self,
      _request: tonic::Request<BarnMessage>,
    ) -> Result<tonic::Response<BarnMessage>, tonic::Status> {
      if let Some(status) = &self.status {
        return Err(status.clone());
      }
      let response = InstallSnapshotResponse {
        vote: Vote::new(1, 1u64),
      };
      let payload = serde_json::to_vec(&response).expect("encode response");
      Ok(tonic::Response::new(BarnMessage { payload }))
    }

    async fn forward_write(
      &self,
      _request: tonic::Request<BarnMessage>,
    ) -> Result<tonic::Response<BarnMessage>, tonic::Status> {
      Err(tonic::Status::unimplemented("not exercised by this test"))
    }

    async fn forward_read(
      &self,
      _request: tonic::Request<BarnMessage>,
    ) -> Result<tonic::Response<BarnMessage>, tonic::Status> {
      Err(tonic::Status::unimplemented("not exercised by this test"))
    }

    async fn node_add(
      &self,
      _request: tonic::Request<NodeAddReq>,
    ) -> Result<tonic::Response<NodeAddRes>, tonic::Status> {
      Err(tonic::Status::unimplemented("not exercised by this test"))
    }
  }

  /// Starts `mock` on an OS-assigned local port and returns the `Node`
  /// clients should dial to reach it, plus the server task's handle so
  /// the caller can abort it once the test is done.
  async fn spawn_mock_server(
    mock: MockBarnApi,
  ) -> (Node, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local addr");

    let handle = tokio::spawn(async move {
      tonic::transport::Server::builder()
        .add_service(BarnApiServer::new(mock))
        .serve_with_incoming(TcpListenerStream::new(listener))
        .await
        .expect("serve mock BarnApi");
    });

    let node = Node {
      api_addr: addr.to_string(),
      ..Default::default()
    };
    (node, handle)
  }

  fn append_entries_request() -> AppendEntriesRequest<TestTypeConfig> {
    AppendEntriesRequest {
      vote: Vote::new(1, 1u64),
      prev_log_id: None,
      entries: Vec::new(),
      leader_commit: None,
    }
  }

  fn install_snapshot_request() -> InstallSnapshotRequest<TestTypeConfig> {
    InstallSnapshotRequest {
      vote: Vote::new(1, 1u64),
      meta: SnapshotMeta {
        last_log_id: None,
        last_membership: StoredMembership::default(),
        snapshot_id: "test".to_string(),
      },
      offset: 0,
      data: Vec::new(),
      done: true,
    }
  }

  #[tokio::test]
  async fn network_round_trips_all_raft_rpcs_through_a_real_server() {
    let (node, server) = spawn_mock_server(MockBarnApi { status: None }).await;
    let mut factory = NetworkFactory::<TestTypeConfig>::new();
    let mut conn = factory.new_client(1, &node).await;
    let option = RPCOption::new(Duration::from_secs(5));

    let append_response = conn
      .append_entries(append_entries_request(), option.clone())
      .await
      .expect("append_entries");
    assert!(matches!(append_response, AppendEntriesResponse::Success));

    let vote_response = conn
      .vote(VoteRequest::new(Vote::new(1, 1u64), None), option.clone())
      .await
      .expect("vote");
    assert!(vote_response.vote_granted);

    let snapshot_response = conn
      .install_snapshot(install_snapshot_request(), option)
      .await
      .expect("install_snapshot");
    assert_eq!(snapshot_response.vote, Vote::new(1, 1u64));

    server.abort();
    let _ = server.await;
  }

  #[tokio::test]
  async fn network_maps_grpc_error_status_to_rpc_network_error() {
    let (node, server) = spawn_mock_server(MockBarnApi {
      status: Some(tonic::Status::unavailable("mock failure")),
    })
    .await;
    let mut factory = NetworkFactory::<TestTypeConfig>::new();
    let mut conn = factory.new_client(1, &node).await;
    let option = RPCOption::new(Duration::from_secs(5));

    let append_result = conn
      .append_entries(append_entries_request(), option.clone())
      .await;
    assert!(matches!(append_result, Err(RPCError::Network(_))));

    let vote_result = conn
      .vote(VoteRequest::new(Vote::new(1, 1u64), None), option.clone())
      .await;
    assert!(matches!(vote_result, Err(RPCError::Network(_))));

    let snapshot_result = conn
      .install_snapshot(install_snapshot_request(), option)
      .await;
    assert!(matches!(snapshot_result, Err(RPCError::Network(_))));

    server.abort();
    let _ = server.await;
  }

  #[test]
  fn encode_then_decode_round_trips_a_value() {
    let vote = Vote::new(1, 1u64);

    let message =
      NetworkConnection::<TestTypeConfig>::encode::<_, Infallible>(&vote)
        .expect("encode");
    let decoded = NetworkConnection::<TestTypeConfig>::decode::<
      Vote<u64>,
      Infallible,
    >(tonic::Response::new(message))
    .expect("decode");

    assert_eq!(decoded, vote);
  }

  #[test]
  fn decode_surfaces_malformed_payload_as_network_error() {
    let message = BarnMessage {
      payload: b"not valid json".to_vec(),
    };

    let result: Result<Vote<u64>, RPCError<u64, Node, Infallible>> =
      NetworkConnection::<TestTypeConfig>::decode(tonic::Response::new(
        message,
      ));

    assert!(matches!(result, Err(RPCError::Network(_))));
  }
}
