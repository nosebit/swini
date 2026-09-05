---
id: 0020
plan: specs/00020-daemon-lifecycle/plan.md
todo: specs/00020-daemon-lifecycle/todo.md
author: @brunomacf
created: 2026-09-03
---

# [Wrap] Regent Lifecycle Management

## T4 — Implement Core Configuration Resolver

Added `regents_dir() -> PathBuf` helper function in `src/core/config.rs` to
compute `~/.swini/regents` without relying on string slicing or empty path
segments.

**plan.md updated:** yes

## T6, T7 & T8 — Domain Modularity, Croft Compound & Gate Architecture

Restructured domain responsibilities from horizontal layers and established the
Croft operational compound:

- `src/croft/gate.rs`: Defines `Gate` (network server and router host) with
  thread-safe interior mutability (`Arc<Mutex<Option<GateState>>>`), exposing
  `pub fn add<S, B>(&self, svc: S)` and
  `pub async fn listen(&self, addr: SocketAddr)`. Removed obsolete `Clerk`
  trait.
- `src/croft/`: Encapsulates `Croft` in `src/croft/mod.rs` as the physical
  operational compound on a single machine, holding `pub plot: Plot`,
  `pub barn: Arc<Barn>`, `pub gate: Gate`, and `pub config: Config`.
- `src/plot/`: Encapsulates `mod.rs` (`Plot`, `PlotRole` and
  `Plot::new(&config)` providing persistent `plot.id`), `clerk/mod.rs`
  (`plot::Clerk` staff member taking `Arc<Croft>` and registering onto
  `croft.gate.add(...)`), and `clerk/api.rs` (`plot::Api` implementing
  `PlotApi`).
- `src/store/barn/`: Encapsulates its own gRPC service handler in
  `src/store/barn/api.rs`, mounting its service directly onto the gate via
  `Barn::spawn(&gate, barn_config)`.
- `src/regent/`: Direct host supervisor spawning `Croft`, hiring `PlotClerk`,
  coordinating peer join handshakes, and starting
  `croft.gate.listen(bind_addr)`.

**plan.md updated:** yes

## T10 — Implement Regent Lifecycle Orchestration

In `src/main.rs`, process daemonization (`daemonize.start()`) was moved before
the Tokio multi-threaded runtime initialization (`tokio::runtime::Builder`) to
avoid Unix `fork()` deadlocks and lost signal handlers on pre-existing worker
threads. In `src/regent/mod.rs`, `std::process::exit(0)` was added upon
receiving termination signals after state cleanup.

**plan.md updated:** yes
