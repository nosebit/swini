---
id: lore
status: living # this document is updated continuously, not versioned like specs
author: @{username}
created: {datetime}
updated: {datetime}
---

# {Project Name} Lore

<!--
One or two paragraphs describing what the project is, what problem it solves,
and where a fuller description already lives (top-level README, docs site,
etc). Point to it instead of duplicating it.

EXAMPLE:

Swini is a distributed workload orchestrator and supervisor written in Rust.
It makes it easy to launch, scale, monitor and keep applications running
continuously across a set of machines. For a full description of the project,
its goals, and its domain language, read the top-level project overview at
[`README.md`](../../README.md).
-->

## Architecture

<!--
Point to (don't duplicate) the source of truth for the codebase map — e.g. a
top-level src/README.md — or list the top-level module directories and what
each owns. Instruct future agents to read module-level READMEs before editing
code in that module.
-->

## Constraints

<!--
Hard rules the codebase must follow: distribution model, language/runtime
version floors, forbidden patterns, import/path conventions, etc. Write them
RFC-2119 style (MUST / MUST NOT / SHOULD / MAY) so an agent can follow them
literally. Include short correct/incorrect code examples for any rule about
code shape.

EXAMPLE:

- MUST NOT use fully-qualified paths inline (e.g. `crate::a::b::c::d::Type`).
- MUST add `use` declarations at the top of each file to bring items into
  scope.
-->

**Comments:**

<!-- Doc comment and inline comment conventions. -->

**Testing:**

<!-- Where tests live, how unit vs e2e tests are run, coverage expectations. -->

## Post-Write Checklist

<!--
The exact, ordered list of commands an agent MUST run after writing or
modifying code, and MUST get passing before considering a task done (format,
lint, build, unit tests, e2e tests, ...). This is read by
`.mad/commands/exec.md` after every implementation task, so keep it accurate
and runnable as-is.

EXAMPLE:

```bash
# 1. Format
cargo fmt

# 2. Lint (warnings are treated as errors)
cargo clippy --all-targets --all-features -- -D warnings

# 3. Compile
cargo build

# 4. Unit tests
cargo nextest run -E 'kind(lib) | kind(bin)'
```
-->
