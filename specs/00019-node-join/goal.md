---
id: 19
author: @brunomacf
created: 2026-08-30
---

# [Goal] Multi-Node Cluster Joining and Membership

Swini clusters consist of multiple nodes operating under distinct roles: server nodes that participate in consensus and maintain authoritative cluster state, and worker nodes that execute workloads. To form and expand a cluster, nodes must be able to join an existing cluster dynamically over the network. This feature introduces node cluster joining and role-aware membership management: enabling new server and worker nodes to contact an active node, register their identity and role in the cluster, and receive the cluster topology so they can participate in cluster operations. It also provides cluster status visibility so operators can inspect all registered nodes and their roles.

## Requirements

- **Dynamic Cluster Joining**: A newly started node can join an existing Swini cluster by connecting to any reachable peer address configured in its join list.
- **Role-Aware Membership Registration**:
  - **Server Nodes**: When a node with the `Server` role joins, it is added as an active consensus member in the Barn cluster store and registered in the global node registry.
  - **Worker Nodes**: When a node with the `Worker` role joins, it is registered in the global node registry as an execution node without participating in Barn consensus storage.
- **Cluster Topology Discovery**: The join operation returns the list and addresses of all active server nodes to the joining node, enabling worker nodes (which lack local Barn consensus storage) to discover where to direct cluster queries and operations.
- **Node Identity & State Registration**: Every node manages its own unique identity (node ID, name, network API address, tags, roles, and join timestamp) and persists this record into the cluster store upon joining.
- **Local Identity & Leadership Tracking**: Each node tracks its local node state and whether it currently acts as the primary (Barn consensus leader) node in the cluster.
- **Automatic Join on Startup**: Nodes configured with peer join addresses automatically initiate the cluster join workflow upon initialization.
- **Cluster Status Inspection**: Operators can inspect the cluster status via a CLI command (`swini status`), viewing a formatted overview of all registered nodes, their IDs, names, roles, and join timestamps.

## Constraints

- **No Resource Probing in Initial Scope**: Dynamic resource telemetry (CPU/memory utilization, health checks, and probing models) is deferred to a future feature. Nodes join with baseline/default resource declarations.
- **Workload Scheduling Out of Scope**: Workload assignment, dispatching, and supervision (pigs/piglets) are handled by the upcoming workload management subsystem.
- **Static Join Peer Configuration**: Peer addresses are supplied via node configuration; automatic discovery mechanisms (e.g. mDNS or gossip-based discovery) are not part of this feature.

## Scenarios

**GIVEN** an existing Swini cluster and a newly started node configured with the `Server` role
**WHEN** the new node initiates a join request to an existing cluster node
**THEN** the new node is registered as a consensus member in the Barn cluster store, added to the cluster node registry, and receives the list of active server nodes in the join response

**GIVEN** an existing Swini cluster and a newly started node configured with the `Worker` role
**WHEN** the new node initiates a join request to an existing cluster node
**THEN** the new node is registered in the cluster node registry without being added to the Barn consensus group, and receives the list of active server nodes to configure its client routing cache

**GIVEN** a node initialized with join addresses in its configuration
**WHEN** the node starts up
**THEN** it automatically contacts a configured peer to execute the join workflow before marking its startup sequence complete

**GIVEN** a server node running in the cluster
**WHEN** Barn leader election occurs
**THEN** the node updates its internal state to reflect whether it is the current cluster primary

**GIVEN** a running Swini cluster with one or more registered server and worker nodes
**WHEN** an operator runs `swini status`
**THEN** the command displays a formatted table of all registered nodes containing their node ID, name, role, and joined-at datetime

<!-- plan-notes (for /mad.plan — not part of the product spec):
- Implement node backend logic (`NodeBackend` or `Node` struct) managing node lifecycle, local identity (`self_node`), and interaction with the Barn store.
- Introduce `src/daemon/config.rs` exposing `DaemonConfig` (including `join_addresses`, node name, API address, role, etc.).
- Add `joined_at` field (timestamp / datetime) to `Node` in `src/node/types.rs`.
- Implement node ID generation logic (porting/adapting `node_id_provide`).
- Implement the Join RPC call and handler:
  - Request payload: `node_id`, `node_name`, `api_addr`, `role` (or roles).
  - Server handler: if role == Server, add as Barn Raft node; write `node/{node_id}` to Barn key-value store.
  - Response: list of server nodes (`Node` info / API addresses) for client-side Barn cache and routing.
  - Handle worker nodes forwarding requests to the primary server node.
- Implement the `swini status` CLI command in `src/cli/` to query cluster nodes from Barn and format the output as a table.
- Keep resource metrics static/empty for now; design probing architecture (push vs pull model) in a future iteration.
- Resolve architectural design between `NodeBackend` holding `Node` vs renaming `Node` to `NodeState` and making `Node` the backend/manager, considering the upcoming `Pig` backend.
-->
