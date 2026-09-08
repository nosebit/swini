# The Barn Store

Barn is swini's replicated key/value item store: the backend meant to implement
the [`RanchStore`](../ranch.rs) trait (which in turn combines
[`ItemStore`](../item.rs) and [`SpreadStore`](../spread.rs), both defined in
`src/store/`). "Barn" is just this implementation's name — the traits it's built
for don't know or care that it's raft-based, so in principle another
implementation could back the same traits differently.

Concretely, Barn is a [Raft](https://raft.github.io/) cluster: living entity
records are replicated across every voting member so the store keeps working
(and keeps agreeing on the same data) as long as a majority of nodes are up. The
actual Raft mechanics — leader election, log replication, node-to-node RPCs —
live under [`raft/`](raft/README.md). This module is the layer on top: what gets
replicated, and how it's durably applied.

## How it works

1. A client wants to change data (set/delete/patch an item) and proposes an
   [`Action`](types.rs) to the cluster.
2. That action is appended to the Raft log and replicated to a quorum of nodes —
   this part is entirely [`raft/`](raft/)'s job, driven by the
   `openraft::Raft<TypeConfig>` handle that [`raft::create`](raft/mod.rs)
   builds.
3. Once committed, every node applies the entry to its own copy of
   [`Storage`](storage.rs) (the state machine), which durably writes the change
   to its local `redb` database and returns an [`ActionResult`](types.rs).
4. Reads ([`ReadAction`](types.rs)/[`ReadResult`](types.rs)) aren't part of the
   replicated log at all — they're answered directly from a node's local
   `Storage`. [`Barn::get`](mod.rs)/`list` (the default, consistent path) proxy
   to whichever node [`Barn`](mod.rs) currently believes is the raft leader —
   locally if that's this node, over gRPC otherwise. [`Barn::stale_get`](mod.rs)
   instead proxies to a randomly picked member, possibly this node itself,
   trading consistency for not needing a leader round trip.

Two separate `redb` databases are involved per node, deliberately kept apart:
the Raft log itself (entries, saved vote, purge watermark — owned by
[`raft::storage::LogStorage`](raft/storage.rs)) and the application's actual
item data (owned by this module's [`Storage`](storage.rs)). openraft's
`storage-v2` API (enabled in `Cargo.toml`) is what allows splitting these into
independent stores instead of one combined `RaftStorage` — see the
[raft README](raft/README.md) for why that split exists and how the two pieces
are wired together in `raft::create`.

## Current status

Barn is load-bearing: [`Barn`](mod.rs) implements `RanchStore` (in turn
`ItemStore` + `SpreadStore`), and [`Barn::spawn`](mod.rs) assembles a running
node — opening [`Storage`](storage.rs), calling [`raft::create`](raft/mod.rs),
and starting a background task that keeps [`Barn`](mod.rs)'s node list current
and forwards raft/storage activity as [`Event`](events.rs)s.
[`src/api`](../../api/README.md) hosts the `BarnApi` gRPC _server_
(`BarnApiHandler`) that answers the RPCs [`raft/network.rs`](raft/network.rs)'s
client side dials, so Barn nodes can now actually talk to each other in
production. Cluster membership (`node_add`/`node_remove`) and forwarding
requests to whichever node currently needs to answer them (the leader for
consistent reads/writes, a random member for stale reads) are implemented per
the module's own `spawn`/trait-implementation code in [`mod.rs`](mod.rs).

Wiring `Barn::spawn`/`Api::listen` into the running daemon's startup sequence is
not yet done — that's a follow-up. Neither is a "worker" Barn variant with no
local raft participation (see [`mod.rs`](mod.rs)'s `node_cache`-related comments
for why the current design already anticipates one).

## Files in this module

| File                       | What's in it                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`mod.rs`](mod.rs)         | [`Barn`](mod.rs): the concrete `RanchStore`/`ItemStore`/`SpreadStore` implementation, its `spawn` constructor, and the private routing/forwarding logic (`get`/`set`/`stale_get`, leader resolution, the raft-metrics-and-storage-events-to-`Event`s background task). `config`, `events`, `raft`, `storage`, and `types` stay private submodules — `Config`, `Event`, `Node`, `Action`, `ActionResult`, `ReadAction`, `ReadResult` are the only names re-exported flat at the `barn::` level. **Start here** if you want to know how a Barn node actually works end to end.                                                                |
| [`config.rs`](config.rs)   | [`Config`](config.rs): the top-level config for a Barn node — `id`, `addr`, `data_dir`, and inlined Raft consensus timers (`heartbeat_interval`, `election_timeout_min`, `election_timeout_max`).                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| [`events.rs`](events.rs)   | [`Event`](events.rs): the flat event enum `Barn` emits over `Store::subscribe` (`ItemCreated`/`ItemPatched`/`ItemRemoved`/`NodeAdded`/`NodeRemoved`/`NodeChanged`), plus its `TryFrom` conversions into the generic `ItemStoreEvent`/`SpreadStoreEvent` shapes `RanchStore::RanchEvent` requires.                                                                                                                                                                                                                                                                                                                                           |
| [`types.rs`](types.rs)     | The vocabulary Barn's state machine speaks: [`Action`](types.rs) (`Set`/`Delete`/`Patch` — what gets proposed and replicated) and its [`ActionResult`](types.rs); [`ReadAction`](types.rs) (`Get`/`List`) and its [`ReadResult`](types.rs). Also declares [`TypeConfig`](types.rs) via openraft's `declare_raft_types!` macro — the concrete `RaftTypeConfig` (`D = Action`, `R = ActionResult`, `Node = raft::Node`) that every other piece of Barn (and of `raft/`) is generic over `C: RaftTypeConfig` for, but that production code actually instantiates with. **Start here** if you want to know what a Barn node can be asked to do. |
| [`storage.rs`](storage.rs) | [`Storage`](storage.rs): the Raft _state machine_ for `TypeConfig` (`RaftStateMachine` + `RaftSnapshotBuilder`). Applies committed `Action`s to a `redb` database (one table for app data, one for metadata like the last-applied log id and membership), and builds/installs snapshots for followers that fall behind. Also answers local `ReadAction`s directly (`read`) and broadcasts an `ItemStoreEvent` every time a `Set`/`Patch`/`Delete` is durably applied (`subscribe`). This is where "committed" becomes "durably stored and readable." **Start here** if you want to know what happens to data once Raft has agreed on it.    |
| [`raft/`](raft/README.md)  | The actual Raft engine wiring: log storage, node-to-node network transport, and `create()`, which assembles all of the above into a running `openraft::Raft` instance. Has its own [README](raft/README.md) — **start there** if you want to know how consensus/replication itself works.                                                                                                                                                                                                                                                                                                                                                   |
