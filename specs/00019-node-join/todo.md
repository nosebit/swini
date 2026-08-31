---
id: 19
plan: specs/00019-node-join/plan.md
author: @brunomacf
created: 2026-08-31
---

# [Todo] Multi-Node Cluster Joining and Membership

Tasks to implement cluster node joining, role-aware membership management, and
cluster status inspection described in [plan.md](./plan.md). Tasks T1-T3
establish domain models, daemon configurations, and protobuf gRPC definitions;
T4-T6 build the cluster state backend, gRPC API handlers, and local node
manager; T7 exposes the `swini status` CLI command, and T8 adds end-to-end
integration tests.

## Tasks

- [x] **T1 — Implement Node Domain Types (`Node`, `NodeRole`)**
  - **Plan reference:**
    [1. src/node/types.rs — Node State & Roles](./plan.md#1-srcnodetypesrs--node-state--roles)
  - **Files:** `src/node/types.rs` (modified), `src/node/mod.rs` (modified),
    `src/main.rs` (modified)
  - **Description:** Define `NodeRole` (`Server`, `Worker`) and `Node` struct
    containing node metadata, resource capacities, role assignments, API
    address, and `joined_at: String` timestamp. Re-export types from
    `src/node/mod.rs` and ensure `mod node;` is registered in `src/main.rs`.
  - **Definition of Done:**
    - `NodeRole` derives `Debug, Clone, Serialize, Deserialize, PartialEq, Eq`
      and implements `Default` (defaulting to `Worker`).
    - `Node` derives `Debug, Clone, Serialize, Deserialize, Default` with
      `joined_at: String` and `roles: Vec<NodeRole>`.
    - Unit tests in `src/node/types.rs` verifying serialization and
      deserialization roundtrip.
    - Passes project [Post-Write Checklist](../../.mad/memory/lore.md).
  - **Depends on:** none

- [x] **T2 — Implement Daemon & API Configuration**
  - **Plan reference:**
    [2. src/daemon/config.rs — Daemon & API Configuration](./plan.md#2-srcdaemonconfigrs--daemon--api-configuration)
  - **Files:** `src/daemon/config.rs` (new), `src/daemon/mod.rs` (new),
    `src/main.rs` (modified)
  - **Description:** Implement `DEFAULT_API_PORT = 7440`, `ApiConfig` with
    `bind_address`, and `Config` struct (`name`, `api`, `data_dir`, `roles`,
    `tags`, `join_addresses`). Re-export `Config`, `ApiConfig`, and
    `DEFAULT_API_PORT` from `src/daemon/mod.rs` and register `mod daemon;` in
    `src/main.rs`.
  - **Definition of Done:**
    - `ApiConfig::default()` binds to `127.0.0.1:7440`.
    - `Config` contains `name`, `api`, `data_dir`, `roles`, `tags`, and
      `join_addresses`.
    - Unit tests in `src/daemon/config.rs` verify default bindings and
      serialization.
    - Passes project [Post-Write Checklist](../../.mad/memory/lore.md).
  - **Depends on:** T1

- [x] **T3 — Define `proto/cluster.proto` & Protobuf Code Generation**
  - **Plan reference:**
    [5. proto/cluster.proto & gRPC Code Generation](./plan.md#5-protoclusterproto--grpc-code-generation)
  - **Files:** `proto/cluster.proto` (new), `build.rs` (modified),
    `src/core/proto/cluster.rs` (new), `src/core/proto/mod.rs` (modified)
  - **Description:** Define `ClusterApi` service with
    `Join(JoinReq) returns (JoinRes)` and
    `Status(StatusReq) returns (StatusRes)`, along with `Node` message
    (including `bool is_primary`). Register `cluster.proto` compilation in
    `build.rs` and re-export the generated module.
  - **Definition of Done:**
    - `proto/cluster.proto` defines `JoinReq`, `JoinRes`, `StatusReq`,
      `StatusRes`, and `Node`.
    - `cargo build` compiles protobuf schemas and generates `swini.cluster`
      tonic client and server stubs without error.
    - Passes project [Post-Write Checklist](../../.mad/memory/lore.md).
  - **Depends on:** none

- [x] **T4 — Implement Cluster Backend (`ClusterBackend`)**
  - **Plan reference:**
    [4. src/cluster/mod.rs — Cluster Backend](./plan.md#4-srcclustermodrs--cluster-backend)
  - **Files:** `src/cluster/mod.rs` (new), `src/main.rs` (modified)
  - **Description:** Implement `ClusterBackend` wrapping the `node/` prefix
    slice of the Barn store. Implement `spawn`, `join`, `status`, and
    `primary_node_id`. For `Server` joins, promote the node in Raft consensus
    via `barn.node_add`, stamp `joined_at`, persist under `node/{id}`, and
    return all registered server nodes. Register `mod cluster;` in
    `src/main.rs`.
  - **Definition of Done:**
    - `ClusterBackend::spawn` initializes a `node/` prefixed slice from
      `Arc<Barn>`.
    - `ClusterBackend::join` adds Server nodes to Barn Raft membership and
      writes serialized node state to Barn.
    - `ClusterBackend::join` sets `joined_at` with current UTC RFC 3339
      timestamp.
    - `ClusterBackend::status` lists and deserializes all registered `node/*`
      records.
    - `ClusterBackend::primary_node_id` returns the ID of the current leader
      server.
    - Unit tests cover server join, worker join, status retrieval, and primary
      node discovery.
    - Passes project [Post-Write Checklist](../../.mad/memory/lore.md).
  - **Depends on:** T1, T3

- [x] **T5 — Implement Cluster gRPC API Handlers (`ClusterApiHandler`)**
  - **Plan reference:**
    [6. src/api/cluster.rs & src/api/mod.rs — API Handlers](./plan.md#6-srcapiclusterrs--srcapimodrs--api-handlers)
  - **Files:** `src/api/cluster.rs` (new), `src/api/mod.rs` (modified)
  - **Description:** Implement `TryFrom<JoinReq> for Node` with validation
    (valid positive ID, non-empty API address, valid non-empty roles). Implement
    `ClusterApiHandler` for `ClusterApi`, mapping domain nodes to proto nodes
    with `to_proto_nodes` and `is_primary`. Register `ClusterApiServer` in
    `Api::listen`.
  - **Definition of Done:**
    - `TryFrom<JoinReq> for Node` validates fields and returns
      `Status::invalid_argument` on invalid inputs.
    - `ClusterApiHandler::join` delegates to `ClusterBackend::join` and returns
      `JoinRes`.
    - `ClusterApiHandler::status` delegates to `ClusterBackend::status` and
      returns `StatusRes`.
    - `Api::new` and `Api::listen` support serving `ClusterApiServer` alongside
      `BarnApiServer`.
    - Unit tests in `src/api/cluster.rs` verify request validation and node
      conversion.
    - Passes project [Post-Write Checklist](../../.mad/memory/lore.md).
  - **Depends on:** T1, T3, T4

- [x] **T6 — Implement Local Node Backend (`NodeBackend`) & Peer Failover**
  - **Plan reference:**
    [3. src/node/mod.rs — Local Node Backend](./plan.md#3-srcnodemodrs--local-node-backend)
  - **Files:** `src/node/mod.rs` (modified)
  - **Description:** Implement deterministic `provide_node_id` reading or
    generating `data_dir/node.id`. Implement `NodeBackend::spawn` to populate
    `self_node` with `config.api.bind_address`. Implement `NodeBackend::join`
    dialing configured peer join addresses sequentially with automatic
    fallback/failover to subsequent addresses if a peer is unreachable, updating
    worker node cache via `barn.node_cache_set`.
  - **Definition of Done:**
    - `provide_node_id` creates `node.id` on disk if missing or reads existing
      ID.
    - `NodeBackend::spawn` initializes `Node` state with configured name, tags,
      roles, and `bind_address`.
    - `NodeBackend::join` iterates over `join_addresses`, calls gRPC `Join`, and
      updates Barn cache for Worker nodes.
    - Sequential peer failover works as expected: succeeds if an initial address
      fails but a subsequent one responds.
    - Unit tests cover ID generation/persistence, self-node initialization, and
      peer fallback logic.
    - Passes project [Post-Write Checklist](../../.mad/memory/lore.md).
  - **Depends on:** T1, T2, T3

- [x] **T7 — Implement `swini status` CLI Command & URL Resolution**
  - **Plan reference:**
    [7. src/cli/mod.rs & src/cli/cluster.rs — CLI Commands & URL Resolution](./plan.md#7-srcclimodrs--srccliclusterrs--cli-commands--url-resolution)
  - **Files:** `src/cli/mod.rs` (modified), `src/cli/cluster.rs` (new)
  - **Description:** Implement `resolve_api_url(daemon_name_or_id)` in
    `src/cli/mod.rs` resolving daemon API URL from environment variables, daemon
    state directories, or default `http://127.0.0.1:7440`. Connect
    `ClusterApiClient` and route `swini status` to `src/cli/cluster.rs`. Format
    output as an aligned table displaying `ID`, `NAME`, `ROLE`, `JOINED AT`, and
    `PRIMARY`.
  - **Definition of Done:**
    - `resolve_api_url` resolves `SWINI_API_URL`, daemon state directories, or
      defaults to `http://127.0.0.1:7440`.
    - `src/cli/mod.rs` parses `status` subcommand and connects
      `ClusterApiClient<Channel>`.
    - `src/cli/cluster.rs` prints aligned terminal table with headers `ID`,
      `NAME`, `ROLE`, `JOINED AT`, `PRIMARY`.
    - Unit tests in `src/cli/mod.rs` verify API URL resolution.
    - Passes project [Post-Write Checklist](../../.mad/memory/lore.md).
  - **Depends on:** T2, T3

- [x] **T8 — End-to-End Multi-Node Join and Cluster Status Integration Tests**
  - **Plan reference:** [Testing Strategy](./plan.md#testing-strategy)
  - **Files:** `tests/cluster_join.rs` (new)
  - **Description:** Write integration tests spinning up multiple in-process
    daemons with ephemeral directories. Test Server-Server join, Worker-Server
    join, Barn cache propagation, and CLI `swini status` output verification.
  - **Definition of Done:**
    - Integration test spinning up Server 1 and joining Server 2: verifies Barn
      membership expansion and `ClusterBackend::status` returning 2 nodes.
    - Integration test spinning up Worker node joining Server: verifies worker
      Barn cache is populated with server topology.
    - E2E test verifying CLI `swini status` output against running cluster.
    - All tests pass via `cargo test`.
    - Passes project [Post-Write Checklist](../../.mad/memory/lore.md).
  - **Depends on:** T1, T2, T3, T4, T5, T6, T7

## Task Order

- **T1**, **T2**, and **T3** can be developed in parallel (T2 depends on T1 for
  types).
- **T4** and **T5** build the cluster server infrastructure and depend on T1,
  T3.
- **T6** implements the client-side local node manager and depends on T1, T2,
  T3.
- **T7** builds the CLI interface and depends on T2, T3.
- **T8** runs last as the end-to-end integration test suite verifying the full
  multi-node join lifecycle across all components.
