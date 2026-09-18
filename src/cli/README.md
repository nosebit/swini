# CLI Module

Command-line interface definitions and dispatch:

- **[`croft.rs`](./croft.rs)**: Scoped CLI subcommands (`status`) and execution
  handler for inspecting specific Croft metadata, Barn resource allocations, and
  live host telemetry.
- **[`mod.rs`](./mod.rs)**: Root CLI entrypoint (`Cli`, `Command`), top-level
  dispatch, and endpoint resolution via `resolve_api_url` supporting
  `SWINI_ADDR` with `local://{name}` URI lookup.
- **[`pig.rs`](./pig.rs)**: Scoped CLI subcommands (`run`, `stop`) for
  deploying, inspecting, and terminating declarative Pig workloads across the
  Ranch.
- **[`regent.rs`](./regent.rs)**: Scoped CLI subcommands (`start`, `stop`,
  `status`) and execution handler for managing the local Regent runtime process.
- **[`status.rs`](./status.rs)**: Root CLI command (`swini status`) querying
  cluster-wide Crofts and rendering a formatted summary table.
