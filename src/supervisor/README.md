# Supervisor Subsystem

The `supervisor` subsystem manages local node task supervision and lifecycle
operations across all Crofts.

## Submodules & Components

- **[`mod.rs`](./mod.rs)**: Defines [`Supervisor`](./mod.rs), which:
  - Discovers assigned replicas ([`Piglet`](../pig/types.rs)) on the local Croft
    (`piglet/{self_croft_id}/...`).
  - Resolves appropriate task drivers via the task engine registry
    ([`TaskEngine`](./task/types.rs)).
  - Launches processes, monitors process telemetry and liveness, restarts
    crashed persistent services, and reports task status back to Barn.
- **[`task/`](./task/README.md)**: Driver abstractions and concrete task
  engines:
  - **[`task/types.rs`](./task/types.rs)**: Core [`TaskEngine`](./task/types.rs)
    trait and [`TaskStats`](./task/types.rs).
  - **[`task/engines/exec.rs`](./task/engines/exec.rs)**:
    [`ExecEngine`](./task/engines/exec.rs) native OS process driver.
