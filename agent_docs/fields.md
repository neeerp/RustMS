# Fields

## Overview

Field-local runtime state lives in `runtime/src/actor/field.rs`.

`FieldActor` owns one live field copy identified by `FieldKey`. A field copy is the unit that holds same-map occupants and broadcasts local gameplay packets.

## Field identity

`FieldKey` lives in `runtime/src/message.rs`:

```rust
pub struct FieldKey {
    pub channel_id: u8,
    pub map_id: i32,
    pub instance_id: u32,
}
```

The current runtime maps connected characters to:

- `channel_id = 0`
- `instance_id = 0`
- `map_id = character.map_id`

## Field character state

Each occupant is stored as a `FieldCharacter`.

`FieldCharacter` includes:

- `id`
- `name`
- `level`
- `job`
- `face`
- `hair`
- `skin`
- `gender`
- `map_id`
- `x`
- `y`
- `stance`

`ClientActor` creates the initial `FieldCharacter` during world reattach.

The seeded position is:

- `x = 240`
- `y = 190`
- `stance = 2`

That seed matches the start-map spawn snapshot used by the field packet builder.

## Ownership model

`WorldServerActor` owns field lookup.

`FieldActor` owns:

- occupant membership
- local join replay
- local leave broadcast
- local movement fanout
- local chat fanout
- live `x/y/stance` updates derived from movement fragments

`FieldActor` does not own:

- map transition orchestration
- NPCs
- mobs
- drops
- reactors
- AOI partitioning
- instance allocation policy

## Field messages

`FieldMessage` lives in `runtime/src/message.rs`:

- `Join`
- `Leave`
- `Chat`
- `Move`

`Move` carries:

- the prebuilt `MovePlayer` packet
- the raw movement fragment used to update local field state

## Join flow

Join handling in `FieldActor` does three things:

1. send all existing occupants to the joining client as `SpawnPlayer`
2. send the joining player to all existing occupants as `SpawnPlayer`
3. insert the new occupant into the field table

The spawn packet is built from the occupant’s `FieldCharacter` snapshot.

## Leave flow

Leave handling:

1. removes the occupant from the field table
2. broadcasts `RemovePlayerFromMap` to the remaining occupants

## Movement flow

Movement handling starts in `PlayerMoveHandler`:

- strip the v83 movement header
- keep the movement fragment
- build a `MovePlayer` packet
- emit `HandlerAction::FieldMove`

The message path is:

1. `PlayerMoveHandler`
2. `ClientActor`
3. `WorldServerActor`
4. `FieldActor`

Inside `FieldActor`, `parse_movement_state` reads the movement fragment and updates the mover’s:

- `x`
- `y`
- `stance`

The movement packet is then broadcast to every other occupant in that field.

## Chat flow

Local chat handling starts in `AllChatHandler`:

- read the message
- read the show mode
- build a `ChatText` packet
- emit `HandlerAction::FieldChat`

The message path is:

1. `AllChatHandler`
2. `ClientActor`
3. `WorldServerActor`
4. `FieldActor`

`FieldActor` broadcasts the chat packet to all occupants of the field, including the sender.

## Field packet builders

Field-local packet builders live in `net/src/packet/build/world/field.rs`.

They build:

- `SpawnPlayer`
- `RemovePlayerFromMap`
- `MovePlayer`
- `ChatText`

The same module also defines:

- `default_spawn_position`
- `parse_movement_state`

Foreign-player look data is synthesized from:

- character appearance fields
- default equipment ids for overall, shoes, and weapon

## Relationship to map changes

`ChangeMapHandler` updates persistent `map_id` and emits map packets directly.

Field membership does not move across field actors during a map change. A field-aware leave/join handoff is a separate piece of work.

## Tests

Field-local behavior is covered in two places:

- unit tests in `runtime/src/actor/field.rs`
- end-to-end integration tests in `integration-harness/tests/`

The integration harness covers:

- same-map presence
- same-map movement
- same-map local chat
