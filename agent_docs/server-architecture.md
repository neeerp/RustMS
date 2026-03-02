# Server Architecture

## Overview

The server-side runtime is split into two binaries in `rust-ms`:

- `login` listens on `0.0.0.0:8484`
- `world` listens on `0.0.0.0:8485`

The actor model lives in `runtime/src/actor`.

## Login server

- `LoginServerActor` accepts TCP connections and spawns one `LoginClientActor` per client.
- Each `LoginClientActor` owns its `PacketReader`, `PacketWriter`, handshake state, and `SessionWrapper`.
- Login packet handlers are resolved through `net::get_handler(..., ServerType::Login)`.
- Handler execution is pushed through `tokio::task::spawn_blocking` because Diesel/database work is synchronous.
- Handlers do not write to sockets directly. They return `HandlerResult` values containing `HandlerAction`s such as:
  - `Reply`
  - `CreateSession`
  - `AttachCharacter`
  - `Disconnect`
- The login actor interprets those actions, writes packets, and persists session state for the world handoff.

## World server

- `world` starts a single `WorldServerActor` plus one `ClientActor` per accepted TCP connection.
- Each `ClientActor` owns the per-connection IO state and communicates with the central world actor over Tokio `mpsc` channels.
- `ClientActor` sends `ClientEvent`s upstream:
  - `Connected`
  - `Disconnected`
  - `MapChanged`
  - `Broadcast`
  - `Whisper`
- `WorldServerActor` is the central registry/router. It keeps:
  - `clients: HashMap<ClientId, ClientEntry>`
  - `maps: HashMap<i32, Vec<ClientId>>`
  - `names: HashMap<String, ClientId>`
- `WorldServerActor` never owns sockets directly. It routes `ServerMessage`s back to client actors through their channels.
- Broadcast and whisper fanout are handled centrally in `WorldServerActor`, not in packet handlers.

## Handler boundary

- Packet handlers live under `net/src/packet/handle`.
- They are selected by opcode in `net/src/handler.rs`.
- Handlers are intended to stay transport-agnostic: they inspect packets plus `HandlerContext`, then emit `HandlerAction`s.
- Network IO and cross-client routing belong in actors, not in handlers.

## Login-to-world transition

- Character selection on the login server emits `AttachCharacter` and replies with a redirect packet.
- The redirect currently points to `127.0.0.1:8485`.
- When the client connects to the world server, the world-side `ClientActor` processes `ReattachSession`, reloads the session from the database, sends keymap and character packets, and registers the player with `WorldServerActor`.
