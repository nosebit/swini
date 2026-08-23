# Swini Development Gude

The swini source code is splitted in the following modules;

- **[`cli/`](./cli/README.md)**: Command-line interface definitions and command
  routing.
- **[`core/`](./core/README.md)**: Shared utilities, tracing, global
  configurations, and telemetry.
- **[`node/`](./node/README.md)**: Local node operations, hardware metric
  collection, and keepalives.
- **[`store/`](./store/README.md)**: Different type of data stores used throught
  the application.

In essence, Swini is composed of several independent subsystems (called
components - the main "actors" in Swini) that communicate with each other
through gRPC and a distributed state machine (Raft) to achieve a shared
distributed state across all the nodes in a cluster. At a high level, the
components are:

- **Drover** (TBD): The workload orchestrator. It transforms user requests into
  `Pigs` (desired state), schedules them across available nodes into `Yards`
  (physical allocation), and actively reconciles the actual state using custom
  engines like docker, exec, etc.
- **Keeper** (TBD): The distributed storage. It stores arbitrary key-value data
  across the cluster using different storage engines like kv and secret engines.
- **Weaver** (TBD): The internal networking layer. It allows pigs to securely
  expose and consume services from other pigs in the cluster.
- **Router** (TBD): The edge networking layer. It allows pigs to securely expose
  services to the outside world wide web.

## Building & Running

Swini uses [Just](https://just.systems/man/en/) and a couple of other tools to
simplify local development. Run the following to make sure you have all the
tools needed tools installed:

```bash
cargo install just just-lsp cargo-nextest cargo-llvm-cov
```

To compile the `swini` binary in debug mode (faster compilation, great for local
testing):

```bash
cargo build
```

This produces an executable at `./target/debug/swini`.

For production-optimized builds:

```bash
cargo build --release
```

This produces an executable at `./target/release/swini`.

## Testing

You can run all tests (unit and e2e) via the command:

```bash
just test

# To run only unit tests run `just test-unit`.
# To run only e2e tests run `just test-e2e`.
# To generate and view test coverage run `just coverage`.
```

## Useful Commands

```bash
# Generate the Swini documentation powered by rustdoc and
# open it in the browser
cargo doc --open
```
