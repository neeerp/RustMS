# Integration Harness Architecture

## Goals

- spin up isolated infra once per suite run
- avoid per-test account fixture coupling
- keep test setup deterministic and easy to debug

## Components

- `integration-harness/src/bin/harnessctl.rs`
  - orchestrates docker lifecycle (`up`, `test`, `down`)
  - validates Docker availability before running compose commands
  - waits for login/world TCP readiness before running tests
- `integration-harness/docker-compose.test.yml`
  - `db`: isolated Postgres
  - `migrate`: one-shot migration runner over `db/migrations/*/up.sql`
  - `login` and `world`: server containers using a shared image
- `Dockerfile.rustms-server`
  - Alpine multi-stage build for `login` and `world`
  - BuildKit cache mounts for Cargo registry/git/target
  - runtime image with required shared libs (`libpq`, `libstdc++`, etc.)

## Lifecycle state model

`harnessctl test` follows this state progression:

1. `down` (best-effort cleanup of stale stack)
2. `build` (compose image build)
3. `up` (compose services start)
4. `ready` (login/world TCP readiness checks pass)
5. `run-tests` (`cargo test -p integration-harness`)
6. `down` (always attempted after tests or startup/test failure)

This keeps startup behavior deterministic and avoids stale migration/container state between runs.

## Test isolation model

- tests no longer load fixture credentials from `integration-harness.toml`
- `integration-harness/src/config.rs` generates random account and character identifiers per test
- `integration-harness/src/preconditions.rs` builds runtime config from env + generated identities and validates reachability first

This provides practical isolation while still sharing one server stack across the suite.

## Runtime configuration flow

Containerized servers use env overrides so they are decoupled from local static config files:

- DB URL: `RUSTMS_DATABASE_URL` (`db/src/settings.rs`)
- login bind addr: `RUSTMS_LOGIN_BIND_ADDR` (`rust-ms/src/bin/login.rs`)
- world bind addr: `RUSTMS_WORLD_BIND_ADDR` (`rust-ms/src/bin/world.rs`)
- world redirect target: `RUSTMS_WORLD_REDIRECT_HOST`, `RUSTMS_WORLD_REDIRECT_PORT` (`net/src/packet/build/login/world.rs`)

## Harness invariants

- fixed host ports are expected for default runs:
  - login: `127.0.0.1:18484`
  - world: `127.0.0.1:18485`
- test address contract:
  - harness reads `HARNESS_LOGIN_ADDR` and `HARNESS_WORLD_ADDR`
  - `harnessctl test` sets these for child test processes
- migration ordering:
  - `migrate` must complete successfully before `login`/`world` start
- random identity constraints:
  - username and character name are generated per test
  - generated names are truncated to Maple-compatible limits (<=13 chars)
- world asset availability:
  - world container requires mounted assets at `/app/assets`

## Performance choices

- shared image name for both login/world services to avoid duplicate builds
- root `.dockerignore` excludes large directories from build context (`target`, `HeavenClient`, etc.)
- world service mounts assets (`../assets:/app/assets:ro`) instead of baking large game data into image layers
- per-service `stop_grace_period: 100ms` to keep local teardown fast

## Failure characteristics

- if compose bring-up fails, `harnessctl test` tears stack down before returning error
- migration job must complete successfully before login/world start
- endpoint checks fail fast if services do not bind expected ports
