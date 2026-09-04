---
id: 0020
plan: specs/00020-daemon-lifecycle/plan.md
author: @brunomacf
created: 2026-09-03
---

# [Todo] Regent Lifecycle Management

Tasks to implement the Swini Regent lifecycle management and Clerk front-office
architecture described in [plan.md](./plan.md). Tasks T1–T5 establish core
dependencies, Protobuf definitions, pure domain entities, hierarchical
configuration, and rotating telemetry. Tasks T6–T8 implement the domain
sub-clerks and central Clerk front-office API. Tasks T9–T11 implement Regent
process supervision, state persistence, lifecycle commands, and CLI routing.
Tasks T12–T13 clean up deprecated modules and add end-to-end lifecycle
verification tests.

## Tasks

- [x] **T1 — Update Dependencies & Build Configuration**
  - **Plan reference:**
    [Data Model / API Changes](./plan.md#dependencies-cargotoml)
  - **Files:** `Cargo.toml` (modified), `build.rs` (modified)
  - **Description:** Add required dependencies (`daemonize`, `serde_yml`,
    `tracing-appender`, `libc`) to `Cargo.toml` and update `build.rs` to compile
    `proto/plot.proto` alongside `proto/barn.proto` (removing
    `proto/cluster.proto`).
  - **Definition of Done:**
    - `Cargo.toml` contains `daemonize = "0.5.0"`, `serde_yml = "0.0.12"`,
      `tracing-appender = "0.2.3"`, and `libc = "0.2.169"`.
    - `build.rs` compiles `proto/plot.proto` and `proto/barn.proto`.
    - Project compiles cleanly via `cargo check`.
  - **Depends on:** none

- [x] **T2 — Implement Plot Protobuf Definition & Generated Modules**
  - **Plan reference:**
    [Plot Protocol Definition](./plan.md#1-protoplotproto--srccoreprotoplotrs--plot-protocol-definition)
  - **Files:** `proto/plot.proto` (new), `src/core/proto/plot.rs` (new),
    `src/core/proto/mod.rs` (modified), `proto/cluster.proto` (deleted),
    `src/core/proto/cluster.rs` (deleted)
  - **Description:** Define the `swini.plot` Protobuf package containing the
    `PlotApi` service (`Join`, `Status` RPCs) and messages (`JoinReq`,
    `JoinRes`, `StatusReq`, `StatusRes`, `Plot`). Register `plot.rs` in
    `src/core/proto/mod.rs` and remove deprecated `cluster` proto files.
  - **Definition of Done:**
    - `proto/plot.proto` defines `PlotApi` with `Join` and `Status` RPCs and all
      message types.
    - `src/core/proto/plot.rs` includes the generated code directly via
      `tonic::include_proto!("swini.plot")`.
    - `src/core/proto/mod.rs` exposes `pub mod plot;` and removes
      `pub mod cluster;`.
    - Post-Write Checklist passes (`cargo fmt`, `clippy`, `cargo build`).
  - **Depends on:** T1

- [x] **T3 — Implement Domain Entities in `src/plot/mod.rs`**
  - **Plan reference:**
    [Domain-Driven Plot Module](./plan.md#2-domain-driven-plot-module-srcplot)
  - **Files:** `src/plot/mod.rs` (new)
  - **Description:** Implement domain data models `Plot` and `PlotRole`
    (`Server`, `Worker`) in `src/plot/mod.rs` with `Display`, `FromStr`,
    `Serialize`, `Deserialize`, and helper methods `is_server`, `is_worker`,
    `id_provide`.
  - **Definition of Done:**
    - `PlotRole` parses `"server"` and `"worker"` case-insensitively and rejects
      invalid roles with an informative error.
    - `Plot` derives
      `Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default`.
    - Unit tests in `src/plot/mod.rs` verify serialization, deserialization, and
      role parsing.
    - Post-Write Checklist passes.
  - **Depends on:** none

- [x] **T4 — Implement Core Configuration Resolver in `src/core/config.rs`**
  - **Plan reference:**
    [Configuration & Merging](./plan.md#6-srccoreconfigrs--configuration--merging)
  - **Files:** `src/core/config.rs` (new), `src/core/mod.rs` (modified)
  - **Description:** Implement `Config` struct with hierarchical resolution:
    base defaults (`name = "main"`, `addr = "127.0.0.1:7440"`,
    `roles = [Server, Worker]`, `data_dir = ~/.swini/regents/{name}` via
    `default_data_dir`), overridden by `./swini.yml` (if present) or explicitly
    specified config path, with `SWINI_ADDR` environment variable override.
  - **Definition of Done:**
    - `default_data_dir(name)` expands to `~/.swini/regents/{name}` (or
      `./.swini/regents/{name}` fallback).
    - `Config::load` loads and merges YAML configuration with base defaults
      using deep map merge.
    - `SWINI_ADDR` environment variable overrides configured `addr`.
    - Unit tests cover default values, YAML parsing/merging, and environment
      variable overrides.
    - Post-Write Checklist passes.
  - **Depends on:** T3

- [x] **T5 — Implement Rotating File & Console Telemetry in
      `src/core/telemetry.rs`**
  - **Plan reference:**
    [Telemetry & Rolling Logs](./plan.md#8-srccoretelemetryrs--telemetry--rolling-logs)
  - **Files:** `src/core/telemetry.rs` (modified)
  - **Description:** Update `core::telemetry::init` to accept
    `console_enabled: bool`, `log_dir: Option<&Path>`, and
    `log_prefix: Option<&str>`, configuring a non-blocking daily rolling file
    appender (`regent.log`) and conditional stdout layer with `SWINI_LOG_LEVEL`
    (fallback `RUST_LOG`) filtering.
  - **Definition of Done:**
    - Telemetry writes daily rotated log files to `{log_dir}/regent.log`.
    - Console layer is active only when `console_enabled == true`.
    - Returns `TelemetryGuard` to preserve worker thread lifecycle.
    - Unit tests verify telemetry initialization without panicking.
    - Post-Write Checklist passes.
  - **Depends on:** none

- [x] **T6 — Implement `plot::Clerk` & `plot::Api` in `src/plot/`**
  - **Plan reference:**
    [Plot Module](./plan.md#2-domain-driven-plot-module-srcplot)
  - **Files:** `src/plot/clerk/mod.rs` (new), `src/plot/clerk/api.rs` (new),
    `src/plot/mod.rs` (modified)
  - **Description:** Implement `plot::Clerk` staff worker in
    `src/plot/clerk/mod.rs` managing local `Plot` state (holding `Arc<Croft>`),
    `spawn(croft)` registering its API handler onto `croft.gate.add(...)`,
    `get(id)`, `list()`, and `join(Plot)`. Implement `plot::Api` in
    `src/plot/clerk/api.rs` providing `plot_from_join_req`,
    `proto_plot_from_plot`, and implementing the tonic `PlotApi` trait (`join`,
    `status`).
  - **Definition of Done:**
    - In `src/plot/clerk/mod.rs`: `Clerk::spawn` takes `Arc<Croft>` and
      registers its tonic server onto `croft.gate.add(...)`.
    - In `src/plot/clerk/mod.rs`: `Clerk::join` adds incoming server plots to
      `Barn` membership and persists them to the `plot/` key prefix.
    - In `src/plot/clerk/api.rs`: `plot_from_join_req` validates non-empty name
      and address.
    - In `src/plot/clerk/api.rs`: `proto_plot_from_plot` maps domain `Plot` into
      `ProtoPlot`.
    - In `src/plot/clerk/api.rs`: `Api` implements `PlotApi` gRPC service
      (`Join`, `Status`).
    - Unit tests in `src/plot/` cover validation, join workflow, and get/list
      operations.
    - Post-Write Checklist passes.
  - **Depends on:** T2, T3, T4

- [x] **T7 — Implement Barn gRPC Service & Direct Gate Registration in
      `src/store/barn/`**
  - **Plan reference:**
    [Self-Contained Barn Transport](./plan.md#3-self-contained-barn-transport-srcstorebarnapirs)
  - **Files:** `src/store/barn/api.rs` (new), `src/store/barn/mod.rs` (modified)
  - **Description:** Implement `Api` in `src/store/barn/api.rs` implementing
    tonic `BarnApi` for Raft consensus dispatch (`append_entries`, `vote`,
    `node_add`, etc.) and register directly to `Gate` during
    `Barn::spawn(&gate, config)`.
  - **Definition of Done:**
    - `src/store/barn/api.rs` implements `BarnApi` trait and dispatches
      `node_add` to `Barn::node_add`.
    - `Barn::spawn(&gate, config)` registers `BarnApiServer<Api>` directly onto
      `gate.add(...)`.
    - Unit tests in `src/store/barn/api.rs` verify handler serialization and
      dispatch.
    - Post-Write Checklist passes.
  - **Depends on:** T1

- [x] **T8 — Implement `Croft` Operational Compound in `src/croft/mod.rs` &
      `Gate` Network Gateway in `src/croft/gate.rs`**
  - **Plan reference:**
    [Croft Operational Compound & Gate Gateway](./plan.md#1-croft-operational-compound-srccroft-and-gate-network-gateway-srccore)
  - **Files:** `src/croft/mod.rs` (new), `src/croft/gate.rs` (new),
    `src/main.rs` (modified)
  - **Description:** Implement `Gate` in `src/croft/gate.rs` with thread-safe
    interior mutability (`Arc<Mutex<Option<GateState>>>`), exposing
    `pub fn add<S, B>(&self, svc: S)` and
    `pub async fn listen(&self, addr: SocketAddr)`. Implement `Croft` in
    `src/croft/mod.rs` bringing together domain `Plot`, storage `Barn`, network
    `Gate`, and runtime `Config`.
  - **Definition of Done:**
    - `Gate::new` creates a thread-safe gRPC router and server.
    - `Croft::spawn` initializes `Plot`, `Barn`, `Gate`, and `Config`.
    - `src/main.rs` registers `mod croft;`.
    - Unit test verifies `Croft::spawn` initializes without errors.
    - Post-Write Checklist passes.
  - **Depends on:** T6, T7

- [x] **T9 — Implement `RegentState` Persistence in `src/regent/state.rs`**
  - **Plan reference:**
    [Regent State Persistence](./plan.md#7-srcregentstaters--regent-state-persistence)
  - **Files:** `src/regent/state.rs` (new), `src/regent/mod.rs` (new)
  - **Description:** Implement `RegentState` struct (`pid: u32`,
    `detached: bool`, `started_at: String`, `config: Config`) with methods
    `save`, `load`, `cleanup`, and `is_alive` (using `libc::kill(pid, 0)`).
  - **Definition of Done:**
    - `RegentState::save` serializes JSON to `{config.data_dir}/state.json`.
    - `RegentState::load` deserializes state from the data directory.
    - `RegentState::is_alive` returns `true` for the current process PID and
      `false` for nonexistent PIDs.
    - `RegentState::cleanup` deletes the `state.json` file.
    - Unit tests cover save, load, cleanup, and liveness check.
    - Post-Write Checklist passes.
  - **Depends on:** T4

- [x] **T10 — Implement Regent Lifecycle Orchestration in `src/regent/mod.rs`**
  - **Plan reference:**
    [Regent Lifecycle Orchestration](./plan.md#9-srcregentmodrs--regent-lifecycle-orchestration)
  - **Files:** `src/regent/mod.rs` (modified), `src/main.rs` (modified),
    `src/daemon/` (deleted)
  - **Description:** Implement `regent::start` (foreground by default,
    background via `daemonize` if `detached == true`, bootstrapping Croft,
    hiring PlotClerk, executing outbound bootstrap join if `join_addresses`
    present, starting `croft.gate.listen`, and trapping `SIGINT`/`SIGTERM`),
    `regent::stop` (terminating process via `SIGTERM` and cleaning up state),
    `regent::status` (inspecting local Regent instances), and `dial_join` /
    `bootstrap_join`. Remove legacy `src/daemon/`.
  - **Definition of Done:**
    - `regent::start` boots Croft and PlotClerk, records `state.json`,
      initializes telemetry, starts Gate listener, and cleans up on shutdown
      signal.
    - `regent::stop` finds PID from `state.json`, sends `SIGTERM`, polls for
      process exit, and cleans up `state.json`.
    - `regent::status` prints tabular overview of all detected local Regents.
    - `bootstrap_join` dials peer address via `PlotApiClient` and registers
      server nodes into Barn cache.
    - `src/main.rs` exposes `mod regent;` and removes `mod daemon;`.
    - Unit tests cover `bootstrap_join` error handling and state cleanup.
    - Post-Write Checklist passes.
  - **Depends on:** T5, T8, T9

- [x] **T11 — Implement Scoped CLI Routing & `SWINI_ADDR` Resolution in
      `src/cli/mod.rs`**
  - **Plan reference:**
    [Root-Level Subcommands & local:// Resolution](./plan.md#10-srcclimodrs--root-level-subcommands--local-resolution)
  - **Files:** `src/cli/mod.rs` (modified), `src/main.rs` (modified)
  - **Description:** Implement scoped CLI subcommands `swini regent start` (with
    `-c/--config` and `-d/--detached`), `swini regent stop [name]`, and
    `swini regent status [name]`. Implement `resolve_api_url` supporting
    `SWINI_ADDR` with `local://{name}` URI lookup from
    `~/.swini/regents/{name}/state.json`.
  - **Definition of Done:**
    - CLI parses `swini regent start`, `swini regent stop`, and
      `swini regent status`.
    - `resolve_api_url` resolves raw sockets (`127.0.0.1:7440`), HTTP URLs
      (`http://...`), and `local://{name}` URIs by reading
      `{default_data_dir(name)}/state.json`.
    - Unit tests cover CLI argument parsing and endpoint resolution across all
      formats.
    - Post-Write Checklist passes.
  - **Depends on:** T10

- [x] **T12 — Clean Up Deprecated Modules & Update Documentation**
  - **Plan reference:**
    [Protobuf & Module Migration](./plan.md#protobuf--module-migration)
  - **Files:** `src/cluster/` (deleted), `src/node/` (deleted), `src/api/`
    (deleted), `src/core/README.md` (modified), `src/clerk/README.md` (new),
    `src/regent/README.md` (new), `src/README.md` (modified)
  - **Description:** Remove deprecated modules (`src/cluster/`, `src/node/`,
    `src/api/`) replaced by `src/clerk/`, `src/core/entities.rs`, and
    `src/regent/`. Create and update module `README.md` files reflecting the
    Ranch, Plot, Regent, and Clerk architecture.
  - **Definition of Done:**
    - Deprecated directories (`src/cluster/`, `src/node/`, `src/api/`) are
      completely removed.
    - `src/clerk/README.md` and `src/regent/README.md` describe module
      responsibilities and files.
    - `src/core/README.md` and `src/README.md` map the updated module
      architecture.
    - Post-Write Checklist passes (`cargo fmt`, `clippy`, `cargo build`, unit
      tests).
  - **Depends on:** T11

- [x] **T13 — Add End-to-End Regent Lifecycle Tests in `tests/`**
  - **Plan reference:**
    [Integration / E2E Tests](./plan.md#integration--e2e-tests)
  - **Files:** `tests/regent_lifecycle.rs` (new)
  - **Description:** Implement integration tests using `assert_cmd` to verify
    `swini regent start` (detached mode), `swini regent status` (listing active
    instance), and `swini regent stop` (graceful termination and state cleanup).
  - **Definition of Done:**
    - Test starts a Regent with a custom name and port in detached mode.
    - Test asserts `swini regent status` displays the running instance.
    - Test asserts `swini regent stop` terminates the process and cleans up
      `state.json`.
    - All tests pass via `cargo nextest run -E 'kind(test)'`.
    - Post-Write Checklist passes.
  - **Depends on:** T11, T12

## Task Order

- **T1** (dependencies & build) and **T3** (domain entities) and **T5**
  (telemetry) can start immediately in parallel.
- **T2** depends on **T1**.
- **T4** (config) depends on **T3**.
- **T6** (`PlotClerk`) depends on **T2, T3, T4**.
- **T7** (`BarnClerk`) depends on **T1**.
- **T8** (`Clerk` front-office) depends on **T6, T7**.
- **T9** (`RegentState`) depends on **T4**.
- **T10** (`regent` lifecycle) depends on **T5, T8, T9**.
- **T11** (`cli` commands) depends on **T10**.
- **T12** (cleanup & docs) depends on **T11**.
- **T13** (e2e tests) depends on **T11, T12**.
