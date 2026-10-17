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
- **Drops** (`field_drops::DropSet`): items and mesos on the ground, with
  ownership and expiry; see [Mobs and combat](./combat.md).
- **Reactors** (`field_reactors::ReactorSet`): boxes and the like placed
  by the map's `reactor` nodes; see "Reactors" in [Mobs and
  combat](./combat.md).
- **Time**: a 100 ms tick (`on_tick`) respawns mobs and reactors and expires
  drops.

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

The client waits for an answer to each move it asks for, so a refusal
answers too. v83 has two refusal packets, each a reason byte
(`net::packet::build::world::map`): `BLOCKED_MAP` (0x83) for map changes and
`BLOCKED_SERVER` (0x84) for channel, cash shop and trade shop moves.
HeavenClient shows the reason in chat or as a notice. The server sends
`BLOCKED_MAP` 1 ("The portal is closed for now.") for a portal to a map or spawn
portal it doesn't have. It sends `BLOCKED_SERVER` 1 for a change to the
player's own channel or to a missing channel, and 2 when a dead player tries
to enter the Cash Shop.

### The Cash Shop

`ENTER_CASHSHOP` (0x28) takes the player off their field, saves them, and
sends `SET_CASH_SHOP` (character data plus the shop's lists) followed by the
cash inventory, gifts, wish list and NX balance
(`net/src/packet/build/world/cash_shop.rs`). The session stays on the channel
and in the world directory, so whispers still arrive, but the client actor
ignores every other opcode until the player leaves. Leaving sends a
`CHANGE_MAP` with no body; the actor answers it like a channel change to its
own channel, and the client reconnects and enters the map it left.

In the shop, `CASHSHOP_OPERATION` (0xE5) buys a commodity and sets the wish
list (`net/src/packet/handle/world/cash_shop.rs`). A purchase is checked against
`Etc.nx` `Commodity.img` (`net::cash_shop`): the item must be on sale and allowed
for the character's gender. `db::cash_shop::buy` then takes the price from the
chosen balance and adds the item to the Cash Inventory in one transaction. The
client gets the item (0x57) and its new balance, or a failure code (0x5C): 0xA5
not enough cash, 0xAA gender, 0xC0 sold out, 0x00 anything else. The wish list
keeps up to ten commodities that are on sale and answers with 0x55. Balances
are only granted through SQL for now.

A package (0x1E, items 910xxxx) is bought the same way, but
`Etc.nx` `CashPackage.img` lists the commodities it holds, and each joins the
Cash Inventory; the client gets them all in 0xA4. Packages can't be gifted,
and every other 9xxxxxx item is refused. Slot expansions come two ways. The Cash Inventory's slot buttons
(0x06 mode 0 with a bag, 0x07 mode 0 for the storage) add four slots for
4,000 NX. The expansion commodities (0x06 or 0x07 mode 1 with the SN; items
9110000 for the storage and 9111000-9114000 for the bags) add eight for their
price. Bags stop at 96 slots and the storage at 48; past that, or with the
wrong bag, the reply is 0x00. A bag's slots are saved with the character
(`characters.equip_slots` and the rest) in the same transaction as the payment
(`db::cash_shop::expand_inventory`), and answer 0x60 (bag, slots). The storage's
are saved on the account (`accounts.storage_slots`, sent in 0x4B) and answer
0x62. There is no storage window yet.

Each sale and gift adds one to its SN in `cash_sales`. The shop's Best Items
are the five best-selling SNs a character of each gender may buy
(`net::cash_shop::best_items`), the same five on every tab.

The client actor also follows its own movement to know roughly where the
player stands. Before saving on a channel change or disconnect, it sets the
spawn point to the nearest start or invisible portal that leads nowhere
(`net::closest_spawn_point`), as Cosmic's `findClosestPlayerSpawnpoint` does,
so the player comes back near where they left instead of at the portal they
entered the map by.

Taking an item out (0x0D) turns the Cash Inventory row into an inventory item
that keeps its cash serial and expiry (`net::cash_shop::inventory_item`), puts
it in the requested slot or the bag's first free one, and then
`db::cash_shop::take_out` deletes the row and saves the character in one
transaction. Putting it back (0x0E) does the reverse with
`db::cash_shop::put_back`, which re-inserts the row under the same serial. The
inventory doesn't keep the commodity, so the row gets an SN that sells the
item in that quantity, preferring one on sale. Either way, a failed write
restores the in-memory inventory. A full bag fails with 0xBB. Pets stay in the
Cash Inventory (0x00) until the server can encode them.

A gift (0x04) names the commodity, a receiving character and a message of 1 to
73 bytes, and carries the birthday code the client asked for. Accounts don't
record a birthday, so any real date passes, and anything else fails with 0xC4.
The receiver must exist (0xA9), belong to another account (0xA8), and be of a
gender the item is for (0xAA). `db::cash_shop::gift` takes the price from the
sender's NX Credit, or from their NX Prepaid if the credit is short (never Maple
Points), and inserts the item into the receiver's account with `gift_from` and
`gift_message` set, in one transaction. The sender gets 0x5E (receiver, item,
count, price) and their new balance. When the receiving account next enters the
shop, `db::cash_shop::open_cash_inventory` splits the unseen gifts (rows with a
message) from the rest and clears their messages. The gifts go out in 0x4D
with sender and message, and the client adds them to the Cash Inventory list
it got in 0x4B, as HeavenMS does.

## Packets

Field packets are built in `net/src/packet/build/world/field.rs`
(`SpawnPlayer`, `RemovePlayerFromMap`, `MovePlayer`, `ChatText`,
`FacialExpression`) and `npc.rs` (`SpawnNpc`). `SpawnPlayer` encodes the
avatar's real equipment through `net::codec::encode_avatar_look`.

Emotes (F1-F7) arrive as `FACE_EXPRESSION`; `FaceExpressionHandler` shows
the others a `FacialExpression` through `FieldRequest::Broadcast`. The sender
has already changed its own face. Emotes above 7 need an emotion cash item
and are dropped.

## Portable chairs

Double-clicking a 301xxxx Set-up item sits the client down at once and sends
USE_CHAIR (0x2B: item id). Like the v83 client, it only does so on the ground,
when not already seated, and at the chair's `info/reqLevel` or above.
`UseChairHandler` checks that the item is a portable chair the player owns,
then sends the field `FieldRequest::Chair(item_id)`. The field records it in
the occupant's `FieldAvatar::chair` and shows the others SHOW_CHAIR (0xC4:
character id, item id). A later joiner gets the chair in `SpawnPlayer`.

The client never stands up on its own. Left, Right, Jump, Sit, or a hit that
deals damage sends CANCEL_CHAIR (0x2A: seat, -1 for a portable chair) once.
So does an inventory operation that removes the last slot holding the chair
the player sits on (dropping, selling, or trading it away). Up does not
cancel. `CancelChairHandler` replies with CANCEL_CHAIR (0xCD: byte
0), which stands the client up, and sends `Chair(0)`, so the others get
SHOW_CHAIR with item 0. Map seats (a seat id other than -1) are not
implemented and get no reply.

The client draws `Item/Install/0301.img/<id>/effect` while the character
sits. The effect is anchored at the face when `pos` is 1, otherwise at the
feet, and drawn behind the character when `z` is negative. `effect2` comes
from later data and is ignored. Sitting heals by the chair's
`recoveryHP`/`recoveryMP`; see "Using items and recovery" in
[Inventory](./inventory.md).

## Tests

- Unit tests in `runtime/src/actor/field.rs`: join/leave visibility, a
  stalled client being kicked without blocking others, a character info
  request reaching only its target, and chairs being shown to the map.
- Integration tests: same-map presence, movement, chat, emotes, portal transfers,
  NPC spawns, cross-channel isolation, starting equipment on spawn, and
  character info (`char_info`), and portable chairs (`portable_chairs`).
