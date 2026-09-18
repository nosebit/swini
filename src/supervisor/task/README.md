# Task Execution Subsystem

The `supervisor::task` subsystem provides driver abstractions and execution
engines for starting, stopping, probing, and sampling workload tasks on the
local host.

## Submodules & Components

- **[`types.rs`](./types.rs)**: Defines the [`TaskEngine`](./types.rs) trait and
  [`TaskStats`](./types.rs) data structure.
- **[`engines/`](./engines/)**: Concrete task engine drivers:
  - **[`engines/exec.rs`](./engines/exec.rs)**:
    [`ExecEngine`](./engines/exec.rs) which delegates native OS process
    management to [`Sys`](../../core/sys/types.rs).
