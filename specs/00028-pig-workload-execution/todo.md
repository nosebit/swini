---
id: 0028
plan: specs/00028-pig-workload-execution/plan.md
author: @brunomacf
created: 2026-09-16
---

# [Todo] Pig Workload Execution and Process Driver

Tasks to implement the end-to-end workload scheduling, execution, and native
process supervision subsystem described in [plan.md](./plan.md). Tasks T1–T4
establish protobuf definitions, system abstraction drivers, domain models, and
YAML configuration parsing; T5–T6 implement the front-office Pig Clerk and
Drover cluster scheduler; T7–T8 implement the Task Engine abstraction,
ExecEngine, and Croft Supervisor actor; T9 provides CLI subcommands
(`swini pig run`/`stop`); and T10 delivers end-to-end integration test coverage.

## Tasks

- [x] **T1 — Define `PigApi` Protobuf Service & Compile Stubs**
  - **Plan reference:**
    [Protobuf Service: `proto/pig.proto`](./plan.md#protobuf-service-protopigproto)
  - **Files:** `proto/pig.proto` (new), `src/core/proto/pig.rs` (new),
    `src/core/proto/mod.rs` (modified)
  - **Description:** Define the `PigApi` gRPC service with `PigRunReq`,
    `PigRunResp`, `PigStopReq`, and `PigStopResp` messages. Expose generated
    tonic protobuf client and server stubs in `src/core/proto/pig.rs` and
    register the `pig` module in `src/core/proto/mod.rs`.
  - **Definition of Done:**
    - `proto/pig.proto` compiles cleanly via `build.rs`.
    - `src/core/proto/pig.rs` exports `PigApiClient` and `PigApiServer`.
    - Post-write checklist (`cargo fmt`, `cargo clippy`, `cargo build`, unit
      tests) passes with zero warnings or errors.
  - **Depends on:** none

- [x] **T2 — Implement Host System Driver Abstraction & Platform Adapters
      (`src/core/sys`)**
  - **Plan reference:** [`src/core/sys/types.rs`](./plan.md#srccoresystypesrs),
    [`src/core/sys/variants/unix.rs`](./plan.md#srccoresysvariantsunixrs),
    [`src/core/sys/variants/macos.rs`](./plan.md#srccoresysvariantsmacosrs),
    [`src/core/sys/variants/linux/`](./plan.md#srccoresysvariantslinuxmodrs--srccoresysvariantslinuxubunturs),
    [`src/core/sys/mod.rs`](./plan.md#srccoresysmodrs)
  - **Files:** `src/core/sys/types.rs` (new), `src/core/sys/variants/mod.rs`
    (new), `src/core/sys/variants/unix.rs` (new),
    `src/core/sys/variants/macos.rs` (new), `src/core/sys/variants/linux/mod.rs`
    (new), `src/core/sys/variants/linux/ubuntu.rs` (new), `src/core/sys/mod.rs`
    (new), `src/core/sys/README.md` (new), `src/core/mod.rs` (modified)
  - **Description:** Define the `Sys` trait (`name`, `spawn`, `terminate`,
    `kill`, `is_alive`, `stats`) and `ProcessStats` in `types.rs`. Group
    concrete implementations under `variants/`: implement `UnixSys` using `libc`
    and `std::process::Command`, `MacSys` delegating to `UnixSys`, and
    `LinuxSys`/`UbuntuSys` reading `/proc/<pid>/stat`. Implement
    `sys::select() -> Arc<dyn Sys>` factory for runtime platform selection.
    Create `src/core/sys/README.md`.
  - **Definition of Done:**
    - `Sys` trait and all variant methods implemented with documentation
      comments (`//!`, `///`) and `# Errors` sections.
    - `UnixSys` spawns child processes with environment variables and working
      directory, sends `SIGTERM`/`SIGKILL`, checks liveness with `kill(pid, 0)`,
      and gathers CPU/memory statistics.
    - Unit tests in `src/core/sys/` verify spawning test commands (`sleep`,
      `echo`), querying stats, and terminating processes.
    - Post-write checklist passes.
  - **Depends on:** none

- [x] **T3 — Implement Workload Domain Models & Key Conventions
      (`src/pig/types.rs`)**
  - **Plan reference:** [`src/pig/types.rs`](./plan.md#srcpigtypesrs),
    [Barn Storage Layout](./plan.md#barn-storage-layout)
  - **Files:** `src/pig/types.rs` (new), `src/pig/mod.rs` (new),
    `src/pig/README.md` (new), `src/lib.rs` or `src/main.rs` (modified)
  - **Description:** Implement core domain entities: `Pig`, `PigConfig`,
    `PigStatus`, `Piglet`, `PigletConfig`, `PigletStatus`, `Placement`,
    `CroftPlacement`, `PlacementMethod`, `YardPlacement`, `Yard`, `Task`,
    `TaskConfig`, `TaskStatus`, `TaskMode`, and `ExecConfig`. Add helper
    methods: `Pig::key()`, `Pig::space()`, `Pig::name()`, `Piglet::key()`,
    `Piglet::space()`, `Piglet::pig_name()`, `Piglet::id()`, and
    `TaskConfig::engine_name(&self)`. Implement JSON and YAML serde
    serialization. Create `src/pig/README.md`.
  - **Definition of Done:**
    - All structs and enums match the technical plan specification with full
      inner/outer doc comments.
    - `TaskConfig::engine_name(&self) -> &str` returns `"exec"` for `TaskConfig`
      with `exec` configured.
    - Key derivation methods match exact storage paths (`pig/{space}/{name}`,
      `piglet/{croft_id}/{space}/{pig_name}/{piglet_id}`).
    - Unit tests cover JSON serialization/deserialization roundtrips and key
      derivation.
    - Post-write checklist passes.
  - **Depends on:** none

- [x] **T4 — Implement Workload Configuration Deserializer & Normalization
      (`src/pig/config.rs`)**
  - **Plan reference:** [`src/pig/config.rs`](./plan.md#srcpigconfigrs)
  - **Files:** `src/pig/config.rs` (new), `src/pig/mod.rs` (modified)
  - **Description:** Implement `PigRunConfig`, `HerdConfig`, and
    `PigDefaultsConfig` YAML configuration deserialization. Implement schema
    validation (non-empty tasks, valid workload/namespace names) and `to_pig()`
    normalization, populating default replica counts, placement constraints, and
    task definitions into domain `Pig` instances.
  - **Definition of Done:**
    - Correctly parses single-workload `PigRunConfig` and multi-workload
      `HerdConfig` YAML manifests.
    - Applies defaults and normalizes configs into `Pig` domain objects.
    - Unit tests cover valid configs, default inheritance, and error rejection
      on invalid/malformed YAML.
    - Post-write checklist passes.
  - **Depends on:** T3

- [x] **T5 — Implement Pig Clerk Front-Office & gRPC Service
      (`src/pig/clerk/`)**
  - **Plan reference:**
    [`src/pig/clerk/mod.rs` & `src/pig/clerk/api.rs`](./plan.md#srcpigclerkmodrs--srcpigclerkapirs)
  - **Files:** `src/pig/clerk/mod.rs` (new), `src/pig/clerk/api.rs` (new),
    `src/pig/clerk/README.md` (new), `src/pig/mod.rs` (modified),
    `src/croft/gate.rs` (modified)
  - **Description:** Implement `PigClerk` wrapping `ItemStore` (Barn). Implement
    `pig_run(config_yaml)` to parse configuration, write `pig/{space}/{name}` to
    Barn, and create initial `Piglet` records concurrently using
    `futures::future::try_join_all`. Implement `pig_stop(space, name)` to set
    desired replicas to 0 and mark stopped in parallel. Implement
    `PigApiService` implementing tonic `PigApiServer` and mount it on the Croft
    Gate router. Create `src/pig/clerk/README.md`.
  - **Definition of Done:**
    - `PigClerk::pig_run` writes Pig and creates initial unplaced Piglet entries
      in Barn.
    - `PigClerk::pig_stop` updates Pig and stops Piglet entries in Barn using
      parallel batch writes.
    - `PigApiService` handles gRPC RPCs and translates errors into tonic status
      codes.
    - Gate router exposes `PigApiServer` alongside existing `CroftApiServer` and
      `BarnApiServer`.
    - Unit tests with mock `ItemStore` verify `pig_run` and `pig_stop` Barn key
      operations.
    - Post-write checklist passes.
  - **Depends on:** T1, T3, T4

- [x] **T6 — Implement Drover Cluster Workload Orchestrator
      (`src/drover/mod.rs`)**
  - **Plan reference:** [`src/drover/mod.rs`](./plan.md#srcdrovermodrs)
  - **Files:** `src/drover/mod.rs` (new), `src/drover/README.md` (new),
    `src/croft/mod.rs` or `src/main.rs` (modified)
  - **Description:** Implement `Drover` orchestrator running on the Primary
    Croft (leader). Implement leader-only reconciliation loop watching `pig/`
    and `croft/` in Barn. Implement
    `schedule(&self, pig: &Pig, count: usize) -> Result<Vec<Yard>, Box<dyn Error>>`
    matching `YardPlacement` against available Croft capacities. Mutate Piglet
    placements and Croft Yard capacity allocations in Barn concurrently using
    `futures::future::try_join_all`. Create `src/drover/README.md`.
  - **Definition of Done:**
    - `Drover::schedule` finds suitable Crofts with available CPU and memory
      capacity based on `PlacementMethod` (`Spread` or `Packed`).
    - Reconciler scales Piglets up/down to match `pig.config.size` and assigns
      `Yard { croft_id, cpu, mem, ... }`.
    - Parallelizes Barn mutations across multiple piglets and crofts via
      `try_join_all`.
    - Unit tests test scheduling logic (capacity exhaustion, multi-node spread).
    - Post-write checklist passes.
  - **Depends on:** T3

- [x] **T7 — Implement Task Engine Abstraction & `ExecEngine`
      (`src/supervisor/task/`)**
  - **Plan reference:**
    [`src/supervisor/task/types.rs`](./plan.md#srccoresystypesrs),
    [`src/supervisor/task/engines/exec.rs`](./plan.md#srcsupervisortaskenginesexecrs--srcsupervisortaskenginesmodrs)
  - **Files:** `src/supervisor/task/types.rs` (new),
    `src/supervisor/task/engines/exec.rs` (new),
    `src/supervisor/task/engines/mod.rs` (new), `src/supervisor/task/mod.rs`
    (new), `src/supervisor/task/README.md` (new)
  - **Description:** Define `TaskEngine` async trait (`name`, `start`, `stop`,
    `status`, `stats`) and `TaskStats` in `types.rs`. Implement `ExecEngine` in
    `engines/exec.rs` backed by `Arc<dyn Sys>` to spawn OS processes with
    command, arguments, env vars, working dir, and capture PID. Implement
    `src/supervisor/task/engines/mod.rs` registering standard engines. Create
    `src/supervisor/task/README.md`.
  - **Definition of Done:**
    - `TaskEngine` trait defined with clear contracts and doc comments.
    - `ExecEngine` implements `start`, `stop` (graceful termination then kill
      timeout), `status` (alive check), and `stats`.
    - Unit tests in `src/supervisor/task/engines/exec.rs` verify process launch,
      env var propagation, and clean process termination.
    - Post-write checklist passes.
  - **Depends on:** T2, T3

- [x] **T8 — Implement Croft Supervisor Actor & Local Task Supervision
      (`src/supervisor/mod.rs`)**
  - **Plan reference:** [`src/supervisor/mod.rs`](./plan.md#srcsupervisormodrs)
  - **Files:** `src/supervisor/mod.rs` (new), `src/supervisor/README.md` (new),
    `src/croft/mod.rs` or `src/main.rs` (modified)
  - **Description:** Implement `Supervisor` maintaining an engine registry
    `engines: HashMap<String, Arc<dyn TaskEngine>>`. Implement `spawn`,
    `with_engines`, `engine(&self, name: &str)`, and background reconciliation
    loop watching `piglet/{self_id}/...` in Barn. Dispatch each task to its
    configured engine via `task.config.engine_name()`, monitor process health,
    detect dead/exited processes, and update `Task` (`TaskStatus`) and `Piglet`
    (`PigletStatus`) in Barn concurrently using `try_join_all`. Create
    `src/supervisor/README.md`.
  - **Definition of Done:**
    - `Supervisor` registers default `ExecEngine` with `sys::select()`.
    - Reconciliation loop starts tasks for assigned piglets, tracks running
      tasks in local state, and detects exits.
    - Updates `TaskStatus` (`Starting`, `Running`, `Stopping`, `Stopped`,
      `Failed`) and `PigletStatus` (`Yarded`, `Starting`, `Running`, `Stopping`,
      `Stopped`, `Failing`, `Failed`) in Barn.
    - Unit tests with mock Barn and test engines verify task spawning and
      failure recovery.
    - Post-write checklist passes.
  - **Depends on:** T3, T7

- [x] **T9 — Implement CLI Workload Subcommands (`src/cli/pig.rs`)**
  - **Plan reference:**
    [`src/cli/pig.rs` & `src/cli/mod.rs`](./plan.md#srcclipigrs--srcclimodrs)
  - **Files:** `src/cli/pig.rs` (new), `src/cli/mod.rs` (modified),
    `src/cli/README.md` (modified)
  - **Description:** Implement CLI command handlers for
    `swini pig run <config_file>` and `swini pig stop <space> <name>`. Read and
    validate the YAML file locally, establish gRPC connection to target Croft
    Gate `PigApi`, invoke RPCs, and print formatted human-readable output and
    error diagnostics. Register the `pig` subcommand under root CLI.
  - **Definition of Done:**
    - `swini pig run` and `swini pig stop` command-line flags and arguments
      parsed with `clap`.
    - Connects to `PigApiServer` over gRPC and submits requests.
    - Unit tests verify CLI argument parsing and error messages.
    - Post-write checklist passes.
  - **Depends on:** T1, T4, T5

- [x] **T10 — Implement End-to-End Workload Execution Tests
      (`tests/pig_execution_test.rs`)**
  - **Plan reference:** [Testing Strategy](./plan.md#testing-strategy)
  - **Files:** `tests/pig_execution_test.rs` (new)
  - **Description:** Write comprehensive integration tests for the full workload
    lifecycle. Test starting local croft nodes, running `swini pig run` with a
    multi-task workload config, asserting Drover schedules Piglets, verifying
    Supervisor starts child processes via `ExecEngine`, confirming Barn status
    transitions to `Running`, and executing `swini pig stop` to verify clean
    process shutdown.
  - **Definition of Done:**
    - Integration tests verify `pig run` end-to-end against a running Croft
      daemon.
    - Integration tests verify `pig stop` tears down running processes.
    - All tests pass via `cargo nextest run -E 'kind(test)'`.
    - Post-write checklist passes.
  - **Depends on:** T5, T6, T8, T9

## Task Order

- **Parallel Foundation (T1, T2, T3):**
  - T1 (Protobuf), T2 (Host Sys abstraction), and T3 (Domain models) are
    independent and can be implemented in parallel.
- **Config & Serialization (T4):**
  - T4 depends on T3 domain models.
- **Front-Office & Orchestration (T5, T6):**
  - T5 (Pig Clerk) can start as soon as T1, T3, and T4 are done.
  - T6 (Drover) can start as soon as T3 is done.
- **Execution & Supervision (T7, T8):**
  - T7 (Task Engines) depends on T2 (Sys) and T3 (Domain models).
  - T8 (Supervisor) depends on T7 and T3.
- **CLI (T9):**
  - T9 (CLI subcommands) can start as soon as T1, T4, and T5 are done.
- **End-to-End Testing (T10):**
  - T10 integration tests run after all components (T5, T6, T8, T9) are
    integrated.
