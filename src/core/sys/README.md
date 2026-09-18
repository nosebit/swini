# Sys Subsystem

The `sys` module provides host system abstractions and platform drivers for
process execution, signaling, liveness checks, and resource sampling.

## Submodules & Components

- **[`types.rs`](./types.rs)**: Defines the [`Sys`](./types.rs) trait and
  [`ProcessStats`](./types.rs) structure.
- **[`variants/`](./variants/)**: Concrete platform adapters implementing
  [`Sys`](./types.rs):
  - **[`variants/unix.rs`](./variants/unix.rs)**: Base Unix/POSIX implementation
    using standard `libc` signaling (`SIGTERM`, `SIGKILL`) and
    `std::process::Command`.
  - **[`variants/macos.rs`](./variants/macos.rs)**: macOS Darwin platform
    adapter delegating to `UnixSys`.
  - **[`variants/linux/mod.rs`](./variants/linux/mod.rs)**: Generic Linux host
    adapter.
  - **[`variants/linux/ubuntu.rs`](./variants/linux/ubuntu.rs)**: Ubuntu Linux
    specialized adapter.
- **[`mod.rs`](./mod.rs)**: Exposes the [`select()`](./mod.rs) runtime factory
  that detects the current host OS and returns an `Arc<dyn Sys>`.
