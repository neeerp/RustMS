# Integration Harness

## Overview

The integration harness lives in the `integration-harness` crate and runs end-to-end flows against already-running login and world servers.

## Config file

The harness reads `./integration-harness.toml` at the workspace root.

Required top-level fields:

- `username`
- `password`
- `character_name`
- `gender`

Optional top-level fields:

- `login_addr` default: `127.0.0.1:8484`
- `world_addr` default: `127.0.0.1:8485`

For multi-player tests, add exactly two `[[players]]` entries:

- one with `role = "sender"`
- one with `role = "recipient"`

Each player entry needs:

- `username`
- `password`
- `character_name`
- optional `gender`

Use `integration-harness.toml.example` as the template.

## Running

Start the login and world servers first, then run:

```sh
cargo test -p integration-harness -- --ignored --nocapture
```

## Notes

- The tests are `#[ignore]`d by default because they require live servers and local fixture config.
- The harness can drive first-login flows, including TOS acceptance and gender selection.
- If the configured character does not already exist, the harness will try to create it during login.
