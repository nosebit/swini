mod barn;
mod cluster;

pub use barn::BarnApiHandler;
pub use cluster::ClusterApiHandler;

use crate::core::proto::barn::barn::barn_api_server::BarnApiServer;
use crate::core::proto::cluster::cluster::cluster_api_server::ClusterApiServer;
use crate::store::barn::Barn;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;

/// A gRPC server exposing swini's stores and cluster API to other nodes.
#[derive(Clone)]
pub struct Api {
  barn_handler: BarnApiHandler,
  cluster_handler: ClusterApiHandler,
}

impl Api {
  pub fn new(barn: Arc<Barn>) -> Self {
    Self {
      barn_handler: BarnApiHandler::new(barn.clone()),
      cluster_handler: ClusterApiHandler::new(barn),
    }
  }

  pub async fn listen(&self, addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    tonic::transport::Server::builder()
      .add_service(BarnApiServer::new(self.barn_handler.clone()))
      .add_service(ClusterApiServer::new(self.cluster_handler.clone()))
      .serve(addr)
      .await?;
    Ok(())
  }
}
