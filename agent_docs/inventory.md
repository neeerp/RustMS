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
`quest`, `only`, `price`, consumable `spec`) and `Character.nx` (equipment
stats `inc*`, `tuc`, and `req*` requirements) on demand. Stacks default to
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
moves only get an enable-actions stat update. Dropping (destination 0) waits
for floor items.

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

HEAL_OVER_TIME (`0x59`) is HeavenClient's natural recovery: every 10 s while
below full it reports +10 HP / +3 MP. The server accepts at most +20 HP and
+10 MP per report and one report per 9 s; others are ignored.

## Tests

`game` unit tests cover stacking, full bags, removal order, merging,
swapping, overall/pants, and gather/sort (replaying the swaps); `net` covers the packet layouts and the handler;
`inventory_moves` (integration) equips, unequips, moves, checks refusals,
observes UPDATE_CHAR_LOOK from a second player, gathers and sorts the equip
bag, and checks persistence.
