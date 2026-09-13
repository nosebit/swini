---
id: 0027
author: @brunomacf
created: 2026-09-08
---

# [Goal] Croft Resource Accounting and Status Inspection

Swini orchestrates distributed workloads across a cluster of machines known as
the **Ranch**. Each machine acts as an operational **Croft**, providing
computational capacity where workloads (**Piglets**) can be placed and
supervised inside isolated execution environments (**Yards**) managed by
**Pigs**.

To enable workload schedulers (such as the upcoming **Drover**) to make informed
placement decisions, the Ranch requires authoritative knowledge of host
resources across all Crofts. This includes total hardware capacity, the amount
of capacity actually available for workloads after OS overhead (**Yardable**
capacity), and the amount currently committed to running workloads (**Reserved**
capacity).

Rather than generating high-frequency consensus churn via constant Raft polling,
Crofts capture baseline hardware capacity and initial yardable resources on
startup, persisting this data into the **Barn** under `croft/{id}`. Operators
and cluster components can inspect this state across the Ranch using
cluster-wide summaries (`swini status`) or detailed single-node views
(`swini croft status <name>`), with support for live telemetry comparisons
(`--live`).

## Requirements

- **Dedicated `CroftResources` Data Compound**:
  - Encapsulate all hardware resource capacity and allocation metrics inside a
    dedicated `CroftResources` structure embedded within `Croft`
    (`pub resources: CroftResources`):
    - `cpu_total`: Total physical CPU capacity (in Hertz).
    - `cpu_yardable`: CPU capacity available for scheduling workloads (total
      minus OS baseline overhead).
    - `cpu_reserved`: CPU capacity currently committed to active workloads on
      this Croft (initialized to 0).
    - `mem_total`: Total physical RAM capacity (in bytes).
    - `mem_yardable`: RAM capacity available for scheduling workloads (total
      minus OS baseline overhead).
    - `mem_reserved`: RAM capacity currently committed to active workloads on
      this Croft (initialized to 0).
  - Provide domain convenience methods for computing available schedulable
    capacity (`cpu_available() = cpu_yardable.saturating_sub(cpu_reserved)` and
    `mem_available() = mem_yardable.saturating_sub(mem_reserved)`).

- **Barn Storage Integration**:
  - Crofts probe local hardware resources on startup and persist baseline
    capacity (`total` and `yardable`, with `reserved` initialized to 0) into the
    Barn upon join/persist.
  - Resource data is persisted as part of the canonical `Croft` record under
    `croft/{id}`.

- **Protobuf and API Integration**:
  - Protobuf schemas define `message CroftResources` and embed it within
    `message Croft` (`CroftResources resources = 8;`).
  - `CroftApi` gRPC service provides resource metrics in response to cluster and
    node status inquiries.
  - Supports fetching instantaneous live physical host metrics (`mem_used`,
    `cpu_used`) for a target Croft when requested.

- **Cluster-Wide Overview CLI (`swini status`)**:
  - Displays a tabular summary of all registered Crofts in the Ranch.
  - Columns include: Croft ID, Name, Roles, Network Address, CPU summary
    (Reserved / Yardable / Total), Memory summary (Reserved / Yardable / Total),
    and Operational Status.

- **Single-Croft Inspection CLI (`swini croft status <name>`)**:
  - Requires a specific Croft `<name>` and displays comprehensive node metadata
    (ID, Roles, Tags, Network Address, Joined At) alongside an in-depth
    breakdown of CPU and Memory allocations from the Barn.
  - Supports an optional `--live` flag to query the target Croft directly for
    instantaneous host metrics, presenting both the persisted Barn allocation
    and live physical utilization (`mem_used`, `cpu_used`) for comparison.

- **Human-Readable Telemetry Formatting**:
  - CPU capacity and allocations are formatted into human-friendly units (MHz,
    GHz).
  - Memory capacity and allocations are formatted into human-friendly units (MB,
    GB, GiB).

## Constraints

- **Non-Goal: Workload Scheduling and Active Reservation Updates**: Dynamic
  manipulation of `reserved` counters during workload lifecycle (scheduling,
  stopping, failing piglets) will be implemented alongside the Drover component
  in a subsequent feature. In this feature, `reserved` is tracked in the data
  model and initialized to 0.
- **Event-Driven Consensus Updates**: Resource allocations in the Barn are
  recorded on discrete lifecycle events (e.g. startup/join). Background periodic
  sampling does not write to Raft storage.
- **Binary-Only Distribution & Stable Rust**: All code adheres to Swini's stable
  Rust toolchain and binary-only distribution requirements.

## Scenarios

**GIVEN** a freshly started Croft machine with 8 CPU cores (3.2 GHz each) and 32
GB RAM **WHEN** the Croft boots and registers with the Ranch **THEN** its
`Croft` record in the Barn reflects a `CroftResources` entity with total CPU
(25.6 GHz), total RAM (32 GB), yardable capacity accounting for OS overhead, and
0 reserved resources

**GIVEN** a running Swini cluster with multiple registered Crofts **WHEN** an
operator runs `swini status` **THEN** the CLI outputs a formatted table listing
all Crofts, their assigned roles, network endpoints, and a summary of their CPU
and memory allocations

**GIVEN** a registered Croft named `"worker-01"` **WHEN** an operator runs
`swini croft status worker-01` **THEN** the CLI displays the Croft's metadata
along with total, yardable, reserved (0), and available CPU and memory metrics
as recorded in the Barn

**GIVEN** a registered Croft named `"worker-01"` **WHEN** an operator runs
`swini croft status worker-01 --live` **THEN** the CLI queries the live Croft
endpoint directly and outputs both the stored Barn resource allocations and
real-time physical host utilization metrics

<!-- plan-notes (for /mad.plan — not part of the product spec):
- Dependencies: Add `sysinfo` (or native host telemetry probing) for detecting CPU count, CPU clock frequency (Hz), total memory, and live memory/CPU consumption.
- Data structures (`src/croft/types.rs`):
  - Add `CroftResources` struct with `cpu_total`, `cpu_yardable`, `cpu_reserved`, `mem_total`, `mem_yardable`, `mem_reserved`.
  - Add methods `cpu_available(&self) -> u64` and `mem_available(&self) -> u64`.
  - Embed `pub resources: CroftResources` inside `Croft`.
- Protobuf (`proto/croft.proto`):
  - Define `message CroftResources` with `cpu_total`, `cpu_yardable`, `cpu_reserved`, `mem_total`, `mem_yardable`, `mem_reserved`.
  - Embed `CroftResources resources = 8;` inside `message Croft`.
  - Add `rpc LiveStatus(LiveStatusReq) returns (LiveStatusRes)` with `mem_used` and `cpu_used`.
- Clerks & APIs (`src/croft/clerk/`):
  - Update `Api::status` and `Api::join` to map and serialize `CroftResources`.
  - Add `Api::live_status` for live resource inspection when `--live` is requested.
- CLI Commands (`src/cli/`):
  - Top-level `swini status` command rendering a tabular cluster overview.
  - Subcommand `swini croft status <name>` (with `--live` flag) in `src/cli/croft.rs`.
  - Pretty formatting utility in `src/core/format.rs` for Hertz (MHz, GHz) and Bytes (MB, GB, GiB).
- Tests:
  - Unit tests for resource serialization, Protobuf conversions, and unit formatting.
  - E2e tests for `swini status` and `swini croft status <name>`.
-->
