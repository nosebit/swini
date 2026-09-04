# Regent Module

In estate stewardship and governance, a **Regent** is an administrator appointed
to govern and manage a domain on behalf of the owner.

In Swini, the **Regent** is the host supervisor process running on each physical
machine. It acts as the estate governor responsible for creating the local
**[`Croft`](../croft/README.md)**, spawning its **Staff** members, coordinating
Ranch-wide joins, and supervising the process lifecycle:

---

## Responsibilities of the Regent

1. **Creating the Croft**: Spawns the physical compound
   ([`Croft::spawn`](../croft/mod.rs)) uniting the local
   [`Plot`](../plot/mod.rs), consensus [`Barn`](../store/barn/mod.rs), network
   [`Gate`](../croft/gate.rs), and active configuration.
2. **Hiring Staff Workers**: Spawns domain employees into the Croft (such as
   hiring the [`PlotClerk`](../plot/clerk/mod.rs)), giving them access to the
   Croft's resources and mounting their APIs to the Gate.
3. **Outbound Cluster Join**: Coordinates initial cluster bootstrapping
   (`bootstrap_join`), reaching out to candidate peer addresses to register the
   local Plot into the Ranch.
4. **Opening the Gate**: Binds and runs the network listener
   (`croft.gate.listen`) to receive incoming gRPC and cluster traffic.
5. **Lifecycle & Supervision**: Handles foreground/background process execution,
   persists runtime state (`state.json`), traps shutdown signals
   (`SIGINT`/`SIGTERM`), and performs graceful cleanup on termination.

---

## Module Structure

| File                         | Responsibility                                                                                                                                   |
| :--------------------------- | :----------------------------------------------------------------------------------------------------------------------------------------------- |
| **[`mod.rs`](./mod.rs)**     | Process lifecycle orchestration (`start`, `stop`, `status`), Croft & staff bootstrapping, cluster joins (`bootstrap_join`), and signal trapping. |
| **[`state.rs`](./state.rs)** | Persistent runtime process state (`state.json`) recording PID, execution mode (`detached`), start timestamp, and active configuration snapshot.  |
