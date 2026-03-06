# RustMS Agent Guide

This repository is a Rust workspace for a MapleStory server plus a nested C++ client (`HeavenClient`).

## Documentation index

- `agent_docs/server-architecture.md`: server-side actor model, handler boundaries, and login-to-world flow
- `agent_docs/integration-harness-usage.md`: commands, prerequisites, and day-to-day harness workflow
- `agent_docs/integration-harness-architecture.md`: harness design, lifecycle, and test isolation model
- `agent_docs/integration-harness-troubleshooting.md`: common harness failures and targeted fixes

## Workspace layout

- `rust-ms`: binaries (`login`, `world`)
- `runtime`: actor runtime and message types
- `net`: packet handlers/builders and handler/action abstractions
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
- The login server currently redirects to world on `127.0.0.1:8485`, so the standard setup is fully local unless that packet builder is changed.

Run the client:

```sh
cd HeavenClient/cmake-build
./HeavenClient
```

## Validate changes

Run the integration harness when validating server behavior changes:

```sh
cargo test -p integration-harness
```

After changing Rust server code, rebuild and restart the affected server binaries before trusting live test or manual client results.

For packet mismatch bugs, inspect both the Rust server packet path and the corresponding `HeavenClient` packet handler.
