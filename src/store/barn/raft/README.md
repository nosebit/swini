# Raft (openraft wiring)

This module wires [`openraft`](https://docs.rs/openraft/0.9.25/openraft/)
into Barn. openraft is a Rust library implementing the
[Raft consensus algorithm](https://raft.github.io/) — it drives leader
election and log replication for you, but it has no idea how to actually
store anything or talk to another node over the network. Those are extension
points the application (this module) has to fill in. That's what every file
here is: one piece of the "fill in the blanks" openraft asks for.

## What Raft actually does (short version)

A Raft cluster is a set of nodes that agree on a single, ordered log of
entries, and apply that log to identical local state machines in the same
order, so every node ends up with the same data. At any time at most one
node is the **leader**; everyone else is a **follower** (or a non-voting
**learner**, kept up to date but not counted toward quorum). Clients only
ever propose new entries to the leader. The leader appends the entry to its
own log and replicates it to followers; once a **majority (quorum)** of
voting nodes have durably persisted it, the entry is **committed** and safe
to apply. If the leader disappears (crash, network partition), the
remaining nodes notice via a randomized election timeout, and one of them
campaigns for a new **term** and, if it gets votes from a majority, becomes
the new leader. That's the whole mechanism swini gets "for free" from
openraft — this module's job is to give it a place to persist things and a
way to reach other nodes.

## The extension points openraft needs, and who implements them here

openraft calls into the application through a handful of traits, generic
over one `C: RaftTypeConfig`. Barn's concrete binding for that generic is
[`TypeConfig`](../types.rs) (declared one level up, in `store/barn/types.rs`
— see the [barn README](../README.md)), so most of the code in this module
is written generically against `C` but is only ever instantiated in
production with `TypeConfig`.

| openraft trait | What it's for | Implemented by |
| --- | --- | --- |
| `RaftLogStorage` + `RaftLogReader` | Durably persist the Raft log itself: entries, the last vote cast, the purge watermark. | [`LogStorage`](storage.rs) in `storage.rs` |
| `RaftStateMachine` + `RaftSnapshotBuilder` | Apply *committed* entries to durable application data, and build/install snapshots. | [`Storage`](../storage.rs) — lives one level up, in `store/barn/storage.rs`, since it's Barn's application logic, not Raft machinery. See the [barn README](../README.md). |
| `RaftNetworkFactory` + `RaftNetwork` | Send the RPCs Raft needs (`AppendEntries`, `Vote`, `InstallSnapshot`) to another node. | [`NetworkFactory`](network.rs) / [`NetworkConnection`](network.rs) in `network.rs` |
| `RaftTypeConfig` | The single generic binding tying concrete types together (`NodeId`, `D` = write payload, `R` = write response, `Node`, `Entry`, `SnapshotData`, ...). | [`TypeConfig`](../types.rs), declared in `store/barn/types.rs` via `declare_raft_types!` |

Why the log and the state machine are two separate implementations in two
separate places: openraft's `storage-v2` API (the `storage-v2` feature
enabled in `Cargo.toml`) splits what used to be one `RaftStorage` trait into
`RaftLogStorage` and `RaftStateMachine` specifically so they *can* be backed
by completely different storage. Barn takes advantage of that: `create()`
below opens its own dedicated `redb` database for the Raft log, while the
state machine's `redb` database is opened separately by whoever constructs
[`Storage`](../storage.rs) and handed to `create()` already built.

## Files in this module

- **[`mod.rs`](mod.rs)** — the entrypoint. `create::<C, SM>(node_id, config,
  db_path, state)` is the one function everything else in this module exists
  to support: it builds an `openraft::Config` from [`Config`](config.rs)
  (validating the heartbeat/election timing), opens the Raft log's `redb`
  database at `db_path` with a `.raft` extension, constructs
  [`LogStorage`](storage.rs) and [`NetworkFactory`](network.rs), and calls
  `openraft::Raft::new(...)` with the state machine (`state: SM`) the caller
  already built. The result is a live `Raft<C>` handle — the thing a future
  Barn node actually holds and calls `.client_write(...)` /
  `.client_read(...)` etc. on. Also re-exports `config::*` and `types::*`,
  so callers just need `raft::create`, `raft::Config`, `raft::Node`.
  **Start here** to see how all the other pieces get assembled.

- **[`config.rs`](config.rs)** — [`Config`](config.rs): the tunable knobs fed
  straight into `openraft::Config` (`heartbeat_interval`,
  `election_timeout_min`/`max`), plus `join_addresses` — the peer addresses a
  new node would contact to join an existing cluster. `join_addresses` isn't
  consumed anywhere yet; joining a running cluster (as opposed to bootstrapping
  a fresh one) is still open work.

- **[`types.rs`](types.rs)** — the identity/metadata types for a cluster
  *member*, as opposed to the application data it stores:
  - [`NodeId`](types.rs): a type alias for `u64`. Not currently referenced
    elsewhere (code that needs a node id gets it via `C::NodeId` on
    `TypeConfig` instead), but documents what a node id *is*.
  - [`NodeRole`](types.rs): `Learner` (replicates data, doesn't vote) or
    `Voter` (counts toward quorum, participates in elections).
  - [`Node`](types.rs): what the cluster knows about one member — id, name,
    `api_addr` (where [`network.rs`](network.rs) dials it), role, and
    whether it's currently believed to be the leader. Implements
    [`SpreadNode`](../../spread.rs) so Barn can eventually plug into the
    generic `SpreadStore` trait. This is openraft's `RaftTypeConfig::Node`
    type.

- **[`storage.rs`](storage.rs)** — [`LogStorage<C>`](storage.rs): the Raft
  log itself, in a `redb` database with two tables — log entries keyed by
  index, and metadata (last purged log id, saved vote). Implements
  `RaftLogStorage` (the write path: `append`, `save_vote`, `truncate`,
  `purge`, `get_log_state`) and `RaftLogReader` (the read path used by
  per-follower replication tasks: `try_get_log_entries`). Two operations are
  easy to confuse: `truncate` deletes the tail of the log to resolve a
  conflict with the leader's log (normal operation, before entries commit);
  `purge` deletes the *head* of the log once entries are already applied and
  no longer needed for replication (routine compaction, after entries
  commit). **Start here** for exactly how/where the Raft log is stored on
  disk.

- **[`network.rs`](network.rs)** — the client side of node-to-node RPC, over
  the generated `BarnApi` gRPC service (see `proto/barn.proto` and
  [`core/proto/barn.rs`](../../../core/proto/barn.rs)).
  [`NetworkFactory<C>`](network.rs) (impl of `RaftNetworkFactory`) builds a
  lazily-connecting gRPC channel to a peer's `api_addr`, without blocking or
  failing if that peer is unreachable right now (per openraft's contract —
  connectivity failures surface later, as RPC errors). The resulting
  [`NetworkConnection<C>`](network.rs) (impl of `RaftNetwork`) is what
  actually calls `append_entries`/`vote`/`install_snapshot`: each request is
  JSON-serialized and wrapped in an opaque `BarnMessage { payload }` envelope
  (`encode`/`decode` helpers) rather than mapped field-by-field onto a
  protobuf message, specifically so this wire contract never needs a
  hand-update just because an openraft request/response type gains a field.
  **Important:** only the client side exists here — there is no `impl BarnApi
  for ...` server anywhere in the codebase yet, so nothing currently answers
  these RPCs in production. The tests in this file spin up a minimal mock
  `BarnApi` server (over a real local TCP socket) purely to exercise this
  client against something.
