# Inventory

## Model

`game::Inventory` holds the worn items (negative positions) and five bags.
Every operation is all-or-nothing and returns `InventoryChange`s in the
order the client must apply them:

| Operation | Behavior |
| --- | --- |
| `add(item, slot_max)` | Fill partial stacks of the same item, then empty slots; `Full` changes nothing |
| `remove(item_id, quantity)` | Take from the lowest slots; `NotEnough` changes nothing |
| `take(kind, slot, quantity)` | Take from one slot (using an item) |
| `move_in_bag(kind, from, to, slot_max)` | Move, merge onto a stack, or swap |
| `equip(from, to)` / `unequip(from, to)` | Wear or take off; overall ⇄ pants and two-handed weapon ⇄ shield take each other off (needs a free slot) |
| `gather(kind)` / `sort(kind)` | Lay the bag out from slot 1, in slot order or by item id (ties keep slot order); stacks are not merged |

`net::packet::build::world::inventory::build_inventory_operation` turns the
changes into INVENTORY_OPERATION (`0x1D`); worn positions travel as the EQUIP
type with a negative slot, and equipment moves add the movement byte that
makes the client refresh the look. `build_item_gain` (SHOW_STATUS_INFO mode 0)
and `build_item_gain_in_chat` (SHOW_ITEM_GAIN_INCHAT mode 3) show gains.

## Item data

`game_data::items::ItemData` reads `Item.nx` (use/setup/etc/cash: `slotMax`,
`quest`, `only`, `price`, `tradeBlock`, consumable `spec`) and `Character.nx`
(equipment stats `inc*`, `tuc`, `req*` requirements, and `tradeBlock`) on
demand. Stacks default to
100 when `slotMax` is absent. `net::items` loads it once
(`RUSTMS_ITEM_NX_PATH`, `RUSTMS_CHARACTER_NX_PATH`), creates items with their
base stats (`create`), and checks wear requirements (`can_wear`: level,
stats, and the `reqJob` branch bits; 0 means anyone). New characters' starting
equipment gets its base stats at creation (the login server reads the same
assets).

## Client moves

ITEM_MOVE (`0x47`: tick, type, from, to, quantity) is handled by
`ItemMoveHandler`: a negative source unequips, a negative destination equips
(after `can_wear`), anything else moves within the bag. The reply is the
INVENTORY_OPERATION; a worn-equipment change also refreshes the field avatar
and sends UPDATE_CHAR_LOOK (`0xC5`) to the other players on the map. Refused
moves only get an enable-actions stat update.

Dropping is a move to destination 0 from a bag slot (`drop_item`; worn items
can't be dropped, and the client never asks). Stars and bullets go as the
whole stack; anything else drops the asked quantity, keeping equipment's
stats (`Loot::Dropped`). Quest and `tradeBlock` items leave the bag but don't
reach the floor: everyone sees them vanish (DROP_ITEM_FROM_MAPOBJECT 3).
MESO_DROP (`0x5E`: tick, amount) drops 10 to 50,000 mesos the player holds;
HeavenClient's meso button asks how many with `UIEnterNumber`. The field
places either kind where it last saw the player, and anyone may pick it up
at once (ownership 2). Dead players can't drop.

GATHER_ITEMS (`0x45`) and SORT_ITEMS (`0x46`) (tick, type) are the item
window's Gather and Sort buttons. The rearrangement goes out as one
INVENTORY_OPERATION of swaps (mode 2), then GATHER_ITEM_RESULT (`0x34`) or
SORT_ITEM_RESULT (`0x35`) (`0`, type), which flips the button between Gather
and Sort. A bag that is already in order gets only the result and an
enable-actions stat update.

HeavenClient cancels any icon drag before applying an INVENTORY_OPERATION,
because the dragged icon points into the icons being rebuilt.

The player is marked dirty and saved with the usual autosave/logout path.

## Using items and recovery

USE_ITEM (`0x48`: tick, slot, item id) checks the slot holds that item,
applies its `spec` (`hp`, `mp`, and percent `hpR`/`mpR`, capped at the
maximums), consumes one, and answers with the inventory change and an HP/MP
stat update. Dead players can't use items.

HEAL_OVER_TIME (`0x59`) is HeavenClient's natural recovery, which follows the
v83 (MapleLegends) client's 30 ms recovery check. HP and MP keep separate
timers, which restart whenever their conditions fail, and go out in separate
reports; an HP heal shows its amount over the player in blue
(`BasicEff.img/NoBlue`).

- HP heals 10 every 10 s while the player stands (not alert) or sits without
  moving. A warrior with Improved HP Recovery (1000000) heals every 5 s. On
  a ladder or rope, only Endure heals HP (1000002 for warriors, 4100002 and
  4200001 for assassins and bandits), every `time` seconds of its level.
  Prone, walking, jumping and attacking heal no HP.
- MP heals 3 every 10 s whatever the player does, while they are alive.
- Skills add their `hp` and `mp` to each heal: Improved HP Recovery and the
  thieves' Endures add HP, and the thieves' Endures, Fighter and Page
  Improved MP Recovery (1110000, 1210000) add MP. A magician adds Improved
  MP Recovery's (2000000) level times their level over 10.
- The map's `info/recovery` scales both amounts (2 in the Sleepywood sauna,
  10 in the Carnival rooms). On such a map (above 1), the worn top's
  `info/recovery` scales them again (1.5 for the sauna robes), and sitting
  scales them by another 1.5. A
  portable chair heals its `info/recoveryHP` or `recoveryMP` instead when it
  has one (the Relaxer heals 50 HP); its timer restarts when it switches
  between that and the natural amount.

The server allows each report up to the player's natural amount
(`net::skills::natural_recovery`) times the map's rate, the top's rate
where that applies, and the 1.5 sitting rate, or the chair's recovery values while the player sits on a chair they
still own (`Player::chair`, set by USE_CHAIR and cleared by CANCEL_CHAIR and
map changes). It takes HP at most once per HP interval (5 or 10 s), and not
within that interval of the player's last PLAYER_MOVE, and MP at most once
per 10 s, each with a second of slack; anything else is ignored. Endure's
intervals are never shorter than 10 s, so climbing needs no separate check.

### Skill and mastery books

USE_SKILL_BOOK (`0x52`: tick, slot, item id) reads a skill book (228xxxx)
or a mastery book (229xxxx). v83 sends it from the item-use path with the
usual exclusive-request gate and checks nothing itself. The book's
`info/skill` lists skills; the server takes the one whose job
(skill / 10000) is the reader's own job, and refuses the book (kept, `used`
0) when there is none, when the reader's level in it is below
`info/reqSkillLevel`, or when their master level is already the book's
`info/masterLevel`. Otherwise it spends the book and rolls `info/success`
(percent, 100 when absent): on success the skill's master level becomes the
book's, sent with UPDATE_SKILLS. SKILL_LEARN_ITEM_RESULT (`0x33`: reader,
mastery, skill, master level, used, success) goes to the reader and the map.
HeavenClient plays the `BasicEff.img/SkillBook` glow on the reader, and for
the reader the Enchant sound and the v83 chat line.

### Equipment scrolls

Dropping a scroll (204xxxx) on the Equip window sends USE_UPGRADE_SCROLL
(`0x56`: tick, scroll slot, equip slot, White Scroll 2 or 1, Legendary
Spirit byte). The client sends it only if the scroll fits the item
(`scroll_rules::fits`, a port of the v83 check) and asks "Would you like to
use the White Scroll?" when the player has one and the scroll would use a
slot. `game-data` reads a scroll's `success`, `cursed`, `inc*` stats, `req`
list, and kind (Chaos `randstat`, Clean Slate `recover`, spikes
`preventslip`, cold `warmsupport`) into `game::scroll::ScrollInfo`;
`game::scroll::apply` rolls it HeavenMS-style (rolls injected, so tested
without randomness). The server refuses, with only the unlock, a dead
player, a scroll that doesn't fit, an equip with no slot left (Clean Slate:
nothing to restore), and cash items. Otherwise one exclusive
INVENTORY_OPERATION takes the scroll and the White Scroll and replaces the
worn equip (or only removes it when the curse destroys it), then
SHOW_SCROLL_EFFECT (`0xA7`: character, success 1/0, cursed, Legendary
Spirit, White Scroll) goes to the scroller and the map, then the look
refresh unless it simply failed. Legendary Spirit requests are refused with
SHOW_SCROLL_EFFECT -1 to the scroller alone; HeavenClient ignores that, as
it has no Legendary Spirit window. HeavenClient plays the Enchant sound and
the scroll glow, and gives the scroller the v83 chat line. It has no sealed
items, so the v83 sealed-item prompts are skipped.

## NPC shops

Clicking an NPC that keeps a shop (`content::shops()`, from the pack's
`shops.json`) opens it instead of its speech: `net::shop::Shop::open` builds
the list and the client actor holds it until the player leaves (mode 3),
talks to another NPC, or changes maps. Rows whose item our v83 data lacks are
dropped, as are price-0 rows other than throwing stars and bullets, which a
shop lists only to recharge.

OPEN_NPC_SHOP (`0x131`): NPC id, row count, then per row the item id, price,
three zero ints (token price, period, unused), and either a bundle size of 1
or, for stars and bullets (`207xxxx`, `233xxxx`), the item's `unitPrice` as an
8-byte double; then the most bought at once.

NPC_SHOP_ACTION (`0x3D`) answers with the inventory change, a meso stat
update, and CONFIRM_SHOP_TRANSACTION (`0x132`) code 0 (bought) or 8 (sold,
recharged). Not enough mesos is 2 and a full bag 3, with nothing changed.
A request the client can't make (wrong row or item, too many, nothing open)
gets only an enable-actions stat update.

- **Buy** (row, item, quantity): the row must hold that item at a price.
  Stars and bullets come as one full set for the price, in a slot of their
  own (`Inventory::add_alone`); a set is `slotMax` plus 10 per level of Claw
  Mastery (4100000) or Gun Mastery (5200000). Anything else is 1 to
  `slotMax` at price × quantity.
- **Sell** (bag slot, item, quantity): the item's `price` each. A set of
  stars or bullets goes whole, for its share of the price of a full set.
  Cash items can't be sold, and a sale that would pass 2,147,483,647 mesos
  is refused.
- **Recharge** (USE slot): stars or bullets the shop lists, below a full
  set, refilled for `unitPrice` per missing one, rounded up. The client
  sends it from the Rech. button on the stack's row in the shop window.

## Storage

An account's storage (`game::Storage`) holds mesos and up to its slot count
of items (4 for a new account), shared by all of the account's characters.
The items keep the order they were stored or sorted in. The client lists each
bag's items in that order and takes one out by its index in the list. Any NPC
with `info/trunkPut` in Npc.nx keeps one (Mr. Lee, 1012009, in Henesys Park);
`trunkPut` and `trunkGet` are its fees. Clicking such an NPC opens the storage
instead of its speech. `ClientActor` loads it (`db::storage::load`) and holds
a `net::storage::OpenStorage` until the player closes it, talks to another
NPC, or changes maps.

TRUNK_RESULT (`0x135`) starts with a mode byte. Open (`0x16`) adds the NPC
id. Open and the change modes then carry the contents: slot count, a u64 of
flags, the mesos if flag 2 is set, then for each bag whose `2 << type` flag
is set, a count byte and the items (`GW_ItemSlotBase` without a position).
The client keeps the bags and mesos that a result leaves out.

TRUNK_REQUEST (`0x3E`) is answered by mode:

- **Take out** (4: tab, index): costs `trunkGet`. The item goes into a bag
  the same way a pickup does. Stars, bullets and equips get a slot of their
  own. Answers TAKEN_OUT (`0x09`) with that bag. Errors: no mesos `0x0c`, a
  one-of-a-kind item the player already has `0x0b`, full bag `0x0a`.
- **Store** (5: slot, item, quantity): costs `trunkPut`. The quantity can be
  part of a stack. A set of stars or bullets and an equip go whole. Cash,
  quest and untradable items can't be stored. Answers STORED (`0x0d`) with
  that bag. Errors: storage full `0x11`, no mesos `0x10`.
- **Sort** (6): orders items by id. Answers SORTED (`0x0f`) with everything.
- **Mesos** (7: amount): a positive amount takes mesos out and a negative one
  puts them in. Answers MESO (`0x13`) with only the mesos.
- **Close** (8).

Any other bad request (wrong slot or item, too many, mesos the storage lacks)
gets `0x12`, which the client shows as "Due to an error, the trade did not
happen." Every change sends the inventory operation and a meso stat update
first. The storage and the player are then saved in one transaction
(`db::storage::save`), so an item is never in both places or neither. If
saving fails, both are restored.

The client's window is `UIStorage`, from v83's CTrunkDlg (layout in
`v83ref` report 163). Stored items are on the left and the player's storable
items on the right; one tab picks the bag for both. Clicking a selected row
again takes it out or stores it, like the shop. The v83 prompts are reproduced
except karma. Stored equips show their name and icon but not their stats.

## Trading

Two players on one map trade in a trading room (mini room type 3). The
client sends PLAYER_INTERACTION (`0x7B`), and `PlayerInteractionHandler`
turns each mode into a `net::trade::TradeRequest`. The server answers with
PLAYER_INTERACTION (`0x13A`); the builders are in
`net::packet::build::world::mini_room`. Other room types and the client's
item CRC answer (`0x14`) are ignored.

The work is split three ways:

- The room lives in the field (`runtime::actor::field_trades::Trades`, one
  per `FieldActor`). It is a plain struct that returns the packets and events
  to send. It knows who is in which room, whose turn it is, and the two
  members' answers. It never holds items.
- Each side's offer lives in its `ClientActor` (`client/trade.rs`). Putting
  up an item or mesos takes it out of the player at once, with the inventory
  operation or meso stat update. The client actor holds it as a
  `net::trade::Offer` until the trade ends. A save during the trade
  (`flush`) adds the offer back into the saved copy, so a crash loses
  nothing.
- `net::trade` holds the rules: fee, put item, put mesos, check, give.

The flow:

1. **Create** (mode 0, type 3): the host gets a room id, unique across
   fields.
2. **Invite** (2: character id). The target must be on the map. A missing
   target gets INVITE_RESULT 1, a busy one 2 with their name. Otherwise the
   target gets INVITE (2: type, host name, room id).
3. **Decline** (3: room id, reason 3 or 4) tells the host; **accept** (4:
   room id) lets the visitor in. The visitor gets ENTER (5) with both
   avatars, and the host gets VISIT (4). A room no longer waiting answers
   ENTER_RESULT `{5, 0, 1}` (closed).
4. **Chat** (6) goes to both members as "Name : text".
5. **Put item** (`0xf`: bag, slot, count, trade slot 1-9) and **mesos**
   (`0x10`). Each side sees the item or the new total marked with whose it
   is. Cash, quest and untradable items are refused, as in storage, and so
   is a bad slot or count. A refusal only unlocks the client (an empty stat
   update). A player of level 15 or below may put up at most 1,000,000
   mesos in a trade; asking for more gets `{0x15}` and nothing moves.
6. **Trade** (`0x11`) locks that side and shows the partner `{0x11}`.

Once both have pressed Trade, the trade settles in two phases, so no item
is ever in both inventories or in neither:

1. The room sends each client actor the partner's offer
   (`TradeEvent::Check`).
2. Each side works out what it would receive, which is the items plus the
   mesos less the fee (`trade::received`). It checks that it can take all
   of it (`trade::check`): room in every bag, no one-of-a-kind item it
   already has, and mesos within the cap. Then it answers the room
   (`RoomRequest::Answer`) with the result and a oneshot for the decision.
3. When both answers are in, the room tells both sides yes or no and
   closes. LEAVE is reason 7 on success, or the first failure's reason:
   8 for no room, 9 for a one-of-a-kind item.
4. Each client actor then takes the partner's offer on yes, or its own back
   on no. It sends the inventory operation, then the meso stat update, then
   LEAVE. The meso update must come first because the client picks its
   success message by how its mesos changed.

The fee on mesos received is 6% from 100,000,000, 5% from 25,000,000, 4%
from 10,000,000, 3% from 5,000,000, 1.8% from 1,000,000, 0.8% from 100,000,
and nothing below that. It is rounded down.

Leaving (`0xa`), changing maps or channels, entering the Cash Shop, or
logging out ends the trade. The leaver gets their offer back with no LEAVE,
and the partner gets theirs back with LEAVE reason 2. If the field dies,
the trade fails with reason 8. Anything that no longer fits when an offer
comes back drops at the player's feet.

## Tests

`game` unit tests cover stacking, full bags, removal order, merging,
swapping, overall/pants, and gather/sort (replaying the swaps); `net` covers the packet layouts and the handler;
`inventory_moves` (integration) equips, unequips, moves, checks refusals,
observes UPDATE_CHAR_LOOK from a second player, gathers and sorts the equip
bag, and checks persistence; `player_drops` drops a weapon for a second player
to pick up and drops mesos earned from snails, checking both losses are saved;
`net::shop` covers buying, selling, recharging, and their refusals, and
`npc_shops` (integration, needs the pack's `shops.json`) opens the Henesys
grocer, is refused for want of mesos, sells a sword, buys an arrow with it,
and checks both are saved. `skill_books` (integration) refuses a mastery
book for another job and teaches Fire Demon to an Arch Mage.
`game::scroll` covers the fit, the slot checks and each kind's roll;
`net`'s scroll handler covers the packets, the White Scroll, the curse and
refusals; `scrolls` (integration) scrolls the starting sword with and
without a White Scroll, is refused a helmet scroll, watches the glow from a
second player, and checks the upgrade is saved. The client's `scroll_rules`
test covers the fit, the White Scroll prompt and the chat lines.
`net::storage` covers each request and its refusals. `storage` (integration)
opens Mr. Lee's storage, stores part of a stack and a whole one for the fee,
is refused too many and too many mesos, moves mesos in, sorts, takes items
out, and after a relogin finds both sides saved. The client's
`storage_contents` test reads the contents of each kind of result.

Trading is tested at three levels:

- `net::trade` covers the fee, putting items and mesos up, the meso limit,
  check, and give. `mini_room` covers the packet layouts, and
  `player_interaction` covers parsing.
- `runtime::actor::field_trades` covers the room's invites, visits, turns,
  the two-phase settle, and leaving.
- `trade` (integration) has two tests:
  - The first invites a missing character, is declined, then trades items
    and mesos less the fee. Along the way it is refused over the low-level
    meso limit. It finds both sides saved after a relogin.
  - The second cancels a trade, then fails one against a full bag. Both
    times everything goes back.
