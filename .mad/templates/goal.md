---
id: 0000
status: draft # draft | proposed | accepted | done | rejected
author: @{username}
created: {datetime}
---

# [Goal] {Feature Name}

<!--
Describe what the feature wants to accomplish and why we need it in the first place.

EXAMPLE:

Swini daemons running on different machines need a shared, consistent view of the
cluster state: which nodes are registered, what resources are available, and where
workloads have been placed. Today there is no such store — each daemon only knows
about itself. This feature introduces the **Barn**, a distributed key-value store
embedded in the daemon that holds the authoritative cluster state and keeps it
consistent across all nodes, even in the presence of failures.
-->

## Requirements

<!--
Functional and Non-Functional requirements for this feature.

EXAMPLE:

- **Cluster-wide reads and writes**: Any daemon must be able to write a key-value
  pair to the Barn and have all other daemons read the same value.
- **Strong consistency**: A read must always return the most recently committed
  write. Stale reads are not acceptable for cluster state.
- **Fault tolerance**: The cluster must remain operational as long as a majority
  of nodes are alive. A minority of nodes going down must not cause data loss or
  service interruption.
- **Atomic writes**: A write either fully commits to the cluster or is fully
  rejected. Partial writes are not acceptable.
- **Leadership transparency**: There is no external coordinator. The Barn is
  fully embedded in the daemon and self-manages leader election internally.
-->

## Constraints

<!--
Any non-goals or general rules for the future should go here.

EXAMPLE:

- The Barn is **internal only** — it is not a general-purpose database exposed
  to end users or external clients. Only daemons communicate with it.
- The Barn stores **cluster state only** (node registrations, resource
  availability, workload placements). It is not responsible for application data,
  logs, or metrics.
- The Barn is **not a document store** — values are plain byte slices keyed by
  string. No querying, filtering, or indexing.
- Performance is secondary to correctness at this stage. Optimizations can be
  addressed in a follow-up spec.
-->

## Scenarios

<!--
Provide a list of user scenarios in BDD format (GIVEN / WHEN / THEN) that must be fulfilled by this feature.

EXAMPLE:

**GIVEN** a three-node Swini cluster
**WHEN** Daemon A writes `nodes/A/resources = {"cpu": 4, "mem": "8gb"}` to the Barn
**THEN** Daemon B and Daemon C can read `nodes/A/resources` and get the same value

**GIVEN** a three-node Swini cluster
**WHEN** one of the three nodes goes down
**THEN** the remaining two nodes continue to accept reads and writes without interruption

**GIVEN** a three-node Swini cluster
**WHEN** two of the three nodes go down (minority becomes majority)
**THEN** the remaining node stops accepting writes and returns an error indicating
the cluster lacks quorum, but does not corrupt or lose existing data

**GIVEN** a freshly started daemon joining an existing cluster
**WHEN** it connects to a peer
**THEN** it receives a full snapshot of the current Barn state and becomes
consistent with the rest of the cluster before serving any reads or writes
-->
