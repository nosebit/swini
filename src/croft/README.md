# Croft Module

In traditional rural agriculture (notably in Scottish and British pastoral
heritage), a **Croft** is an established working farmstead. In Swini a Croft
represents the canonical domain concept and operational compound for a single
node in the distributed system including:

- **Croft**: The node's persistent domain entity holding its unique 64-bit ID,
  name, network address, assigned roles, tags, and join timestamp.
- **LiveCroft**: The active local host machine specialization embedding the base
  `Croft`, local consensus `Barn`, and network `Gate`.
- **The Barn**: The consensus key-value storage engine (`RanchStore`) used
  throughout the cluster/Ranch.
- **The Gate**: The network entrance where incoming gRPC requests arrive.
- **Operation Config**: The runtime configuration loaded by the Regent and used
  to spawn the Croft.

The Croft has a set of "staff" members which operate within the Croft and are
managed by the Croft Regent. The Regent creates the `LiveCroft` and then spawns
staff members to operate in that Croft. For example, the **Croft Clerk** is
spawned by the Regent and placed into the Croft to handle cluster registration
and discovery by managing `Croft` records in the Barn.

---

## Architectural Role of `Croft` and `LiveCroft`

The [`Croft`](./types.rs) struct is the base domain entity representing any node
on the Ranch:

```rust
pub struct Croft {
  pub id: u64,
  pub name: String,
  pub addr: String,
  pub roles: Vec<CroftRole>,
  pub tags: Vec<String>,
  pub joined_at: String,
}
```

The [`LiveCroft`](./mod.rs) struct is the active running compound on the local
physical machine:

```rust
pub struct LiveCroft {
  base: Croft,
  pub barn: Arc<Barn>,
  pub gate: Gate,
}
```

By implementing `std::ops::Deref<Target = Croft>`, all fields and methods on the
underlying `Croft` (`live_croft.id`, `live_croft.name`, `live_croft.addr`,
`live_croft.roles`, `live_croft.is_server()`, etc.) are accessed transparently
on `LiveCroft`, and `&LiveCroft` coerces automatically to `&Croft`.

### Staff Workers

Staff members (such as the **[`CroftClerk`](./clerk/mod.rs)**) are hired into a
Croft by receiving a shared `Arc<LiveCroft>` handle:

- They inspect identity directly via `croft.id`, `croft.name`, `croft.addr`,
  `croft.roles`.
- They persist and retrieve entity records via `croft.barn`.
- They station their API listeners at the entrance via `croft.gate.add(...)`.

The host **[`Regent`](../regent/mod.rs)** process acts as the supervisor on that
machine, responsible for spawning the `LiveCroft`, hiring its staff,
coordinating Ranch-wide joins, and opening the `Gate` to listen for network
traffic.

---

## Module Structure

| File                           | Responsibility                                                                                      |
| :----------------------------- | :-------------------------------------------------------------------------------------------------- |
| **[`mod.rs`](./mod.rs)**       | Active runtime compound [`LiveCroft`](./mod.rs).                                                    |
| **[`types.rs`](./types.rs)**   | Base domain entity ([`Croft`](./types.rs)) and role definitions ([`CroftRole`](./types.rs)).        |
| **[`config.rs`](./config.rs)** | Configuration loading, YAML merging, and path resolutions ([`Config`](./config.rs)).                |
| **[`gate.rs`](./gate.rs)**     | Network gateway [`Gate`](./gate.rs) managing the underlying tonic gRPC server and router.           |
| **[`clerk/`](./clerk/mod.rs)** | Domain staff worker ([`Clerk`](./clerk/mod.rs)) and gRPC service handler ([`Api`](./clerk/api.rs)). |
