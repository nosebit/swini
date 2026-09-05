# Swini Development Guide

The Swini source code is organized into the following modules:

- **[`cli/`](./cli/README.md)**: Command-line interface definitions and command
  routing (`swini regent start|stop|status`).
- **[`core/`](./core/README.md)**: Foundational telemetry and protobuf types.
- **[`croft/`](./croft/README.md)**: Host operational compound uniting machine
  identity, configuration, storage `Barn`, network `Gate`, and domain `Clerk`.
- **[`regent/`](./regent/README.md)**: Host supervisor process managing local
  Croft lifecycle, process detachment, signal handling, and state persistence
  (`regent.json`).
- **[`store/`](./store/README.md)**: Distributed and local storage engines
  (including Raft consensus engine `Barn`).

In essence, Swini operates a cluster of machines as a **Ranch**, where each
machine acts as an operational **Croft** (housing its identity, **Barn**, and
**Gate**) supervised by a **Regent** and staffed by domain workers such as the
**Croft Clerk**.

## Building & Running

Swini uses [Just](https://just.systems/man/en/) and standard cargo tooling:

```bash
cargo build
```

## Testing

Run unit and integration tests via `cargo nextest`:

```bash
cargo nextest run
```
