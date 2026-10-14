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
  `1_000_000_000`. SPAWN_NPC carries each NPC's `rx0`/`rx1` range. In
  HeavenClient, NPCs with a `move` animation walk to a random x in that range
  now and then, and stand at the end or at a wall or ledge. The walk is
  client-local: as in v83, where each client controls its own NPCs and the
  server only echoes their moves, players don't see the same positions.
- **Mobs** (`field_mobs::MobSet`): spawn points, live mobs, controllers,
  and damage; see [Mobs and combat](./combat.md).
- **Drops** (`field_drops::DropSet`): items and mesos on the ground, with
  ownership and expiry; see [Mobs and combat](./combat.md).
- **Time**: a 100 ms tick (`on_tick`) respawns mobs and expires drops.

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
| `FromClient { from, request }` | A `net::FieldRequest` from a handler: move a mob it controls, apply an attack, show the others a packet, pick up or throw down a drop, or pass a character info request on |

All sends to clients use `ClientHandle::send` (`try_send`); a client whose
queue is full is disconnected rather than allowed to block the field.

## Character info

Double-clicking a character sends CHAR_INFO_REQUEST (0x61: tick, character
id). `CharInfoRequestHandler` passes it to the requester's field as
`FieldRequest::CharInfo`. If that character is on the map, the requester
included, the field sends their client actor
`ServerMessage::CharInfoRequested` with the requester's handle. The target
answers with CHAR_INFO (0x3D, `build::world::info::build_character_info`)
built from its own `Player`, so the level and fame are current rather than
the field's projection. Guild, alliance, pets, mount, wish list, and monster
book are empty; the medal is the item worn in the medal slot. A request for
someone not on the map gets no answer.

## Map changes

`ChangeMapHandler` resolves the destination (portal link or permitted map-id
warp), updates the player's map and spawn point, sends the warp, and returns
`WorldAction::ChangeField`. The client actor then leaves the old field and
joins the new one at the spawn portal's position.

World entry uses the same spawn-portal lookup (`net::spawn_position`) instead
of a hard-coded position.

## Packets

Field packets are built in `net/src/packet/build/world/field.rs`
(`SpawnPlayer`, `RemovePlayerFromMap`, `MovePlayer`, `ChatText`,
`FacialExpression`) and `npc.rs` (`SpawnNpc`). `SpawnPlayer` encodes the
avatar's real equipment through `net::codec::encode_avatar_look`.

Emotes (F1-F7) arrive as `FACE_EXPRESSION`; `FaceExpressionHandler` shows
the others a `FacialExpression` through `FieldRequest::Broadcast`. The sender
has already changed its own face. Emotes above 7 need an emotion cash item
and are dropped.

## Tests

- Unit tests in `runtime/src/actor/field.rs`: join/leave visibility, a
  stalled client being kicked without blocking others, and a character info
  request reaching only its target.
- Integration tests: same-map presence, movement, chat, emotes, portal transfers,
  NPC spawns, cross-channel isolation, starting equipment on spawn, and
  character info (`char_info`).
