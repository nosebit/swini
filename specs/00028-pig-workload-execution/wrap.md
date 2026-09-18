---
id: 0028
plan: specs/00028-pig-workload-execution/plan.md
todo: specs/00028-pig-workload-execution/todo.md
author: @brunomacf
created: 2026-09-16
---

# [Wrap] Pig Workload Execution and Process Driver

Implementation summary, architectural patterns, and plan deviations for the Pig
workload execution, process supervision, and cluster scheduling subsystem.

## Architectural Patterns & Deviations

### Drover Lifecycle Management & Primary Leadership Coordination (T6)

In the initial draft of [plan.md](./plan.md#srcdrovermodrs), `Drover::spawn` was
designed to run on all Crofts with an internal periodic 500ms loop polling
`barn.is_leader().await`.

To eliminate unnecessary follower polling overhead, avoid failover latency, and
prevent split-brain execution races during cluster leader transitions, this was
refined to an event-driven model:

- `Barn` provides a reactive watch channel via
  [`Barn::watch_is_leader`](../../src/store/barn/mod.rs).
- `Regent` acts as the Primary lifecycle supervisor. When the local `LiveCroft`
  becomes the Primary (Raft leader), Regent spawns `Drover::run()`.
- When the node steps down from leader, Regent immediately aborts the `Drover`
  background task, ensuring zero unplaced reconciliation work on non-Primary
  nodes.

**plan.md updated:** no — preserved in wrap.md to document the evolution from
polling to event-driven supervision.

### UnixSys Driver Naming Conventions (T2)

Method names on [`UnixSys`](../../src/core/sys/variants/unix.rs) were aligned to
the project's `<noun>_<verb>` convention:

- `spawn_process` $\rightarrow$ `process_spawn`
- `terminate_process` $\rightarrow$ `process_terminate`
- `kill_process` $\rightarrow$ `process_kill`
- `check_alive` $\rightarrow$ `process_is_alive`
- `sample_stats` $\rightarrow$ `process_stats`

**plan.md updated:** no — documented in wrap.md.

### End-to-End Test Suite Layout (T10)

In alignment with repository test harness conventions (under
`tests/e2e/main.rs`), workload integration tests were placed in
[`tests/e2e/pig.rs`](../../tests/e2e/pig.rs) and registered in
[`tests/e2e/main.rs`](../../tests/e2e/main.rs) rather than as a standalone
integration binary `tests/pig_execution_test.rs`.

**plan.md updated:** no — recorded in wrap.md to reflect repository integration
testing architecture.
