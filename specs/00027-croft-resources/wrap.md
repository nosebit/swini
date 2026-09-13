---
id: 0027
plan: specs/00027-croft-resources/plan.md
todo: specs/00027-croft-resources/todo.md
author: @brunomacf
created: 2026-09-11
---

# [Wrap] Croft Resource Accounting and Status Inspection

## T3 / T5 — LiveCroft Persist and Standalone Regent Bootstrap

During E2E integration, `regent::start` was updated to initialize the local Barn Raft cluster when `config.join_addresses` is empty, and `LiveCroft::persist` was updated to retry writes briefly (up to 3 seconds with 100ms pauses) so that initial node registration cleanly tolerates asynchronous Raft leader election on cluster bootstrap.

**plan.md updated:** no — implementation detail for runtime stability during standalone node bootstrap.

## T8 — E2E Test Suite Layout

Per project conventions in `tests/README.md` and `tests/e2e/main.rs`, E2E tests were implemented in `tests/e2e/status.rs` and registered in `tests/e2e/main.rs` rather than as an isolated `tests/cli_status.rs` binary.

**plan.md updated:** no — preserved in wrap.md to keep plan.md as the intended design record while aligning with existing repository test harness architecture.
