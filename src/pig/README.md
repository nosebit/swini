# Pig Module

The `pig` module defines workload domain models, configuration schemas, and
front-office services for deploying and managing workloads in Swini.

## Submodules & Components

- **[`types.rs`](./types.rs)**: Core domain models for workloads:
  - **[`Pig`](./types.rs)**: Cluster-level workload definition and desired
    state.
  - **[`Piglet`](./types.rs)**: Individual workload replica placed on a target
    Croft.
  - **[`Yard`](./types.rs)**: Dedicated resource reservation bounds for a
    Piglet.
  - **[`Task`](./types.rs)**: Monitored process execution state within a Piglet.
  - **[`Placement`](./types.rs)**: Scheduling rules, Croft tags, and resource
    sizing.
- **[`config.rs`](./config.rs)**: Declarative YAML configuration parser and
  normalizer ([`PigRunConfig`](./config.rs), [`HerdConfig`](./config.rs),
  [`PigDefaultsConfig`](./config.rs)).
- **[`clerk/`](./clerk/README.md)**: Front-office clerk and `PigApi` gRPC
  service implementation ([`PigClerk`](./clerk/mod.rs)).
