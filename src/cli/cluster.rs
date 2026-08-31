use crate::core::proto::cluster::cluster::cluster_api_client::ClusterApiClient;
use crate::core::proto::cluster::cluster::StatusReq;
use std::error::Error;
use tonic::transport::Channel;

pub async fn status(
  mut client: ClusterApiClient<Channel>,
) -> Result<(), Box<dyn Error>> {
  let res = client.status(StatusReq {}).await?.into_inner();

  println!(
    "{:<8} {:<20} {:<15} {:<30} {:<10}",
    "ID", "NAME", "ROLE", "JOINED AT", "PRIMARY"
  );
  for node in res.nodes {
    let roles = node.roles.join(",");
    let is_primary = if node.is_primary { "yes" } else { "no" };
    println!(
      "{:<8} {:<20} {:<15} {:<30} {:<10}",
      node.id, node.name, roles, node.joined_at, is_primary
    );
  }
  Ok(())
}
