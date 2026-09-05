---
id: 0023
goal: specs/00023-croft-plot-unification/goal.md
author: @brunomacf
created: 2026-09-05
---

# [Plan] Unify Plot into Croft

This plan details the consolidation of the `Plot` and `Croft` domain abstractions into a single unified `Croft` concept representing an individual host machine on the Ranch. It completely eliminates the `src/plot/` subsystem and the redundant `CroftRecord` struct by making **`Croft`** the single canonical domain entity and runtime compound across storage, networking, and CLI. It also aligns filesystem layout by storing Croft instances under `~/.swini/crofts/{name}/` with their supervising Regent state persisted as `regent.json`.

See [goal.md](./goal.md) for the product requirements this design satisfies.

## Architecture

```mermaid
graph TD
  subgraph Ranch Cluster
    subgraph Croft A [Croft: Server]
      GA[Gate: gRPC 127.0.0.1:7440]
      BA[(Barn Store: Raft)]
      CCA[CroftClerk & CroftApi]
      GA --- CCA
      CCA --- BA
    end

    subgraph Croft B [Croft: Worker]
      GB[Gate: gRPC 127.0.0.1:7441]
      BB[(Barn Store: Client)]
      CCB[CroftClerk & CroftApi]
      GB --- CCB
    end

    Croft B -- CroftApi::Join --> Croft A
    BA -. replicated key croft/{id} .- BB
  end
```

The refactoring affects the following components and modules:
- **`proto/croft.proto` & `src/core/proto/`**: Replaces `proto/plot.proto` and `src/core/proto/plot.rs` with `proto/croft.proto` (`swini.croft` package) and `src/core/proto/croft.rs`.
- **`src/croft/`**:
  - `types.rs`: Introduces pure data type `CroftRole` (`Server`, `Worker`).
  - `config.rs`: Encapsulates configuration loading from `src/core/config.rs` into `src/croft/config.rs` (`Config`, defaults, YAML merging), defaulting instances to `~/.swini/crofts/{name}`.
  - `mod.rs`: Defines the single canonical `Croft` entity. Holds domain identity (`id`, `name`, `addr`, `roles`, `tags`, `joined_at`) and local runtime infrastructure (`barn`, `gate`) directly as public fields. Implements `Croft::spawn(&config)` and `persist()`.
  - `clerk/mod.rs` & `clerk/api.rs`: Relocates domain clerk to `CroftClerk` (`src/croft/clerk/`) operating on key prefix `croft/` and mounting `CroftApiServer` on `croft.gate`.
  - `gate.rs`: Adapts tests to mount `CroftApi`.
- **`src/plot/`**: Completely removed from the repository.
- **`src/core/`**: Remains focused on cross-cutting primitives (`telemetry.rs`, `proto/`).
- **`src/regent/` & `src/cli/`**: Regent bootstrap join and lifecycle handlers transition to `CroftClerk`, `CroftApiClient`, and `croft::Config`, writing runtime process state to `{data_dir}/regent.json`.

## Implementation Details

### 1. Protobuf Definition (`proto/croft.proto`)

Replaces `proto/plot.proto` with canonical Croft service definitions and messages:

```protobuf
syntax = "proto3";

package swini.croft;

service CroftApi {
  rpc Join(JoinReq) returns (JoinRes);
  rpc Status(StatusReq) returns (StatusRes);
}

message JoinReq {
  uint64 id = 1;
  string name = 2;
  string addr = 3;
  repeated string roles = 4;
  repeated string tags = 5;
}

message JoinRes {
  repeated Croft server_crofts = 1;
}

message StatusReq {}

message StatusRes {
  repeated Croft crofts = 1;
  uint64 primary_id = 2;
}

message Croft {
  uint64 id = 1;
  string name = 2;
  string addr = 3;
  repeated string roles = 4;
  repeated string tags = 5;
  string joined_at = 6;
  bool is_primary = 7;
}
```

### 2. Domain Roles (`src/croft/types.rs`)

Pure data definitions for operational roles assigned to a Croft:

```rust
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Operational role assigned to a Croft within the Ranch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CroftRole {
  Server,
  Worker,
}

impl fmt::Display for CroftRole {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      CroftRole::Server => write!(f, "server"),
      CroftRole::Worker => write!(f, "worker"),
    }
  }
}

impl FromStr for CroftRole {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.trim().to_lowercase().as_str() {
      "server" => Ok(CroftRole::Server),
      "worker" => Ok(CroftRole::Worker),
      other => Err(format!(
        "Unknown Croft role '{}'. Expected 'server' or 'worker'.",
        other
      )),
    }
  }
}
```

### 3. Croft Configuration & Filesystem Layout (`src/croft/config.rs`)

Encapsulates Croft configuration loading, YAML merging, and path resolutions under `~/.swini/crofts/{name}`:

```rust
use crate::croft::CroftRole;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

pub const DEFAULT_ADDR: &str = "127.0.0.1:7440";
pub const DEFAULT_NAME: &str = "main";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
  #[serde(default = "default_name")]
  pub name: String,

  #[serde(default = "default_addr")]
  pub addr: SocketAddr,

  #[serde(default)]
  pub data_dir: PathBuf,

  #[serde(default = "default_roles")]
  pub roles: Vec<CroftRole>,

  #[serde(default)]
  pub tags: Vec<String>,

  #[serde(default)]
  pub join_addresses: Vec<String>,
}

/// Computes the root directory housing all local Crofts on this host: `~/.swini/crofts`.
pub fn crofts_dir() -> PathBuf {
  let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
  PathBuf::from(home).join(".swini").join("crofts")
}

/// Computes the default root data directory for a named Croft: `~/.swini/crofts/{name}`.
pub fn default_data_dir(name: &str) -> PathBuf {
  crofts_dir().join(name)
}

impl Config {
  pub fn load(path: Option<&Path>) -> Result<Self, Box<dyn Error>> {
    // Loads defaults, overrides with swini.yml or custom path, applies SWINI_* env vars
    // ...
  }
}
```

### 4. Canonical Croft Entity (`src/croft/mod.rs`)

Consolidates all machine identity fields directly into `Croft`. When serialized to JSON (or deserialized from Barn), runtime infrastructure fields (`barn`, `gate`) are automatically skipped:

```rust
pub mod clerk;
pub mod config;
pub mod gate;
pub mod types;

pub use clerk::Clerk;
pub use config::Config;
pub use gate::Gate;
pub use types::CroftRole;

use crate::store::barn::{Barn, Config as BarnConfig};
use crate::store::ItemStore;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::Path;
use std::sync::Arc;

/// Key prefix in Barn storage where Croft records are stored.
pub const CROFT_PREFIX: &str = "croft/";

/// Domain representation and operational compound of a host machine on the Ranch.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Croft {
  /// Unique 64-bit identifier for this Croft.
  pub id: u64,
  /// Human-readable node name (e.g., "worker-01").
  pub name: String,
  /// Canonical network address (e.g., "127.0.0.1:7440") where Gate listens.
  pub addr: String,
  /// Assigned roles in the cluster (e.g., Server, Worker).
  pub roles: Vec<CroftRole>,
  /// Informational grouping tags associated with the Croft.
  pub tags: Vec<String>,
  /// ISO-8601 UTC timestamp when the Croft joined the Ranch.
  pub joined_at: String,

  /// Consensus replicated key-value storage engine (present on local Croft).
  #[serde(skip)]
  pub barn: Option<Arc<Barn>>,
  /// Network gRPC gateway for service registration (present on local Croft).
  #[serde(skip)]
  pub gate: Option<Gate>,
}

impl Croft {
  /// Spawns a local [`Croft`] compound, initializing its ID, storage, and network gate.
  pub async fn spawn(config: &Config) -> Result<Self, Box<dyn Error>> {
    let id = Self::id_provide(&config.data_dir)?;
    let gate = Gate::new();
    let barn = Arc::new(
      Barn::spawn(
        &gate,
        BarnConfig {
          id,
          addr: config.addr.to_string(),
          data_dir: config.data_dir.join("data"),
          heartbeat_interval: 100,
          election_timeout_min: 300,
          election_timeout_max: 600,
        },
      )
      .await?,
    );

    Ok(Self {
      id,
      name: config.name.clone(),
      addr: config.addr.to_string(),
      roles: config.roles.clone(),
      tags: config.tags.clone(),
      joined_at: chrono::Utc::now().to_rfc3339(),
      barn: Some(barn),
      gate: Some(gate),
    })
  }

  /// Provides a unique, persistent 64-bit ID stored at `{data_dir}/croft.id`.
  fn id_provide(data_dir: &Path) -> Result<u64, Box<dyn Error>> {
    std::fs::create_dir_all(data_dir)?;
    let id_path = data_dir.join("croft.id");
    if id_path.exists() {
      let content = std::fs::read_to_string(&id_path)?;
      if let Ok(id) = content.trim().parse::<u64>() {
        return Ok(id);
      }
    }

    let new_id = rand::random::<u64>() & 0x7FFF_FFFF_FFFF_FFFF;
    std::fs::write(&id_path, new_id.to_string())?;
    Ok(new_id)
  }

  /// Persists this Croft's state directly into the local Barn.
  pub async fn persist(&self) -> Result<(), Box<dyn Error>> {
    if let Some(ref barn) = self.barn {
      let key = format!("{}{}", CROFT_PREFIX, self.id);
      let payload = serde_json::to_vec(self)?;
      barn.set(&key, payload).await?;
    }
    Ok(())
  }

  pub fn is_server(&self) -> bool {
    self.roles.contains(&CroftRole::Server)
  }

  pub fn is_worker(&self) -> bool {
    self.roles.contains(&CroftRole::Worker)
  }
}
```

### 5. Croft Clerk & gRPC Service Handler (`src/croft/clerk/`)

- `src/croft/clerk/mod.rs`: Manages Barn persistence under `CROFT_PREFIX = "croft/"`.
- `src/croft/clerk/api.rs`: Implements `CroftApi` tonic server trait and wire conversions.

```rust
// src/croft/clerk/mod.rs
use crate::croft::{Croft, CroftRole, CROFT_PREFIX};
use crate::store::barn::Node as BarnNode;
use crate::store::{ItemStore, SpreadNode, SpreadStore};
use std::error::Error;
use std::sync::Arc;

#[derive(Clone)]
pub struct Clerk {
  pub croft: Arc<Croft>,
}

impl Clerk {
  pub fn spawn(croft: Arc<Croft>) -> Result<Self, Box<dyn Error>> {
    let clerk = Self { croft: croft.clone() };
    if let Some(ref gate) = croft.gate {
      gate.add(Api::new(clerk.clone()).into_server());
    }
    Ok(clerk)
  }

  pub async fn get(&self, id: u64) -> Result<Option<Croft>, Box<dyn Error>> {
    let barn = self.croft.barn.as_ref().ok_or("Local Barn required")?;
    let key = format!("{}{id}", CROFT_PREFIX);
    if let Some(bytes) = barn.get(&key).await? {
      let croft: Croft = serde_json::from_slice(&bytes)?;
      return Ok(Some(croft));
    }
    Ok(None)
  }

  pub async fn list(&self) -> Result<Vec<Croft>, Box<dyn Error>> {
    let barn = self.croft.barn.as_ref().ok_or("Local Barn required")?;
    let mut crofts = Vec::new();
    let prefix = CROFT_PREFIX.to_string();
    let pairs = barn.list(Some(&prefix)).await?;
    for (_key, bytes) in pairs {
      if let Ok(croft) = serde_json::from_slice::<Croft>(&bytes) {
        crofts.push(croft);
      }
    }
    Ok(crofts)
  }

  pub async fn join(&self, mut incoming: Croft) -> Result<Vec<Croft>, Box<dyn Error>> {
    let barn = self.croft.barn.as_ref().ok_or("Local Barn required")?;
    if incoming.is_server() {
      let barn_node = BarnNode::new(incoming.id, incoming.addr.clone());
      barn.node_add(barn_node).await?;
    }
    if incoming.joined_at.is_empty() {
      incoming.joined_at = chrono::Utc::now().to_rfc3339();
    }
    let key = format!("{}{}", CROFT_PREFIX, incoming.id);
    let payload = serde_json::to_vec(&incoming)?;
    barn.set(&key, payload).await?;

    let all = self.list().await?;
    let server_crofts = all.into_iter().filter(|c| c.is_server()).collect();
    Ok(server_crofts)
  }
}
```

### 6. Regent Lifecycle & State File (`src/regent/mod.rs` & `src/regent/state.rs`)

Persists Regent runtime state as `{data_dir}/regent.json`:

```rust
// src/regent/state.rs
impl RegentState {
  pub fn path(data_dir: &Path) -> PathBuf {
    data_dir.join("regent.json")
  }

  pub fn save(&self) -> Result<(), Box<dyn Error>> {
    std::fs::create_dir_all(&self.config.data_dir)?;
    let state_path = Self::path(&self.config.data_dir);
    let json = serde_json::to_string_pretty(self)?;
    std::fs::write(state_path, json)?;
    Ok(())
  }

  pub fn load(data_dir: &Path) -> Result<Self, Box<dyn Error>> {
    let state_path = Self::path(data_dir);
    let content = std::fs::read_to_string(state_path)?;
    let state = serde_json::from_str(&content)?;
    Ok(state)
  }
}
```

And `candidate_data_dirs` searches `crate::croft::config::crofts_dir()`, `sandbox/.data/{name}`, and `.data/{name}`.

## Data Model / API Changes

### Barn Storage Keys
- **Previous key format**: `plot/{id}`
- **New key format**: `croft/{id}`
- **Value**: JSON-encoded `Croft` (identity and metadata)

### Filesystem Storage
- **Directory**: `~/.swini/regents/{name}/` &rarr; `~/.swini/crofts/{name}/`
- **Supervisor state**: `{data_dir}/state.json` &rarr; `{data_dir}/regent.json`
- **Persistent node ID**: `{data_dir}/plot.id` &rarr; `{data_dir}/croft.id`

### Protobuf & gRPC
- **Service**: `swini.plot.PlotApi` &rarr; `swini.croft.CroftApi`
- **Messages**: `swini.plot.Plot` &rarr; `swini.croft.Croft`, `JoinReq`, `JoinRes`, `StatusReq`, `StatusRes`

## Testing Strategy

- **Unit Tests (`cargo nextest run -E 'kind(lib) | kind(bin)'`)**:
  - `src/croft/types.rs`: `CroftRole` string parsing and serialization roundtrips.
  - `src/croft/config.rs`: YAML parsing with `CroftRole` and `crofts_dir()` paths.
  - `src/croft/mod.rs`: `Croft::spawn` initialization, `croft.id` provisioning, JSON serialization roundtrip (verifying `barn` & `gate` are skipped), and `croft.persist()`.
  - `src/croft/clerk/mod.rs`: `CroftClerk` `join`, `get`, and `list` returning `Vec<Croft>` against mock/in-memory Barn Raft store.
  - `src/croft/clerk/api.rs`: Protobuf validation for `JoinReq` and RPC roundtrips for `join` and `status`.
  - `src/regent/mod.rs` & `state.rs`: `regent.json` lifecycle and multi-node bootstrap join using `Croft`.
- **E2E Tests (`cargo nextest run -E 'kind(test)'`)**:
  - `tests/e2e/regent.rs`: Full CLI lifecycle tests (`start`, `status`, `stop`).
- **Checklist**:
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo build`

## Risks & Open Questions

- **Migration**: Existing dev test clusters or data directories using `~/.swini/regents/` will use `~/.swini/crofts/` with `regent.json`. Since Swini is in active pre-release development, this clean break is desirable.

## Out of Scope

- `CroftWatcher` host resource metrics collector (CPU, memory, disk sync to Barn) — deferred to a subsequent feature spec.
- Dynamic runtime role switching.
