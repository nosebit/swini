---
id: 0027
goal: specs/00027-croft-resources/goal.md
author: @brunomacf
created: 2026-09-08
---

# [Plan] Croft Resource Accounting and Status Inspection

This plan implements host hardware telemetry probing, resource persistence in
the Barn, gRPC list and status endpoints (`List` for cluster-wide queries and
`Status` for single-node queries with optional telemetry), and operator CLI
tools (`swini status` and `swini croft status <name> [--live]`).

The backend domain and wire protocols strictly operate on raw numbers (Hertz and
bytes) using consistent 3-letter naming (`cpu` and `mem`), while presentation
formatting is placed in a reusable `src/core/format.rs` utility consumed by the
CLI. Cluster status lives in `src/cli/status.rs` and scoped node commands live
in `src/cli/croft.rs`. See [goal.md](./goal.md) for the product requirements
this design satisfies.

## Architecture

```mermaid
graph TD
    subgraph Host["Host Machine (Croft)"]
        SYS[sysinfo Prober] -->|cpu_total, mem_total (Hz/Bytes)| LC[LiveCroft]
        LC -->|CroftResources| BN[(Barn Consensus\ncroft/id)]
        GATE[Gate gRPC] --> CLERK_API[CroftClerk Api (api.rs)]
        CLERK_API -->|delegate| CLERK[CroftClerk (mod.rs)]
        CLERK -->|Read croft/id| BN
        CLERK -->|Sample Telemetry (if live)| SYS
    end

    subgraph CLI["Operator CLI (Frontend)"]
        FMT[core::format\nformat_hertz, format_bytes]
        STATUS["src/cli/status.rs\nswini status"] -->|CroftApi::List| CLERK_API
        STATUS --> FMT
        CROFT["src/cli/croft.rs\nswini croft status"] -->|CroftApi::Status| CLERK_API
        CROFT --> FMT
    end
```

### Module Layout:

- **`src/core/format.rs` [NEW]**: Universal formatting utilities
  (`format_hertz`, `format_bytes`) supporting arbitrary scale units (`B` to
  `PiB`, `Hz` to `PHz`).
- **`src/core/mod.rs` [MODIFY]**: Exports `pub mod format;`.
- **`src/croft/resources.rs` [NEW]**: Raw resource data model (`CroftResources`,
  `CroftTelemetry`), capacity calculations, and hardware resource prober via
  `sysinfo`.
- **`src/croft/types.rs` [MODIFY]**: Embeds `pub resources: CroftResources`
  inside `Croft`.
- **`src/croft/mod.rs` [MODIFY]**: Probes initial host resources via
  `CroftResources::probe()` during `LiveCroft::spawn` and populates
  `base.resources`.
- **`proto/croft.proto` [MODIFY]**: Replaces global Status with `List`
  (`ListReq`/`ListRes`), updates `Status` to require `name` and optional `live`
  (`StatusReq`/`StatusRes`), and defines `CroftResources` and `CroftTelemetry`.
- **`src/croft/clerk/` [MODIFY]**: Implements domain functions `list` and
  `status` on `Clerk` in `src/croft/clerk/mod.rs`, and gRPC wrapper delegation
  in `api.rs`.
- **`src/cli/status.rs` [NEW]**: Implements root cluster summary command
  (`swini status`).
- **`src/cli/croft.rs` [NEW]**: Implements scoped Croft subcommands
  (`swini croft status <name> [--live]`).
- **`src/cli/mod.rs` [MODIFY]**: Registers top-level `swini status` and
  `swini croft` subcommand routing and dispatch.

## Implementation Details

### 1. Formatting Utilities (`src/core/format.rs`)

Houses reusable unit formatting functions for frontend presentation across CLI
commands.

```rust
//! Reusable human-readable formatting utilities for metrics, frequencies, and byte sizes.

/// Formats a frequency in Hertz into a human-readable unit string (Hz, kHz, MHz, GHz, THz, PHz).
pub fn format_hertz(hz: u64) -> String {
  const UNITS: &[&str] = &["Hz", "kHz", "MHz", "GHz", "THz", "PHz"];
  let mut val = hz as f64;
  let mut idx = 0;
  while val >= 1000.0 && idx + 1 < UNITS.len() {
    val /= 1000.0;
    idx += 1;
  }
  if idx == 0 {
    format!("{:.0} {}", val, UNITS[idx])
  } else {
    format!("{:.2} {}", val, UNITS[idx])
  }
}

/// Formats a byte count into a human-readable binary unit string (B, KiB, MiB, GiB, TiB, PiB).
pub fn format_bytes(bytes: u64) -> String {
  const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
  let mut val = bytes as f64;
  let mut idx = 0;
  while val >= 1024.0 && idx + 1 < UNITS.len() {
    val /= 1024.0;
    idx += 1;
  }
  if idx == 0 {
    format!("{:.0} {}", val, UNITS[idx])
  } else {
    format!("{:.2} {}", val, UNITS[idx])
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn format_hertz_scales_properly() {
    assert_eq!(format_hertz(500), "500 Hz");
    assert_eq!(format_hertz(2_400_000), "2.40 MHz");
    assert_eq!(format_hertz(3_200_000_000), "3.20 GHz");
    assert_eq!(format_hertz(16_000_000_000), "16.00 GHz");
  }

  #[test]
  fn format_bytes_scales_properly() {
    assert_eq!(format_bytes(512), "512 B");
    assert_eq!(format_bytes(1024 * 1024 * 4), "4.00 MiB");
    assert_eq!(format_bytes(1024 * 1024 * 1024 * 16), "16.00 GiB");
    assert_eq!(format_bytes(1024 * 1024 * 1024 * 1024 * 2), "2.00 TiB");
  }
}
```

### 2. `CroftResources` and Telemetry Probing (`src/croft/resources.rs`)

Strictly backend data structures and hardware telemetry probing.

```rust
//! Resource tracking, capacity calculations, and hardware telemetry probing for Crofts.

use serde::{Deserialize, Serialize};
use sysinfo::System;

/// Accounting of hardware capacity and active allocations for a Croft in raw units (Hz / Bytes).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CroftResources {
  /// Total physical CPU capacity in Hertz (e.g. 8 cores * 3.2 GHz = 25_600_000_000 Hz).
  pub cpu_total: u64,
  /// CPU capacity available for scheduling workloads (Hertz).
  pub cpu_yardable: u64,
  /// CPU capacity committed to active workloads on this Croft (Hertz, initialized to 0).
  pub cpu_reserved: u64,
  /// Total physical RAM in bytes.
  pub mem_total: u64,
  /// RAM capacity available for scheduling workloads in bytes.
  pub mem_yardable: u64,
  /// RAM committed to active workloads on this Croft (bytes, initialized to 0).
  pub mem_reserved: u64,
}

impl CroftResources {
  /// Returns remaining schedulable CPU capacity in Hertz.
  pub fn cpu_available(&self) -> u64 {
    self.cpu_yardable.saturating_sub(self.cpu_reserved)
  }

  /// Returns remaining schedulable RAM capacity in bytes.
  pub fn mem_available(&self) -> u64 {
    self.mem_yardable.saturating_sub(self.mem_reserved)
  }

  /// Probes local host hardware capacity and returns baseline `CroftResources`.
  pub fn probe() -> Self {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpus = sys.cpus();
    let cpu_count = cpus.len() as u64;
    let avg_freq_mhz = if !cpus.is_empty() {
      cpus.iter().map(|c| c.frequency()).sum::<u64>() / cpu_count
    } else {
      2500
    };
    let cpu_total = cpu_count * avg_freq_mhz * 1_000_000;
    // Default yardable: 90% of total capacity (leaving 10% for OS overhead)
    let cpu_yardable = (cpu_total as f64 * 0.90) as u64;

    let mem_total = sys.total_memory();
    let mem_yardable = (mem_total as f64 * 0.90) as u64;

    Self {
      cpu_total,
      cpu_yardable,
      cpu_reserved: 0,
      mem_total,
      mem_yardable,
      mem_reserved: 0,
    }
  }
}

/// Instantaneous live host telemetry snapshot.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CroftTelemetry {
  pub mem_used: u64,
  pub cpu_used: f32,
}

impl CroftTelemetry {
  /// Samples instantaneous live CPU and RAM consumption.
  pub fn sample() -> Self {
    let mut sys = System::new_all();
    sys.refresh_all();
    let cpu_used = sys.global_cpu_usage();
    let mem_used = sys.used_memory();
    Self {
      mem_used,
      cpu_used,
    }
  }
}
```

### 3. Protobuf Wire Schema (`proto/croft.proto`)

```protobuf
syntax = "proto3";

package swini.croft;

service CroftApi {
  rpc Join(JoinReq) returns (JoinRes);
  rpc List(ListReq) returns (ListRes);
  rpc Status(StatusReq) returns (StatusRes);
}

message CroftResources {
  uint64 cpu_total = 1;
  uint64 cpu_yardable = 2;
  uint64 cpu_reserved = 3;
  uint64 mem_total = 4;
  uint64 mem_yardable = 5;
  uint64 mem_reserved = 6;
}

message ListReq {}

message ListRes {
  repeated Croft crofts = 1;
  uint64 primary_id = 2;
}

message StatusReq {
  string name = 1;
  bool live = 2;
}

message CroftTelemetry {
  uint64 mem_used = 1;
  float cpu_used = 2;
}

message StatusRes {
  Croft croft = 1;
  CroftTelemetry telemetry = 2;
}

message Croft {
  uint64 id = 1;
  string name = 2;
  string addr = 3;
  repeated string roles = 4;
  repeated string tags = 5;
  string joined_at = 6;
  bool is_primary = 7;
  CroftResources resources = 8;
}

message JoinReq {
  uint64 id = 1;
  string name = 2;
  string addr = 3;
  repeated string roles = 4;
  repeated string tags = 5;
  CroftResources resources = 6;
}

message JoinRes {
  repeated Croft server_crofts = 1;
}
```

### 4. CroftClerk Implementation (`src/croft/clerk/`)

`Clerk` in `src/croft/clerk/mod.rs` exposes domain functions `list` and
`status`. `Api` in `src/croft/clerk/api.rs` wraps `Clerk` and handles Protobuf
serialization/deserialization.

```rust
// In src/croft/clerk/mod.rs
impl Clerk {
  /// Returns all registered Crofts in the Ranch.
  pub async fn list(&self) -> Result<Vec<Croft>, Box<dyn Error>> {
    let mut crofts = Vec::new();
    let prefix = CROFT_PREFIX.to_string();
    let pairs = self.croft.barn.list(Some(&prefix)).await?;
    for (_key, bytes) in pairs {
      if let Ok(croft) = serde_json::from_slice::<Croft>(&bytes) {
        crofts.push(croft);
      }
    }
    Ok(crofts)
  }

  /// Queries the domain status of a specific Croft, optionally sampling live telemetry.
  pub async fn status(
    &self,
    name: &str,
    live: bool,
  ) -> Result<Option<(Croft, Option<CroftTelemetry>)>, Box<dyn Error>> {
    let all = self.list().await?;
    let found = all.into_iter().find(|c| c.name == name);
    match found {
      Some(croft) => {
        let telemetry = if live {
          Some(CroftTelemetry::sample())
        } else {
          None
        };
        Ok(Some((croft, telemetry)))
      }
      None => Ok(None),
    }
  }
}

// In src/croft/clerk/api.rs
#[tonic::async_trait]
impl CroftApi for Api {
  async fn list(&self, _request: Request<ListReq>) -> Result<Response<ListRes>, Status> {
    let crofts = self.clerk.list().await.map_err(|e| Status::internal(e.to_string()))?;
    let proto_crofts = crofts.into_iter().map(ProtoCroft::from).collect();
    Ok(Response::new(ListRes {
      crofts: proto_crofts,
      primary_id: 0,
    }))
  }

  async fn status(&self, request: Request<StatusReq>) -> Result<Response<StatusRes>, Status> {
    let req = request.into_inner();
    let name = req.name.trim();
    if name.is_empty() {
      return Err(Status::invalid_argument("Croft name cannot be empty"));
    }

    match self.clerk.status(name, req.live).await {
      Ok(Some((croft, telemetry))) => Ok(Response::new(StatusRes {
        croft: Some(ProtoCroft::from(croft)),
        telemetry: telemetry.map(ProtoCroftTelemetry::from),
      })),
      Ok(None) => Err(Status::not_found(format!("Croft '{}' not found", name))),
      Err(e) => Err(Status::internal(e.to_string())),
    }
  }
}
```

### 5. CLI Commands (`src/cli/status.rs`, `src/cli/croft.rs`, `src/cli/mod.rs`)

`src/cli/status.rs` implements cluster status, and `src/cli/croft.rs` implements
scoped Croft inspection.

```rust
// src/cli/status.rs
use crate::core::format::{format_bytes, format_hertz};
use crate::core::proto::croft::croft_api_client::CroftApiClient;
use crate::core::proto::croft::ListReq;
use std::error::Error;

/// Displays cluster-wide summary table of all Crofts across the Ranch.
pub async fn run() -> Result<(), Box<dyn Error>> {
  let endpoint = crate::cli::resolve_api_url(None)?;
  let mut client = CroftApiClient::connect(endpoint).await?;
  let res = client.list(ListReq {}).await?.into_inner();

  println!(
    "{:<10} {:<15} {:<12} {:<20} {:<24} {:<24} {:<8}",
    "ID", "NAME", "ROLES", "ADDR", "CPU (RES/YARD/TOT)", "MEM (RES/YARD/TOT)", "STATUS"
  );
  println!("{:-<120}", "");

  for croft in res.crofts {
    let cpu_summary = format!(
      "{} / {} / {}",
      format_hertz(croft.resources.as_ref().map(|r| r.cpu_reserved).unwrap_or(0)),
      format_hertz(croft.resources.as_ref().map(|r| r.cpu_yardable).unwrap_or(0)),
      format_hertz(croft.resources.as_ref().map(|r| r.cpu_total).unwrap_or(0))
    );
    let mem_summary = format!(
      "{} / {} / {}",
      format_bytes(croft.resources.as_ref().map(|r| r.mem_reserved).unwrap_or(0)),
      format_bytes(croft.resources.as_ref().map(|r| r.mem_yardable).unwrap_or(0)),
      format_bytes(croft.resources.as_ref().map(|r| r.mem_total).unwrap_or(0))
    );
    println!(
      "{:<10} {:<15} {:<12} {:<20} {:<24} {:<24} {:<8}",
      croft.id, croft.name, croft.roles.join(","), croft.addr, cpu_summary, mem_summary, "Active"
    );
  }
  Ok(())
}
```

```rust
// src/cli/croft.rs
use crate::core::format::{format_bytes, format_hertz};
use crate::core::proto::croft::croft_api_client::CroftApiClient;
use crate::core::proto::croft::StatusReq;
use clap::Subcommand;
use std::error::Error;

#[derive(Subcommand, Debug, PartialEq, Eq)]
pub enum Command {
  /// Inspects a specific Croft's metadata and resources
  Status {
    /// Target Croft name
    name: String,
    /// Include live host telemetry
    #[arg(short, long)]
    live: bool,
  },
}

pub async fn run(cmd: Command) -> Result<(), Box<dyn Error>> {
  match cmd {
    Command::Status { name, live } => status(name, live).await,
  }
}

async fn status(name: String, live: bool) -> Result<(), Box<dyn Error>> {
  let endpoint = crate::cli::resolve_api_url(Some(&name))?;
  let mut client = CroftApiClient::connect(endpoint).await?;

  let res = client.status(StatusReq { name: name.clone(), live }).await?.into_inner();
  let croft = res.croft.ok_or("Croft data not found")?;

  println!("Croft: {} (ID: {})", croft.name, croft.id);
  println!("Address:   {}", croft.addr);
  println!("Roles:     {}", croft.roles.join(", "));
  println!("Tags:      {}", if croft.tags.is_empty() { "-".to_string() } else { croft.tags.join(", ") });
  println!("Joined At: {}", croft.joined_at);

  if let Some(r) = &croft.resources {
    println!("\nRESOURCES (BARN)");
    println!("CPU Total:     {}", format_hertz(r.cpu_total));
    println!("CPU Yardable:  {}", format_hertz(r.cpu_yardable));
    println!("CPU Reserved:  {}", format_hertz(r.cpu_reserved));
    println!("CPU Available: {}", format_hertz(r.cpu_yardable.saturating_sub(r.cpu_reserved)));
    println!();
    println!("Memory Total:     {}", format_bytes(r.mem_total));
    println!("Memory Yardable:  {}", format_bytes(r.mem_yardable));
    println!("Memory Reserved:  {}", format_bytes(r.mem_reserved));
    println!("Memory Available: {}", format_bytes(r.mem_yardable.saturating_sub(r.mem_reserved)));
  }

  if let Some(telemetry) = res.telemetry {
    println!("\nLIVE TELEMETRY");
    println!("Live CPU Usage:    {:.1}%", telemetry.cpu_used);
    let total_mem = croft.resources.as_ref().map(|r| r.mem_total).unwrap_or(1);
    println!(
      "Live Memory Used:  {} / {} ({:.1}%)",
      format_bytes(telemetry.mem_used),
      format_bytes(total_mem),
      (telemetry.mem_used as f64 / total_mem as f64) * 100.0
    );
  }

  Ok(())
}
```

```rust
// src/cli/mod.rs
pub mod croft;
pub mod regent;
pub mod status;

use clap::{Parser, Subcommand};
use std::error::Error;

#[derive(Parser, Debug, PartialEq, Eq)]
#[command(author, version, about = "Swini Workload Orchestrator", long_about = None)]
pub struct Cli {
  #[command(subcommand)]
  pub command: Command,
}

#[derive(Subcommand, Debug, PartialEq, Eq)]
pub enum Command {
  /// Manages the local Swini Regent process
  Regent {
    #[command(subcommand)]
    command: regent::Command,
  },
  /// Manages or inspects Crofts on the Ranch
  Croft {
    #[command(subcommand)]
    command: croft::Command,
  },
  /// Displays cluster status summary across all Crofts
  Status,
}

async fn dispatch(cli: Cli) -> Result<(), Box<dyn Error>> {
  match cli.command {
    Command::Regent { command } => regent::run(command).await,
    Command::Croft { command } => croft::run(command).await,
    Command::Status => status::run().await,
  }
}
```

## Data Model / API Changes

- `sysinfo = "0.33"` added to `Cargo.toml`.
- New module `src/core/format.rs` with `format_hertz` and `format_bytes`.
- New struct `CroftResources` in `src/croft/resources.rs` using `mem_total`,
  `mem_yardable`, `mem_reserved`.
- New struct `CroftTelemetry` in `src/croft/resources.rs` using `mem_used`,
  `cpu_used`.
- `Croft` struct expanded with `pub resources: CroftResources`.
- Protobuf `CroftApi` updated with `rpc List` and `rpc Status` (taking
  `StatusReq { name, live }`, returning `CroftTelemetry`).
- CLI commands: `swini status` (in `src/cli/status.rs`) and
  `swini croft status <name> [--live]` (in `src/cli/croft.rs`).

## Testing Strategy

- **Unit tests**:
  - `src/core/format.rs`: verifying `format_hertz` and `format_bytes` outputs
    against known values across all scale tiers (`B` through `PiB`, `Hz` through
    `PHz`).
  - `src/croft/resources.rs`: probing host telemetry via
    `CroftResources::probe()` (verifying non-zero CPU frequency and memory
    capacity), and available capacity calculations (`cpu_available()`,
    `mem_available()`).
  - Protobuf conversions: testing round-trip serialization and deserialization
    of `Croft` containing `CroftResources`.
- **Integration / gRPC tests**:
  - Testing `Api::list` and `Api::status` handlers on a running `LiveCroft`.
  - Verifying `Join` properly persists and returns `CroftResources`.
- **E2E tests (`tests/cli_status.rs`)**:
  - Starting a test Regent/Croft and asserting `swini status` and
    `swini croft status <name>` outputs formatted table/details with exit
    code 0.
- **Post-Write Checklist**:
  - `cargo fmt`, `cargo clippy -- -D warnings`, `cargo build`,
    `cargo nextest run`.

## Risks & Open Questions

- **Sysinfo Cross-Platform Support**: `sysinfo` works seamlessly across macOS
  and Linux, which matches Swini's target operating systems.

## Out of Scope

- Workload placement algorithms (Drover/Pig/Piglet scheduling).
- Dynamic reservation mutation during Piglet execution.
