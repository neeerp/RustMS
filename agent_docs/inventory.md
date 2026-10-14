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

HEAL_OVER_TIME (`0x59`) is HeavenClient's natural recovery. As in v83, it
comes only at rest: after every 10 s spent standing, lying prone or sitting
without sending movement, the client reports +10 HP / +3 MP if below full.
The server accepts at most +20 HP and +10 MP per report, one report per 9 s,
and none within 9 s of the player's last PLAYER_MOVE; others are ignored.

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
  set, refilled for `unitPrice` per missing one, rounded up.

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
and checks both are saved.
