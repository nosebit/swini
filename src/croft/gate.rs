//! Croft Gate network server for gRPC service registration and listening.
//!
//! Exposes [`Gate`], which manages the underlying tonic [`Server`] router,
//! allowing components within the Croft to register gRPC endpoints and listen
//! on network sockets.

use std::error::Error;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tonic::transport::server::Router;
use tonic::transport::Server;

enum GateState {
  Builder(Server),
  Router(Router),
}

/// Network gateway and gRPC server host for the local Croft compound.
#[derive(Clone)]
pub struct Gate {
  state: Arc<Mutex<Option<GateState>>>,
}

impl std::fmt::Debug for Gate {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Gate").finish()
  }
}

impl Default for Gate {
  fn default() -> Self {
    Self::new()
  }
}

impl Gate {
  /// Instantiates a new [`Gate`] wrapping a fresh tonic server builder.
  pub fn new() -> Self {
    Self {
      state: Arc::new(Mutex::new(Some(GateState::Builder(Server::builder())))),
    }
  }

  /// Mounts a tonic gRPC service handler onto the internal router.
  pub fn add<S, B>(&self, svc: S)
  where
    S: tonic::server::NamedService
      + tonic::codegen::Service<
        tonic::codegen::http::Request<tonic::body::Body>,
        Response = tonic::codegen::http::Response<B>,
        Error = std::convert::Infallible,
      > + Clone
      + Send
      + Sync
      + 'static,
    S::Future: Send + 'static,
    B: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    B::Error: Into<tonic::codegen::StdError> + Send,
  {
    if let Ok(mut guard) = self.state.lock() {
      if let Some(state) = guard.take() {
        let next = match state {
          GateState::Builder(mut builder) => {
            GateState::Router(builder.add_service(svc))
          }
          GateState::Router(router) => {
            GateState::Router(router.add_service(svc))
          }
        };
        *guard = Some(next);
      }
    }
  }

  /// Starts listening on the specified socket address, serving all registered
  /// gRPC services.
  ///
  /// # Errors
  /// Returns an error if the server fails to bind or encountered network errors
  /// while serving.
  pub async fn listen(&self, addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    let state = self
      .state
      .lock()
      .map_err(|_| "failed to lock gate state")?
      .take();

    if let Some(GateState::Router(router)) = state {
      router.serve(addr).await?;
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::proto::croft::croft_api_server::CroftApiServer;
  use crate::croft::clerk::Api as CroftApiHandler;
  use crate::croft::clerk::Clerk;
  use crate::croft::Config;

  #[tokio::test]
  async fn gate_initializes_and_mounts_services() {
    let gate = Gate::default();
    assert!(gate.state.lock().unwrap().is_some());

    let dir = tempfile::tempdir().unwrap();
    let config = Config {
      addr: "127.0.0.1:0".parse().unwrap(),
      data_dir: dir.path().to_path_buf(),
      ..Default::default()
    };
    let croft =
      std::sync::Arc::new(crate::croft::Croft::spawn(&config).await.unwrap());
    let clerk = Clerk::new(croft.clone());
    let handler = CroftApiHandler::new(clerk);

    // Add first service (transition Builder -> Router)
    gate.add(CroftApiServer::new(handler));
    // Add second service (Router -> Router)
    if let Some(ref barn) = croft.barn {
      gate.add(barn.api());
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let bound_addr = listener.local_addr().unwrap();
    drop(listener);

    let gate_clone = gate.clone();
    let server = tokio::spawn(async move {
      let _ = gate_clone.listen(bound_addr).await;
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    server.abort();
  }
}
