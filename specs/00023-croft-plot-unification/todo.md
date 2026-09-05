---
id: 0023
plan: specs/00023-croft-plot-unification/plan.md
author: @brunomacf
created: 2026-09-05
---

# [Todo] Unify Plot into Croft

Tasks to consolidate `Plot` and `Croft` domain abstractions into a single
unified `Croft` concept representing a host machine on the Ranch, as specified
in [plan.md](./plan.md). Tasks T1–T3 prepare protobuf definitions, domain types,
and encapsulated configuration; T4–T5 build the canonical `Croft` entity and
`CroftClerk` front-office; T6–T7 update the Regent lifecycle supervisor, delete
the obsolete `plot/` subsystem, and update docs and tests.

## Tasks

- [x] **T1 — Replace `proto/plot.proto` with `proto/croft.proto` and update
      proto bindings**
  - **Plan reference:**
    [1. Protobuf Definition](./plan.md#1-protobuf-definition-protocroftproto)
  - **Files:** `proto/croft.proto` (new), `proto/plot.proto` (deleted),
    `src/core/proto/croft.rs` (new), `src/core/proto/plot.rs` (deleted),
    `src/core/proto/mod.rs` (modified)
  - **Description:** Define the `CroftApi` service (`Join`, `Status`) and
    message types (`JoinReq`, `JoinRes`, `StatusReq`, `StatusRes`, `Croft`)
    under package `swini.croft` in `proto/croft.proto`. Remove
    `proto/plot.proto`. Create `src/core/proto/croft.rs` including
    `"swini.croft"` and update `src/core/proto/mod.rs` to expose
    `pub mod croft;`.
  - **Definition of Done:**
    - `proto/croft.proto` exists with package `swini.croft` and service
      `CroftApi`.
    - `proto/plot.proto` is deleted.
    - `src/core/proto/croft.rs` includes generated `swini.croft` bindings.
    - `cargo build` compiles tonic protobuf stubs without errors.
    - Passes Post-Write Checklist (`cargo fmt`, `cargo clippy`, `cargo build`).
  - **Depends on:** none

- [x] **T2 — Implement `src/croft/types.rs` with `CroftRole`**
  - **Plan reference:**
    [2. Domain Roles](./plan.md#2-domain-roles-srccrofttypesrs)
  - **Files:** `src/croft/types.rs` (new)
  - **Description:** Implement `CroftRole` (`Server`, `Worker`) enum with
    `Display`, `FromStr`, and Serde serialization support in
    `src/croft/types.rs`. Add unit tests verifying role parsing and JSON
    roundtrips.
  - **Definition of Done:**
    - `CroftRole` parses `"server"` and `"worker"` (case-insensitively) and
      returns an error for invalid roles.
    - Unit tests in `src/croft/types.rs` test string parsing and JSON
      serialization roundtrips.
    - Passes Post-Write Checklist.
  - **Depends on:** none

- [x] **T3 — Relocate and encapsulate `Config` in `src/croft/config.rs`**
  - **Plan reference:**
    [3. Croft Configuration & Filesystem Layout](./plan.md#3-croft-configuration--filesystem-layout-srccroftconfigrs)
  - **Files:** `src/croft/config.rs` (new), `src/core/config.rs` (deleted),
    `src/core/mod.rs` (modified)
  - **Description:** Move `Config` into `src/croft/config.rs`, updating it to
    use `CroftRole`. Update filesystem resolution functions `crofts_dir()`
    (`~/.swini/crofts`) and `default_data_dir(name)` (`~/.swini/crofts/{name}`).
    Delete `src/core/config.rs` and update `src/core/mod.rs`.
  - **Definition of Done:**
    - `Config` uses `CroftRole`.
    - `crofts_dir()` returns `~/.swini/crofts`.
    - Unit tests in `src/croft/config.rs` test default values, custom YAML
      loading, and environment variable overrides.
    - Passes Post-Write Checklist.
  - **Depends on:** T2

- [x] **T4 — Redefine `Croft` compound and persistence in `src/croft/mod.rs`**
  - **Plan reference:**
    [4. Canonical Croft Entity](./plan.md#4-canonical-croft-entity-srccroftmodrs)
  - **Files:** `src/croft/mod.rs` (modified)
  - **Description:** Update `Croft` struct to hold machine identity (`id`,
    `name`, `addr`, `roles`, `tags`, `joined_at`) and local runtime
    infrastructure (`barn: Option<Arc<Barn>>`, `gate: Option<Gate>`) with
    `#[serde(skip)]`. Implement `Croft::spawn(&config)`, private `id_provide`
    (`{data_dir}/croft.id`), `persist()`, `is_server()`, and `is_worker()`.
    Re-export `clerk`, `config`, `gate`, and `types`.
  - **Definition of Done:**
    - `Croft` holds identity and optional runtime infrastructure skipped by
      serde.
    - `Croft::spawn` initializes `Barn` and `Gate`.
    - `persist()` saves JSON-serialized `Croft` to Barn under `croft/{id}`.
    - Unit tests verify `Croft::spawn`, `croft.id` provisioning, and JSON
      serialization roundtrip (ensuring `barn`/`gate` are skipped).
    - Passes Post-Write Checklist.
  - **Depends on:** T1, T2, T3

- [x] **T5 — Implement `CroftClerk` and `CroftApi` in `src/croft/clerk/`**
  - **Plan reference:**
    [5. Croft Clerk & gRPC Service Handler](./plan.md#5-croft-clerk--grpc-service-handler-srccroftclerk)
  - **Files:** `src/croft/clerk/mod.rs` (new), `src/croft/clerk/api.rs` (new),
    `src/croft/gate.rs` (modified)
  - **Description:** Implement `Clerk` in `src/croft/clerk/mod.rs` operating on
    Barn key prefix `croft/` with `get`, `list`, `join` methods. Implement gRPC
    handler in `src/croft/clerk/api.rs` for `CroftApi` (`Join`, `Status`),
    protobuf validation, and wire conversions. Update `src/croft/gate.rs` tests
    to use `CroftApi`.
  - **Definition of Done:**
    - `Clerk::spawn` mounts `CroftApiServer` onto `croft.gate`.
    - `Clerk::get`, `list`, and `join` operate on `croft/{id}` storing and
      returning `Croft` entities.
    - `api.rs` implements tonic `CroftApi` service with protobuf validation and
      conversion.
    - Unit tests in `src/croft/clerk/mod.rs` and `src/croft/clerk/api.rs` verify
      storage operations and RPC handling.
    - Passes Post-Write Checklist.
  - **Depends on:** T1, T4

- [x] **T6 — Update Regent lifecycle and state in `src/regent/`**
  - **Plan reference:**
    [6. Regent Lifecycle & State File](./plan.md#6-regent-lifecycle--state-file-srcregentmodrs--srcregentstaters)
  - **Files:** `src/regent/mod.rs` (modified), `src/regent/state.rs` (modified),
    `src/cli/regent.rs` (modified), `src/store/barn/config.rs` (modified)
  - **Description:** Update `RegentState` to serialize to
    `{data_dir}/regent.json` and use `crate::croft::Config`. Update
    `regent::start` to hire `CroftClerk`, `dial_join` to use `CroftApiClient`
    and `&Croft`, and `candidate_data_dirs` to inspect
    `crate::croft::config::crofts_dir()`. Update `src/cli/regent.rs` and Barn
    comments.
  - **Definition of Done:**
    - `RegentState` saves and loads `{data_dir}/regent.json`.
    - `bootstrap_join` and `dial_join` coordinate cluster joins via `CroftApi`.
    - `status` and `stop` locate active Crofts under `~/.swini/crofts/`.
    - Unit tests in `src/regent/mod.rs` and `src/regent/state.rs` pass.
    - Passes Post-Write Checklist.
  - **Depends on:** T3, T4, T5

- [x] **T7 — Remove `src/plot/` subsystem and update documentation & e2e tests**
  - **Plan reference:** [Architecture](./plan.md#architecture) &
    [Testing Strategy](./plan.md#testing-strategy)
  - **Files:** `src/plot/` (deleted), `src/main.rs` (modified), `src/README.md`
    (modified), `src/croft/README.md` (modified), `tests/e2e/regent.rs`
    (modified)
  - **Description:** Delete `src/plot/` directory entirely. Remove
    `pub mod plot;` from `src/main.rs`. Update `src/README.md` and
    `src/croft/README.md` with the unified Croft architecture. Update E2E CLI
    tests to verify `regent start`, `status`, and `stop` against the new layout.
  - **Definition of Done:**
    - `src/plot/` is deleted and no dangling `plot` imports exist.
    - `src/README.md` and `src/croft/README.md` accurately document `Croft`,
      `CroftClerk`, and `croft::Config`.
    - All unit and e2e tests pass
      (`cargo nextest run -E 'kind(lib) | kind(bin)'` and
      `cargo nextest run -E 'kind(test)'`).
    - Passes complete Post-Write Checklist.
  - **Depends on:** T6

## Task Order

- **T1** (protobuf) and **T2** (domain roles) have no dependencies and can be
  worked on first.
- **T3** (config) depends on **T2**.
- **T4** (Croft compound) depends on **T1**, **T2**, and **T3**.
- **T5** (CroftClerk & CroftApi) depends on **T1** and **T4**.
- **T6** (Regent lifecycle & join) depends on **T3**, **T4**, and **T5**.
- **T7** (Plot removal, docs, e2e tests) finishes the migration once all
  components are wired to Croft.
