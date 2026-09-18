# Pig Clerk Subsystem

The `pig::clerk` subsystem provides front-office operations and network API
services for submitting, inspecting, and terminating workloads in Swini.

## Submodules & Components

- **[`mod.rs`](./mod.rs)**: Defines [`PigClerk`](./mod.rs), which interacts
  directly with Barn storage to persist and update [`Pig`](../types.rs) records.
- **[`api.rs`](./api.rs)**: Defines the tonic gRPC [`PigApi`](./api.rs) handler
  mounted onto Croft [`Gate`](../../croft/gate.rs).
