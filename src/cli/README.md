# CLI Module

Command-line interface definitions and dispatch:

- **[`mod.rs`](./mod.rs)**: Root CLI entrypoint (`Cli`, `Command`), top-level
  dispatch, and endpoint resolution via `resolve_api_url` supporting
  `SWINI_ADDR` with `local://{name}` URI lookup.
- **[`regent.rs`](./regent.rs)**: Scoped CLI subcommands (`start`, `stop`,
  `status`) and execution handler for managing the local Regent runtime process.
