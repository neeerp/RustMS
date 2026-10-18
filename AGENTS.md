# RustMS Agent Guide

This repository is a Rust workspace for a MapleStory server plus a nested C++ client (`HeavenClient`).

## Documentation index

- `agent_docs/server-architecture.md`: crates, actor topology, state ownership, persistence, sessions, handler registry, and how to add a feature
- `agent_docs/fields.md`: field actors, occupants, map changes, and field packets
- `agent_docs/npc-dialogue.md`: asset-backed conversations, runtime ownership, and future quest integration
- `agent_docs/quests.md`: the v83 client-driven quest flow, supported requirements/actions, persistence, and rewards
- `agent_docs/map-travel.md`: asset portal links, active beginner route, legacy map variants, and map/portal scripts
- `agent_docs/combat.md`: mob spawning and control, melee attacks, EXP and level-ups, spending AP and SP, drops and pickup, touch damage and death
- `agent_docs/parties.md`: party state and flows, EXP sharing, party chat, and the client's party window and HP displays
- `agent_docs/buddies.md`: buddy list storage, requests, channel visibility, buddy chat, and the client's Friend tab
- `agent_docs/inventory.md`: inventory operations, item data, item moves, NPC shops, storage, and their packets
- `agent_docs/server-content.md`: the ignored content pack (script text, drops), placeholders, and the importer
- `agent_docs/client-testing.md`: fast client logic tests and private-display GUI verification
- `agent_docs/quest-window.md`: original quest UI assets, layout, controls, and visual validation
- `agent_docs/client.md`: HeavenClient origin, license, NX file versions, feature status, and fixed protocol deviations
- `agent_docs/scripting.md`: proposed decision for data-driven quests plus Lua scripts
- `agent_docs/integration-harness-usage.md`: commands, prerequisites, and day-to-day harness workflow
- `agent_docs/integration-harness-architecture.md`: harness design, lifecycle, and test isolation model
- `agent_docs/integration-harness-troubleshooting.md`: common harness failures and targeted fixes

## Workspace layout

- `rust-ms`: binaries (`login`, `world`)
- `runtime`: actor runtime and message types
- `net`: packet handlers/builders, the opcode registry, and shared encoders (`net::codec`)
- `game`: domain model (`Player`, stats, look, inventory, skills, keymap, quests)
- `game-data`: static data read from NX files (see `assets/game-data/README.md`)
- `db`: Diesel-backed persistence and repositories
- `packet`: packet type plus read/write helpers
- `crypt`: MapleStory crypto helpers
- `integration-harness`: async end-to-end test harness
- `HeavenClient`: standalone C++ client used for local manual testing

## Build and launch the servers

Run everything from the workspace root unless noted otherwise.

During feature work, the normal rebuild loop is:

```sh
cargo build --workspace
```

If you only changed server code and want a quick rebuild of the binaries:

```sh
cargo build -p rust-ms
```

### Launch the login server

```sh
RUST_LOG=runtime=info cargo run -p rust-ms --bin login
```

### Launch the world server

In a second terminal:

```sh
RUST_LOG=runtime=info cargo run -p rust-ms --bin world
```

## Build and launch the client

During feature work, rebuild from `HeavenClient` with:

```sh
cd HeavenClient
cmake --build cmake-build -j"$(nproc)"
```

Notes:

- `HeavenClient/MapleStory.h` has `USE_CRYPTO` enabled; keep that on for RustMS.
- The generated executable is `HeavenClient/cmake-build/HeavenClient`.
- The client reads host/port from a `Settings` file. Defaults are `ServerIP = 127.0.0.1` and `ServerPort = 8484`.
- By default the world server hosts three channels on `127.0.0.1:8485`, `8486`, and `8487` (one port per channel; configure with `RUSTMS_LOGIN_CHANNELS`), so the standard setup is fully local.
- Set `RUSTMS_ALLOW_DEBUG_WARPS=1` on the world server to allow map-id warps for every player during local testing.

Run the client:

```sh
cd HeavenClient/cmake-build
./HeavenClient
```

For automated native-client interaction, prefer a private Xvfb display and
separate Settings directory so tests do not steal the user's desktop focus.
See `agent_docs/client-testing.md` for setup and input/screenshot commands.

## Validate changes

Run the integration harness when validating server behavior changes:

```sh
cargo run -p integration-harness --bin harnessctl -- test
```

Use `harnessctl` as the default integration-harness entrypoint because it brings up the isolated Docker stack, runs the tests against the harness endpoints, and tears the stack down.

If another checkout (for example a worktree used by a parallel session) may be running the harness, give yours its own stack first; see "Running several stacks at once" in `agent_docs/integration-harness-usage.md`.

If you need to run a specific integration test while reusing an already-running harness stack, use:

```sh
cargo run -p integration-harness --bin harnessctl -- up
HARNESS_LOGIN_ADDR=127.0.0.1:18484 HARNESS_WORLD_ADDR=127.0.0.1:18485 cargo test -p integration-harness --test presence_same_map -- --nocapture
cargo run -p integration-harness --bin harnessctl -- down
```

After changing Rust server code, rebuild and restart the affected server binaries before trusting live test or manual client results.

For packet mismatch bugs, inspect both the Rust server packet path and the corresponding `HeavenClient` packet handler.
