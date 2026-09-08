---
id: 0025
author: @brunomacf
created: 2026-09-05
---

# [Goal] Croft Data Entity and LiveCroft Runtime Model

Swini models a cluster of machines as a **Ranch** populated by living domain
entities called **Crofts**. To maintain a durable, cluster-wide ledger of these
entities, the Ranch relies on the **Barn**—the distributed consensus storage
engine.

Previously, storage abstractions and entity representations suffered from
confusion between passive data entities and active runtime infrastructure:

1. The domain entity blended persistent identity with local runtime handles
   (optional Barn and Gate handles), causing awkward unwraps and cognitive
   friction when distinguishing a remote Croft from the local running machine.
2. Attempting to introduce separate noun concepts (such as Card or Seed) created
   "dual-entity syndrome", where functions and APIs were forced to convert
   between artificial secondary types rather than operating directly on Crofts.

This feature establishes a clean, repeatable architectural pattern for Swini by
resolving the distinction between stored data and its active runtime
counterpart:

1. **The Base Data Entity (`Croft`)**: Represents any Croft across the Ranch—its
   persistent identity, address, assigned roles, tags, and join timestamp. It is
   100% serializable (JSON, Protobuf, Barn KV storage).
2. **The Live Runtime Specialization (`LiveCroft`)**: Represents the active,
   physical host machine on which Swini is executing. A `LiveCroft` extends
   `Croft` via composition (`base: Croft`) and
   `std::ops::Deref<Target = Croft>`, embedding required local consensus storage
   (`barn: Arc<Barn>`) and network gateway (`gate: Gate`).
3. **The `Live<Entity>` Pattern**: Establishes the standard convention across
   Swini that prefixing a domain entity with `Live` represents its active
   runtime execution compound (e.g. `LiveCroft` is the live execution
   environment of a `Croft`).
4. **Clean Generic Key-Value Storage (`ItemStore` & `RanchStore`)**: Maintains
   standard, unpretentious database key-value storage abstractions (`ItemStore`,
   `ItemStoreEvent`, `RanchStore`), where the Barn persists `Croft` records
   under `croft/{id}`.

## Requirements

- **Croft Entity Unification**:
  - `Croft` is the primary domain data entity across the entire system (`id`,
    `name`, `addr`, `roles`, `tags`, `joined_at`, `is_server()`, `is_worker()`).
  - Protobuf messages, Barn storage records (`croft/{id}`), and Clerk queries
    (`list`, `get`, `join`) operate directly on `Croft`.
  - 1-to-1 Protobuf conversion between domain `Croft` and wire `ProtoCroft`
    without intermediate dummy request hacks.

- **`LiveCroft` Runtime Specialization**:
  - `LiveCroft` represents the active local host instance, containing
    `base: Croft`, `pub barn: Arc<Barn>`, and `pub gate: Gate`.
  - Implements `std::ops::Deref<Target = Croft>` so all base fields and methods
    are accessible transparently and `&LiveCroft` automatically coerces to
    `&Croft`.
  - `LiveCroft::spawn` initializes the ID, Barn, Gate, and base `Croft`.
  - `LiveCroft::persist` serializes `self.base` directly into local Barn
    storage.

- **Staff & Supervisor Alignment**:
  - `CroftClerk` operates on `Arc<LiveCroft>` for local infrastructure access,
    and serves `Croft` entities for cluster queries and joins.
  - `Regent` boots `LiveCroft`, hires `CroftClerk`, performs outbound joins with
    `&LiveCroft` (as `&Croft`), and opens `LiveCroft`'s Gate.

- **Storage Trait Ergonomics (`ItemStore` & `RanchStore`)**:
  - Generic storage trait is named `ItemStore` with `type Key` and `type Item`,
    emitting `ItemStoreEvent<K, I>` (`ItemCreated`, `ItemPatched`,
    `ItemRemoved`).
  - `RanchStore` combines `ItemStore<Key = String, Item = Vec<u8>>` and
    `SpreadStore`.

## Constraints

- **Scope Boundary**: Focuses on the `Croft` / `LiveCroft` structural
  unification, `ItemStore` / `RanchStore` alignment, and `CroftClerk` / `Regent`
  coordination.
- **Backward Compatibility**: Internal key naming conventions (`croft/{id}`) and
  Raft consensus mechanics remain unchanged.

## Scenarios

**GIVEN** an active Swini host process **WHEN** the Regent boots the local node
**THEN** it instantiates a `LiveCroft` holding active `Barn` consensus storage
and `Gate` networking alongside its base `Croft` identity

**GIVEN** a running `LiveCroft` with local Barn storage **WHEN** the Croft Clerk
saves or queries Croft records **THEN** the clerk stores and retrieves pure
`Croft` entities from the Barn under `croft/{id}`

**GIVEN** an external node joining the Ranch **WHEN** it sends a join
registration request to an active Croft Clerk **THEN** the Croft Clerk registers
the incoming `Croft` in the Barn, updates Raft consensus membership if it is a
server, and returns all registered server `Croft` entities
