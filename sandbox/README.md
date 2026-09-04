# Sandbox Testing Environment

This directory contains sandbox configurations to spin up and test multiple
isolated Swini Regents locally on a single machine.

All runtime state, logs, and database files are placed under `sandbox/.data/`
(git ignored).

---

## Configurations

The sandbox models all role configurations across a **3-node Raft consensus
cluster** and a **pure worker node**:

| File                               | Regent Name | Address          | Roles              | Tags                       | Join Peers                                           | Description                                     |
| :--------------------------------- | :---------- | :--------------- | :----------------- | :------------------------- | :--------------------------------------------------- | :---------------------------------------------- |
| **[`swini-1.yml`](./swini-1.yml)** | `swini-1`   | `127.0.0.1:7441` | `server`           | `zone-a`, `primary`        | _None (Seed Server)_                                 | Dedicated Server node (bootstraps Raft quorum). |
| **[`swini-2.yml`](./swini-2.yml)** | `swini-2`   | `127.0.0.1:7442` | `server`           | `zone-a`, `secondary`      | `127.0.0.1:7441`                                     | Dedicated Server node (joins Raft quorum).      |
| **[`swini-3.yml`](./swini-3.yml)** | `swini-3`   | `127.0.0.1:7443` | `server`, `worker` | `zone-b`, `tertiary`       | `127.0.0.1:7441`, `127.0.0.1:7442`                   | Dual Server + Worker node (3rd voting peer).    |
| **[`swini-4.yml`](./swini-4.yml)** | `swini-4`   | `127.0.0.1:7444` | `worker`           | `zone-b`, `compute`, `gpu` | `127.0.0.1:7441`, `127.0.0.1:7442`, `127.0.0.1:7443` | Non-voting dedicated Worker node.               |

---

## Quickstart

### 1. Build the Swini Binary

```bash
cargo build
```

### 2. Start all 4 Regents (Detached / Background Mode)

```bash
cargo run -- regent start -c sandbox/swini-1.yml -d
cargo run -- regent start -c sandbox/swini-2.yml -d
cargo run -- regent start -c sandbox/swini-3.yml -d
cargo run -- regent start -c sandbox/swini-4.yml -d
```

### 3. Check Running Status

```bash
cargo run -- regent status
```

You should see a tabular overview of all 4 running instances:

```text
NAME            PID      ADDR                   STATUS     STARTED AT
-------------------------------------------------------------------------------
swini-1         12345    127.0.0.1:7441         Running    2026-09-04T15:30:00+00:00
swini-2         12346    127.0.0.1:7442         Running    2026-09-04T15:30:01+00:00
swini-3         12347    127.0.0.1:7443         Running    2026-09-04T15:30:02+00:00
swini-4         12348    127.0.0.1:7444         Running    2026-09-04T15:30:03+00:00
```

### 4. Inspect Logs

Each Regent streams its daily rolling logs into `sandbox/.data/{name}/logs/`:

```bash
cat sandbox/.data/swini-1/logs/regent.log.*
cat sandbox/.data/swini-2/logs/regent.log.*
cat sandbox/.data/swini-3/logs/regent.log.*
cat sandbox/.data/swini-4/logs/regent.log.*
```

### 5. Stop Regents

```bash
cargo run -- regent stop swini-1
cargo run -- regent stop swini-2
cargo run -- regent stop swini-3
cargo run -- regent stop swini-4
```

Or query status for a specific instance:

```bash
cargo run -- regent status swini-1
```
