# Integration Harness

## Overview

The integration harness lives in the `integration-harness` crate and runs end-to-end flows against running login and world servers.

The harness reads `integration-harness.toml` at the workspace root.

## Config

`HarnessConfig` uses the top-level fields in `integration-harness.toml`:

- `username`
- `password`
- `character_name`
- `gender`
- optional `login_addr`
- optional `world_addr`

`MultiHarnessConfig` uses:

- optional `login_addr`
- optional `world_addr`
- at least two `[[players]]` entries

Two-player flows require exactly one player with `role = "sender"` and one player with `role = "recipient"`.

Each `[[players]]` entry needs:

- `role`
- `username`
- `password`
- `character_name`
- optional `gender`

Use `integration-harness.toml.example` as the template.

## Preconditions

The tests load config through `integration_harness::preconditions`.

Those helpers:

- load the config file
- fail immediately when the config file is missing
- check that the login server is reachable
- check that the world server is reachable

If a server is down, the tests fail during the precondition step before packet flow begins.

## Running

Run the harness with:

```sh
cargo test -p integration-harness -- --nocapture
```

Run a single test with:

```sh
cargo test -p integration-harness presence_same_map -- --nocapture
```

## Coverage

The harness test suite contains:

- `login_to_world_happy_path`
- `whisper_between_two_players`
- `presence_same_map`
- `movement_same_map`
- `local_chat_same_map`

The same-map tests assume both fixture players land on the same starting map.

## Behavior under test

The harness covers:

- login handshake and world handoff
- character load and `SetField`
- same-map foreign-player spawn packets
- same-map movement replication
- same-map local chat
- whisper delivery and failure handling

Two-player flows read same-map presence packets before asserting later actions such as whisper or chat.
