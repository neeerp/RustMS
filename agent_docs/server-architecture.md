# Server Architecture

## Overview

RustMS runs two binaries in `rust-ms/src/bin/`:

- `login` listens on `0.0.0.0:8484`
- `world` listens on `0.0.0.0:8485`

The actor model lives in `runtime/src/actor`.

## Login server

- `LoginServerActor` accepts TCP connections and spawns one `LoginClientActor` per client.
- Each `LoginClientActor` owns its `PacketReader`, `PacketWriter`, handshake state, and `SessionWrapper`.
- Login packet handlers are resolved through `net::get_handler(..., ServerType::Login)`.
- Handler execution runs inside `tokio::task::spawn_blocking` because database work is synchronous.
- Login handlers return `HandlerResult` values containing `HandlerAction`s such as:
  - `Reply`
  - `CreateSession`
  - `AttachCharacter`
  - `Disconnect`
- The login actor interprets those actions, writes packets, and persists session state for the world handoff.

## World server

- `world` starts one `WorldServerActor` plus one `ClientActor` per accepted TCP connection.
- Each `ClientActor` owns the per-connection IO state and communicates with `WorldServerActor` over Tokio `mpsc` channels.
- `WorldServerActor` owns:
  - `clients: HashMap<ClientId, ClientEntry>`
  - `fields: HashMap<FieldKey, FieldHandle>`
  - `names: HashMap<String, ClientId>`
- `ClientEntry` stores the client sender channel, character name, and current `FieldKey`.
- `WorldServerActor` routes:
  - `Whisper` directly through the character-name index
  - `FieldChat` to the client’s field
  - `FieldMove` to the client’s field
  - `Broadcast` through the legacy broadcast path

## Field identity

Field identity lives in `runtime/src/message.rs`:

```rust
pub struct FieldKey {
    pub channel_id: u8,
    pub map_id: i32,
    pub instance_id: u32,
}
```

Connected clients also carry a `FieldCharacter` snapshot containing:

- character identity and appearance fields
- `map_id`
- live `x`
- live `y`
- live `stance`

`FieldCharacter::field_key()` maps every connected character to:

- `channel_id = 0`
- `instance_id = 0`
- `map_id = character.map_id`

## Field actors

Field-local runtime state lives in `runtime/src/actor/field.rs`.

- `FieldActor` owns one live field copy identified by `FieldKey`.
- `FieldActor` keeps an occupant table keyed by `client_id`.
- Each occupant stores:
  - the client’s `ServerMessage` sender
  - the mutable `FieldCharacter` snapshot

`FieldActor` handles:

- `FieldMessage::Join`
- `FieldMessage::Leave`
- `FieldMessage::Chat`
- `FieldMessage::Move`

Join behavior:

- replay all existing occupants to the joining client
- broadcast the joining character to existing occupants
- insert the new occupant into the field table

Leave behavior:

- remove the occupant
- broadcast `RemovePlayerFromMap` to the remaining occupants

Movement behavior:

- parse the movement fragment with `parse_movement_state`
- update the mover’s `x`, `y`, and `stance`
- broadcast the movement packet to all other occupants

Chat behavior:

- broadcast the chat packet to all occupants of the field

## Packet handlers and actions

Packet handlers live under `net/src/packet/handle`.

Same-map gameplay handlers emit field-local actions:

- `AllChatHandler` builds a `ChatText` packet and returns `HandlerAction::FieldChat`
- `PlayerMoveHandler` extracts the movement fragment, builds a `MovePlayer` packet, and returns `HandlerAction::FieldMove`

`ClientActor` translates those actions into `ClientEvent::FieldChat` and `ClientEvent::FieldMove`, then sends them to `WorldServerActor`.

## Login-to-world flow

- Character selection on the login server emits `AttachCharacter` and replies with a redirect packet.
- The redirect target is `127.0.0.1:8485`.
- After the client connects to the world server, `ClientActor` processes `ReattachSession`.
- `ClientActor` reloads the session and character from the database, sends keymap and character packets, and constructs a `FieldCharacter`.
- That `FieldCharacter` seeds:
  - `map_id` from the persisted character row
  - `x = 240`
  - `y = 190`
  - `stance = 2`
- `ClientActor` sends `ClientEvent::Connected`.
- `WorldServerActor` resolves or creates the target `FieldActor` and forwards `FieldMessage::Join`.

## Map changes

`ChangeMapHandler` lives in `net/src/packet/handle/world/change_map.rs`.

Behavior:

- read the target map and portal name from the packet
- update `chr.character.map_id` when `target != -1`
- persist the character row
- send `SetField`/warp packets and a stat update

Map change handling does not remove the character from one `FieldActor` and join them to another `FieldActor`.

## Legacy broadcast path

`BroadcastScope` remains in `net/src/handler.rs`.

The world actor supports:

- `BroadcastScope::Map`
- `BroadcastScope::MapExcludeSelf`
- `BroadcastScope::World`
- `BroadcastScope::WorldExcludeSelf`

That path is separate from field-local presence, movement, and local chat.

## Related docs

- [Fields](./fields.md)
- [Integration Harness](./integration-harness.md)
