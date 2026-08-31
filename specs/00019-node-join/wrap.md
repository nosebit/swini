---
id: 19
plan: specs/00019-node-join/plan.md
todo: specs/00019-node-join/todo.md
author: @brunomacf
created: 2026-08-31
---

# [Wrap] Multi-Node Cluster Joining and Membership

## T1 — Implement Node Domain Types (`Node`, `NodeRole`)

Registered `mod node;` in `src/main.rs` to expose the module to the binary
crate.

**plan.md updated:** no — standard module registration for the binary root.

## T2 — Implement Daemon & API Configuration

Registered `mod daemon;` in `src/main.rs` and added `#[allow(unused_imports)]`
on `daemon/mod.rs` re-exports for clean compilation while full daemon runtime
orchestrator is built in a subsequent issue.

**plan.md updated:** no — minor compiler warning suppression.

## T4 — Implement Cluster Backend (`ClusterBackend`)

- Added explicit `chrono` dependency with serde feature in `Cargo.toml` for RFC
  3339 timestamping.
- Re-exported `RaftConfig` (`pub use raft::Config as RaftConfig;`) under
  `#[cfg(test)]` in `src/store/barn/mod.rs` so tests can construct
  `barn::Config` without exposing the internal `raft` submodule in production
  builds.
- Registered `mod cluster;` in `src/main.rs`.

**plan.md updated:** no — helper visibility adjustments.

## T5 — Implement Cluster gRPC API Handlers (`ClusterApiHandler`)

`ClusterBackend` operates strictly on `barn.slice("node/")` and is only consumed
by the cluster API handlers. `ClusterApiHandler::new(barn: Arc<Barn>)` now
encapsulates and instantiates `ClusterBackend::new(barn)` directly. This
preserves the clean, minimal `Api::new(barn: Arc<Barn>)` signature across the
codebase without requiring callers to construct and pass `ClusterBackend`
separately.

**plan.md updated:** yes — updated architecture, `ClusterBackend::new`,
`ClusterApiHandler::new`, and `Api::new` signatures.

## T8 — End-to-End Multi-Node Join and Cluster Status Integration Tests

- Added `multi_node_cluster_join_over_grpc` in `src/cluster/mod.rs` verifying
  multi-node cluster bootstrapping, join over real gRPC, and worker node cache
  synchronization.
- Created `tests/e2e/cluster.rs` and registered `mod cluster;` in
  `tests/e2e/main.rs` verifying CLI execution via `assert_cmd`.

**plan.md updated:** no — integration tests consolidated into the existing test
runner structure.
