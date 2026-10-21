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
`build_inventory_grow` (INVENTORY_GROW `0x1E`: bag type, new slot count)
widens a bag outside the Cash Shop, as the first job advancement does.

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

## Expiry

Items may carry an expiry date (`Item::expiration`, saved as
`inventory_items.expires_at`): cash items bought for a period, and quest
gifts with a `period` in minutes (`net::items::give_until`). v83's client
shows the date but removes nothing, so the server takes the items when it
passes, as Cosmic's expiration task does: `net::items::expire_items` takes
every item, worn or in a bag, whose date has passed, except pets (their
lives run out separately) and items dated 2079 or later (the client's "no
expiry"). It runs as the player enters the game, before CHAR_INFO is built,
and again whenever the next date comes while they play (the client actor
waits for `next_expiry`; not in the Cash Shop). Either way the client gets
SHOW_STATUS_INFO mode 8 with the item ids ("... has been expired, and
therefore, deleted from your inventory."); in play it also gets the
INVENTORY_OPERATION removals and, if a worn item went, the refitted HP and MP
and the new look for the map. Sealed items (mode 11) and expiring skills
(mode 13) aren't implemented.

Items with `info/expireOnLogout` (127 of them, mostly event and PQ items)
go when their owner logs out: the client actor's cleanup takes them
(`net::items::drop_logout_items`) before the final save, but not on a channel
change. The client shows nothing. A crash before the save leaves them, since
the server can't yet tell a fresh login from a channel change.

## One-of-a-kind items

An item with `info/only` comes alone, and not to someone holding one, worn
or not (`net::items::refuses_only`). Each way in refuses it differently, as
v83 has no single code (report c005):
- Pickup: SHOW_STATUS_INFO mode 0, -2 ("This item is unavailable for the
  pick-up.") plus the unlock, and the drop goes back on the ground.
- Quest reward: UPDATE_QUEST_INFO mode 14.
- NPC shop: result 3 (inventory full).
- Script `GiveItem`: the effect isn't applied.
- Taking it out of the Cash Shop: inventory full. The client refuses this one
  itself.

## Client moves

ITEM_MOVE (`0x47`: tick, type, from, to, quantity) is handled by
`ItemMoveHandler`: a negative source unequips, a negative destination equips
(after `can_wear`), anything else moves within the bag. The reply is the
INVENTORY_OPERATION; a worn-equipment change also refreshes the field avatar
and sends UPDATE_CHAR_LOOK (`0xC5`) to the other players on the map. Refused
moves only get an enable-actions stat update.

HeavenClient checks a wear before asking (`UIItemInventory::can_wear_equip`),
in v83's order (FUN_004f1c2d): a second Unique Equipped Item is refused; a
tamed monster or saddle that doesn't fit the saddle or monster worn on the same
layer (both normal or both cash) is refused without a word; then stats too low;
then a cash tamed monster or saddle with no normal one under it is refused
("You must have a tamed monster in order to equip this." / "You must be
equipped with a saddle to equip this."). A saddle fits the monsters its
`Character/TamingMob/0191xxxx.img` lists beside `info`, compared by their last
two digits as v83's 64-bit mask does, so one ending 64 or above fits nothing
(`mount::saddle_fits`). v83's "You can ride the tamed cash monster when it is
equipped with a Cash Saddle." follows a check the silent one already made, so
it never shows and isn't done.

Dropping is a move to destination 0 from a bag slot (`drop_item`; worn items
can't be dropped, and the client never asks). Stars and bullets go as the
whole stack; anything else drops the asked quantity, keeping equipment's
stats (`Loot::Dropped`). Quest and `tradeBlock` items leave the bag but don't
reach the floor: everyone sees them vanish (DROP_ITEM_FROM_MAPOBJECT 3).
MESO_DROP (`0x5E`: tick, amount) drops 10 to 50,000 mesos the player holds;
HeavenClient's meso button asks how many with `UIEnterNumber`, which, as in
v83, starts at 10 and takes no less. The field
places either kind where it last saw the player, and anyone may pick it up
at once (ownership 2). Dead players can't drop.

A map whose `fieldLimit` has bit 0x400000 keeps items and mesos off the
ground. An `info/everlast` map keeps thrown items off it: v83 silently does
nothing when an item is dragged out of the inventory there (FUN_004f31a5),
HeavenClient does the same, and the server refuses the drop. Mesos are not
stopped (the client's meso path wasn't traced). As in v83, HeavenClient asks its usual questions first and then says
"You cannot drop the item here." in the skill chat colour instead of sending
the drop. It also refuses an item drop while the player is alert from a hit
or an attack ("You can't do it right now. Catch your breath and then try
again.") or dead ("You can't do it once you're dead."). The server refuses
item and meso drops on such a map too (`FieldTemplate::may_drop`) with an
enable-actions stat update.

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

v83 sends USE_ITEM only for potions, food, cures, transformations and the
like (`200`, `201`, `202`, `205`, `221`, `236`, `238` and `245`xxxx,
`game_data::is_use_item` and HeavenClient's `field_limit::is_use_item`).
Every other USE kind has its own request. The server refuses USE_ITEM for
other kinds, and for items with nothing it can apply (no HP, MP, cure or
timed stat), so they aren't used up for nothing. HeavenClient's double click
and quick slots send nothing for kinds that have no request yet.

A map whose `fieldLimit` has bit 0x400 allows only the items its
`info/allowedItem` lists (eleven event maps list ten items). HeavenClient
says "This item cannot be used here." in the skill chat colour when the
player double-clicks or presses a quick slot for anything else of the kinds
v83 checks there: potions, food, cures and transformations
(`field_limit::may_use`). Other kinds aren't checked. The
server applies the same rule (`FieldTemplate::may_use`) and answers a refused
USE_ITEM with an enable-actions stat update. Double-clicking any item while
dead says "You can't do it once you're dead."

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

### Return scrolls

USE_RETURN_SCROLL (`0x55`: tick, slot, item id) reads a return scroll
(203xxxx), which v83 sends after its own checks (`FUN_00a1e1ce`). The scroll's
`spec/moveTo` is a map, or 999999999 for the nearest town (the map's
`info/returnMap`). A map whose `fieldLimit` has bit 0x20 refuses every scroll,
and bit 0x100 those that name a map, with "You can't use it here in this
map.". A scroll that names a map must stay in the player's area unless its
`spec/ignoreContinent` is set ("You cannot go to that place."). The areas
come from `Map.nx` `Map/AreaCode.img`, keyed by a map id's first two digits;
event maps (millions digit 9) and the ships (20009xxxx) are in none
(`GameData::same_area`, HeavenClient's `field_limit::same_area`).

The server checks the same rules, and that the target exists and isn't the
current map, then spends the scroll and warps the player to the target's
`sp` portal. A refusal keeps the scroll and answers with an enable-actions
stat update.

Event invitations (2031000-2031008) also have `spec/returnMapQR`, which v83
never reads. The server saves the map the invitation was used in under that
number (`saved_locations`), and the event map's exit script takes the player
back there (`photoOut`, `StudioZone_Out`, `outBabyBird`; see
`map-travel.md`). The event maps' field limits refuse every scroll, so the
exit is the only way out. Two are refused: 2031002, whose moon map is left
through an NPC that has no script yet, and 2031000, whose map isn't in the
data. 2032000 (`randomMoveInFieldSet`) names no map and is refused too.

### Skill and mastery books

USE_SKILL_BOOK (`0x52`: tick, slot, item id) reads a skill book (228xxxx)
or a mastery book (229xxxx). v83 sends it from the item-use path with the
usual exclusive-request gate and checks nothing itself. Like v83,
HeavenClient sends it from the USE tab double-click but not from a quick slot. The book's
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

### Megaphones

A megaphone (507xxxx) is used through USE_CASH_ITEM (`0x4F`: Cash slot, item
id, the megaphone's fields, tick), in `world/megaphone.rs`. The kind is the
item id's thousands digit, as the v83 client reads it: 1 and 8 are plain
megaphones, 2 super, 6 (and 5 with a last digit of 6 or more) item
megaphones, 7 triple; 5 up to 5 is Maple TV, which isn't done. The fields
are the text and, but for the plain megaphone, the ear byte; the item
megaphone adds whether an item is shown and then its inventory kind and
slot, and the triple megaphone starts with its line count (1 to 3). Every
line is at most 60 bytes and goes out as `<name> : <text>`, which the client
splits on " : ". The plain megaphone needs a level above 10.

BROADCAST_MSG (`0x44`) carries the line: type 2 to every player on the
speaker's channel (`BroadcastScope::Channel`), and type 3 (super, with the
zero-based channel and ear), 8 (item, then whether an item follows and the
item as the inventory encodes it) and 10 (triple: the first line, the count,
the rest, channel, ear) to the whole world, the speaker included. The client
actor hands the line to the world directory (`WorldAction::Broadcast`), which
knows every client's channel. One exclusive INVENTORY_OPERATION takes the
megaphone first. Anything refused (too long, too low, an item that isn't
there, Maple TV) gets only an empty stat change to unlock the client.

HeavenClient shows each line in the chat log (`ServerMessageHandler`,
`UIChatBar::send_megaphone`) as v83 draws it: plain megaphones in blue on a
pale blue strip, super and triple ones in purple on pink, item megaphones in
brown on yellow with `[item name]` after the speaker's name. Every row but
the plain megaphone's starts with the `UIWindow.img/Megaphone` channel tag
(`ch0` with the ear, `ch1` without) and the two-digit channel. A triple
megaphone's later lines take the first line's name. Lines are dropped on
event maps (`Util/Megaphones.h`); v83 shows them there to GMs, but
HeavenClient doesn't know who is a GM.

Two other server message types aren't megaphones. Type 6 (a line with an
item ID) is shown as a blue chat line; v83 also links the item, which
HeavenClient leaves out. Type 7 (an NPC ID and the text) opens that NPC's
say dialog with OK and no NEXT or BACK; closing it sends nothing back
(0xa235fa).

Double-clicking a megaphone in the Cash tab opens its dialog
(`UIItemInventory::use_megaphone`), as v83 does:

- Plain: a text prompt ("Enter your comments through the megaphone.", 60
  characters). After OK, an empty line is dropped, then text the curse filter
  won't take (`CurseData::valid`), a level of 10 or below, and a line over 60 bytes are refused with v83's notices.
- Super and triple: `UIMegaphone`, a modal window on
  `UIWindow.img/Megaphone` (one line) or `ArtMegaphone` (three), with the
  "Display whisper icon" checkbox (on at first). It closes before the lines
  are checked as above; the triple megaphone asks "One of the lines has not
  been applied." before sending with an empty line, and sends the lines up
  to the last one typed. Tab moves to the next line. Preview writes "This is
  a preview option for the Triple Megaphone." and the lines up to the last
  one typed to the own chat log only, and nothing when all are empty.
  "Special character" opens the item's samples (`Item.nx`
  `Cash/0507.img/<id>/ui` and `sample`) in a modal popup: a sample replaces
  all three lines, "CREATE NEW" empties them, and Escape or a click outside
  closes it.
- Item: `UIItemMegaphone` on `ItemMegaphone`, which leaves the game playable.
  An item dropped on it from the inventory shows in its well with its name
  and tooltip; clicking the well takes it back. OK with nothing typed does
  nothing, and with no item asks first. It sends the line itself, refusing
  with "Other requests are in process." while a request is waiting.

The item megaphone can't be opened in a shop or trade ("Not available").
The gates are in `Util/Megaphones.h`.

#### Avatar megaphones

An avatar megaphone (539xxxx) comes through USE_CASH_ITEM too: four lines,
the ear byte, and the speaker's look, which the server ignores and takes
from the player (`avatar_megaphone` in `world/megaphone.rs`). A level of 10
or below is refused with AVATAR_MEGAPHONE_RESULT (`0x6E`) 'T'; four empty
lines, or one too long, get only the unlock. The banner, AVATAR_MEGAPHONE
(`0x6F`: item id, name, the four lines, the zero-based channel, the ear and
the look as character lists encode it), goes to the world directory
(`WorldAction::AvatarMegaphone`), which shows it to every client at once.
v83 shows one banner at a time and replaces it the moment another arrives,
so the directory keeps the rest in line (`runtime/src/actor/avatar_banners.rs`)
and shows each for the 10 s the client keeps it up. One that would wait past
15 s is refused with 'S' ("The waiting line is longer than 15 seconds.") and
the megaphone kept; otherwise the megaphone goes as soon as the banner is in
line. The server never sends CLEAR_AVATAR_MEGAPHONE (`0x70`).

In HeavenClient the dialog is `UIMegaphone` on `UIWindow.img/AvatarMegaphone`:
one box of up to 97 characters that wraps into four lines, which are sent as
the client wraps them, and the whisper-icon checkbox. Nothing typed is
refused with "Please enter the text."; curse words as for the other megaphones.
The packet carries the look `Player::look_entry` builds from the equipped
items. The banner is `UIAvatarMegaphone`: the item's `info/path` art from
`Map.nx` `MapHelper.img/AvatarMegaphone` in the top right corner, the speaker
standing in it with the item's `info/emotion` face, the name on the short or
long plate, and the four lines. It slides in from the right edge over 225 ms
with a small wobble, and out again after 10 s, when the next replaces it at
once, or on 0x70. The line also goes to the chat log in its own colours, with the
channel tag, as `<name> : <lines>`. A banner that arrives in the cash shop
is dropped.

### Chalkboard

A chalkboard (537xxxx, `world/chalkboard.rs`) is used through USE_CASH_ITEM
with its note, 1 to 40 bytes, and isn't used up. The note goes in
`Player.chalkboard` (not saved) and the field avatar. CHALKBOARD (`0xA4`:
character id, 1, note) goes to the holder and the map, then an empty stat
change unlocks the holder's client; the holder's client needs the 164 too,
since it only lets the player click the board away once told. SPAWN_PLAYER
writes the board (1 and the note, or 0) after the mini room balloon, so
later arrivals see it. CLOSE_CHALKBOARD (`0x32`, empty) puts it away with
CHALKBOARD `id, 0` to the holder and the map.

The board comes along to the next map, where the client actor tells the
holder's client again (v83 keeps its own board across the change anyway).
Entering a map where shops open (`info/personalShop`, the Free Market rooms)
puts it away, as the v83 client does there; the server refuses a board in
those maps too. Losing the last chalkboard doesn't put the board away yet.
The server doesn't mask curse words in the note: v83 clients star them out of
other players' boards themselves.

HeavenClient draws the board (`Character/ChalkboardBalloon`, layout in
`Util/Chalkboard.h`) from `ChatBalloon.img/adboard/0` as v83 does: white
text wrapped by character at 196 px, one line on an odd number of 14 px
tiles so the arrow is centred, a longer note on 14 tiles, 5 px over the
head. Another player's note, from CHALKBOARD or SPAWN_PLAYER, is trimmed,
has its curse words starred out and is cut to 40 bytes; the player's own is
only trimmed. The player's own board has the `Basic.img/BtClose` button,
which takes a press and a release on it and does nothing while a mini room
is open. It sends CLOSE_CHALKBOARD and logs "The chalkboard has closed.";
the board stays until CHALKBOARD `id, 0`. The board follows the player to
the next map, except to one where shops open, where it goes away with "You
cannot use the chalkboard here.". Double-clicking a chalkboard in the Cash
tab asks "Enter a note." (40 bytes; a blank note sends nothing), unless a
mini room is open ("Not available") or shops open on the map ("You can't use
it here in this map.").

### Teleport rocks

A character keeps maps for the teleport rocks (504xxxx): five for the
regular rock, ten for the VIP one (any 504 id outside 5040xxx). The lists
are `Player.teleport_rocks` (`game::teleport_rock`), saved in the
`teleport_rocks` table by place, and SET_FIELD's character data writes
every slot, an empty one as 999999999. TROCK_ADD_MAP (`0x66`: 1 to keep
the player's map or 0 to drop one, the VIP flag, and the map id when
dropping) answers with MAP_TRANSFER_RESULT (`0x2A`: 3 kept or 2 dropped,
the VIP flag, the list's slots). The client offers to keep only maps off
Maple Island and outside the event maps (`(id / 1000000) % 100 == 9`);
the server refuses those, maps whose `fieldLimit` forbids rocks (`0x40`),
a full list and a map already kept with result 10.

The rock is used through USE_CASH_ITEM with 0 and a kept map id, or 1 and
a character's name (`world/teleport_rock.rs`). A name goes to the world
directory (`WorldAction::RockToCharacter`), which asks the character's
client actor on the same channel where they stand; the answer comes back to
the user's actor, which finishes with `to_character` and lands on the spawn
point nearest them. Either way the rock is refused with MAP_TRANSFER_RESULT
`result, vip` and an empty stat change, since 42 doesn't unlock the client:
6 for a name not found (or the user's own), 9 for the map they are on, 11
on Maple Island under level 7, and 8 for the rest: a map not kept, a dead
user, a map with `fieldLimit` `0x40` or `0x2` on either end, another
continent (`id / 100000000`) for the regular rock, or a map that sends
players elsewhere on login (`forcedReturn`) but on Maple Island, as Cosmic
does. Otherwise one exclusive INVENTORY_OPERATION takes one rock and the
user is warped.

HeavenClient reads both lists from the character data
(`Character/TeleportRock`) and keeps them from MAP_TRANSFER_RESULT 2 and 3.
Double-clicking a rock in the Cash tab opens `UITeleportRock`
(`UIWindow.img/Teleport`), centred and modal, unless a request is still
waiting ("Other requests are in process.", which the chalkboard checks too),
the player is dead ("You can't do it while you're dead."), or the map is an
event map (the millions digit 9) or its `fieldLimit` has `0x40` or `0x2` ("You
cannot teleport to this map."). It lists the kept maps by name (a map with
`info/link` under the linked map's), five at a time with a scroll bar on the
VIP rock, and counts them as `[used/size]`. ADD MAP asks before sending
TROCK_ADD_MAP for the current map, unless the list is full, already has it, or
the map can't be kept; DEL. MAP asks before dropping the selected map. MOVE
asks before using the rock on the selected map, or uses it at once on the
typed name once the name is valid and isn't the player's own. Enter is MOVE
and Escape closes the window, which closes before the rock is used. A refused
rock shows the result as a notice, naming the character for result 6.

### Meso sacks

A meso sack (5200000-5200002) holds `info/meso` mesos (1, 5 or 10 million).
The client asks "Will you use the meso bag to receive N mesos?" and sends
USE_CASH_ITEM with no body. The server takes one sack and adds the mesos:
STAT_CHANGED with the mesos, INVENTORY_OPERATION, and SHOW_STATUS_INFO
mode 5 ("You have gained mesos"). A sack that would pass 2^31 - 1 mesos
stays shut with only the unlock.

### Writs of Solomon

A Writ of Solomon (237xxxx) gives its `spec/exp` not as EXP but as gacha
EXP, which the player takes in later. It is `CharacterStats.gacha_exp`,
saved in the `gacha_exp` table (a row only while some is left, since
`characters` is at diesel's 32-column limit), sent after fame in the
character stats and as stat `0x200000`, an int. USE_SOLOMON_ITEM (`0x9D`:
tick, USE slot, item id) takes one writ and answers with its
INVENTORY_OPERATION and the new gacha EXP (`world/solomon.rs`); a writ above
its `info/maxLevel` (50), while gacha EXP is left, or not in that slot gets
only an empty stat change. USE_GACHA_EXP (`0x9E`: tick) takes in up to
level 50 what finishes the level, and the whole next level too if the rest
covers it, or all of it if it doesn't finish this one, as Cosmic's
gainGachaExp does: `WorldAction::GainExp` adds it without an EXP message,
and the stat update shows what is left.

HeavenClient keeps the stat (`CharStats::get_gachaexp`). Double-clicking a
writ sends USE_SOLOMON_ITEM, or logs "Your level is too high to use the
selected item." above its `maxLevel` and "You'll need to use all your
remaining EXP in order to use a new Writ of Solomon." while gacha EXP is
left (v83 FUN_00a12685). Hovering over the EXP numbers on the status bar
shows "EXP : exp / need", with "  GachaponEXP : n" while some is left (v83
FUN_008d76d7); double-clicking them up to level 50 with some left asks "Do
you wish to charge the EXP earned from The Writs of Solomon?", and Yes sends
USE_GACHA_EXP (FUN_008d7611). The rules are `Character/GachaExp.h`. v83's
tooltip over the level and name is not drawn.

### Catch items

A catch item (227xxxx) is thrown at a mob to catch it, and turns into the
item its `info/create` names: the Pheromone Perfume (2270000) catches a
Tamable Hog (9300101, map 922200000) and becomes the Hog mount (1902000).
USE_CATCH_ITEM (`0x51`: tick, USE slot, item id, mob object id) is handled
in `world/catch_item.rs`. The handler checks that the player is alive, the
item is in that slot and what it makes would fit, then asks the field
(`FieldRequest::Catch`). The field catches a mob of the item's `mob`
template that stands within the item's area of the player (`left`, `top`,
`right`, `bottom`, plus 100 px slack, since the client finds the mob by its
body and the server knows its feet) and is below `mobHP` percent of its HP.
It broadcasts CATCH_MONSTER_WITH_ITEM (`0xFC`: oid, item, 1) and
KILL_MONSTER (leave type 1), and the client actor trades the item for what
it makes (`ServerMessage::Caught`). A mob above `mobHP` gets
BRIDLE_MOB_CATCH_FAIL (`0x4F`: 0, item, 0), "the monster is too strong".
A mob weak enough is caught `bridleProp` percent of the time (the Element
Rock, 2270002, has 50); otherwise the map sees CATCH_MONSTER_WITH_ITEM with
result 0, the mob stays and the item is kept. What `bridlePropChg` (1.2 on
the rock) does is unknown, so it is ignored. An item with a `useDelay` (the
rock's 800 ms, the Fish Net's 3000 ms) thrown again within it gets
BRIDLE_MOB_CATCH_FAIL result 1, which shows its `delayMsg`; the wait runs
from the last throw that got past it (`Player::last_catch`, not saved).
Every answer but a catch ends with an empty stat change, which unlocks the
client.

HeavenClient throws one on double-click (500 ms between throws) or from a
quick slot (`Player::use_catch_item`, v83 FUN_00a09bdf). Riding a tamed
monster, it does nothing. Without room for what it makes, it says "You do
not have enough slots in your item inventory." for an equip, else "<tab>
item inventory is full.". It looks for the first mob of the item's template
to have entered the map that overlaps the area around the player, whichever
way they face (`MapMobs::find_catchable`); finding none, it says the item's
`bridleMsgType` line ("Unable to find a tamable monster in this area." and
others) and sends nothing. It shows CATCH_MONSTER_WITH_ITEM as the item's
`Effect/ItemEff.img` animation over the mob, or its `fail` frames, and plays
`Game.img/CatchSuccess` or `CatchFaild` for the Element Rock. The rules are
`Character/CatchItemRules.h`; every line is in the skill chat colour.

### Vicious' Hammer and Vega's Spell

Both come through USE_CASH_ITEM. Vicious' Hammer (5570000) sends the equip's
tab and slot after the item id (`world/hammer.rs`). The server refuses with
VICIOUS_HAMMER (`354`: `0x3e`, reason) when the equip is not upgradable
(1: it has no upgrade slots in its template, is pet equipment, or came from
the Cash Shop), when it has had two hammers already (2), or when it is the
Horntail Necklace (3). A hammer that isn't in the slot, a tab other than
Equip, or a dead player gets reason 1 too, since the window waits for an
answer. Otherwise the hammer adds one upgrade slot and one to
the equip's `vicious` count (saved in `equipment_stats.vicious`, sent in the
equip's stats). The server answers with INVENTORY_OPERATION and then
`354: 0x39, 0, applied`. When the client's gauge is full, it sends
USE_HAMMER (`0x104`: code, flag), and the server's `354: 0x3d, 0` closes the
window.

Vega's Spell (5610000 for 10% scrolls, 5610001 for 60% ones) sends the
equip's tab and slot, then the scroll's tab and slot (`world/vega.rs`). The
scroll must be on `Etc.nx`'s `VegaSpell.img` list (`game-data`'s
`VegaSpells`; `prob` is a real or the text `[R8]0.1`) at the spell's rate,
must fit the equip, and the equip may not be a cash item. The spell raises
10% to 30% and 60% to 90%, and the scroll is applied like any other. The
roll is decided at once. VEGA_SCROLL (`358`) sends `0x40` (success) or
`0x45` (failure), which starts the window's roll. The inventory change and
`0x41` or `0x43` follow 2.6 s later, through `WorldAction::ReplyLater`, so
the equip's new stats don't show before the roll ends. The client actor
keeps those packets in `ClientActor::later` and sends them when due, or at
once before any other inventory change, since they carry the slots' states
from when they were queued. Entering the Cash Shop drops them. A refused
spell gets only an empty stat change, which unlocks the client; as in v83,
the window then stays until it is closed, since any code but the four would
close it without unlocking (report c004c).
Whether the map should see the scroll's effect (`0xA7`) is unknown.

HeavenClient opens `UIViciousHammer` on double-clicking a hammer and
`UIVegaSpell` on a Vega's Spell (both use UIWindow.img in UIClassic.nx).
Only dragging puts an item in: the drop reads the icon's source tab and
slot (`Icon::get_source`).

The hammer window runs the client's own checks (`vicious_hammer::check` in
`Character/Inventory/CashUpgrades.h`) and says why it refuses. Start sends
the hammer and plays `EffectP`. The gauge fills 160 columns at 59 a second.
Once the gauge is full and the answer is in, the window sends USE_HAMMER,
waiting a second after a success while `EffectE` plays. The last answer
closes the window with "Increased available upgrade by 1. N upgrades are
left.", the reason, or "Unknown Error (n).". Esc closes it only before
Start, and it is dragged by its top.

The Vega window takes an equip from the Equip tab and a scroll on the list
from the Use tab ("This scroll cannot be used.", or "This item cannot be
used." for anything else). It shows the scroll's rate and the raised one in
its `Count` digits. Start checks the scroll fits, then sends and plays
`EffectArrow`. The start code plays `EffectSpelling`, loops
`EffectTwinkling` and the `VegaTwinkling` sound, and fills the gauge at 63
px/s. When the gauge passes 159 px and the result code is in, `UIVegaResult`
opens `SuccessWnd` or `FailWnd` with the equip, its effect and its sound,
and OK, Esc or Return closes it. Hovering over an equip in either window
shows its tooltip.

The equip tooltip has a "NUMBER OF VICIOUS' HAMMER APPLIED : n" line
("2(MAX)" at two) for any equip that could take a hammer. Like v83's, the
tooltip widens to 20 px past its widest stat line.

Inferred, not taken from the client: what the two rate boxes mean, that the
Vega windows close on Esc (the Vega window only before Start, since the
spell is spent once sent), that Start refuses a scroll whose rate the spell
doesn't raise and treats an unlock with no start code as a refusal, and the
result window's hover area.

### Item Maker

The Maker skill (`base*10000+1007`, base being `job/1000*1000`, or 2001 for
Evan) opens the client's Maker window, which sends ITEM_MAKE (`0x71`,
`world/maker.rs`; reports c006, c006b and c006e). `game-data`'s
`MakerData` reads the recipes from `Etc.nx`'s `ItemMake.img` (each named for
what it makes, in a group for the jobs that see it) and, from `Item.nx`,
each etc item's `info/lv`, the levels each monster crystal (4260000 to
4260008) is made from, and what each gem (`Etc/0425.img`) does.

Mode 1 (`1, itemId, stimulator flag, gem count, gems`) makes a recipe. The
server checks what the window checks: the player is alive, has the level,
the Maker level, the held item, the worn item and the quest states the
recipe asks, every ingredient, and the fee. The fee is the client's: the
recipe's `meso` plus a tenth of it for each part (one for the recipe, five
for a stimulator, three for each gem plus three times its grade), floored to
a thousand. A recipe with a `randomReward` list (the gems) makes one of its
outcomes by weight. Mode 3 (`3, itemId`) takes 100 of an etc item with a
level and gives the crystal for that level. Random making (mode 2) is
refused: no recipe has the `hide` node that would make one reachable.

Equipment may take its recipe's stimulator (`catalyst`; the packet sends
only the flag) and up to `tuc` gems, one of each kind (`id%10000/100`), the
attack kinds 0 and 1 on weapons only. A stat gem adds its `inc*` value; a
level gem does nothing, v83 equipment having no such stat; `randStat` and
`randOption` move each nonzero stat of their set by up to their amount
either way. Without a stimulator the item has its own stats; with one, each
nonzero stat but speed gains up to 2 (5 for defense, HP and MP), and one
time in ten the item is destroyed with everything it took (`217: 1`, the
"destroyed" dialog). The success roll and the stimulator's effect are not
in the client; these are Cosmic's.

Mode 4 (`4, itemId, 1, slot`) takes apart the Equip tab's item in `slot`
when a recipe makes it, for half of each crystal the recipe takes (Cosmic;
what v83 gave is unknown). Its fee is the client's: a tenth of the recipe's
`meso`, and half that again for each grade above -1, floored to a thousand.
The grade (FUN_005ca1b8) sums the item's stats above its own (HP and MP a
tenth): below 0 is -1, to 5 is 0, to 22 is 1, to 39 is 2, then 3. It answers
`217: 0, 4, itemId, n, (crystal, count)*, meso`.

The window waits for MAKER_RESULT (`217`) after every ITEM_MAKE, a
request cut short included. Success sends a meso stat change (for a recipe
with a fee; a crystal costs none), INVENTORY_OPERATION, then `217: 0, 1, 0, item,
count, n, (id, count)*, gems, stimulator, meso` for a recipe (`217: 1, 1, 1`
and no item when destroyed) or `217: 0, 3, crystal, item` for a crystal,
then SHOW_ITEM_GAIN_INCHAT `0x10, 0`, which plays the
Maker's success effect and sound. A refusal is `217: 2` ("Production failed
due to an unknown error."), or 4 to 7 when the Equip, Use, Setup or Etc tab
has no room for what was made. A one-of-a-kind item already held is
refused like an unknown error. Whether the map should see the effect is
unknown, so it isn't sent.

The client's window is `UIMaker` (`UIWindow.img/Maker`, rules in
`Character/Inventory/MakerRules.h`; reports c006b and c006d). Casting a
Maker skill, from a key or the skill book, opens it and sends nothing. Two
v83 drop lists (`Basic.img/ComboBox4`, ten 16 px rows and a scroll bar past
that) pick a category, then an item: the categories are the fixed ones, the
gems, each kind of equipment the listed recipes make (named from the string
pool, " ONE_HANDED MACE" and all), Use, Setup and Etc, then "???". A recipe
is listed when the player has its level, Maker level, held and worn items
and quest states. A recipe shows its ingredients with "owned/needed" counts
(faded when short) and the fee; for a monster crystal the window shows
`randomRecipe` and "0/100" until an etc drop with a level is dropped on its
slot (in HC a dragged icon lands on the next click), and a click on the drop
takes it back out. Equipment shows its stimulator and `tuc` gem slots
(`notUse` past them); a stimulator or gem dropped on them is checked in
v83's order with its notices, raises the fee, and comes out on a click or
once none are left, staying in for the next make otherwise. For a
disassembly an equipment from a recipe is dropped on the first slot ("This
item cannot be disassembled." otherwise), and the fee follows its grade.
Create is enabled each frame while every ingredient is there; the
stimulator and gems are optional. It asks first, checks for room
(FUN_0082744f: a free slot in the tab the item goes to, or for a
disassembly one free Etc slot per ingredient of the recipe), then the fee
("You do not have enough mesos.", nothing for a disassembly), fills the gauge for 5.1 s with `Sound/UI.img/ItemMake` looping,
sends, and waits for MAKER_RESULT. That writes v83's grey chat lines (what
was gained, each ingredient lost, the mesos) and the result dialog, and
SHOW_ITEM_GAIN_INCHAT `0x10` plays `BasicEff.img/ItemMake/Success` or
`Failure`; any other code there does nothing. The selection stays for the
next make. Cancel closes the window at any time, as v83's Cancel does; so does
Esc, though the first Esc only closes an open list. The skill book keeps SP
off the Maker skills, which have `disable` set.

Not done in the window: random making (selectable but Create stays off;
v83's always ends in "Production failed."). The drop lists can't be opened while
the gauge fills, which v83 allows, and they don't take the arrow keys and
Enter, which HC sends to the player. The window opens centred, where v83 remembers its
place, and is dragged by its top 20 px, a height not checked against v83.

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
  own. Answers TAKEN_OUT (`0x09`) with that bag. Errors: no mesos `0x0b`, a
  one-of-a-kind item the player already has `0x0c`, full bag `0x0a`.
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

## Personal shops

A player with a store permit (`514xxxx`, in the cash bag) opens a personal
shop (mini room type 4) on a Free Market map and sells items in bundles to
up to three visitors at a time. Shops use the same PLAYER_INTERACTION
packets as trading. `PlayerInteractionHandler` turns the shop modes into a
`net::personal_shop::PersonalShopRequest`. Visiting, chatting and leaving
stay `TradeRequest`s, shared with trading rooms. Maps allow shops where
Map.nx sets `info/personalShop` (the Free Market rooms).

The work is split three ways, as for trading:

- The room lives in the field (`runtime::actor::field_shops::Shops`). It
  holds the members, title, balloon skin, whether the shop is open, the
  blacklist, the banned names, and a display copy of the listings. It also
  collects balloon changes, which the field shows by setting the owner's
  `FieldAvatar::mini_room` and broadcasting UPDATE_CHAR_BOX (`0xA5`). The
  same box goes into the owner's spawn packet.
- The stock lives in the owner's `ClientActor` (`client/personal_shop.rs`).
  Listing an item takes it out of the inventory, and the actor holds it as
  `net::personal_shop::Listing`s until it sells or the shop closes. `flush`
  adds the unsold stock back into the saved copy, as for a trade offer.
- `net::personal_shop` holds the rules: tax, permit, list item, check
  purchase, check sale, sell, unsold.

A visit names only a room id, so the field sends it to the shops if one has
that id and to the trades otherwise. Room ids come from one counter for
both kinds. A player in one kind of room counts as busy for the other. The
visitor's actor makes no state until the room says it is in: a trading
room's `TradeEvent::Started`, or a shop's `ShopEvent::Entered`.

The flow:

1. **Create** (mode 0, type 4: title, a byte, permit slot, permit id). A
   permit not in that slot is refused silently. Off a Free Market map the
   owner gets ENTER_RESULT `{5, 0, 0xf}`. Otherwise the owner gets the empty
   shop.
2. **Put item** (`0x16`: bag, slot, bundles, per bundle, price) and **take
   back** (`0x1b`: index) work only before the shop opens. Stars, bullets
   and equipment go whole, as one bundle. The storage rules decide what may
   be sold. Each change sends the inventory operation, then the listings
   (`0x19`) to everyone in the shop. Taking back an item with no room for it
   gets the inventory-full notice.
3. **Open**: the client sends its blacklist (`0x1e`), then `{0xb, 1}`.
   Opening needs at least one listing. The balloon shows, and visitors may
   enter. Names on the blacklist (`0x1f` adds, `0x20` removes) stay out.
4. **Visit** (4: room id). A closed shop answers ENTER_RESULT 1, a full one
   2, and a banned name `0x11`. The visitor gets ENTER (5) with the members
   and listings, and the others get VISIT (4).
5. **Buy** (`0x17`: index, bundles). See below. The buyer gets the items
   and the new listings, or BUY_RESULT (`0x18`) with a reason: 1 out of
   stock, 2 not enough mesos, 3 too expensive, 5 inventory full, 6
   one-of-a-kind, `0xe` over the low-level meso limit. The owner gets SOLD
   (`0x1a`: index, bundles, buyer) and the mesos less tax.
6. **Ban** (`0x1c`: slot, name; `0x1d` when the visitor's client turns
   itself out after an hour) sends that visitor LEAVE reason 5 (`0xf` for
   overstaying). They can't come back while the shop is open.

A purchase crosses two client actors, so it runs in steps that never lose
or copy an item:

1. The buyer asks the room for the listing (`Quote`) and checks it can pay
   and carry it (`check_purchase`).
2. It takes the mesos and sends the room a `Sale` with a oneshot for the
   answer. The room passes it to the owner, or drops it if the buyer is no
   longer a visitor.
3. The owner checks the sale against its stock (`check_sale`) and answers.
   Only if the buyer is still waiting does it take the bundles off the
   listing and add the mesos less tax.
4. The buyer takes the items on yes. On no, a dropped sale, or no answer
   within 5 seconds, it gets its mesos back and BUY_RESULT.

The tax is 3% from 100,000,000 mesos, 2.5% from 25,000,000, 2% from
10,000,000, 1.5% from 5,000,000, 0.9% from 1,000,000, 0.4% from 100,000,
and nothing below that. It is worked out in doubles and rounded down, as
the client does.

When the last bundle sells, the shop closes: the visitors get LEAVE reason
`0xe` and the owner `{0xa, 0, 0xe}`. The owner leaving (`0xa`), changing
maps or channels, entering the Cash Shop, or logging out also closes it.
The owner gets the unsold stock back, and the visitors get LEAVE reason 3.

## Tests

`game` unit tests cover stacking, full bags, removal order, merging,
swapping, overall/pants, and gather/sort (replaying the swaps); `net` covers the packet layouts and the handler;
`inventory_moves` (integration) equips, unequips, moves, checks refusals,
observes UPDATE_CHAR_LOOK from a second player, gathers and sorts the equip
bag, and checks persistence; `player_drops` drops a weapon for a second player
to pick up and drops mesos earned from snails, checking both losses are saved;
`net::items` covers which items expire and the next date, and
`item_expiry` (integration) starts Rupi's Lost Paper, has the paper run out
while playing, and finds a worn cape that expired offline gone at login.
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
`net`'s megaphone handler covers each kind's line, scope and refusals;
`megaphones` (integration) hears a plain megaphone on the speaker's channel
and the super, item and triple ones on another, then finds the used slot
empty. `net`'s chalkboard handler covers the board, the refused notes and
closing it; `chalkboard` (integration) holds one up for a watcher, takes it
to another map and back, closes it, and has a Free Market room put it away.
`game::teleport_rock` covers the lists; `net`'s teleport rock handler covers
keeping and dropping maps, each refusal, the trips by map and to a character,
and the rock used; `teleport_rocks` (integration) keeps Henesys, rocks back to
it, is refused the same map and a missing name, rocks to a watcher, and rocks
to Henesys again after a relog. The client's `teleport_rock` test covers the
lists, the checks on the current map and a typed name, and the refusals.
`net`'s Solomon handlers cover the writ and its refusals and how much
gacha EXP is taken; `solomon` (integration) uses a writ, is refused a
second, takes the EXP two levels at once, and takes the rest after a
relogin. The client's `gacha_exp` test covers the refusals, when the EXP may
be taken, and the tooltip.
`net`'s catch item handler covers what may be thrown, and `field_mobs` which
mobs a catch takes; `catch_items` (integration) throws the perfume at a Hog
out of reach, then beside it, and finds the Hog mount in its place. The
client's `catch_items` test covers the lines.
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

Personal shops are tested the same way: `net::personal_shop` covers the
tax, listing, purchase and sale checks, and the sale; `player_interaction`
covers parsing; `runtime::actor::field_shops` covers opening, seats, chat,
purchases, bans, the blacklist, and closing. `personal_shop` (integration)
has two tests:

- The first is refused a shop off the Free Market. It lists items and takes
  one back, opens, and sells to a visitor less tax. It refuses a purchase
  of more than is left, then sells out, which closes the shop. Both sides
  are saved after a relogin.
- The second bans a visitor, who can't come back. The owner gets the stock
  back on leaving and on logging out.
