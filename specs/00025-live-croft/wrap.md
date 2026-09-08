---
id: 0025
plan: specs/00025-live-croft/plan.md
todo: specs/00025-live-croft/todo.md
author: @brunomacf
created: 2026-09-05
---

# [Wrap] Croft Data Entity and LiveCroft Runtime Model

Implementation summary and architectural pattern documentation for the `Croft`
data entity, `LiveCroft` runtime specialization, and `ItemStore` / `RanchStore`
abstractions.

## Architectural Patterns

### The `Live<Entity>` Pattern

In Swini, persistent entities across the Ranch (such as `Croft`) represent pure
serializable state. When an entity is actively instantiated on the local host
machine with live runtime infrastructure (like network gates, background
consensus engines, or supervision handles), it is modeled as `Live<Entity>`
(e.g. `LiveCroft`).

`Live<Entity>` embeds `base: <Entity>` and implements
`std::ops::Deref<Target = <Entity>>` to provide seamless subtyping:

- All base fields and methods are directly accessible (`live_croft.id`,
  `live_croft.name`, `live_croft.is_server()`).
- Any function accepting `&<Entity>` automatically accepts `&Live<Entity>` via
  Deref coercion.
- Storage records, cluster queries, and wire messages deal exclusively with the
  primary entity name (`Croft`).

### Storage Metaphor

- `ItemStore` provides generic key-value storage (`ItemCreated`, `ItemPatched`,
  `ItemRemoved`).
- `RanchStore` combines `ItemStore` with `SpreadStore`.
- The `Barn` implements `RanchStore`, acting as the replicated ledger holding
  `Croft` records under `croft/{id}`.
