---
id: 0028
author: @brunomacf
created: 2026-09-16
---

# [Goal] Pig Workload Execution and Process Driver

Swini is a distributed workload orchestrator designed to operate a cluster of
machines as a **Ranch**. Previous milestones established Croft clustering,
distributed consensus storage in the **Barn**, and host resource accounting.
However, Swini currently lacks the ability to define, deploy, execute, and
supervise user applications.

This feature introduces the foundational workload execution framework in Swini.
Operators define applications as **Pigs** in YAML configuration files, deploy
them across the Ranch using the CLI (`swini pig run`), and manage their lifecycle
(`swini pig stop`). Workloads are farrowed into identical **Piglets** placed onto
target Crofts inside resource-bounded **Yards**. On each Croft, tasks are
executed as native OS processes via an **`exec` task driver** and continuously
supervised to ensure desired replica counts and process availability.

## Requirements

- **Declarative Workload Manifests**:
  - Operators define workloads in YAML files (defaulting to `pigs.yml`, or
    custom paths specified via `--config` / `-c`).
  - Support top-level `pigs: [...]` declarations as well as nested
    `herds: [ { name: "...", pigs: [...] } ]` groupings.
  - Herds act as weak organizational groupings; pig names must remain unique
    across all herds within a space.
  - A default space (`"main"`) is applied when not explicitly configured.

- **Hierarchical Path Identification**:
  - Entities are canonically addressed and tracked across the cluster using
    hierarchical path strings:
    - **Pig**: `<space>/<pig_name>` (e.g. `main/web` or `prod/api`)
    - **Piglet**: `<space>/<pig_name>/<piglet_id>` (e.g. `prod/api/a1f9c2`)
    - **Task**: `<space>/<pig_name>/<piglet_id>/<task_name>` (e.g. `prod/api/a1f9c2/server`)

- **Workload Lifecycle CLI Operations**:
  - `swini pig run [--config <path>] [name_1 ... name_n]`:
    - Parses the manifest and deploys all specified pigs. If no names are
      provided, deploys all pigs defined in the configuration.
    - Persists the desired workload state to the Ranch control plane.
  - `swini pig stop [--config <path>] <pig_path | name>`:
    - Initiates a graceful shutdown of the specified Pig and all its active
      Piglets across the cluster.

- **Placement & Yard Resource Reservations**:
  - Each Pig specifies its desired replica `size`, placement policy (`spread`,
    `packed`, `exclusive`, tag matching), and Yard resource bounds (CPU and
    memory).
  - Schedulers ensure Piglets are placed only on Crofts with sufficient
    available Yardable capacity.

- **Native OS `exec` Task Driver**:
  - Executes task binaries and commands directly on target host machines with
    configured argument lists.
  - Manages process lifecycle (spawning, process ID tracking, SIGTERM signal
    termination, and exit status capturing).

- **Task Supervision & Failure Recovery**:
  - Local Croft supervisors monitor running task processes continuously.
  - **Service (Long-Running) Tasks**: Automatically restarted upon unexpected
    termination or crash up to a maximum retry threshold before marking the
    task as failed.
  - **Batch (One-Shot) Tasks**: Execute once to completion; successful exit (code
    0) transitions the task to completed/stopped without restarting.
  - State transitions reflect throughout the hierarchy:
    - Task: `Starting` $\rightarrow$ `Running` $\rightarrow$ `Stopping` $\rightarrow$ `Stopped` / `Failed`
    - Piglet: `Yarded` $\rightarrow$ `Starting` $\rightarrow$ `Running` $\rightarrow$ `Stopping` $\rightarrow$ `Stopped` / `Failed`
    - Pig: `Received` $\rightarrow$ `Updating` $\rightarrow$ `Running` $\rightarrow$ `Stopping` $\rightarrow$ `Stopped` / `Failed`

- **Cluster-Wide Desired-State Reconciliation**:
  - Maintains convergence between desired Pig configuration and actual running
    Piglets across the Ranch.
  - Automatically provisions new Piglets when scaling up or replacing failed
    instances, and terminates Piglets when scaling down or stopping.

## Constraints

- **Single Task Driver (`exec`)**: This milestone implements the native OS
  process execution engine only. Container engines (e.g. Docker, OCI) are
  out of scope and deferred to future specifications.
- **Process-Level Health Supervision**: Supervision is based on OS process
  liveness (process running vs. terminated/crashed). Protocol health checks
  (HTTP/TCP endpoints and custom exec probes) are out of scope for this initial
  specification.
- **Stable Toolchain & Binary Distribution**: Complies with binary-only
  distribution rules, stable Rust edition 2021, and project lint/testing
  standards.

## Scenarios

**GIVEN** a running Swini cluster and a `pigs.yml` manifest containing a Pig
`echo` with `size: 2` using the `exec` driver
**WHEN** an operator runs `swini pig run`
**THEN** the Ranch control plane schedules 2 `echo` Piglets across eligible
Crofts, the Crofts spawn the respective OS processes, and both Piglets reach the
`Running` state

**GIVEN** a `pigs.yml` manifest containing multiple herds and pigs
**WHEN** an operator runs `swini pig run echo-api`
**THEN** only the `echo-api` Pig is submitted and started across the cluster,
leaving other defined pigs untouched

**GIVEN** a running Piglet executing a long-running service task
**WHEN** the underlying OS process crashes or is killed externally
**THEN** the local Croft supervisor detects the termination, records the exit,
and restarts the task process automatically

**GIVEN** a running Piglet executing a batch task configured to run once
**WHEN** the task process exits with code 0
**THEN** the supervisor records the task as completed without triggering a
restart

**GIVEN** an active running Pig `default/echo` with 2 running Piglets
**WHEN** an operator executes `swini pig stop echo`
**THEN** the Pig transitions to `Stopping`, the local supervisors send `SIGTERM`
to the task processes, the Piglets stop cleanly, and the Pig reaches `Stopped`

<!-- plan-notes (for /mad.plan — not part of the product spec):
- Module layout (`src/pig/`):
  - `src/pig/mod.rs`: Module exports, `LivePig` active compound, and Barn key constants (`PIG_PREFIX = "pig/"`, `PIGLET_PREFIX = "piglet/"`).
  - `src/pig/types.rs`: Domain data structures (`Pig`, `PigSpec`, `PigStatus`, `Piglet`, `PigletSpec`, `PigletStatus`, `Task`, `TaskSpec`, `TaskStatus`, `TaskMode`, `Yard`, `Placement`, `PlacementMethod`, `ExecTask`).
  - `src/pig/config.rs`: Manifest parsing for `pigs.yml` supporting `space`, `pigs`, and `herds`.
  - `src/pig/clerk/mod.rs` & `src/pig/clerk/api.rs`: `PigClerk` domain clerk operating in `LiveCroft`, registering `PigApi` gRPC service onto the `Gate`.
- Protobuf (`proto/pig.proto`):
  - Messages for `Pig`, `PigSpec`, `Piglet`, `Task`, `Yard`, `Placement`.
  - `PigApi` service with `Run(PigRunReq) returns (PigRunRes)`, `Stop(PigStopReq) returns (PigStopRes)`, `Status(PigStatusReq) returns (PigStatusRes)`.
- Active Entities & Coordination:
  - `LivePig`: Encapsulates a running Pig's reconciliation loop on the Primary Croft, managing desired replica counts and driving state transitions.
  - `Drover` / `Reconciler` (`src/drover/`): Cluster-level workload coordinator managing LivePigs and cluster-wide reconciliation.
  - `Scheduler` (`src/drover/scheduler.rs`): Places Piglets onto Crofts based on `CroftResources` (Yardable CPU/memory) and placement constraints (spread/packed/exclusive/tags).
  - `Supervisor` (`src/drover/supervisor.rs`): Node-level supervisor spawned by Regent on each Croft, polling local Piglet tasks and driving OS process lifecycle via `ExecEngine`.
  - `ExecEngine` (`src/drover/task/engine/exec.rs`): Spawns child processes (`std::process::Command`), tracks PIDs, samples CPU/memory (`sysinfo`), and sends `SIGTERM` on stop.
- CLI (`src/cli/pig.rs` & `src/cli/mod.rs`):
  - `swini pig run [--config <path>] [names...]`
  - `swini pig stop [--config <path>] <name>`
- Storage Keys in Barn:
  - `pig/{space}/{name}` -> Serialized `Pig` JSON
  - `piglet/{node_id}/{space}/{name}/{piglet_id}` -> Serialized `Piglet` JSON
- Tests:
  - Unit tests for manifest deserialization, domain type serialization, path parsing, and exec engine process management.
  - E2e tests for `swini pig run` and `swini pig stop` on multi-node or local cluster setups.
-->
