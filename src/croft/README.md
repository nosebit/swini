# Croft Module

In traditional rural agriculture (notably in Scottish and British pastoral
heritage), a **Croft** is an established working farmstead. In Swini a Croft
represents the canonical domain concept and operational compound for a single
node in the distributed system including:

- **Croft Identity**: The node's unique 64-bit ID, name, network address,
  assigned roles, and tags.
- **The Barn**: The consensus key-value storage engine used throughout the
  cluster/Ranch.
- **The Gate**: The network entrance where incoming gRPC requests arrive.
- **Operation Config**: The runtime configuration loaded by the Regent and used
  to spawn the Croft.

The Croft has a set of "staff" members which operate within the Croft and are
managed by the Croft Regent. The Regent creates the Croft and then spawns staff
members to operate in that Croft. For example, the **Croft Clerk** is spawned by
the Regent and placed into the Croft to handle cluster registration and
discovery.

---

## Architectural Role of `Croft`

The [`Croft`](./mod.rs) struct is the concrete compound and domain entity
representing a machine on the Ranch:

```rust
pub struct Croft {
  pub id: u64,
  pub name: String,
  pub addr: String,
  pub roles: Vec<CroftRole>,
  pub tags: Vec<String>,
  pub joined_at: String,
  pub barn: Option<Arc<Barn>>,
  pub gate: Option<Gate>,
}
```

### Staff Workers

Staff members (such as the **[`CroftClerk`](./clerk/mod.rs)**) are hired into a
Croft by receiving a shared `Arc<Croft>` handle:

- They inspect identity directly via `croft.id`, `croft.name`, `croft.addr`,
  `croft.roles`.
- They persist and retrieve domain records via `croft.barn`.
- They station their API listeners at the entrance via
  `croft.gate.as_ref().unwrap().add(...)`.

The host **[`Regent`](../regent/mod.rs)** process acts as the supervisor on that
machine, responsible for spawning the `Croft`, hiring its staff, coordinating
Ranch-wide joins, and opening the `Gate` to listen for network traffic.

---

## Module Structure

| File                           | Responsibility                                                                                      |
| :----------------------------- | :-------------------------------------------------------------------------------------------------- |
| **[`mod.rs`](./mod.rs)**       | Canonical domain entity and operational compound [`Croft`](./mod.rs).                               |
| **[`types.rs`](./types.rs)**   | Cluster role definitions ([`CroftRole`](./types.rs)).                                               |
| **[`config.rs`](./config.rs)** | Configuration loading, YAML merging, and path resolutions ([`Config`](./config.rs)).                |
| **[`gate.rs`](./gate.rs)**     | Network gateway [`Gate`](./gate.rs) managing the underlying tonic gRPC server and router.           |
| **[`clerk/`](./clerk/mod.rs)** | Domain staff worker ([`Clerk`](./clerk/mod.rs)) and gRPC service handler ([`Api`](./clerk/api.rs)). |
