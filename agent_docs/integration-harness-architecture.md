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
- world bind host: `RUSTMS_WORLD_BIND_HOST` (`rust-ms/src/bin/world.rs`)
- channels (names, advertised host/port, capacity): `RUSTMS_LOGIN_CHANNELS`
  (`net/src/login_world.rs`), read by both servers. The world binds each
  channel on its advertised port, so the compose file maps those ports 1:1.
- debug map-id warps: `RUSTMS_ALLOW_DEBUG_WARPS=1` (`net/src/policy.rs`)
- ship route clock speed: `RUSTMS_CONTI_SCALE=60`, so a ship's 15-minute
  cycle takes 15 seconds, and `RUSTMS_CONTI_BALROG=100`, so the Crimson
  Balrog attacks every Ellinia-Orbis trip (`net/src/conti.rs`)
- Monster Book card drop chance: `RUSTMS_CARD_DROP_RATE=1` so every kill
  drops its card (`net/src/monster_book.rs`)
- mob spawning: `RUSTMS_FULL_SPAWN=1` keeps a mob on every spawn point,
  bosses included from the start, instead of the capacity-limited v83 rule
  (`runtime/src/actor/field_mobs.rs`)

## Harness invariants

- default host ports (override with `HARNESS_LOGIN_PORT`, `HARNESS_WORLD_PORT`,
  `HARNESS_DB_PORT`):
  - login: `127.0.0.1:18484`
  - channels 1-3: `127.0.0.1:18485`, `18486`, `18487` (world port + n)
- tests connect to the address in the login redirect, as a real client does,
  and send the canonical v83 `PLAYER_LOGGEDIN` (character id only)
- test address contract:
  - harness reads `HARNESS_LOGIN_ADDR` and `HARNESS_WORLD_ADDR`
  - `harnessctl test` sets these for child test processes
- migrations:
  - both servers apply embedded migrations at startup (advisory-locked); there is
    no separate migration container
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

## Packet reads and cancellation

`MapleTestConnection` keeps incomplete encrypted frames in a persistent buffer.
`read_packet` can be interrupted by a timeout without discarding part of a
header or body. It decrypts and advances cipher state only when a full frame is
available; extra bytes for the next frame remain buffered. The production
`runtime::io::PacketReader` follows the same rule because its read future is
also cancelled by `ClientActor`'s `select!` when a server message arrives.

Regression tests deliberately interrupt a partial header and a partial body,
then resume and decode two consecutive packets. This tests TCP fragmentation
and cipher sequencing without requiring PostgreSQL or a Docker stack.

Outbound test-client packets combine the encrypted header and body into one
frame write, with TCP_NODELAY enabled for request/response traffic. This avoids
introducing delayed-ACK waits by writing the four-byte header separately.

## Remaining reliability and speed work

The harness remains useful for checking the real protocol and persistence.
Use in-process tests for frequent gameplay-rule feedback and keep a focused
end-to-end suite for integration boundaries.

Current limitations worth improving next:

- `drain_spawn_npcs` waits for entry's last packet, `CLAIM_STATUS_CHANGED`,
  but then still relies on a 100 ms quiet interval for packets that follow.
  Prefer explicit packet expectations/readiness barriers to infer completion
  from silence.
- Random identities isolate accounts, but tests within one executable can
  share maps and channels. Scope fixtures to dedicated locations or route
  background packets through a matcher instead of assuming the next packet
  belongs to the test.
- Fixed Compose project names and ports prevent independent concurrent runs.
- TCP port readiness proves that listeners are open, not that login-to-world
  behavior is healthy.
- The harness shares some protocol definitions with the server. Keep byte-level
  fixtures or client-derived contract checks so a shared encoding mistake
  cannot make both sides agree on an invalid protocol.
- Full lifecycle cleanup still needs explicit handling of external interrupts.

The wrapper records stage timings and captures failure logs before destroying
the stack. These measurements help separate slow rules/tests from compilation
and environment startup costs.
