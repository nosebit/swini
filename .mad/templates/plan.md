---
id: 0000
status: draft # draft | proposed | accepted | done | rejected
goal: specs/{slug}/goal.md
author: @{username}
created: {datetime}
---

# [Plan] {Feature Name}

<!--
One or two paragraphs summarizing the technical approach at a high level: the
overall strategy and the main components involved. Do not restate the product
requirements — link back to the goal instead.

EXAMPLE:

The Barn is implemented as a new `barn` module inside the daemon, embedding a
Raft consensus group over the set of cluster nodes. Each daemon runs a Raft
node; writes are proposed to the current leader and only acknowledged once
committed to a majority of the log. Reads are served locally once a node's
applied index has caught up. See [goal.md](./goal.md) for the product
requirements this design satisfies.
-->

## Architecture

<!--
Describe how the new/changed components fit into the existing system. Name
the concrete modules, files, or services touched. A short diagram (mermaid or
ASCII) is welcome but not required.

EXAMPLE:

```mermaid
graph LR
  A[Daemon A] -- Raft RPC --> B[Daemon B]

A -- Raft RPC --> C[Daemon C] A --> AB[(barn::Store)] B --> BB[(barn::Store)] C
--> CB[(barn::Store)]

````

New module: `src/barn/` — houses the Raft node, the replicated log, and the
in-memory key-value store. It is wired into `src/daemon/mod.rs` alongside the
existing `cluster` module, which it depends on for peer discovery.
-->

## Implementation Details

<!--
Break the implementation into concrete components. For EACH component,
include a short code snippet (struct/type definition, function signature, or
interface) that shows exactly what will be built — not just a description.
Use the project's real conventions (see .mad/memory/lore.md).

EXAMPLE:

### `barn::Store`

The core replicated key-value store, applied from the Raft log.

```rust
use crate::barn::raft;

/// In-memory, Raft-replicated key-value store.
pub struct Store {
    data: std::collections::HashMap<String, Vec<u8>>,
    applied_index: u64,
}

impl Store {
    /// Applies a committed log entry to the store. Called only by the Raft
    /// apply loop, never directly.
    pub fn apply(&mut self, entry: raft::LogEntry) {
        match entry.op {
            raft::Op::Set { key, value } => {
                self.data.insert(key, value);
            }
            raft::Op::Delete { key } => {
                self.data.remove(&key);
            }
        }
        self.applied_index = entry.index;
    }

    /// Reads a key from local state. May return a stale value relative to
    /// the leader if this node's applied_index lags.
    pub fn get(&self, key: &str) -> Option<&[u8]> {
        self.data.get(key).map(Vec::as_slice)
    }
}
````

### `barn::Client`

Public API daemons use to interact with the Barn.

```rust
use crate::barn::raft;

pub struct Client {
    raft: raft::Handle,
}

impl Client {
    /// Writes a key-value pair. Resolves once committed to a majority of
    /// the cluster. Returns an error if quorum cannot be reached.
    pub async fn set(&self, key: String, value: Vec<u8>) -> Result<(), barn::Error> {
        self.raft.propose(raft::Op::Set { key, value }).await
    }
}
```

-->

## Data Model / API Changes

<!--
Any new types, schemas, wire formats, or public-facing APIs introduced or
changed by this feature. Omit this section if not applicable.

EXAMPLE:

New Raft log entry format (`barn::raft::Op`):

```rust
pub enum Op {
    Set { key: String, value: Vec<u8> },
    Delete { key: String },
}
```
-->

## Testing Strategy

<!--
How this feature will be verified, per the project's testing rules in
.mad/memory/lore.md. Call out anything non-obvious to test (failure modes,
concurrency, edge cases).

EXAMPLE:

- Unit tests for `barn::Store::apply` covering set/delete/overwrite.
- Unit tests for leader election and quorum loss using a simulated 3-node
  Raft group (no real networking).
- E2e test spinning up 3 real daemon processes, writing on one, reading from
  the others, and asserting convergence.
- E2e test killing the leader mid-cluster and asserting a new leader is
  elected and writes resume.
-->

## Risks & Open Questions

<!--
Technical risks, trade-offs, or decisions still open. It's fine to have
entries here — they surface what a reviewer should scrutinize.

EXAMPLE:

- Raft log compaction/snapshotting is not covered by this plan; the log will
  grow unbounded until a follow-up spec addresses it.
- Using an existing Raft crate vs. a custom implementation is still open —
  defaulting to `raft-rs` unless review pushes back.
-->

## Out of Scope

<!--
Explicitly excluded from this plan, deferred to a follow-up spec.

EXAMPLE:

- Log compaction and snapshot transfer to new/lagging nodes.
- Dynamic cluster membership changes (adding/removing nodes at runtime).
-->
