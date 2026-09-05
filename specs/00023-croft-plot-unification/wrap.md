---
id: 0023
plan: specs/00023-croft-plot-unification/plan.md
todo: specs/00023-croft-plot-unification/todo.md
author: @brunomacf
created: 2026-09-05
---

# [Wrap] Unify Plot into Croft

## T4 & T5 — Debug Implementations for Croft Compound & Gate

Implemented manual `std::fmt::Debug` for `Croft` and `Gate` to cleanly handle skipped internal state and `Option<Arc<Barn>>` without requiring `Barn` or tonic routers to implement `Debug`.

**plan.md updated:** yes

## T6 — Bootstrap Join Signature

Updated `bootstrap_join` signature in `src/regent/mod.rs` to take `(&Config, &Croft)` directly, cleanly accessing `config.join_addresses` without needing a redundant config reload or accessor.

**plan.md updated:** yes
