//! CLI subcommand handlers and argument structures for Swini Croft operations.
//!
//! Exposes [`Command`] for scoped Croft commands (`status`) and delegates
//! execution via [`run`].

use crate::core::format::{format_cpu, format_memory};
use crate::core::proto::croft::croft_api_client::CroftApiClient;
use crate::core::proto::croft::{Croft as ProtoCroft, StatusReq};
use clap::Subcommand;
use std::error::Error;

/// Scoped subcommands under `swini croft`.
#[derive(Subcommand, Debug, PartialEq, Eq)]
pub enum Command {
  /// Displays detailed information and resource allocations for a Croft
  Status {
    /// Name of the Croft to inspect
    name: String,
    /// Samples live telemetry (CPU/Memory) directly from the host
    #[arg(long)]
    live: bool,
  },
}

/// Dispatches `swini croft` commands.
///
/// # Errors
/// Returns an error if the subcommand execution fails.
pub async fn run(
  cmd: Command,
  endpoint: tonic::transport::Endpoint,
) -> Result<(), Box<dyn Error>> {
  match cmd {
    Command::Status { name, live } => status(name, live, endpoint).await,
  }
}

/// Queries and displays status for a specific Croft.
///
/// # Errors
/// Returns an error if gRPC connection or query fails.
pub async fn status(
  name: String,
  live: bool,
  endpoint: tonic::transport::Endpoint,
) -> Result<(), Box<dyn Error>> {
  let mut client = CroftApiClient::connect(endpoint).await?;

  let res = client
    .status(StatusReq {
      name: name.clone(),
      live,
    })
    .await?
    .into_inner();

  let croft = res.croft.ok_or("Croft data not returned by server")?;
  display_croft_detail(&croft);

  if let Some(telemetry) = res.telemetry {
    println!();
    println!("LIVE TELEMETRY");
    let total_cpu =
      croft.resources.as_ref().map(|r| r.cpu_total).unwrap_or(0.0);
    let cpu_percent = if total_cpu > 0.0 {
      (telemetry.cpu_used / total_cpu) * 100.0
    } else {
      0.0
    };
    println!(
      "Live CPU Used:     {} / {} ({:.1}%)",
      format_cpu(telemetry.cpu_used),
      format_cpu(total_cpu),
      cpu_percent
    );
    let total_mem =
      croft.resources.as_ref().map(|r| r.mem_total).unwrap_or(0.0);
    let mem_percent = if total_mem > 0.0 {
      (telemetry.mem_used / total_mem) * 100.0
    } else {
      0.0
    };
    println!(
      "Live Memory Used:  {} / {} ({:.1}%)",
      format_memory(telemetry.mem_used),
      format_memory(total_mem),
      mem_percent
    );
  }

  Ok(())
}

/// Prints formatted Croft metadata and Barn resource allocations to stdout.
pub fn display_croft_detail(croft: &ProtoCroft) {
  println!("Croft: {} (ID: {})", croft.name, croft.id);
  println!("Address:   {}", croft.addr);
  println!("Roles:     {}", croft.roles.join(", "));
  println!(
    "Tags:      {}",
    if croft.tags.is_empty() {
      "-".to_string()
    } else {
      croft.tags.join(", ")
    }
  );
  println!("Joined At: {}", croft.joined_at);

  if let Some(r) = &croft.resources {
    println!();
    println!("RESOURCES (BARN)");
    println!("CPU Total:     {}", format_cpu(r.cpu_total));
    println!("CPU Yardable:  {}", format_cpu(r.cpu_yardable));
    println!("CPU Reserved:  {}", format_cpu(r.cpu_reserved));
    println!(
      "CPU Available: {}",
      format_cpu((r.cpu_yardable - r.cpu_reserved).max(0.0))
    );
    println!();
    println!("Memory Total:     {}", format_memory(r.mem_total));
    println!("Memory Yardable:  {}", format_memory(r.mem_yardable));
    println!("Memory Reserved:  {}", format_memory(r.mem_reserved));
    println!(
      "Memory Available: {}",
      format_memory((r.mem_yardable - r.mem_reserved).max(0.0))
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::proto::croft::CroftResources as ProtoCroftResources;
  use clap::Parser;

  #[derive(Parser, Debug)]
  struct TestCli {
    #[command(subcommand)]
    command: Command,
  }

  #[test]
  fn cli_parse_croft_status_command() {
    let parsed =
      TestCli::try_parse_from(["swini", "status", "worker-01"]).unwrap();
    assert_eq!(
      parsed.command,
      Command::Status {
        name: "worker-01".to_string(),
        live: false,
      }
    );

    let parsed_live =
      TestCli::try_parse_from(["swini", "status", "worker-01", "--live"])
        .unwrap();
    assert_eq!(
      parsed_live.command,
      Command::Status {
        name: "worker-01".to_string(),
        live: true,
      }
    );
  }

  #[test]
  fn display_croft_detail_renders() {
    let croft = ProtoCroft {
      id: 42,
      name: "node-42".to_string(),
      addr: "127.0.0.1:7440".to_string(),
      roles: vec!["server".to_string(), "worker".to_string()],
      tags: vec!["fast".to_string()],
      joined_at: "2026-09-08T12:00:00Z".to_string(),
      is_primary: false,
      resources: Some(ProtoCroftResources {
        cpu_total: 16000.0,
        cpu_yardable: 14400.0,
        cpu_reserved: 2000.0,
        mem_total: 32000.0,
        mem_yardable: 28800.0,
        mem_reserved: 4000.0,
      }),
    };

    display_croft_detail(&croft);
  }
}
