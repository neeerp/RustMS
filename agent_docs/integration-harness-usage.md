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

Run the full suite with managed infrastructure:

```sh
cargo run -p integration-harness --bin harnessctl -- test
```

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
- `HARNESS_WORLD_ADDR`

`harnessctl test` sets these automatically to fixed host ports.

## Coverage

The suite currently includes:

- `login_to_world_happy_path`
- `whisper_between_two_players`
- `presence_same_map`
- `movement_same_map`
- `local_chat_same_map`
- `portal_map_transfer_round_trip`
- `invalid_direct_map_target_disconnects_client`
