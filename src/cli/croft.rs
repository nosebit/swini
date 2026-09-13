//! CLI subcommand handlers and argument structures for Swini Croft operations.
//!
//! Exposes [`Command`] for scoped Croft commands (`status`) and delegates
//! execution via [`run`].

use crate::core::format::{format_bytes, format_hertz};
use crate::core::proto::croft::croft_api_client::CroftApiClient;
use crate::core::proto::croft::{Croft as ProtoCroft, StatusReq};
use clap::Subcommand;
use std::error::Error;

/// Scoped subcommands for inspecting and managing Crofts on the Ranch.
#[derive(Subcommand, Debug, PartialEq, Eq)]
pub enum Command {
  /// Inspects a specific Croft's metadata, Barn resources, and live telemetry
  Status {
    /// Target Croft name
    name: String,
    /// Include instantaneous live host telemetry
    #[arg(short, long)]
    live: bool,
  },
}

/// Executes Croft subcommands by delegating to specific command handlers.
///
/// # Errors
/// Returns an error if subcommand execution fails.
pub async fn run(cmd: Command) -> Result<(), Box<dyn Error>> {
  match cmd {
    Command::Status { name, live } => status(name, live).await,
  }
}

/// Queries and displays status for a specific Croft.
///
/// # Errors
/// Returns an error if endpoint resolution, gRPC connection, or query fails.
pub async fn status(name: String, live: bool) -> Result<(), Box<dyn Error>> {
  let endpoint = crate::cli::resolve_api_url(Some(&name))?;
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
    println!("Live CPU Usage:    {:.1}%", telemetry.cpu_used);
    let total_mem = croft.resources.as_ref().map(|r| r.mem_total).unwrap_or(1);
    let percent = if total_mem > 0 {
      (telemetry.mem_used as f64 / total_mem as f64) * 100.0
    } else {
      0.0
    };
    println!(
      "Live Memory Used:  {} / {} ({:.1}%)",
      format_bytes(telemetry.mem_used),
      format_bytes(total_mem),
      percent
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
    println!("CPU Total:     {}", format_hertz(r.cpu_total));
    println!("CPU Yardable:  {}", format_hertz(r.cpu_yardable));
    println!("CPU Reserved:  {}", format_hertz(r.cpu_reserved));
    println!(
      "CPU Available: {}",
      format_hertz(r.cpu_yardable.saturating_sub(r.cpu_reserved))
    );
    println!();
    println!("Memory Total:     {}", format_bytes(r.mem_total));
    println!("Memory Yardable:  {}", format_bytes(r.mem_yardable));
    println!("Memory Reserved:  {}", format_bytes(r.mem_reserved));
    println!(
      "Memory Available: {}",
      format_bytes(r.mem_yardable.saturating_sub(r.mem_reserved))
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
        cpu_total: 16_000_000_000,
        cpu_yardable: 14_400_000_000,
        cpu_reserved: 2_000_000_000,
        mem_total: 32_000_000_000,
        mem_yardable: 28_800_000_000,
        mem_reserved: 4_000_000_000,
      }),
    };

    display_croft_detail(&croft);
  }
}
