---
id: 0020
author: @brunomacf
created: 2026-08-31
---

# [Goal] Regent Lifecycle Management

Swini operates a cluster of machines as a **Ranch**, where each individual
machine acts as an operational **Croft** housing computational resources
(**Plot**), local consensus storage (**Barn**), and network entry (**Gate**). To
supervise a Croft on behalf of the ranch owner (the user), a host process called
the **Regent** runs on each machine. The Regent boots the **Croft**, spawn
domain staff workers such as the **Plot Clerk**, and coordinates peer join
handshakes across the Ranch.

To operate Swini smoothly on developer workstations and servers, operators
require clean lifecycle commands under the `regent` CLI subcommand group:
starting the Regent in foreground or detached mode (`swini regent start`),
querying active local instances (`swini regent status`), targeting specific
local instances via `local://{name}` URI resolution, and cleanly stopping
instances (`swini regent stop`). Furthermore, local development and multi-plot
Ranch testing require the ability to run multiple isolated Regent instances
side-by-side on a single host.

## Requirements

- **Configurable Regent Startup (`swini regent start`)**:
  - Start the Regent in foreground mode by default, streaming logs directly to
    the console and blocking until terminated (e.g. via Ctrl+C).
  - Support running in the background as a detached daemon process via a
    `--detached` / `-d` flag.
  - Accept an optional configuration file path parameter (`--config` / `-c`).
  - Automatically load configuration using a precedence hierarchy: default
    values overridden by local `./swini.yml` if present in the current working
    directory, or by the explicitly provided configuration file.
  - Persist Regent metadata and state under a global user directory
    (`~/.swini/regents/{name}/state.json`) based on the Regent's configured name
    (defaulting to `"main"`).
- **Multi-Regent Local Isolation**:
  - Allow multiple distinct Swini Regents to operate simultaneously on a single
    host by giving each instance a unique name, data directory, and network bind
    port.
- **Local Regent Targeting**:
  - Enable CLI commands to target specific local Regents using `local://{name}`
    URI addressing (e.g., via `SWINI_ADDR`), resolving the active endpoint from
    its stored local state.
- **Regent Status Inspection (`swini regent status`)**:
  - Query and display status for all active local Regents, or inspect the
    Ranch-wide Plot topology if connected to an active Regent.
  - Display vital process metrics including Regent name, PID, gRPC API address,
    status (running or dead/stale), and start timestamp.
- **Graceful Termination (`swini regent stop`)**:
  - Stop a running Regent instance by name (or default `"main"`), sending
    graceful shutdown signals to allow components to persist state and unbind
    network ports cleanly, followed by cleanup of runtime state files.
- **Croft Operational Compound & Gate Network Entry**:
  - The Regent instantiates the **Croft** (`src/croft/mod.rs`), which
    encapsulates the domain **Plot**, consensus storage **Barn**, network
    **Gate**, and active configuration.
  - Components register their gRPC services (`PlotApi`, `BarnApi`) onto the Gate
    (`gate.add(...)` / `croft.gate.add(...)`).
  - Domain staff workers (such as `PlotClerk` in `src/plot/clerk/`) are
    instantiated with `Arc<Croft>`, mounting their endpoints directly to the
    Croft's Gate.
  - The Gate binds to the configured network address and serves all registered
    endpoints.
- **Dual Telemetry & Logging**:
  - When running in foreground mode (default), output structured, formatted logs
    to the console (stdout) in addition to the file log.
  - When running in detached mode (`--detached`), disable console output and
    direct logs exclusively to the persistent log file.
  - Support automatic daily log rotation for log files under `{data_dir}/logs/`
    to prevent unbounded log growth.
  - Support configurable log filtering via `SWINI_LOG_LEVEL` environment
    variable (with `RUST_LOG` fallback).

## Constraints

- **Single-Host Process Coordination**: The lifecycle commands manage local
  processes on the current machine. Remote Ranch coordination is mediated
  through the Clerk gRPC API.
- **Process Management Boundaries**: Process supervision is implemented natively
  in Rust (PID tracking and POSIX signals) without depending on external system
  service managers (systemd, launchd) in this phase.
- **Log Streaming Deferred**: Real-time log following (`swini regent logs`) is
  deferred to a follow-up feature; daily file logs are produced to
  `{data_dir}/logs/regent.log` for inspection.
- **Binary Distribution Only**: Follows Swini's core architecture rules
  (binary-only, stable Rust, no crates.io publication).

## Scenarios

**GIVEN** no configuration file is explicitly provided and no local YAML file
exists **WHEN** an operator runs `swini regent start` **THEN** the Regent boots
in foreground mode by default using default settings (name `"main"`, default
port `7440`, and data directory `~/.swini/regents/main`), records state in
`~/.swini/regents/main/state.json`, streams logs to the console and writes
rotated daily logs to `~/.swini/regents/main/logs/regent.log`, and blocks until
terminated

**GIVEN** a configuration file defining `name: "worker-1"` and
`addr: "127.0.0.1:7441"` **WHEN** an operator runs
`swini regent start --config ./worker-1.yml --detached` **THEN** the Regent
forks into the background, records its state in
`~/.swini/regents/worker-1/state.json`, writes logs exclusively to
`~/.swini/regents/worker-1/logs/regent.log` with console logging disabled, and
returns control to the terminal

**GIVEN** one or more Regents are running on the local host **WHEN** an operator
runs `swini regent status` **THEN** the CLI lists all detected Regent instances
with their name, PID, API bind address, and running state

**GIVEN** a running Regent named `"main"` **WHEN** an operator runs
`swini regent stop main` **THEN** the process receives a termination signal,
shuts down its Clerk server and Barn stores cleanly, and the CLI removes or
marks the state file as stopped

**GIVEN** multiple Regents running locally (`"main"` on port 7440 and
`"worker-1"` on port 7441) **WHEN** an operator executes a CLI command with
`SWINI_ADDR=local://worker-1` **THEN** the CLI resolves the endpoint address of
`"worker-1"` from `~/.swini/regents/worker-1/state.json` and connects to
`http://127.0.0.1:7441`

<!-- plan-notes (for /mad.plan — not part of the product spec):
- Scoped Subcommands:
  - `swini regent start` (with `-c/--config`, `-d/--detached`)
  - `swini regent stop` (with optional `name`)
  - `swini regent status` (with optional `name`)
- Terminology & Architecture:
  - `Croft`: Physical computing compound on a single machine in `src/croft/mod.rs` holding `plot`, `barn`, `gate`, and `config`.
  - `Plot`: Parcel of computational resources in `src/plot/` (`types.rs`, `clerk/mod.rs`, `clerk/api.rs`).
  - `Ranch`: The cluster of Plots / Crofts.
  - `Regent`: Host process/supervisor managing local Croft in `src/regent/`.
  - `Gate`: Network gateway host in `src/core/gate.rs` with `add(&self, svc)`.
  - `Barn`: Self-contained storage and Raft consensus in `src/store/barn/`, registering `barn.api()` directly to `Gate`.
  - `PlotClerk`: Domain staff worker in `src/plot/clerk/`, taking `Arc<Croft>` and registering `plot.api()` onto `croft.gate`.
- Configuration loading & merging (`src/core/config.rs`):
  - Add YAML deserialization support (`serde_yml`).
  - Implement config hierarchy: Default Config -> CWD `swini.yml` (if no path given) -> `--config <path>` (if specified) -> Environment variables (`SWINI_*`).
  - Resolve global paths: base directory `~/.swini/regents/{name}` containing `state.json`, `data/`, and `logs/regent.log`.
- Regent start architecture & component wiring (`src/regent/mod.rs`):
  - Foreground execution default; detached mode when `--detached` / `-d` is passed.
  - If detached: fork first via `daemonize` before initializing async runtimes or logging background threads.
  - Spawns Croft compound (`Croft::spawn(&config)`).
  - Hires PlotClerk staff member (`PlotClerk::spawn(croft.clone())`).
  - Executes outbound Ranch join if `config.join_addresses` is present.
  - Starts Gate gRPC listener and awaits shutdown signals.
- Telemetry & Logging (`src/core/telemetry.rs`):
  - Daily rolling file appender writing to `{data_dir}/logs/regent.log`.
  - Console layer enabled when foreground, disabled when detached.
  - `SWINI_LOG_LEVEL` filtering with `RUST_LOG` fallback.
- CLI Commands (`src/cli/mod.rs`):
  - Scoped subcommands: `swini regent start`, `swini regent stop`, `swini regent status`.
  - `resolve_api_url` supporting `local://{name}` looking up `~/.swini/regents/{name}/state.json`.
-->
