---
id: 13
status: draft # draft | in-progress | done
plan: specs/00013-barn-store/plan.md
author: @brunomacf
created: 2026-08-22
---

# [Todo] Complete the Barn Cluster Store

Tasks to implement the Barn cluster store described in
[plan.md](./plan.md). T1–T6 are independent, mostly small cleanups to
existing files (trait/struct shape changes, a new `Event` type, `Storage`'s
local read path). T7 assembles the `Barn` struct and `spawn` on top of all
of them. T8 adds the trait implementations. T9–T10 build the `api` module
that exposes Barn over gRPC. T11 is the cross-cutting integration test.
T12 brings the module READMEs up to date once everything else is done.

## Tasks

- [ ] **T1 — Drop `name` from `SpreadNode`, `raft::Node`, and `NodeAddReq`**
  - **Plan reference:** [`SpreadNode` / `raft::Node` drop `name`](./plan.md#spreadnode--raftnode-drop-name-srcstorespreadrs-srcstorebarnrafttypesrs-protobarnproto)
  - **Files:** `src/store/spread.rs` (modified), `src/store/barn/raft/types.rs`
    (modified), `proto/barn.proto` (modified)
  - **Description:** Remove `name` from `SpreadNode::new`/drop
    `SpreadNode::name()`, update `raft::Node`'s fields and its `SpreadNode`
    impl to match, and drop `name` from the `NodeAddReq` message.
  - **Definition of Done:**
    - `SpreadNode` trait is `fn new(id: u64, api_addr: String) -> Self` +
      `id()`/`api_addr()` only.
    - `raft::Node` has no `name` field; its `SpreadNode` impl matches the
      plan's snippet exactly (`role: NodeRole::Voter, is_leader: false` on
      construction).
    - `NodeAddReq` in `proto/barn.proto` has only `id`/`api_addr`.
    - `cargo build` regenerates the `BarnApi` bindings from the updated
      proto without errors.
    - Passes the project's Post-Write Checklist.
  - **Depends on:** none

- [ ] **T2 — Remove `join_addresses` from `raft::Config`**
  - **Plan reference:** [`raft::Config`](./plan.md#raftconfig-srcstorebarnraftconfigrs)
  - **Files:** `src/store/barn/raft/config.rs` (modified),
    `src/store/barn/raft/mod.rs` (modified — drop `join_addresses:
    Vec::new()` from `tests::test_config`)
  - **Description:** Drop the now-unnecessary `join_addresses` field per
    the plan's Risks & Open Questions discussion (cluster join moves to a
    future "node" module).
  - **Definition of Done:**
    - `raft::Config` has exactly `heartbeat_interval`,
      `election_timeout_min`, `election_timeout_max`.
    - `raft::create`'s existing tests still pass unmodified apart from the
      test-helper edit.
    - Passes the project's Post-Write Checklist.
  - **Depends on:** none

- [ ] **T3 — Re-export `ClusterStore` from `store`**
  - **Plan reference:** [Architecture — New/updated files](./plan.md#architecture)
  - **Files:** `src/store/mod.rs` (modified)
  - **Description:** Add `pub use cluster::*;` so `ClusterStore` is
    reachable outside `store::cluster` for the first time.
  - **Definition of Done:**
    - `crate::store::ClusterStore` resolves from any module in the crate.
    - Passes the project's Post-Write Checklist.
  - **Depends on:** none

- [ ] **T4 — Update `Config` (`src/store/barn/config.rs`)**
  - **Plan reference:** [`Config`](./plan.md#config-srcstorebarnconfigrs)
  - **Files:** `src/store/barn/config.rs` (modified)
  - **Description:** Rename `BarnConfig` to `Config` and add `node_id: u64`,
    `api_addr: String`, `data_dir: PathBuf`, keeping the existing
    `raft: raft::Config` field.
  - **Definition of Done:**
    - `Config` matches the plan's snippet exactly (field names, types,
      order).
    - Passes the project's Post-Write Checklist.
  - **Depends on:** none

- [ ] **T5 — Implement `barn::Event`**
  - **Plan reference:** [`barn::Event`](./plan.md#barnevent-srcstorebarneventsrs)
  - **Files:** `src/store/barn/events.rs` (new), `src/store/barn/mod.rs`
    (modified — add `mod events;` and `pub use events::Event;`)
  - **Description:** Implement the flat `Event` enum
    (`ItemCreated`/`ItemPatched`/`ItemRemoved`/`NodeAdded`/`NodeRemoved`/
    `NodeChanged`) and its `TryFrom` impls into `ItemStoreEvent<String,
    Vec<u8>>` and `SpreadStoreEvent<raft::Node>`.
  - **Definition of Done:**
    - `Event` and both `TryFrom` impls match the plan's snippet.
    - Unit tests: each `Event` variant round-trips through the matching
      `TryFrom` and returns `Err(())` for the non-matching conversion
      (e.g. `Event::NodeAdded` fails `TryInto<ItemStoreEvent<..>>`).
    - Passes the project's Post-Write Checklist.
  - **Depends on:** none

- [ ] **T6 — `Storage`: local reads + applied-write events**
  - **Plan reference:** [`Storage`: local reads + applied-write events](./plan.md#storage-local-reads--applied-write-events-srcstorebarnstoragers)
  - **Files:** `src/store/barn/storage.rs` (modified)
  - **Description:** Add the `events_tx` broadcast channel + `subscribe`,
    and the `pub(super) fn read(&self, action: ReadAction) -> ReadResult`
    local-read path (`Get` via single lookup, `List` via prefix-filtered
    range scan). Wire `apply_action` to send on `events_tx` after each
    successful `Set`/`Patch`/`Delete` (mapped 1:1 to
    `ItemCreated`/`ItemPatched`/`ItemRemoved`).
  - **Definition of Done:**
    - `read` and `subscribe` are `pub(super)`, not `pub(crate)`.
    - `read(Get)` returns the stored value, `None` for a missing key.
    - `read(List)` returns only keys matching the given prefix.
    - Unit tests: `read` covers present/absent/prefix cases; applied-write
      event emission is covered for `Set`→`ItemCreated`,
      `Patch`→`ItemPatched`, `Delete`→`ItemRemoved`, alongside the
      existing `apply`/snapshot tests in this file.
    - Passes the project's Post-Write Checklist.
  - **Depends on:** none

- [ ] **T7 — `Barn` struct, `spawn`, and private helpers**
  - **Plan reference:** [`Barn`](./plan.md#barn-srcstorebarnmodrs)
  - **Files:** `src/store/barn/mod.rs` (modified — module
    declarations/re-exports, `Barn` struct, `spawn`, `scoped_key`,
    `nodes_from_metrics`, `is_leader`, `current_leader_node`, `raft`
    accessor, `dial`, `local_read`, `local_write`, `forward_read`,
    `forward_write`, `stale_get`, `random_node`, `event_forwarder_start`),
    `Cargo.toml` (modified — add `rand` dependency)
  - **Description:** Assemble the `Barn` struct and its `spawn`
    constructor (opens `Storage`, calls `raft::create`, seeds `node_cache`
    synchronously from the first metrics snapshot, starts
    `event_forwarder_start`), plus every private helper the trait
    implementations in T8 will call. Include the `pub(crate) fn
    raft(&self) -> &openraft::Raft<TypeConfig>` accessor `api::barn` needs
    (per the plan's note right after the `BarnApiHandler` snippet).
  - **Definition of Done:**
    - `mod config;`/`mod events;`/`mod raft;`/`mod storage;`/`mod types;`
      stay private; `Config`, `Event`, `Node`, `NodeRole`, `Action`,
      `ActionResult`, `ReadAction`, `ReadResult` are re-exported flat —
      nothing outside `store::barn` can write `store::barn::raft::..` or
      `store::barn::types::..`.
    - `spawn` returns a `Barn` whose `node_cache` is already populated
      (not empty) immediately after `spawn` returns, for a freshly
      bootstrapped node.
    - `is_leader`/`current_leader_node`/`random_node`/`nodes_from_metrics`
      read/derive from `node_cache`, never call `self.raft.metrics()`
      directly (only `event_forwarder_start` and `node_add`/`node_remove`
      touch `raft.metrics()`/the raft handle directly).
    - `event_forwarder_start` both refreshes `node_cache` and emits
      `NodeAdded`/`NodeRemoved`/`NodeChanged`/`ItemCreated`/`ItemPatched`/
      `ItemRemoved` `Event`s, matching the plan's snippet.
    - Unit tests: `is_leader`/`current_leader_node` against a seeded
      `node_cache` (leader present, no leader yet); `random_node` picking
      self vs. a peer.
    - Passes the project's Post-Write Checklist.
  - **Depends on:** T1, T4, T5, T6

- [ ] **T8 — Trait implementations for `Barn`**
  - **Plan reference:** [Trait implementations](./plan.md#trait-implementations-srcstorebarnmodrs)
  - **Files:** `src/store/barn/mod.rs` (modified — append `impl Store`,
    `impl ItemStore`, the `write` helper, `impl SpreadStore`, `impl
    ClusterStore`)
  - **Description:** Implement `Store`, `ItemStore` (`slice`/`get`/`set`/
    `delete`/`list`), `SpreadStore` (`node`/`node_list`/`node_add`/
    `node_remove`/`node_cache_set`), and `ClusterStore` for `Barn`, exactly
    per the plan's snippet.
  - **Definition of Done:**
    - `get`/`list`/`write` route locally when `is_leader().await` is true
      and forward to the resolved leader otherwise, returning an error
      when no leader is known in `node_cache`.
    - `node_add` bootstraps via `raft.initialize` when membership is
      empty, and add-learner-then-promote-to-voter otherwise.
    - `node_remove` demotes-then-removes.
    - `node_cache_set` overwrites `node_cache` wholesale.
    - `slice` returns a clone with the prefix extended, and `list` strips
      `self.prefix` back off returned keys.
    - Unit tests: `node_add`'s two branches (bootstrap vs. add-learner +
      promote), using the same in-memory `redb` + real `openraft::Raft`
      pattern `raft/mod.rs`'s existing tests use; leader-aware routing
      (`get`/`set` served locally when leader, "no known leader yet"
      error when `node_cache` is empty).
    - Passes the project's Post-Write Checklist.
  - **Depends on:** T7, T3

- [ ] **T9 — `api::barn::BarnApiHandler`**
  - **Plan reference:** [`api::barn::BarnApiHandler`](./plan.md#apibarnbarnapihandler-srcapibarnrs)
  - **Files:** `src/api/barn.rs` (new)
  - **Description:** Implement the `BarnApi` gRPC service: `append_entries`/
    `vote`/`install_snapshot` decode into openraft's request types and call
    `self.barn.raft()`'s matching inherent method (same `serde_json`-in-
    `BarnMessage` envelope as `NetworkConnection::encode`/`decode` in
    `raft/network.rs`); `forward_write`/`forward_read` decode into
    `Action`/`ReadAction` and call `Barn::local_write`/`local_read`;
    `node_add` decodes `NodeAddReq` and calls `SpreadStore::node_add`.
  - **Definition of Done:**
    - All six `BarnApi` methods are implemented per the plan's snippet;
      none return `unimplemented`.
    - `encode`/`decode` helpers reuse the same wire format as
      `NetworkConnection` in `raft/network.rs` (opaque `serde_json`
      payload in `BarnMessage`).
    - Unit tests: `encode`/`decode` round-trip for at least one raft
      request/response type and one `Action`/`ActionResult` pair.
    - Passes the project's Post-Write Checklist.
  - **Depends on:** T8

- [ ] **T10 — `api::Api`**
  - **Plan reference:** [`api::Api`](./plan.md#apiapi-srcapimodrs)
  - **Files:** `src/api/mod.rs` (new), `src/main.rs` (modified — add `mod
    api;`)
  - **Description:** Implement `Api::new(barn: Arc<Barn>)` (constructs
    `BarnApiHandler`) and `Api::listen(&self, addr: SocketAddr)` (starts a
    `tonic::transport::Server` with the `BarnApiServer` registered),
    structured so a second service can be added later with one more field
    and one more `.add_service(...)` call.
  - **Definition of Done:**
    - `Api::listen` binds and serves `BarnApi` on the given address.
    - `src/main.rs` declares `mod api;`.
    - Passes the project's Post-Write Checklist.
  - **Depends on:** T9

- [ ] **T11 — Integration test: multi-node Barn cluster over real gRPC**
  - **Plan reference:** [Testing Strategy — Integration tests](./plan.md#testing-strategy)
  - **Files:** `src/store/barn/mod.rs` (modified — `#[cfg(test)]` module)
  - **Description:** Following the `spawn_mock_server` pattern already in
    `raft/network.rs`'s tests, but backed by a real `BarnApiHandler` +
    `Api::listen` on `127.0.0.1:0` instead of `MockBarnApi`: form a
    two-or-three-node cluster via `node_add`, write on one node and read
    it from another via `get`, exercise `stale_get` against a follower,
    and assert a `NodeAdded`/`NodeRemoved`/item `Event` actually arrives
    on a subscriber.
  - **Definition of Done:**
    - Test spins up real `Barn` + `Api` instances communicating over real
      local TCP sockets (no mocked transport).
    - Covers: write-then-read-from-another-node convergence, `stale_get`
      returning a follower's local view, at least one `NodeAdded`/
      `NodeRemoved` event and one item event observed via `subscribe`.
    - `cargo nextest run -E 'kind(lib) | kind(bin)'` passes (this is a
      `#[cfg(test)]` module, not a `tests/` e2e binary, per the plan's
      testing-strategy note that this feature has no CLI-facing surface).
    - Passes the project's Post-Write Checklist.
  - **Depends on:** T8, T9, T10

- [ ] **T12 — Update module documentation**
  - **Plan reference:** [Architecture — New/updated files](./plan.md#architecture)
  - **Files:** `src/store/barn/README.md` (modified), `src/store/barn/raft/README.md`
    (modified), `src/api/README.md` (new)
  - **Description:** Update the "Current status" sections of the two Barn
    READMEs to reflect that `Barn` now implements `ClusterStore` and is
    load-bearing (no longer "foundational, not yet load-bearing"), and add
    a new module README for `src/api/` describing `Api`/`BarnApiHandler`
    per the project's module-README convention.
  - **Definition of Done:**
    - `src/store/barn/README.md`'s "Current status" and "Files in this
      module" sections accurately describe the post-implementation state
      (no stale claims that nothing implements `ClusterStore`/
      `SpreadStore`/`ItemStore`, or that there's no `BarnApi` server).
    - `src/store/barn/raft/README.md` no longer states "only the client
      side exists" if that's no longer true.
    - `src/api/README.md` exists and describes the module's intent and
      each file in it, per lore.md's module-README convention.
  - **Depends on:** T7, T8, T9, T10

## Task Order

T1, T2, T3, T4, T5, T6 touch disjoint files and have no dependencies on
each other — they can all be done in parallel (by different engineers or
agents) without conflicting. T2 and T3 aren't hard prerequisites for
anything else in this list but are part of the same plan and should land
before T12's documentation pass.

T7 is the first task that needs several others finished (T1, T4, T5, T6)
since it assembles `Barn` on top of all of them — it can't start until
those land. T8 (trait impls) follows T7 directly and also needs T3
(`ClusterStore` reachable). T9 → T10 are strictly sequential (T10's
`Api::mod.rs` declares `mod barn;` for T9's file). T11 needs the full
stack (T8, T9, T10) since it's a real end-to-end test. T12 should be done
last, once T7–T10 have landed, so the READMEs describe the actual final
state rather than an intermediate one.
