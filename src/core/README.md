# Core Module

Foundational infrastructure, configuration, and generated definitions shared
across Swini:

- **[`config.rs`](./config.rs)**: Hierarchical configuration resolution
  (`Config`, `regents_dir`, `default_data_dir`).
- **[`telemetry.rs`](./telemetry.rs)**: Structured tracing and rolling daily log
  file appenders.
- **[`proto/`](./proto/)**: Generated gRPC service traits and protobuf types
  (`swini.plot`, `swini.barn`).
