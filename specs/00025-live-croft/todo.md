---
id: 0025
plan: specs/00025-live-croft/plan.md
author: @brunomacf
created: 2026-09-05
---

# [Todo] Croft Data Entity and LiveCroft Runtime Model

Tasks to implement the `Croft` base entity, `LiveCroft` runtime specialization,
and `ItemStore` / `RanchStore` abstractions described in [plan.md](./plan.md).

## Tasks

- [x] **T1 — Implement `ItemStore` and `ItemStoreEvent`**
  - **Plan reference:**
    [1. ItemStore and ItemStoreEvent](./plan.md#1-itemstore-and-itemstoreevent-srcstoreitemrs)
  - **Files:** `src/store/item.rs` (new), `src/store/mod.rs` (modified),
    `src/store/README.md` (modified)
  - **Description:** Implement `ItemStore` (with associated type `Item`) and
    `ItemStoreEvent<K, I>` (`ItemCreated`, `ItemPatched`, `ItemRemoved`). Remove
    `src/store/seed.rs`. Update `src/store/mod.rs` and `src/store/README.md`.
  - **Definition of Done:**
    - `ItemStore` defined with `type Key` and `type Item`.
    - `ItemStoreEvent<K, I>` defined with `ItemCreated`, `ItemPatched`,
      `ItemRemoved`.
    - `src/store/mod.rs` re-exports `item::*`.
    - Unit tests in `src/store/item.rs` pass.
  - **Depends on:** none

- [x] **T2 — Implement `RanchStore`**
  - **Plan reference:** [2. RanchStore](./plan.md#2-ranchstore-srcstoreranchrs)
  - **Files:** `src/store/ranch.rs` (modified), `src/store/mod.rs` (modified),
    `src/store/README.md` (modified)
  - **Description:** Update `RanchStore` combining
    `ItemStore<Key = String, Item = Vec<u8>>` and `SpreadStore`.
  - **Definition of Done:**
    - `RanchStore` requires `ItemStore` and `SpreadStore`.
    - Unit tests pass.
  - **Depends on:** T1

- [x] **T3 — Update Barn for `ItemStore` and `RanchStore`**
  - **Plan reference:** [Barn Storage Alignment](./plan.md#module--file-changes)
  - **Files:** `src/store/barn/events.rs` (modified),
    `src/store/barn/storage.rs` (modified), `src/store/barn/mod.rs` (modified),
    `src/store/barn/README.md` (modified)
  - **Description:** Update `Barn` to emit `ItemStoreEvent`s (`ItemCreated`,
    `ItemPatched`, `ItemRemoved`) and implement
    `ItemStore<Key = String, Item = Vec<u8>>` and `RanchStore`.
  - **Definition of Done:**
    - `Barn::Event` includes `ItemCreated`, `ItemPatched`, `ItemRemoved` and
      implements `TryFrom<Event> for ItemStoreEvent<String, Vec<u8>>`.
    - `Storage` emits `ItemStoreEvent`s.
    - `Barn` implements `ItemStore` and `RanchStore`.
    - All tests in `src/store/barn/` pass.
  - **Depends on:** T2

- [x] **T4 — Implement `Croft` base entity and `LiveCroft` compound with
      `Deref`**
  - **Plan reference:**
    [3. Croft Base Data Entity](./plan.md#3-croft-base-data-entity-srccrofttypesrs)
    and
    [4. LiveCroft Runtime Specialization](./plan.md#4-livecroft-runtime-specialization-srccroftmodrs)
  - **Files:** `src/croft/types.rs` (modified), `src/croft/mod.rs` (modified),
    `src/croft/README.md` (modified)
  - **Description:** Define `Croft` in `src/croft/types.rs` with `is_server()`
    and `is_worker()` helpers. Define `LiveCroft` in `src/croft/mod.rs`
    embedding `base: Croft`, required `barn: Arc<Barn>`, and required
    `gate: Gate`. Implement `std::ops::Deref<Target = Croft>` for `LiveCroft`.
    Implement `LiveCroft::spawn` and `LiveCroft::persist`.
  - **Definition of Done:**
    - `Croft` cleanly serializes and deserializes from JSON without
      infrastructure fields.
    - `LiveCroft` exposes all `Croft` attributes and helper methods
      transparently via `Deref`.
    - `LiveCroft::persist` writes the serialized `Croft` under `croft/{id}` in
      the Barn.
    - Unit tests in `src/croft/types.rs` and `src/croft/mod.rs` pass.
  - **Depends on:** T3

- [x] **T5 — Update `CroftClerk` and `CroftApi` to operate on `Croft`**
  - **Plan reference:**
    [5. CroftClerk & Conversions](./plan.md#5-croftclerk--conversions-srccroftclerk)
  - **Files:** `src/croft/clerk/mod.rs` (modified), `src/croft/clerk/api.rs`
    (modified)
  - **Description:** Refactor `Clerk::get`, `Clerk::list`, and `Clerk::join` to
    accept and return `Croft`. Implement `TryFrom<JoinReq> for Croft`,
    `TryFrom<ProtoCroft> for Croft`, and `From<Croft> for ProtoCroft` in
    `api.rs`.
  - **Definition of Done:**
    - `Clerk::get` returns `Result<Option<Croft>, ...>`.
    - `Clerk::list` returns `Result<Vec<Croft>, ...>`.
    - `Clerk::join` accepts `Croft` and returns `Result<Vec<Croft>, ...>`.
    - `ProtoCroft` <-> `Croft` conversions work directly and preserve
      `joined_at`.
    - Unit tests in `src/croft/clerk/` pass.
  - **Depends on:** T4

- [x] **T6 — Update `Regent` runtime and lifecycle coordination**
  - **Plan reference:**
    [6. Regent Coordination](./plan.md#6-regent-coordination-srcregentmodrs)
  - **Files:** `src/regent/mod.rs` (modified), `src/regent/README.md` (modified)
  - **Description:** Update `start()`, `bootstrap_join()`, and `dial_join()` in
    `src/regent/mod.rs` to work with `LiveCroft` and `Croft`.
  - **Definition of Done:**
    - `dial_join` takes `&Croft` and returns `Vec<Croft>`.
    - `start()` boots `LiveCroft` and accesses `live_croft.gate` and
      `live_croft.barn` directly.
    - Unit tests in `src/regent/mod.rs` pass.
  - **Depends on:** T5

- [x] **T7 — Full workspace verification and test suite execution**
  - **Plan reference:** Post-Write Checklist
  - **Files:** all touched files
  - **Description:** Run the full Post-Write Checklist (`cargo fmt`,
    `cargo clippy`, `cargo build`, `cargo nextest run`).
  - **Definition of Done:**
    - `cargo fmt --check` passes cleanly.
    - `cargo clippy --all-targets --all-features -- -D warnings` reports zero
      warnings.
    - `cargo build` succeeds.
    - `cargo nextest run -E 'kind(lib) | kind(bin)'` passes all unit tests.
    - `cargo nextest run -E 'kind(test)'` passes all e2e tests.
  - **Depends on:** T6
