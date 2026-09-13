# Core Module

Foundational infrastructure, formatting utilities, configuration, and generated
definitions shared across Swini:

- **[`format.rs`](./format.rs)**: Reusable metric formatting for frequencies
  (`format_hertz`) and byte sizes (`format_bytes`).
- **[`telemetry.rs`](./telemetry.rs)**: Structured tracing and rolling daily log
  file appenders.
- **[`proto/`](./proto/)**: Generated gRPC service traits and protobuf types.
