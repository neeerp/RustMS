# Fields

## Overview

`FieldActor` (`runtime/src/actor/field.rs`) owns one live copy of a map,
identified by `FieldKey { channel_id, map_id, instance_id }`
(`runtime/src/message.rs`). Every connected character is in exactly one field:
`channel_id` is the channel whose listener accepted the connection,
`map_id` is the character's map, and `instance_id` is `0` until instanced
content exists.

Fields are created on demand by `FieldRegistry` and kept alive afterwards.

## What a field owns

- **Occupants**: a `ClientHandle` plus a `game::FieldAvatar` per character
  (id, name, level, job, avatar look, live `x`/`y`/`stance`). The avatar is a
  projection built by the owning `ClientActor`; the field only updates the
  position from movement packets.
- **Static NPCs** from the map data, with object ids starting at
  `1_000_000_000`.
- **Mobs** (`field_mobs::MobSet`): spawn points, live mobs, controllers,
  and damage; see [Mobs and combat](./combat.md).
- **Time**: a 100 ms tick (`on_tick`) respawns mobs; drop expiry goes here.

A field does not own character state (stats, inventory, quests). Those live
in the client's `Player`.

## Messages

`FieldMessage`:

| Message | Effect |
| --- | --- |
| `Join { handle, avatar }` | Send map NPCs and existing occupants to the newcomer, announce the newcomer to everyone, then add it |
| `Leave { client_id }` | Remove the occupant and broadcast `RemovePlayerFromMap` |
| `UpdateAvatar { client_id, avatar, announce }` | Replace the projection (keeping the live position), e.g. after a level-up, and show the others `announce` |
| `Chat { from, packet }` | Broadcast to everyone, sender included |
| `Move { from, packet, movement_bytes }` | Update the mover's position from the movement fragment, broadcast to everyone else |
| `NpcInteraction { from, object_id, request_id }` | Validate the NPC is on this map, then tell the client which NPC id to talk to |
| `FromClient { from, request }` | A `net::FieldRequest` from a handler: move a mob it controls, apply an attack, or show the others a packet |

All sends to clients use `ClientHandle::send` (`try_send`); a client whose
queue is full is disconnected rather than allowed to block the field.

## Map changes

`ChangeMapHandler` resolves the destination (portal link or permitted map-id
warp), updates the player's map and spawn point, sends the warp, and returns
`WorldAction::ChangeField`. The client actor then leaves the old field and
joins the new one at the spawn portal's position.

World entry uses the same spawn-portal lookup (`net::spawn_position`) instead
of a hard-coded position.

## Packets

Field packets are built in `net/src/packet/build/world/field.rs`
(`SpawnPlayer`, `RemovePlayerFromMap`, `MovePlayer`, `ChatText`) and
`npc.rs` (`SpawnNpc`). `SpawnPlayer` encodes the avatar's real equipment
through `net::codec::encode_avatar_look`.

## Tests

- Unit tests in `runtime/src/actor/field.rs`: join/leave visibility, and a
  stalled client being kicked without blocking others.
- Integration tests: same-map presence, movement, chat, portal transfers,
  NPC spawns, cross-channel isolation, and starting equipment on spawn.
