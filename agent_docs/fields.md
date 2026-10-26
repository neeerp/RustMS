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
- **Trading rooms** (`field_trades::Trades`): who is in which room, and
  their turns and answers. The offers stay with the client actors; see
  "Trading" in [Inventory](./inventory.md).
- **Kites** (`field_kites::KiteSet`): the kites players flew here; see
  "Kites" below.
- **Time**: a 100 ms tick (`on_tick`) respawns mobs and reactors and expires
  drops. A third job's clone room, a mini-dungeon's copy and a fourth job's
  solo room put all their mobs and reactors back when their last player
  leaves (`MobSet::renew`, `ReactorSet::renew`,
  `npc_scripts::resets_when_empty`), but not when that player only warps
  within it (`FieldMessage::Leave`'s `staying`). The tick also hands
  reactors the items that have lain on them long enough.

Some maps are copies of another (mini-dungeons, the Dojo): their node has
only `info`, and `info/link` names the map they copy. `game-data` and the
client's `Stage::load_map` (`NxHelper::Map::get_map_body`) take the body
(footholds, portals, life, reactors, tiles) from the linked map and keep
the copy's own `info`, which can differ (BGM, view range, field limit,
forced return).

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
| `Trade { from, request }` | A `RoomRequest` for the sender's trading room; answers come back as `ServerMessage::Trade` |
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
the field's projection. It lists the pets out, without their equipment.
It shows the worn mount's level, EXP and tiredness. Alliance, wish list,
and monster book are empty; the medal is the item
worn in the medal slot. A request for someone not on the map gets no answer.

## Fame

The fame buttons in someone else's character info send GIVE_FAME (0x5F:
character id, `u8` up). `GiveFameHandler` ignores fame for yourself and tells a
giver below level 15 they can't (FAME_RESPONSE 0x26 mode 2); otherwise it
returns `WorldAction::GiveFame`. The giver's client actor records the gift in
`fame_log` (`db::fame::give`), refusing an unknown character (mode 1), a second
gift within a day (mode 3), or a gift to the same character within 30 days
(mode 4). Then it passes `FieldRequest::Fame` to its field. If the target is
on the map, the field sends their actor `ServerMessage::Famed`. The target
changes its own fame by one, within ±30000, and sends the stat update and
mode 5 (giver, up) to its player and mode 0 (target, up, new fame) to the
giver. A target not on the map sends the giver `ServerMessage::FameMissed`,
which deletes the log row (`db::fame::forget`) and answers mode 1, so the miss
doesn't spend the day.

## Map changes

`ChangeMapHandler` resolves the destination (portal link or permitted map-id
warp), updates the player's map and spawn point, sends the warp, and returns
`WorldAction::ChangeField`. The client actor then leaves the old field and
joins the new one at the spawn portal's position.

A Mystic Door warp (`net::door_warp`) lands at a town door portal, or back
at the door through SET_FIELD's chase position
([Mystic Door](./combat.md#mystic-door)).

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
ignores every other opcode but `CHECK_CASH`, `COUPON_CODE`, `NOTE_ACTION` and
the Name Change checks (0x10 and 0x15) until the player leaves. Leaving sends a `CHANGE_MAP` with no body; the actor
answers it like a channel change to its own channel, and the client reconnects
and enters the map it left.

In the shop, `CASHSHOP_OPERATION` (0xE5) buys a commodity and sets the wish
list (`net/src/packet/handle/world/cash_shop.rs`). A purchase is checked against
`Etc.nx` `Commodity.img` (`net::cash_shop`): the item must be on sale and allowed
for the character's gender. Cash items that don't work here yet count as sold
out, alone or in a package: Character Transfer and Maple Life, and Remote
Gachapon's class but for Miu Miu and the Remote Gachapon Ticket
(`INERT_CLASSES`). `db::cash_shop::buy` then takes the price from the
chosen balance and adds the item to the Cash Inventory in one transaction. The
client gets the item (0x57) and its new balance, or a failed purchase (0x58,
which v83's Buy Avatar goes on after) with its code: 0xA5 not enough cash, 0xAA
gender, 0xC0 sold out (followed by the SN), 0x00 anything else. Other requests
fail with 0x5C. The wish list
keeps up to ten commodities that are on sale and answers with 0x55. Balances
are only granted through SQL for now.

`CHECK_CASH` (0xE4) sends the balances again. v83 sends it from the Check
button and whenever its window is activated in the shop. v83 locks the shop
after each request until an answer comes, so an operation the server doesn't
serve gets failure 0x00. The queries v83 sends on its own (0x28 for each
commodity it hasn't cached, and 0x21) get no answer, which would show a
notice for each.

A package (0x1E, items 910xxxx) is bought the same way, but
`Etc.nx` `CashPackage.img` lists the commodities it holds, and each joins the
Cash Inventory; the client gets them all in 0x89 (v83 `0x479a1b`, whose
trailing short is the MaplePoints received, 0 here). Packages can't be gifted,
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

The Extra Character Slot Coupon (5430000, commodity 50200083, 6,900 NX) is
not an item: it adds a character slot to the account at once (report c085).
Only its own request sells it, op 0x08 (a byte, the cash type, the SN); BUY and
gifts refuse it. `db::cash_shop::buy_character_slot` raises
`accounts.character_slots` and pays in one transaction, up to 15 slots; at 15
the reply is 0x00 and nothing is paid. Success answers 0x64 (the new count),
then the balance, which unlocks the shop. 0x4B ends with the count, and the
login server's character list shows it. The Maple Life items (5431000 and
5432000) are still sold out.

Commodities 80000000-89999999 cost mesos (report b094): four Etc quest items
for 1 meso, such as the Golden Bell (80000001, item 4031191). Only their own
request sells them, op 0x20 (the SN); BUY and gifts refuse them. The item goes
straight into the character's bag, and the price is charged once whatever the
count (`net::cash_shop::buy_with_mesos`). The client gets the meso stat
update, the inventory add, then 0x8D (an int count, then per item a short, the
slot and the item ID) and the balance. A short purse fails with 0xC2, a full
bag with 0xBB, the other gender with 0xAA and anything off sale with 0xC0. As with NPC shops, the
character is saved later, not at once.

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
restores the in-memory inventory. A full bag fails with 0xBB. A pet comes out
named after its kind, or as it was if it has been out before; see `pets.md`.

A gift (0x04) names the commodity, a receiving character and a message of 1 to
73 bytes, and carries the birthday code the client asked for. Accounts don't
record a birthday, so any real date passes, and anything else fails with 0xC4.
The receiver must exist (0xA9), belong to another account (0xA8), and be of a
gender the item is for (0xAA). `db::cash_shop::gift` takes the price from the
sender's NX Credit, or from their NX Prepaid if the credit is short (never Maple
Points), and inserts the item into the receiver's account with `gift_from` and
`gift_message` set, in one transaction. The sender gets 0x5E (receiver, item,
count, price) and their new balance. When the receiving account next enters the
shop, `db::cash_shop::open_cash_inventory` picks out the unseen gifts (rows
with a message) and clears their messages. The gifts are in the Cash Inventory
list sent in 0x4B, and 0x4D then names each with its sender and message for the
client to announce; v83's handler adds nothing to the list. Notes sent back
(NOTE_ACTION, 0x83) aren't handled yet.

#### Coupons

The Coupon button's dialog sends `COUPON_CODE` (0xE6): an empty string, then
the code. Coupons live in `coupons` (the code in upper case, an optional
expiry, and who redeemed it when) and `coupon_rewards` (kind 0 an item, 1 NX
Credit, 2 Maple Points, 3 NX Prepaid, 4 mesos). They are made by a GM's
`!coupon` (see "Chat commands" in `server-architecture.md`), by SQL, or by
`db::coupon::create`. A code is redeemed once, by one
account. The server trims the typed code and upper-cases it, then answers with
a failure (0x5C) or what the coupon gave (0x59), always followed by the
balances, which is what lets v83 take the next request:

- 0xB0: no such coupon. The third in a row on one connection is 0xB1 instead,
  and v83 then leaves the shop by itself; a redemption starts the count again.
- 0xB3: already used; 0xB2: expired.
- 0xBB: a bag can't take the items or the mesos would overflow. Nothing is
  given and the coupon stays unused.

Otherwise `net::coupon::grant` puts the items for the bags and the mesos on the
character (INVENTORY_OPERATION and a stat update) and sorts out the rest:
cash items (5xxxxxx, or equipment marked cash) go to the Cash Inventory with
the SN of a commodity selling them, if any, and the NX to the account.
`db::coupon::redeem` marks the coupon used, adds the NX and the Cash Inventory
items, and saves the character, in one transaction that locks the coupon's row,
so two accounts can't both have it. 0x59 lists the cash items, the Maple Points,
each bag item `(short quantity, short 0, int item id)` and the mesos for the
client's notice (report b072); NX Credit and Prepaid show only in the
balances.

#### Name Change

Name Change (5400000, commodity 50600000, 10,000 NX) asks for a new name that
takes effect at the next logout (report c060; `db::name_change`,
`runtime/src/actor/client/name_change.rs`). The shop's button sends op 0x10
(character id, birthday code). A bad birthday code fails with 0xC4, and a
character below level 10 gets `0x149` result 4. Otherwise `0x149` says 0
(the rules sheet, then the name dialog), 1 (a request waits), or 3 (a name
changed less than 30 days ago). The name dialog checks with op 0x15, the
login server's name check opcode, which in the shop answers `0x148` (name,
taken): taken when the name breaks the creation rules, is the character's
own, or belongs to a character or a waiting request, ignoring case. The
login server's check also counts waiting requests.

The purchase is CASHSHOP_OPERATION 0x2E (SN, old name, new name). It is paid
from NX Prepaid only, at the commodity's price, and gifting it fails.
`db::name_change::request` re-checks everything, pays, and puts a receipt in
the Cash Inventory, in one transaction; unique indexes on `name_changes`
allow one waiting request per character and per name. The client gets the
receipt in `0x9E` (the cash item record, as in 0x57) and the balances.

Using the receipt from the Cash bag (USE_CASH_ITEM with `u8 1`) cancels the
request it stands for, with no refund, and takes the receipt: `0x71` 0. A
receipt that stands for no waiting request of the character's answers
`0x78`. Dropped, the receipt vanishes like a pair ring; the request stays.
When the player logs out (not on a channel change), after the save,
`db::name_change::apply` renames the character, or, if another character
took the name meanwhile, marks the request failed and refunds the NX
Prepaid, with no month to wait; either way the receipt goes from the Cash
Inventory and the bags. The world applies every waiting request when it
starts; one that fails waits for the next logout. Deleting the character
refunds its request. Gifts not yet thanked for (`cash_items.gift_from`)
follow the new name; notes, owner tags and the like keep the old one. Name
Change Coupons (5401000) stay sold out.

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

## Kites

A kite or banner (5080000-5080003) is used from the Cash tab with three
lines of text: USE_CASH_ITEM (0x4F) with the slot, item id, the lines
joined with `\n`, and the tick (report c014). `net::packet::handle::world::kite`
refuses it in the Free Market (`map_id / 10,000,000 == 91`, as the v83
client does) with CANNOT_SPAWN_KITE (0x10E, "You can't fly it there.") and
an unlock, and refuses more than three lines or a line over 60 characters
with an unlock alone. Otherwise it takes one item and sends the field
`FieldRequest::Kite`. The field flies it where its flier stands, with
their name, under an id from `object_ids::KITES`: SPAWN_KITE (0x10F: id,
item, lines, name, x, y) goes to everyone there and to whoever comes
later. After an hour (`KITE_TIME`, Cosmic's) REMOVE_KITE (0x110: type 0,
id) fades it out. A client drops its kites itself on a map change. Cosmic
sends the foothold id where the client reads y; RustMS sends y.

On the client, double-clicking a kite in the Cash tab
(`UIItemInventory::use_kite`) checks v83's gates in order: an open dialog
gives "Not available", the Free Market gives "Not available in the Free
Market." in the chat log, and otherwise `UIKiteWrite` asks for the three
lines (UIClassic `UIWindow.img/MultiLine/backgrnd2`, the item's `msgIn` as
the prompt, 60 characters a line, Enter does nothing, Esc cancels). OK
joins the lines with `kite::join`, refuses what the curse filter refuses,
and sends the packet. CANNOT_SPAWN_KITE shows its notice. `MapKites` keeps
the field's kites: the item's `iconReward` animation 100 px above and 3 px
left of the flier's feet, bobbing 3 px once every 2 s from a random phase,
drawn after every map layer and before the mists. REMOVE_KITE type 0 stops
the bob and fades the picture over a second, with the kite already out of
reach of the mouse; other types drop it at once. With the mouse over a
kite's 60x80 box, the one-line tooltip shows the flier's name run straight
into the item's `msg` ("Lanecis wishing you a happy new year."), as v83
does. A double click opens `UIKiteView` (`backgrnd`): the name in orange,
the `msg` in cream after it, and the lines in white, the third holding
whatever follows the second break. Esc, Enter and the close button close
it. A shop balloon takes the mouse before a kite, and a click on the
player's own quest alert bulb opens its menu even under a kite. The rules are in
`Gameplay/MapleMap/KiteRules.h`, tested by `tests/kite_rules.cpp`.

Differences from v83: HC opens one viewer at a time where v83 can stack
several. UIClassic's `backgrnd2` has a header band above its icon and lined
rows, so the prompt and fields sit 50 px lower than the exe puts them. Tab
cycles only the three lines, where v83 also stops on OK and Cancel.
HeavenClient draws chat balloons with their owners, so a kite covers an
NPC's balloon (v83's layer for those balloons wasn't checked).

## Tests

- Unit tests in `runtime/src/actor/field.rs`: join/leave visibility, a
  stalled client being kicked without blocking others, a character info
  request reaching only its target, and chairs being shown to the map.
- Integration tests: same-map presence, movement, chat, emotes, portal transfers,
  NPC spawns, cross-channel isolation, starting equipment on spawn, and
  character info (`char_info`), and portable chairs (`portable_chairs`).
