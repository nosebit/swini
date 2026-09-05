---
id: 0023
author: @brunomacf
created: 2026-09-05
---

# [Goal] Unify Plot into Croft

Swini operates a cluster of machines as a **Ranch**, where each individual machine represents an operational farmstead with compute resources, local consensus storage (**Barn**), and network access (**Gate**).

Previously, Swini separated the concept of the machine into two distinct layers: a **Plot** (representing the host machine's identity, metadata, and roles) and a **Croft** (a runtime wrapper uniting the Plot, Barn, and Gate). In practice, this separation introduced unnecessary conceptual duplication and confusion—nodes in the cluster were referred to interchangeably as Plots or Crofts, and clerks, storage keys, and network RPCs operated across this blurred boundary.

This feature eliminates the **Plot** abstraction entirely, elevating **Croft** as the single canonical concept representing a host machine in the Ranch. A Croft holds its own identity, configuration, network gateway, and consensus store directly, simplifying the domain model, cluster membership protocol, and operator CLI.

## Requirements

- **Single Host Compound Representation**:
  - Eliminate the standalone `Plot` concept and establish `Croft` as the sole domain representation of a host machine in the Ranch.
  - A Croft directly maintains its unique machine identity (64-bit ID, name, network address), cluster roles (Server, Worker), tags, and join timestamp.
- **Croft Cluster Membership & Storage**:
  - Cluster state stores Croft records directly in the Barn consensus store under `croft/{id}`.
  - Joining nodes register themselves as Crofts within the Ranch, synchronizing membership with peer Crofts.
- **Croft Clerk & Service APIs**:
  - Replace the Plot Clerk with a **Croft Clerk** domain staff worker that manages Croft records, handles registration handshakes, and serves Croft status inquiries.
  - Provide client and gRPC APIs under the Croft domain for querying and discovering Crofts across the Ranch.
- **Regent & CLI Integration**:
  - The Regent host process boots a Croft directly and hires the Croft Clerk to manage local and cluster-wide Croft operations.
  - Status commands and topology reporting present cluster state in terms of Crofts.

## Constraints

- **Scope Boundary**: This feature focuses strictly on consolidating the Plot and Croft domain models, storage records, APIs, and CLI representations. Dynamic resource watching (e.g. host CPU/memory monitoring daemon) is deferred to a subsequent feature.
- **Clean Terminology**: Deprecate and remove all `Plot` references across protobuf definitions, source modules, and documentation in favor of `Croft`.

## Scenarios

**GIVEN** a running Swini Regent
**WHEN** the Regent boots on a host machine
**THEN** it instantiates a local Croft directly with its identity, Barn storage, and Gate network entry, and hires a Croft Clerk to manage Croft operations

**GIVEN** a newly started Croft configured with peer join addresses
**WHEN** it executes a cluster join handshake with a remote peer
**THEN** the remote peer's Croft Clerk persists the new Croft record under `croft/{id}` in the Barn and returns all active Server Crofts

**GIVEN** a cluster with registered Crofts
**WHEN** an operator inspects cluster topology via status commands or APIs
**THEN** the system returns the list of registered Crofts with their names, addresses, roles, and cluster state

<!-- plan-notes (for /mad.plan — not part of the product spec):
- Domain model consolidation:
  - Delete `src/plot/` module and migrate necessary role/record types into `src/croft/`.
  - Rename `PlotRole` to `CroftRole` (`Server`, `Worker`).
  - Introduce `CroftRecord` (serializable raw data struct) stored in Barn under `croft/{id}` (JSON format).
  - Update `Croft` struct (`src/croft/mod.rs`) to hold `id`, `name`, `addr`, `roles`, `tags`, `joined_at` directly alongside `barn`, `gate`, and `config`.
  - Move ID persistence helper (`id_provide`) to `Croft` (persisting to `{data_dir}/croft.id`).
- Protobuf & gRPC service update:
  - Replace `proto/plot.proto` with `proto/croft.proto` (`swini.croft` package).
  - Define `CroftApi` service with `Join(JoinReq) -> JoinRes` and `Status(StatusReq) -> StatusRes`.
  - Message `Croft` in proto representing the wire format / client view of a Croft record.
  - Update `build.rs` to compile `proto/croft.proto`.
- Clerk migration (`src/croft/clerk/`):
  - Rename/relocate `PlotClerk` to `CroftClerk` (`src/croft/clerk/mod.rs` & `api.rs`).
  - Update key prefix from `plot/` to `croft/`.
  - Handler mounts `CroftApiServer` onto `croft.gate`.
- Regent & Lifecycle integration (`src/regent/mod.rs`):
  - Update `dial_join` and outbound handshake to call `CroftApiClient`.
  - Update `PlotClerk::spawn` call to `CroftClerk::spawn`.
- CLI & Documentation updates:
  - Update `src/cli/regent.rs`, `src/README.md`, `src/croft/README.md`.
  - Update unit and e2e tests.
-->
