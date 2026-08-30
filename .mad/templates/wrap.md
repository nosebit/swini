---
id: 0000
plan: specs/{slug}/plan.md
todo: specs/{slug}/todo.md
author: @{username}
created: {datetime}
---

# [Wrap] {Feature Name}

<!--
Running log of deviations from plan.md discovered during /mad.exec, one
section per task, in the order they were found. Only tasks that deviated
from plan.md get an entry — a task implemented exactly as planned doesn't
appear here. This file is append-only and never rewritten: it exists so
plan.md can stay a clean "what we intended" record while this one captures
"where reality diverged," making it possible to see later how accurate the
original plan was.

EXAMPLE:

## T3 — Re-export `ClusterStore` from `store`

Also had to add `pub use raft::Node;` alongside `ClusterStore` — plan.md's
snippet only mentioned re-exporting `ClusterStore` itself, but `Node` turned
out to be part of its public signature too.

**plan.md updated:** yes — [Architecture](./plan.md#architecture) now lists
both re-exports.

## T7 — `Barn` struct, `spawn`, and private helpers

`spawn` needed an extra `Arc<Mutex<..>>` around `node_cache` that plan.md's
snippet didn't show, since `event_forwarder_start` and the trait methods in
T8 both mutate it concurrently.

**plan.md updated:** no — left as noted here; the snippet's intent (populate
`node_cache` on spawn) still holds, only the concurrency wrapper changed.
-->
