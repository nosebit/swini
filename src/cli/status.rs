//! Cluster-wide status query and summary table presentation.
//!
//! Exposes [`run`], which queries the cluster via [`CroftApi::List`] and
//! renders a formatted ASCII table summarizing all registered Crofts and their
//! aggregate CPU and memory allocations.

use crate::core::format::{format_cpu, format_memory};
use crate::core::proto::croft::croft_api_client::CroftApiClient;
use crate::core::proto::croft::{Croft as ProtoCroft, ListReq};
use std::error::Error;

/// Queries and displays a formatted summary table of all Crofts across the
/// Ranch.
///
/// # Errors
/// Returns an error if gRPC communication fails.
pub async fn run(
  endpoint: tonic::transport::Endpoint,
) -> Result<(), Box<dyn Error>> {
  let mut client = CroftApiClient::connect(endpoint).await?;
  let res = client.list(ListReq {}).await?.into_inner();

  render_cluster_table(&res.crofts);
  Ok(())
}

/// Renders a list of Crofts into a formatted table output on stdout.
pub fn render_cluster_table(crofts: &[ProtoCroft]) {
  if crofts.is_empty() {
    println!("No Crofts registered in the Ranch.");
    return;
  }

  println!(
    "{:<10} {:<15} {:<12} {:<22} {:<24} {:<24} {:<8}",
    "ID",
    "NAME",
    "ROLES",
    "ADDR",
    "CPU (RES/YARD/TOT)",
    "MEM (RES/YARD/TOT)",
    "STATUS"
  );
  println!("{:-<117}", "");

  for croft in crofts {
    let cpu_summary = format!(
      "{} / {} / {}",
      format_cpu(
        croft
          .resources
          .as_ref()
          .map(|r| r.cpu_reserved)
          .unwrap_or(0.0)
      ),
      format_cpu(
        croft
          .resources
          .as_ref()
          .map(|r| r.cpu_yardable)
          .unwrap_or(0.0)
      ),
      format_cpu(croft.resources.as_ref().map(|r| r.cpu_total).unwrap_or(0.0))
    );
    let mem_summary = format!(
      "{} / {} / {}",
      format_memory(
        croft
          .resources
          .as_ref()
          .map(|r| r.mem_reserved)
          .unwrap_or(0.0)
      ),
      format_memory(
        croft
          .resources
          .as_ref()
          .map(|r| r.mem_yardable)
          .unwrap_or(0.0)
      ),
      format_memory(
        croft.resources.as_ref().map(|r| r.mem_total).unwrap_or(0.0)
      )
    );
    println!(
      "{:<10} {:<15} {:<12} {:<22} {:<24} {:<24} {:<8}",
      croft.id,
      croft.name,
      croft.roles.join(","),
      croft.addr,
      cpu_summary,
      mem_summary,
      "Active"
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::proto::croft::CroftResources as ProtoCroftResources;

  #[test]
  fn render_cluster_table_empty_and_populated() {
    // Empty list check
    render_cluster_table(&[]);

    // Populated list check
    let crofts = vec![ProtoCroft {
      id: 100,
      name: "node-100".to_string(),
      addr: "127.0.0.1:7440".to_string(),
      roles: vec!["server".to_string()],
      tags: vec!["zone-a".to_string()],
      joined_at: "2026-09-08T15:00:00Z".to_string(),
      is_primary: false,
      resources: Some(ProtoCroftResources {
        cpu_total: 16000.0,
        cpu_yardable: 14400.0,
        cpu_reserved: 0.0,
        mem_total: 16000.0,
        mem_yardable: 14400.0,
        mem_reserved: 0.0,
      }),
    }];

    render_cluster_table(&crofts);
  }
}
