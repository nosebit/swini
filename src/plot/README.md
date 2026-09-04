# Plot Module

In agriculture and land stewardship, a **Plot** is a distinct, bounded parcel of
land allocated within an estate. It defines the physical terrain, identity, and
designated purpose of that piece of ground.

In Swini, the entire distributed cluster forms the **Ranch**, while each
individual physical machine or host acts as an operational **Croft**. Within
that Croft, the **Plot** represents the computational parcel itself — its unique
machine identity, network coordinates, capability attributes, and cluster
responsibilities.

---

## The Plot in the Swini Mental Model

Every node participating in a Swini cluster is identified as a
[`Plot`](./mod.rs):

- **Plot Identity (`id`, `name`)**: A unique 64-bit identifier (persisted in
  `{data_dir}/plot.id`) and a human-readable node name (e.g., `worker-01`).
- **Network Coordinates (`addr`)**: The canonical endpoint (e.g.,
  `127.0.0.1:7440`) where the node's [`Gate`](../croft/gate.rs) receives
  traffic.
- **Roles (`roles`)**: The designation of the parcel:
  - `Server`: Participates in Raft consensus, metadata storage in the
    [`Barn`](../store/barn/mod.rs), and cluster coordination.
  - `Worker`: Executes computational workloads and tasks (supervised by the
    Drover).
- **Tags (`tags`)**: Informational and scheduling markers representing physical
  hardware capabilities or topology (e.g., `"gpu"`, `"high-mem"`, `"zone-a"`).
- **Join Record (`joined_at`)**: The timestamp recording when this parcel was
  commissioned into the Ranch.

---

## Staff Worker: The Plot Clerk

Operating inside each [`Croft`](../croft/mod.rs) is a dedicated staff member:
the **[`PlotClerk`](./clerk/mod.rs)**.

Just as a municipal clerk or estate registrar keeps the land registry, records
deeds, and processes incoming arrivals, the `PlotClerk` manages all plot-related
affairs across the Ranch:

1. **Processing Joins**: When a new node joins the cluster, it contacts the
   `PlotClerk` via the gRPC [`PlotApi`](./clerk/api.rs). The clerk records the
   new Plot into cluster-wide storage at `plot/{id}` in the
   [`Barn`](../store/barn/mod.rs).
2. **Server Plot Discovery**: The clerk returns the active set of `Server` plots
   so newly joined workers and clients know where to dial the consensus store.
3. **Cluster Inquiries**: Answers status and inspection queries about all
   registered plots across the Ranch.

---

## Module Structure

| File                                 | Responsibility                                                                                                                                                                |
| :----------------------------------- | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **[`mod.rs`](./mod.rs)**             | Pure domain definitions for [`Plot`](./mod.rs) and [`PlotRole`](./mod.rs), persistent ID provisioning (`id_provide`), and role inspection helpers (`is_server`, `is_worker`). |
| **[`clerk/mod.rs`](./clerk/mod.rs)** | The [`Clerk`](./clerk/mod.rs) domain staff worker, holding an `Arc<Croft>` handle, writing plot records to `croft.barn`, and registering its API to `croft.gate`.             |
| **[`clerk/api.rs`](./clerk/api.rs)** | Tonic gRPC handler implementing [`PlotApi`](./clerk/api.rs) for `join` and `status` remote procedure calls.                                                                   |
