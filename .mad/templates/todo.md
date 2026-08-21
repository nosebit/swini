---
id: 0000
status: draft # draft | in-progress | done
plan: specs/{slug}/plan.md
author: @{username}
created: {datetime}
---

# [Todo] {Feature Name}

<!--
One short paragraph describing what this task list covers. Link back to
plan.md instead of restating its content.

EXAMPLE:

Tasks to implement the Barn distributed key-value store described in
[plan.md](./plan.md). Tasks T1-T2 build the local store and Raft wiring; T3-T4
build the client API and expose it to the daemon.
-->

## Tasks

<!--
Break plan.md's "Implementation Details" and "Data Model / API Changes"
sections into concrete, atomic tasks. Each task must be:

- Small enough to implement and verify in one sitting.
- Traceable to the exact plan.md section/snippet it implements.
- Checkable by a reviewer without re-reading the whole plan.

Order tasks so dependencies come before dependents.

EXAMPLE:

- [ ] **T1 — Implement `barn::Store`**
  - **Plan reference:** [barn::Store](./plan.md#barnstore)
  - **Files:** `src/barn/store.rs` (new), `src/barn/mod.rs` (modified —
    register module)
  - **Description:** Implement the in-memory `Store` struct and its
    `apply`/`get` methods as specified in plan.md.
  - **Definition of Done:**
    - `Store::apply` handles `Set` and `Delete` ops and updates
      `applied_index`.
    - `Store::get` returns `Option<&[u8]>`.
    - Unit tests cover set/delete/overwrite/get-missing.
  - **Depends on:** none

- [ ] **T2 — Wire `barn::Store` into the Raft apply loop**
  - **Plan reference:** [Architecture](./plan.md#architecture)
  - **Files:** `src/barn/raft.rs` (modified)
  - **Description:** Call `Store::apply` for every committed log entry.
  - **Definition of Done:**
    - Committed entries update the store's `applied_index` in order.
    - Unit test simulating a 3-entry log applied in order.
  - **Depends on:** T1

- [ ] **T3 — Implement `barn::Client::set`**
  - **Plan reference:** [barn::Client](./plan.md#barnclient)
  - **Files:** `src/barn/client.rs` (new)
  - **Description:** Public API daemons use to propose writes through Raft.
  - **Definition of Done:**
    - `Client::set` returns `Ok(())` once committed to a majority.
    - `Client::set` returns an error if quorum can't be reached.
    - E2e test: write on one daemon, read the same value on another.
  - **Depends on:** T2
-->

## Task Order

<!--
Optional: call out anything not already obvious from "Depends on" above —
e.g. tasks that can run in parallel, or a task that should be done last
because it touches shared wiring.

EXAMPLE:

T1 and T2 must be sequential (T2 depends on T1). T3 can start as soon as T2 is
done. There is nothing here that can be parallelized across engineers/agents
without conflicting on `src/barn/mod.rs`.
-->
