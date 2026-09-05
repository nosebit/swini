# Swini

<p align="center">
  <img src="website/logo_name.png" alt="Swini" width="300" />
</p>

<p align="center">
  <a href="https://github.com/nosebit/swini/actions"><img src="https://github.com/nosebit/swini/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
  <a href="https://github.com/nosebit/swini/releases"><img src="https://img.shields.io/github/v/release/nosebit/swini.svg" alt="GitHub Release" /></a>
  <a href="https://codecov.io/gh/nosebit/swini"><img src="https://codecov.io/gh/nosebit/swini/graph/badge.svg" alt="codecov" /></a>
</p>

Swini is like Kubernetes, but dirtier 🐷. It is a distributed workload
orchestrator and supervisor that makes it easy to launch, scale, monitor and
keep your applications running continuously across a set of machines.

The entire Swini infrastructure operates a cluster of machines as a **Ranch**.
Each physical machine acts as an operational **Croft** (a working farmstead)
that unites its local computational parcel (the **Plot**), distributed consensus
storage (the **Barn**), and network entrypoint (the **Gate**). Supervising the
Croft on behalf of the ranch owner is a host process called the **Regent**. The
Regent boots the Croft, spawns domain staff workers (such as the **Plot
Clerk**), coordinates peer joins across the Ranch, and manages the process
lifecycle.

The **Barn** is powered by the Raft consensus protocol and distributes its data
across a set of plots designated as **Servers**, providing a consistent and
highly available view of the entire cluster state. A special server plot called
the **Leader** is responsible for receiving all data write operations from any
plot in the Ranch. When a new operation is received, the leader first propagates
it to all other server plots (called **Followers**). As soon as a majority of
followers acknowledge they received the operation, the leader safely commits the
data change to its local storage and then instructs the follower plots to do the
same. If the leader fails for any reason, a new leader is elected to finish
committing any pending operations, guaranteeing no data is ever lost because
every server maintains a complete, fully synchronized local copy of the store.

On top of this infrastructure, the primary workload abstraction in Swini is
called a **Pig**, a cluster-wide creature that "farrows" (spawns and places)
identical siblings called _Piglets_ onto **Plots** across the Ranch, actively
supervising them to ensure they match a desired state at any time. A **Piglet**
is a plot-level creature responsible for executing and supervising one or more
_Tasks_ inside the piglet **Yard**, a slice of the plot resources reserved for
that specific piglet. A **Task** is the smallest unit of work in Swini and can
be a container, a process or other kind of workload.

The cluster state in the Barn is used by another component called the **Drover**
to ensure a Pig's desired state is respected across the Ranch. The Drover
running on the Barn leader plot (also called the **Primary** plot) is
responsible for herding Piglets across the cluster. It finds a Plot with enough
available resources by checking the Barn, carves out a properly sized **Yard**
to reserve those resources, and places a single Piglet into that Yard. Once
assigned, it delegates to the local Drover running on that specific Plot to
actively supervise the Piglet on behalf of the Pig.

## Quick Start

To start a Swini Regent on a single machine with default configuration, simply
run:

```bash
swini regent start
```

This will start the Regent as a foreground process by default, streaming
structured logs to the console and rotating daily log files to disk. You can use
the `--detached` (`-d`) flag to start it as a background process:

```bash
swini regent start --detached
```

You can inspect all running local instances or stop a running Regent:

```bash
# Check status of local regents
swini regent status

# Stop a running regent by name (defaults to "main")
swini regent stop main
```

You can configure the Regent by providing a configuration file via the
`--config` (`-c`) argument:

```bash
swini regent start --config /path/to/swini.yml
```

The configuration file in its simplest form looks like this:

```yml
name: plot-1
addr: 127.0.0.1:7440
data_dir: ~/.swini/regents/plot-1
roles:
  - server
  - worker
tags:
  - zone-a
join_addresses: []
```

## Run a Pig

Swini looks very similar to docker-compose when it comes to how we define the
pigs to be run. You first need to create a `pigs.yml` file specifying the pigs
you want to run like this:

```yml
space: default
pigs:
  - name: web
    size: 2
    piglet:
      placement:
        yard:
          cpu: 1000
          memory: 512
      tasks:
        - name: main
          container:
            image: "nginx:latest"
```
