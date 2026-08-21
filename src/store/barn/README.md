# The Barn Store

Barn is swini's replicated key/value store: the backend meant to implement the
[`ClusterStore`](../cluster.rs) trait (which in turn combines
[`ItemStore`](../item.rs) and [`SpreadStore`](../spread.rs), both defined in
`src/store/`). "Barn" is just this implementation's name — the traits it's built
for don't know or care that it's raft-based, so in principle another
implementation could back the same traits differently.

Concretely, Barn is a [Raft](https://raft.github.io/) cluster: application data
is replicated across every voting member so the store keeps working (and keeps
agreeing on the same data) as long as a majority of nodes are up. The actual
Raft mechanics — leader election, log replication, node-to-node RPCs — live
under [`raft/`](raft/README.md). This module is the layer on top: what gets
replicated, and how it's durably applied.

## How it works

1. A client wants to change data (set/delete/patch a key) and proposes an
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
   `Storage`, optionally proxied to the leader when a caller can't tolerate
   stale (follower) data. (This dispatch/proxying isn't implemented yet — see
   "Current status" below.)

Two separate `redb` databases are involved per node, deliberately kept apart:
the Raft log itself (entries, saved vote, purge watermark — owned by
[`raft::storage::LogStorage`](raft/storage.rs)) and the application's actual
key/value data (owned by this module's [`Storage`](storage.rs)). openraft's
`storage-v2` API (enabled in `Cargo.toml`) is what allows splitting these into
independent stores instead of one combined `RaftStorage` — see the
[raft README](raft/README.md) for why that split exists and how the two pieces
are wired together in `raft::create`.

## Current status

This module is foundational, not yet load-bearing: `mod.rs` declares its
submodules privately (no `pub use`), nothing implements `ClusterStore` /
`SpreadStore` / `ItemStore` for Barn yet, and there's no `BarnApi` gRPC _server_
implementation (only the client side, in [`raft/network.rs`](raft/network.rs)) —
so no Barn node can actually talk to another one in production yet.
`raft::create` is fully working and tested, but nothing outside `raft/mod.rs`'s
own tests calls it yet. Treat this module as the pieces a future `Barn` struct
will assemble, not as a ready-to-use store.

## Files in this module

| File                       | What's in it                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`mod.rs`](mod.rs)         | Just module declarations (`config`, `raft`, `storage`, `types`). No public re-exports yet — see "Current status".                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| [`config.rs`](config.rs)   | [`BarnConfig`](config.rs): the top-level config for a Barn node. Currently just wraps [`raft::Config`](raft/config.rs); this is where non-Raft Barn-level settings (if any end up being needed) would go.                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| [`types.rs`](types.rs)     | The vocabulary Barn's state machine speaks: [`Action`](types.rs) (`Set`/`Delete`/`Patch` — what gets proposed and replicated) and its [`ActionResult`](types.rs); [`ReadAction`](types.rs) (`Get`/`List`) and its [`ReadResult`](types.rs). Also declares [`TypeConfig`](types.rs) via openraft's `declare_raft_types!` macro — the concrete `RaftTypeConfig` (`D = Action`, `R = ActionResult`, `Node = raft::Node`) that every other piece of Barn (and of `raft/`) is generic over `C: RaftTypeConfig` for, but that production code actually instantiates with. **Start here** if you want to know what a Barn node can be asked to do. |
| [`storage.rs`](storage.rs) | [`Storage`](storage.rs): the Raft _state machine_ for `TypeConfig` (`RaftStateMachine` + `RaftSnapshotBuilder`). Applies committed `Action`s to a `redb` database (one table for app data, one for metadata like the last-applied log id and membership), and builds/installs snapshots for followers that fall behind. This is where "committed" becomes "durably stored and readable." **Start here** if you want to know what happens to data once Raft has agreed on it.                                                                                                                                                                |
| [`raft/`](raft/README.md)  | The actual Raft engine wiring: log storage, node-to-node network transport, and `create()`, which assembles all of the above into a running `openraft::Raft` instance. Has its own [README](raft/README.md) — **start there** if you want to know how consensus/replication itself works.                                                                                                                                                                                                                                                                                                                                                   |
