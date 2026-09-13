---
id: 0027
plan: specs/00027-croft-resources/plan.md
author: @brunomacf
created: 2026-09-08
---

# [Todo] Croft Resource Accounting and Status Inspection

Tasks to implement Croft hardware resource accounting, Barn storage persistence,
gRPC cluster/node inspection endpoints, and operator CLI commands
(`swini status` and `swini croft status <name> [--live]`) as specified in
[plan.md](./plan.md).

## Tasks

- [x] **T1 — Add `sysinfo` dependency and implement universal formatters in
      `src/core/format.rs`**
  - **Plan reference:**
    [1. Formatting Utilities](./plan.md#1-formatting-utilities-srccoreformatrs)
  - **Files:** `Cargo.toml` (modified), `src/core/format.rs` (new),
    `src/core/mod.rs` (modified), `src/core/README.md` (modified)
  - **Description:** Add `sysinfo = "0.33"` to `Cargo.toml`. Create
    `src/core/format.rs` with `format_hertz(hz: u64) -> String` and
    `format_bytes(bytes: u64) -> String` using scaling loops. Register
    `pub mod format;` in `src/core/mod.rs` and update `src/core/README.md`.
  - **Definition of Done:**
    - `format_hertz` correctly formats scaling frequencies (`Hz`, `kHz`, `MHz`,
      `GHz`, `THz`, `PHz`).
    - `format_bytes` correctly formats scaling binary byte quantities (`B`,
      `KiB`, `MiB`, `GiB`, `TiB`, `PiB`).
    - Unit tests in `src/core/format.rs` verify all unit scale tiers.
    - Passes
      [Post-Write Checklist](../../.mad/memory/lore.md#post-write-checklist).
  - **Depends on:** none

- [x] **T2 — Implement `CroftResources` and `CroftTelemetry` in
      `src/croft/resources.rs`**
  - **Plan reference:**
    [2. CroftResources and Telemetry Probing](./plan.md#2-croftresources-and-telemetry-probing-srccroftresourcesrs)
  - **Files:** `src/croft/resources.rs` (new), `src/croft/mod.rs` (modified),
    `src/croft/README.md` (modified)
  - **Description:** Implement `CroftResources` with fields (`cpu_total`,
    `cpu_yardable`, `cpu_reserved`, `mem_total`, `mem_yardable`,
    `mem_reserved`), methods `cpu_available()`, `mem_available()`, and
    constructor `probe()`. Implement `CroftTelemetry` with `mem_used`,
    `cpu_used`, and sampler `sample()`. Register in `src/croft/mod.rs` and
    update `src/croft/README.md`.
  - **Definition of Done:**
    - `CroftResources::probe()` queries `sysinfo` and populates non-zero
      hardware capacities and default 90% yardable capacity.
    - `CroftTelemetry::sample()` samples instantaneous live CPU and RAM
      consumption.
    - `cpu_available()` and `mem_available()` calculate
      `yardable.saturating_sub(reserved)`.
    - Unit tests in `src/croft/resources.rs` test probing, sampling, and
      available capacity calculations.
    - Passes
      [Post-Write Checklist](../../.mad/memory/lore.md#post-write-checklist).
  - **Depends on:** T1

- [x] **T3 — Integrate `CroftResources` into `Croft` domain entity and
      `LiveCroft::spawn`**
  - **Plan reference:**
    [3. Croft Type and LiveCroft Integration](./plan.md#3-croft-type-and-livecroft-integration-srccrofttypesrs--srccroftmodrs)
  - **Files:** `src/croft/types.rs` (modified), `src/croft/mod.rs` (modified)
  - **Description:** Add `pub resources: CroftResources` to `Croft`. Initialize
    `base.resources = CroftResources::probe()` in `LiveCroft::spawn`. Ensure
    JSON serialization and Barn persistence round-trip.
  - **Definition of Done:**
    - `Croft` serializes and deserializes `resources` field cleanly.
    - `LiveCroft::spawn` initializes base Croft with probed resources.
    - Unit tests in `src/croft/types.rs` and `src/croft/mod.rs` verify `Croft`
      serialization and Barn persist/read.
    - Passes
      [Post-Write Checklist](../../.mad/memory/lore.md#post-write-checklist).
  - **Depends on:** T2

- [x] **T4 — Update Protobuf wire definitions in `proto/croft.proto`**
  - **Plan reference:**
    [3. Protobuf Wire Schema](./plan.md#3-protobuf-wire-schema-protocroftproto)
  - **Files:** `proto/croft.proto` (modified)
  - **Description:** Add `message CroftResources` and `message CroftTelemetry`.
    Add `message ListReq` and `message ListRes`. Update
    `message StatusReq { string name = 1; bool live = 2; }` and
    `message StatusRes { Croft croft = 1; CroftTelemetry telemetry = 2; }`.
    Embed `CroftResources resources = 8;` in `message Croft` and
    `CroftResources resources = 6;` in `message JoinReq`. Update
    `service CroftApi` with `rpc List` and `rpc Status`.
  - **Definition of Done:**
    - `proto/croft.proto` defines `List`, `Status`, `Join`, `CroftResources`,
      `CroftTelemetry`.
    - `cargo build` compiles tonic protobuf build successfully.
    - Passes
      [Post-Write Checklist](../../.mad/memory/lore.md#post-write-checklist).
  - **Depends on:** T3

- [x] **T5 — Implement `CroftClerk` `list` and `status` domain methods and gRPC
      API handler**
  - **Plan reference:**
    [4. CroftClerk Implementation](./plan.md#4-croftclerk-implementation-srccroftclerk)
  - **Files:** `src/croft/clerk/mod.rs` (modified), `src/croft/clerk/api.rs`
    (modified), `src/croft/clerk/README.md` (modified)
  - **Description:** Implement domain methods on `Clerk` in
    `src/croft/clerk/mod.rs`: `list(&self) -> Result<Vec<Croft>, ...>` and
    `status(&self, name: &str, live: bool) -> Result<Option<(Croft, Option<CroftTelemetry>)>, ...>`.
    Update `Api` in `src/croft/clerk/api.rs` to handle Protobuf conversions
    (`From`/`TryFrom`) for `CroftResources`, `CroftTelemetry`, and `Croft`,
    delegating `CroftApi::list` to `self.clerk.list()` and `CroftApi::status` to
    `self.clerk.status()`. Update `src/croft/clerk/README.md`.
  - **Definition of Done:**
    - `Clerk::status` returns target `Croft` and optional `CroftTelemetry`
      (sampled if `live == true`), or `None` if not found.
    - `Api::list` delegates to `clerk.list()` and returns `ListRes`.
    - `Api::status` delegates to `clerk.status()` and returns `StatusRes` or
      `NOT_FOUND`.
    - Unit tests in `src/croft/clerk/mod.rs` and `src/croft/clerk/api.rs` test
      domain queries, conversions, and gRPC endpoints.
    - Passes
      [Post-Write Checklist](../../.mad/memory/lore.md#post-write-checklist).
  - **Depends on:** T3, T4

- [x] **T6 — Implement `swini status` cluster summary CLI command in
      `src/cli/status.rs`**
  - **Plan reference:**
    [5. CLI Commands](./plan.md#5-cli-commands-srcclistatusrs-srcclicroftrs-srcclimodrs)
  - **Files:** `src/cli/status.rs` (new), `src/cli/mod.rs` (modified),
    `src/cli/README.md` (modified)
  - **Description:** Implement `swini status` in `src/cli/status.rs`. Resolve
    API URL, invoke `CroftApi::List`, and render formatted ASCII table
    displaying `ID`, `NAME`, `ROLES`, `ADDR`, `CPU (RES/YARD/TOT)`,
    `MEM (RES/YARD/TOT)`, and `STATUS`. Register in `src/cli/mod.rs` dispatch
    and update `src/cli/README.md`.
  - **Definition of Done:**
    - `swini status` parses and executes via `crate::cli::dispatch`.
    - Table columns display formatted human-readable units via
      `crate::core::format`.
    - Unit tests in `src/cli/status.rs` test command parsing and table
      formatting.
    - Passes
      [Post-Write Checklist](../../.mad/memory/lore.md#post-write-checklist).
  - **Depends on:** T1, T5

- [x] **T7 — Implement `swini croft status <name> [--live]` scoped CLI command
      in `src/cli/croft.rs`**
  - **Plan reference:**
    [5. CLI Commands](./plan.md#5-cli-commands-srcclistatusrs-srcclicroftrs-srcclimodrs)
  - **Files:** `src/cli/croft.rs` (new), `src/cli/mod.rs` (modified),
    `src/cli/README.md` (modified)
  - **Description:** Implement `swini croft status <name> [--live]` in
    `src/cli/croft.rs`. Resolve target API URL, invoke `CroftApi::Status`, and
    display detailed Croft metadata, Barn resource allocations, and optional
    Live Telemetry. Register `Croft` subcommand in `src/cli/mod.rs` dispatch and
    update `src/cli/README.md`.
  - **Definition of Done:**
    - `swini croft status <name>` displays metadata and Barn resource
      allocations.
    - `swini croft status <name> --live` includes Live Telemetry block with live
      CPU usage % and live memory consumption.
    - Unit tests in `src/cli/croft.rs` test CLI parsing and display formatting.
    - Passes
      [Post-Write Checklist](../../.mad/memory/lore.md#post-write-checklist).
  - **Depends on:** T1, T5

- [x] **T8 — End-to-end integration tests for `swini status` and
      `swini croft status`**
  - **Plan reference:** [Testing Strategy](./plan.md#testing-strategy)
  - **Files:** `tests/e2e/status.rs` (new), `tests/e2e/main.rs` (modified)
  - **Description:** Create e2e tests starting a live Regent/Croft, executing
    `swini status` and `swini croft status <name> [--live]` via `assert_cmd`,
    asserting zero exit codes and expected formatted outputs.
  - **Definition of Done:**
    - E2e tests verify `swini status` renders cluster table.
    - E2e tests verify `swini croft status <name>` outputs single-node details
      and resources.
    - E2e tests verify `swini croft status <name> --live` includes live
      telemetry.
    - `cargo nextest run -E 'kind(test)'` passes cleanly.
    - Passes
      [Post-Write Checklist](../../.mad/memory/lore.md#post-write-checklist).
  - **Depends on:** T6, T7

## Task Order

- **T1** (universal formatters & `sysinfo`) starts first.
- **T2** (`CroftResources` / `CroftTelemetry`) depends on T1.
- **T3** (`Croft` entity integration) depends on T2.
- **T4** (Protobuf schema) can run after T3.
- **T5** (`CroftClerk` domain methods and API handler) depends on T3 and T4.
- **T6** (`src/cli/status.rs`) and **T7** (`src/cli/croft.rs`) depend on T1 and
  T5, and can be implemented in parallel.
- **T8** (e2e tests) runs last, depending on T6 and T7.
