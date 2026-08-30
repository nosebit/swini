# API Module

This module hosts swini's gRPC server surface: the generic [`Api`](mod.rs)
struct that binds a `tonic::transport::Server` to an address and answers
whichever gRPC services swini exposes, plus one submodule per service
implementation.

Deliberately thin and additive: `Api` holds one field per registered service
handler, and `Api::listen` registers each of them with `.add_service(...)` on
the same underlying `tonic` server. Adding a new gRPC service later means adding
one field to `Api`, one parameter to `Api::new`, and one more
`.add_service(...)` call — not a new server.

## Files in this module

- [`mod.rs`](mod.rs) — [`Api`](mod.rs): `Api::new(barn: Arc<Barn>)` constructs
  the service handlers `Api` currently knows about (today, just
  [`BarnApiHandler`](barn.rs)), and `Api::listen(addr)` starts serving them all
  on that address until the server shuts down.
- [`barn.rs`](barn.rs) — [`BarnApiHandler`](barn.rs): the server side of the
  `BarnApi` gRPC service defined in `proto/barn.proto` (the client side lives in
  [`store::barn::raft::network`](../store/barn/raft/README.md)).
  `append_entries`/`vote`/`install_snapshot` decode into openraft's own request
  types and call the matching method on the wrapped `Barn`'s raft handle.
  `forward_write`/`forward_read` decode into
  [`store::barn::Action`](../store/barn/README.md)/`ReadAction` and call
  `Barn::local_write`/`local_read` — used whenever a request lands on a node
  that isn't who it needs to be (not the current leader, or a future worker-mode
  `Barn` that never is). `node_add` decodes the typed `NodeAddReq` and calls
  `SpreadStore::node_add`. Every RPC's payload uses the same opaque
  `serde_json`-in-`BarnMessage` envelope as `NetworkConnection::encode`/`decode`
  in `raft/network.rs`, so this wire contract never needs a hand-update just
  because an openraft or Barn message type gains a field.
