# Drover Subsystem

The `drover` subsystem acts as the cluster-level workload orchestrator and
scheduler running exclusively on the Primary Croft (Raft consensus leader).

## Components

- **[`mod.rs`](./mod.rs)**: Defines [`Drover`](./mod.rs), which:
  - Continuously reconciles cluster workload declarations
    ([`Pig`](../pig/types.rs)) against active replica states
    ([`Piglet`](../pig/types.rs)) in Barn.
  - Implements the [`schedule()`](./mod.rs) algorithm to find candidate Crofts
    matching resource requirements and tags according to the specified
    [`PlacementMethod`](../pig/types.rs) (`Spread` vs `Packed`).
  - Carves out [`Yard`](../pig/types.rs) allocations and synchronizes Croft
    resource reservations in parallel.
  - Lifecycle is managed dynamically by the Regent daemon via
    [`Barn::watch_is_leader`](../store/barn/mod.rs), starting when the node
    becomes Primary and terminating immediately upon step-down.
