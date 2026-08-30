mod barn;

pub use barn::BarnApiHandler;

use crate::core::proto::barn::barn::barn_api_server::BarnApiServer;
use crate::store::barn::Barn;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;

/// A gRPC server exposing swini's stores to other nodes. Deliberately
/// thin: one field per registered service handler, all added to the same
/// `tonic::transport::Server` in `listen`. `BarnApiHandler` is the first
/// (and for now only) registered service — adding a second one later is
/// one more field on `Api`, one more `Self::new` parameter, and one more
/// `.add_service(...)` call.
#[derive(Clone)]
pub struct Api {
  barn_handler: BarnApiHandler,
}

impl Api {
  pub fn new(barn: Arc<Barn>) -> Self {
    Self {
      barn_handler: BarnApiHandler::new(barn),
    }
  }

  pub async fn listen(&self, addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    tonic::transport::Server::builder()
      .add_service(BarnApiServer::new(self.barn_handler.clone()))
      .serve(addr)
      .await?;
    Ok(())
  }
}
