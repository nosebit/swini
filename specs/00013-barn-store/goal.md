---
id: 13
author: @brunomacf
created: 2026-08-22
---

# [Goal] Complete the Barn Cluster Store

Swini already has the individual pieces of the Barn — the raft consensus engine,
the durable state-machine storage, and the trait definitions Barn is meant to
fulfill — but nothing assembles them into a store that can actually be started
up and used, and Barn nodes cannot yet reach each other over the network. This
feature finishes the Barn: it becomes a concrete, runnable cluster store that
other parts of Swini can spawn, that keeps every node's copy of cluster data
consistent, that can be watched for changes in real time, and that is reachable
by other nodes over the network.

## Requirements

- **Startable node**: Given a node's identity and where to store its data, a
  Barn node can be started up (spawned) and immediately participates in the
  cluster's consensus and storage.
- **Full cluster store behavior**: Once running, a Barn node supports the
  complete set of operations expected of Swini's cluster store — reading,
  writing, patching, and deleting cluster data, and managing which nodes are
  part of the cluster.
- **Consistent reads by default**: Reading a key returns the most recently
  committed value by default, never a possibly-stale local copy.
- **Optional fast/stale reads**: Callers who don't need the very latest value
  and prefer lower latency can explicitly opt into reading from a nearby node
  instead of paying the cost of a consistency check.
- **Live change notifications**: Clients can subscribe to a Barn node to be
  notified in real time both when cluster data changes and when cluster
  membership changes (a node joining, leaving, or changing role).
- **Reachable over the network**: Other nodes in the cluster can reach a running
  Barn node over the network in order to participate in consensus and serve
  requests — this does not work today.
- **Room to grow**: The networking layer introduced for the Barn should be able
  to host additional Swini services in the future, not just Barn's.

## Constraints

- This feature does not include wiring the Barn into the running daemon's
  startup sequence. Being able to spawn a Barn node and having it work correctly
  is enough; making the daemon actually launch one on boot is a follow-up
  feature.
- Automatic discovery of peer nodes is out of scope. How a node learns the
  network address of its peers is not addressed here.

## Scenarios

**GIVEN** a node's identity and a directory to store its data **WHEN** the node
is spawned **THEN** it becomes a running Barn instance that participates in the
cluster's storage and consensus

**GIVEN** a running Barn node **WHEN** a client writes a new key and another
client reads that same key back afterward through the default (consistent) read
path **THEN** the reader sees the newly written value, regardless of which node
in the cluster it asks

**GIVEN** a running Barn node whose local copy of the data may be behind the
rest of the cluster (e.g. a follower that hasn't fully caught up) **WHEN** a
client explicitly opts into a fast/stale read **THEN** the client receives that
node's local view of the data, even if it does not reflect the very latest
committed write

**GIVEN** a client subscribed to a Barn node's events **WHEN** another client
writes, patches, or deletes a key, or a node joins, leaves, or changes role in
the cluster **THEN** the subscriber receives an event describing that change

**GIVEN** two Barn nodes running on different machines that are part of the same
cluster **WHEN** they need to replicate data or hold a leader election **THEN**
they can reach each other over the network to do so

<!-- plan-notes (for /mad.plan — not part of the product spec):
- Implement `Barn` in `src/store/barn/mod.rs` as a concrete implementation of
  `ClusterStore` (and therefore `SpreadStore` + `ItemStore`).
- `Barn` exposes a `spawn` function taking a `BarnConfig` (node_id, data_dir,
  etc.). `spawn` constructs the Barn `Storage` (`src/store/barn/storage.rs`)
  and the raft instance (via `raft::create` in `src/store/barn/raft/mod.rs`).
- `Barn` subscribes to the underlying raft's events/metrics and forwards them
  as `SpreadStoreEvent`s to its own subscribers.
- May take loose inspiration from
  `/Users/bruno/Work/nosebit/projects/swini/#/cli2/src/store/barn/mod.rs`, but
  should not closely follow its structure.
- The `ItemStore::get` implementation proxies reads to the current raft
  leader for consistency. A separate `stale_get` function proxies reads to a
  random node instead, for lower-latency reads that can tolerate staleness.
- Networking: introduce a `src/api` module exposing an `Api` struct with
  `new()` and `listen(addr)` functions. `Api::new` constructs a
  `BarnApiHandler`. `src/api/barn.rs` implements the generated `BarnApi`
  proto service via that `BarnApiHandler` struct. Structure `Api` so
  additional proto services can be registered alongside Barn's later.
-->
