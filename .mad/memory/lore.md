# Swini Lore

Swini is a distributed workload orchestrator and supervisor written in Rust. It
makes it easy to launch, scale, monitor and keep applications running
continuously across a set of machines.

For a full description of the project, its goals, and its domain language, read
the top-level project overview at [`README.md`](../../README.md).

## Architecture

Swini is written in Rust and for a map of the source code and what each module
is responsible for, read the [`src/README.md`](../../src/README.md). Each module
directory contains its own `README.md` with further details. Read the relevant
module README before modifying code in that module.

## Constraints

- Swini is a **binary-only** distribution. It MUST NOT be published to
  crates.io.
- The minimum supported Rust edition is **2021**. Nightly features MUST NOT be
  used.
- The pinned toolchain is defined in `rust-toolchain.toml`. Always use stable.
- MUST NOT use fully-qualified paths inline (e.g. `crate::a::b::c::d::Type`).
- MUST add `use` declarations at the top of each file to bring items into scope.
- MAY use at most one level of path depth inline after importing the parent:

  ```rust
  // Correct
  use crate::core::cluster;
  cluster::join(addr)?;

  // Incorrect — path is too long inline
  crate::core::cluster::join(addr)?;
  ```

**Comments:**

- MUST add `///` doc comments to all public functions, types, traits, and
  modules.
- SHOULD add inline `//` comments to explain non-obvious logic, edge cases, or
  intentional design decisions that a reviewer might question.
- Code MUST be readable by a human reviewer who did not write it. If a block of
  code requires context to understand, add a comment explaining the context.

**Documentation:**

- MUST: every directory under `src/` that groups related files (a module) has
  its own `README.md` describing the module's intent/responsibility and briefly
  explaining what each file in it does. Write it clearly enough that a software
  engineer or AI Agent unfamiliar with the module can understand it without
  reading all the code first.
- MUST: keep a module's `README.md` up to date — update it whenever files are
  added, removed, renamed, or repurposed within that module.

**Testing:**

- MUST write unit tests for all new functions and logic.
- Unit tests live in a `#[cfg(test)]` module at the bottom of the same file as
  the code under test.
- Run unit tests with:
  ```bash
  cargo nextest run -E 'kind(lib) | kind(bin)'
  ```
- MUST write e2e tests for all user-facing CLI behavior.
- E2e tests live in the `tests/` directory and use `assert_cmd` + `predicates`.
- Run e2e tests with:
  ```bash
  cargo nextest run -E 'kind(test)'
  ```
- Aim for high test coverage. Every new feature must have both unit and e2e
  coverage. Do not leave public functions untested.

<!----->

## Post-Write Checklist

After writing or modifying any Rust code, MUST run the following in order and
fix any errors before considering the task complete:

```bash
# 1. Format
cargo fmt

# 2. Lint (warnings are treated as errors)
cargo clippy --all-targets --all-features -- -D warnings

# 3. Compile
cargo build

# 4. Unit tests
cargo nextest run -E 'kind(lib) | kind(bin)'

# 5. E2e tests (if relevant to what was changed)
cargo nextest run -E 'kind(test)'
```

Do not report a task as done if any of these steps fail.
