# Integration Harness Usage

## Overview

The integration harness lives in the `integration-harness` crate and runs end-to-end flows against an isolated Docker stack.

The stack includes:

- a dedicated PostgreSQL container
- a login server container
- a world server container

## Prerequisites

- Docker daemon running
- current user can access `/var/run/docker.sock`
- Rust toolchain available for `cargo test`

If your user was just added to the `docker` group, start a new shell session (or run `newgrp docker`).

If Docker is installed and running but your current shell is not in the `docker` group yet, you can run the harness through `sg` instead of restarting the shell:

```sh
sg docker -c 'cargo run -p integration-harness --bin harnessctl -- test'
```

## Wrapper CLI

Use `harnessctl` as the canonical entrypoint:

```sh
cargo run -p integration-harness --bin harnessctl -- test
```

Available subcommands:

- `up` - build/start containers and wait for readiness
- `down` - stop containers and remove volumes
- `test` - run full lifecycle (`up -> test -> down`)

## Common workflows

For the fast development loop, run the in-process logic, actor, and transport
tests without Docker or local NX assets:

```sh
cargo test -p net -p runtime -p integration-harness --lib
```

This is the appropriate lane for dialogue transitions and future quest rules.
It complements the real-network suite; it does not verify migrations, persistence,
login redirects, or client rendering. Asset-loader checks remain in `game-data`.

Run the full suite with managed infrastructure:

```sh
cargo run -p integration-harness --bin harnessctl -- test
```

`harnessctl test` prints separate image-build, startup/readiness, cargo-test,
and total-lifecycle timings. The cargo-test time includes compilation. First
runs may download/build images and dependencies; use a warmed run when comparing
feedback time. A full container lifecycle should not be expected to complete in
hundreds of milliseconds.

Observed on the local development machine after compilation (October 2026):

| Check | Elapsed time |
| --- | --- |
| Fast lane above (14 tests, reduced debug info) | 0.338 s wall time |
| Network suite test execution (16 tests) | 12.84 s summed executable times |
| Full lifecycle with cached images and a small host rebuild | 20.829 s |

These are measurements, not timing guarantees. Before coalescing test-client
frame writes, a warmed network suite took 16.48 s in test execution.

On failure, the wrapper prints the last 200 log lines from each container before
teardown. Save its output if you want to inspect the failure after cleanup.
The Docker build uses BuildKit cache mounts and requires Docker Buildx to be
installed (`docker buildx version`).

If the current shell does not have Docker socket access yet:

```sh
sg docker -c 'cargo run -p integration-harness --bin harnessctl -- test'
```

Run a single test while reusing a running stack:

```sh
cargo run -p integration-harness --bin harnessctl -- up
HARNESS_LOGIN_ADDR=127.0.0.1:18484 HARNESS_WORLD_ADDR=127.0.0.1:18485 cargo test -p integration-harness presence_same_map -- --nocapture
cargo run -p integration-harness --bin harnessctl -- down
```

Equivalent `sg` workflow when the shell lacks Docker group access:

```sh
sg docker -c 'cargo run -p integration-harness --bin harnessctl -- up'
HARNESS_LOGIN_ADDR=127.0.0.1:18484 HARNESS_WORLD_ADDR=127.0.0.1:18485 cargo test -p integration-harness presence_same_map -- --nocapture
sg docker -c 'cargo run -p integration-harness --bin harnessctl -- down'
```

## Addresses

The harness reads endpoints from environment variables:

- `HARNESS_LOGIN_ADDR`
- `HARNESS_WORLD_ADDR` (channel 1; channel `n` is this port + `n`)

`harnessctl test` sets these automatically.

Tests that need state the protocol can't create, such as an NX balance, use
`integration_harness::database` (`set_nx_credit`). It connects to
`HARNESS_DATABASE_URL`, which `harnessctl test` also sets; when reusing a
stack, pass it yourself, for example
`HARNESS_DATABASE_URL=postgres://maplestory:pass@127.0.0.1:15432/maplestory_test`.

## Running several stacks at once

Each checkout (for example a git worktree) can run its own stack by giving it
a distinct project, ports, and image tag. `harnessctl` and the compose file
read:

| Variable | Default |
| --- | --- |
| `HARNESS_PROJECT_NAME` | `rustms-integration` |
| `HARNESS_LOGIN_PORT` | `18484` |
| `HARNESS_WORLD_PORT` | `18485` (channels use this, +1, +2; the vote hook +3) |
| `HARNESS_DB_PORT` | `15432` |
| `HARNESS_IMAGE` | `rustms-server:integration` |
| `HARNESS_ASSETS_DIR` | `../assets` (must be a real directory, not symlinks, because it is bind-mounted) |

Without overrides, two checkouts share one project and `harnessctl down` in
one tears down the other's stack.

## Coverage

Each file in `integration-harness/tests/` is one test binary; list them with
`ls integration-harness/tests`. They cover login to world, character
creation with starting equipment, channel redirects and changes, presence,
movement, chat, whisper, portal and direct warps, NPC spawns and dialogue,
and the beginner quests.
