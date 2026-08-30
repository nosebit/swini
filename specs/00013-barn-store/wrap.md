---
id: 13
plan: specs/00013-barn-store/plan.md
todo: specs/00013-barn-store/todo.md
author: @brunomacf
created: 2026-08-23
---

# [Wrap] Complete the Barn Cluster Store

## T7 — `Barn` struct, `spawn`, and private helpers

Three deviations from `plan.md`'s snippet, all in `random_node`:

1. `rand::thread_rng()` doesn't exist in the `rand` version actually
   resolved by `cargo add rand` (0.10.2) — it was renamed `rand::rng()`.
2. `SliceRandom` no longer provides `.choose()` on a `Vec`/slice in
   `rand` 0.10 — that moved to a new `IndexedRandom` trait.
3. `pub use raft::{Node, NodeRole};` (plan.md's re-export list) was
   trimmed to just `Node`. Nothing in the code actually implemented
   through T10 references `NodeRole` externally — `Node` is used by
   `api::barn`'s `node_add` handler, but role information never leaves
   `store::barn` in the current codebase, and `clippy -D warnings` fails
   permanently on a genuinely unused `pub use` in a binary crate. Trivial
   to re-add once a real external consumer needs it (likely the future
   "node" module).

Also, the DoD's "`random_node` picking self vs. a peer" is only partially
covered here — T7 tests self-picking (single-node cluster) and the
no-nodes-yet case. A "peer" candidate requires `SpreadStore::node_add`
(T8) and a second real node, so that case is deferred to T11's multi-node
integration test rather than added here.

**plan.md updated:** yes — the `Barn` snippet's `random_node` now uses
`rand::rng()`/`IndexedRandom`, the re-export lists (Architecture bullet,
`mod.rs` snippet, Data Model bullet) drop `NodeRole`, and the `rand`
dependency bullet notes the version and API rename.

## T9 — `api::barn::BarnApiHandler`

No deviation in shape — plan.md's snippet used `decode(...)`/`encode(...)`
in the handler bodies without spelling out those two helpers' own
implementations. Wrote them as straightforward `serde_json` free
functions (`decode<T: DeserializeOwned>`, `encode<T: Serialize>`,
`to_status<E: Display>`), matching the wire format plan.md describes
(same envelope as `NetworkConnection::encode`/`decode` in
`raft/network.rs`). Not logged as a deviation since it fills in detail
the plan left implicit rather than departing from its intended shape.

**plan.md updated:** no — nothing to reconcile.
