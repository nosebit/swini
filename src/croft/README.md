# Croft Module

In traditional rural agriculture (notably in Scottish and British pastoral
heritage), a **Croft** is an established working farmstead. In Swini a Croft
represents all the infrastructure necessary for the functioning of a single node
in the distributed system including:

- **The Plot of Land**: This represents the underlying host resources (the base,
  the soil) available to be used within the Croft.
- **The Barn**: This is the main data storage used throughout the entire
  Cluster/Ranch.
- **The Gate**: This is the entry point of the Croft where messages targeting
  this specific Croft arrive.
- **Operation Config**: This is the main config received by the Croft Regent and
  placed into the Croft itself so everyone knows.

The Croft has a set of "staff" members which operate within the Croft and are
managed by the Croft Regent. The Regent creates the Croft and then spawns
several staff members (employees) to operate in that Croft. For example, the
Plot Clerk is spawned by the Regent and placed into the Croft to handle
Plot-related affairs and messages.

---

## Architectural Role of `Croft`

The [`Croft`](./mod.rs) struct is the concrete compound that bundles these
foundational infrastructure elements together on a single machine:

```rust
pub struct Croft {
  pub plot: Plot,
  pub barn: Arc<Barn>,
  pub gate: Gate,
  pub config: Config,
}
```

### Staff Workers

Staff members (such as the **[`PlotClerk`](../plot/clerk/mod.rs)**) are hired
into a Croft by receiving a shared `Arc<Croft>` handle:

- They inspect local identity via `croft.plot`.
- They persist and retrieve domain records via `croft.barn`.
- They station their API listeners at the entrance via `croft.gate.add(...)`.

The host **[`Regent`](../regent/mod.rs)** process acts as the overall estate
manager on that machine, responsible for spawning the `Croft`, hiring its staff,
coordinating Ranch-wide joins, and opening the `Gate` to listen for network
traffic.

---

## Module Structure

| File                       | Responsibility                                                                                                                                                            |
| :------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **[`mod.rs`](./mod.rs)**   | Operational compound [`Croft`](./mod.rs) unifying local [`Plot`](../plot/mod.rs), [`Barn`](../store/barn/mod.rs), [`Gate`](./gate.rs), and [`Config`](../core/config.rs). |
| **[`gate.rs`](./gate.rs)** | Network gateway [`Gate`](./gate.rs) managing the underlying tonic gRPC server and router.                                                                                 |
