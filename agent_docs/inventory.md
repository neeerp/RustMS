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
and the new look for the map. A sealed item whose date passes stays: its
seal is lifted instead (see Item Guards), and the client gets mode 11 ("%s's
seal has expired.") with those ids. Expiring skills (mode 13) aren't
implemented.

Items with `info/expireOnLogout` (127 of them, mostly event and PQ items)
go when their owner logs out: the client actor's cleanup takes them
(`net::items::drop_logout_items`) before the final save, but not on a channel
change. The client shows nothing. A world stopped without that cleanup
leaves in-game sessions behind; the restarted world clears them and deletes
those characters' expireOnLogout items (`drop_stale_logout_items` in
`runtime/src/world.rs`). Loading a character brings HP and MP down within
the maximum, since no buff survives a logout or channel change.

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
number (`saved_locations`; the scripts' own kinds, such as the Free Market's
0, are below 1000, and the smallest `returnMapQR` is 1056), and the event
map's exit script takes the player
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
refresh unless it simply failed. Legendary Spirit (1003, 10001003,
20001003) opens a dialog that scrolls an equip in the Equip tab: its
request names a positive slot and ends in 1. The server takes it, with the
same rules (pet equipment excepted from the cash refusal), only from a
player who has one of the three skills, and answers with Legendary Spirit
set in SHOW_SCROLL_EFFECT and no look refresh. Any refused dialog request
gets SHOW_SCROLL_EFFECT -1 to the scroller alone, which the dialog shows as
"You cannot use a Scroll with this item." HeavenClient's window is
`UIEnchantSkill` (v83 CUIEnchantSkill, UIWindow.img/EnchantSkill), opened by
the skill's key: an equip dropped in from the Equip tab (a cash one refused
unless it is a pet's), then a scroll from the Use tab, sent at once with a
White Scroll if the box (shown only to a player holding one) is ticked. Its
own answer goes to the window alone: -1 shows the refusal; otherwise it plays
the Enchant sound and its effect, then the chat line, the scroll sound and
glow, and closes on a cursed equip or waits for the next scroll. Cancel,
Close and Esc wait for the answer. For other scrolls, HeavenClient plays
the scroll sound and glow, and gives the scroller the v83 chat line. A sealed scroll
goes on nothing, and a sealed equip asks first whether to risk it (v83
0x114f, "Due to the potency of the scroll..."; No keeps the scroll).

### Megaphones

A megaphone (507xxxx) is used through USE_CASH_ITEM (`0x4F`: Cash slot, item
id, the megaphone's fields, tick), in `world/megaphone.rs`. The kind is the
item id's thousands digit, as the v83 client reads it: 1 and 8 are plain
megaphones, 2 super, 6 (and 5 with a last digit of 6 or more) item
megaphones, 7 triple; 5 up to 5 is Maple TV (below). The fields
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
there) gets only an empty stat change to unlock the client.

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

The item megaphone can't be opened while a dialog is open ("Not available").
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

#### Maple TV

The Maple TV items (5075000 to 5075005: Messenger, Star Messenger, Heart
Messenger, then the Megassenger of each) also come through USE_CASH_ITEM
(`world/maple_tv.rs`). The last digit is the kind: the Messengers (0 and 3)
start with a mode byte (3 with a receiver, 1 without), the Megassengers (3 to
5) with the ear byte, all but the Stars name a receiver (the Hearts must),
and five lines follow, the rows of the dialog's one wrapped box, at most 60
bytes each here. A heart without a receiver or a line too long gets only the
unlock; empty text is taken, as the v83 client sends it.

The world directory shows one message at a time to the sender's channel, or
to the world for a Megassenger with the ear ticked
(`runtime/src/actor/maple_tv.rs`). A message naming a receiver goes first to
the receiver's client actor (`ServerMessage::MapleTvPartner`), which adds the
receiver's look; a receiver who isn't online is refused with TV_RESULT
(`0x157`) 1, 2 ("You've entered the wrong user name."), the item kept. A
message that would wait over an hour is refused with 1, 3. Otherwise the item
goes at once and TV_RESULT 0 answers, and the message waits its turn.
SEND_TV (`0x155`: flag 3 with the receiver's look, else 1; the screen, 0
plain, 1 star, 2 heart; the sender's look; sender and receiver names; the five
lines; the seconds the line takes to clear; the receiver's look) puts it up,
and REMOVE_TV (`0x156`, empty) takes it down when its time is up: 30 s for
the Star Megassenger, 60 s for the Heart Megassenger and 15 s for the rest.
The client has no timer of its own, so these are Cosmic's. A player coming
online sees the message up if it reaches them. The client shows those seconds
only in its "This message will be sent to Maple TV in %d seconds." question.

v83 draws the TV only in maps whose `info` sets `MapleTV` and its anchors,
and no map in the v83 data does (report c056).

In HeavenClient, using one opens `UIMapleTv`. This is v83's modal dialog on
`UIWindow.img/MapleTV`: the To bar, one box that wraps onto five rows,
and the ear for a Megassenger. A Messenger names a receiver only with its
checkbox ticked, and unticking it empties the name. Enter does nothing, and
only OK sends. Once the dialog closes:

1. The client asks the 0xf79 question with the seconds from the last
   SEND_TV (`MapleTv::line_wait`; REMOVE_TV sets it back to 0).
2. A heart with no receiver is refused with 0xf78.
3. Text the filter won't take is refused with "Cursing is not allowed.".
   Otherwise its curse words are starred out and it is sent
   (`UseMapleTvPacket`).

TV_RESULT's codes 1 to 3 show StringPool 0xf75 to 0xf77
(`Util/MapleTv.h`, `tests/maple_tv.cpp`). SEND_TV is read only for that
wait, as there is no TV to draw.

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
which takes a press and a release on it and does nothing while a dialog
is open. It sends CLOSE_CHALKBOARD and logs "The chalkboard has closed.";
the board stays until CHALKBOARD `id, 0`. The board follows the player to
the next map, except to one where shops open, where it goes away with "You
cannot use the chalkboard here.". Double-clicking a chalkboard in the Cash
tab asks "Enter a note." (40 bytes; a blank note sends nothing), unless a
dialog is open ("Not available") or shops open on the map ("You can't use
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

### Miu Miu

Miu Miu the Traveling Merchant (5450000, sold in fives) brings Mu Mu's
shop (NPC 9090000) to the player. In the Cash tab the client (v83
FUN_00a0a63f, type 0x25) refuses it with the chat line "You can't use it
here in this map." where the map's `fieldLimit` has bit 0x40000 (the mini
dungeons, the party quest maps and many more), and with "Not available"
while a dialog is open; otherwise it waits for the last request and sends
USE_CASH_ITEM with no body. The server checks the same bit and the item,
and, unless the player is trading, keeps a shop or has an NPC window
open (a conversation, a shop, storage or Duey), takes one Miu Miu
(INVENTORY_OPERATION) and opens the shop as an NPC would (OPEN_NPC_SHOP).
Anything else gets only the unlock. Mu Mu's list is Cosmic's shop 1338, in
`game-data/shop_additions.toml`; the pack has no shop for him.

The Use tab's own path for classes 545 and 239 (FUN_00a10075, op 0x6F)
checks the same bit and sends the slot and item. No Use item of either
class exists in our data, so 0x6F is not routed.

### Cash mists

Passed Gas (5281000) and Floral Scent (5281001) leave a cloud around their
user for 60 seconds (`Item.nx Cash/0528.img`: `info/lt`, `info/rb`,
`info/time`, read into `ItemInfo.mist`). The client sends USE_CASH_ITEM with
nothing after the item, at once, unless the user's own cloud of that item
still lasts or they stand in anyone's; then it says "An Effect Item is
currently in use." (`MapMists::item_busy`, v83 FUN_00431866). The server
(`world/cash_mist.rs`) takes one and asks the field (`FieldRequest::ItemMist`),
which checks the same and, if it is free, adds a type 3 mist owned by the user
with the item as its skill (AFFECTED_AREA_CREATED to the map), and shows
the item's `effect` puff on the user: SHOW_ITEM_GAIN_INCHAT mode 20 to them,
SHOW_FOREIGN_EFFECT mode 20 to the others. The cloud does nothing to anyone.
ITEM_EMOTION (`0xE2`, the item id) makes the user's client pick one of the
item's `emotion` faces by weight and send it as FACE_EXPRESSION, which the
server allows from the last cash mist used, for the next face only
(`Player::mist_face`).
Report c008 has the client's detail. The puff's ConsumeEffect sound isn't in
this Sound.nx.

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

### Scripted items

A scripted item (243xxxx) names a script in `spec/script`
(`ConsumeSpec.script`). In the Use tab the client (v83 FUN_00a09b26, after
the Writs of Solomon and before the map's item checks) waits for the last
request and sends USE_SCRIPTED_ITEM (`0x4E`: tick, USE slot, item id). The
handler (`world/scripted_item.rs`) checks that the player is alive and the
item is in that slot, and the client actor runs `scripts::item` where the
player stands (the last position they moved to). Every answer ends with the
unlock. Two such items exist, both on the Mushroom Kingdom's way in, and
both work only within 210 px of one portal, taking every one held (as
Cosmic's item scripts do):

| Item | Script | Where | What it does |
|---|---|---|---|
| Killer Mushroom Spore (2430014, quest 2318's reward) | `killarmush` | `106020300`, by `obstacle` | Sets marker 32500, after which `obstacle` lets the player by without a Poison Spore; a pink line |
| Thorn Remover (2430015, given as quest 2324 starts) | `removethorns` | `106020400`, by `right00`, while 2324 is under way | Sets 2324's record to 1 (`Effect::QuestRecord`), which its `Check/1 infoex` asks for; the Secretary of Magic (1300004) then completes it (3300 EXP), which opens `gotocastle`; a pink line |

Elsewhere nothing happens and the item is kept. The pink lines are
placeholders (`item.mc_spore.used`, `item.mc_thorns.removed`).

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

### Item Tag and Magical Sandglasses

Both are dragged from the Cash tab onto a worn equip in the Equip window
(v83 FUN_004f4bb7, report c009) and come through USE_CASH_ITEM with the worn
slot (negative) after the item id. The Item Tag (5060000) prints its user's
name as the equip's owner (`world/item_tag.rs`); the client asks "Woulid like
to print your name on %s?" (sic) first, or says "The item already has a
printed name on it.." without sending. A Magical Sandglass (550xxxx, Item.nx
`info/addTime` seconds and `info/maxDays`, read into `ItemInfo.sandglass`)
adds its time to a time-limited equip. Nothing happens for a permanent,
cash or sealed equip (`FLAG_LOCK`); one with Character.nx `info/notExtend`
(the two medals 1140000 and 1141000) gets "Time limit extension is not
possible on this item.", and a date past now plus `maxDays` "You cannot
extend the effective date beyond %d days."; otherwise the client asks "When
used on %s,  the effective date will be extended by %s. Would you like to
use it?". The server checks the same, takes the item and re-sends the
equip through INVENTORY_OPERATION, or only unlocks the client.

The equip tooltip shows the owner over the name, in orange with a white
"'s", like v83's (FUN_008f49bc); the client's checks are in
`Character/Inventory/EquipCash.h`. Any other item with an owner says
"Specially made by" and the owner, centred under its expiry line
(`item_properties::shows_maker`), except greeting cards (430xxxx) and the
two wedding invitations, whose owner field means something else, and pets.
The inventory, storage, trading room, personal shop, Duey and NPC shop
tooltips all show an item's own expiry, flags and owner. An equip in
storage, a trade, a personal shop, a parcel or an Owl of Minerva result
shows the full equip tooltip with its own stats, upgrades and owner, the
way the packet sent it (`storage_contents::Stored`,
`EquipTooltip::set_stored`), not the equip as Character.nx makes it.

### Item Guards

The Item Guard (5060001) seals an item for good; the timed ones (5061000-3)
seal it for their Item.nx `info/protectTime` days (7, 30, 90, 365; read into
`ItemInfo.protect_time`). A seal is `FLAG_LOCK`, and a timed one also dates
the item. Double-clicking a guard in the Cash tab opens its window
(`UIItemProtector`, v83 CUIItemProtector, report c009). An item dragged in
from the Item window stays there, its icon drawn and its tooltip shown on
hover, and OK asks before anything goes: "%s Would you like to seal this
item?", or for a timed guard the date the seal would end ("... will be
sealed until" or, for a seal already timed, "... will extend the item seal
until"). Yes sends USE_CASH_ITEM with the item's tab and slot as two ints
after the item id. Only equipment, stars, bullets and 22500xx can be sealed,
not a cash item, one gone at logout, or one with a date of its own; a
lasting seal takes no more, and a timed one may be extended (from its end);
the Item Guard makes it lasting, which report c009 doesn't settle. Quest
items aren't kept out, though String.nx says they are. The client refuses those with "This item cannot be
sealed." and takes potions a pet uses with a warning. The server checks the
same (`world/item_guard.rs`), takes the guard and re-sends the item, or
only unlocks the client. Yes checks the guard and the item asked about are
still in place, the packet naming only slots. Worn equips are refused: v83
only takes drops from the Item window.

A sealed item can't be dropped, sold or traded, in a trade or a personal
shop (`Item::sealed`), sent by Duey, nor taken apart by the Maker. It can
be stored, as in v83 (report c013), and a timed seal in storage runs on
until the item comes out. HeavenClient says "Sealed items cannot be sold,
traded, or dropped." (v83's 0xd58, report c013) before anything else at
each place v83 does: a drop on the ground, into a trade or a personal shop,
onto the Maker, a sale in an NPC shop (the item stays listed), a
double-click on a USE item, a USE or cash item dropped on a worn equip, and
Vega's Start. It shows "Seal Released on" with a timed seal's date in the
tooltip. When a timed seal's date passes, the expiry sweep lifts the seal
and the date and re-sends the item.

### Dialogs that lock the game

v83 keeps one "current dialog" (0xbe7910, set by the dialog base 0x4eddb3):
a trade or personal shop, an NPC shop, storage, Duey, and the item
megaphone's, Item Guard's, Karma's, the hammer's, Vega's and the kite's
windows, among those HeavenClient has (`UI::dialog_open`, report c013). While one is open,
general chat says "You can't chat right now.", and trade invites, shop
permits and the cash items that open a window or ask a question say "Not
available": the item megaphone, meso sacks, AP and SP resets, the
chalkboard, the Item Guard, Karma, the hammer, Vega and the Quick Delivery
Coupon. Other cash items (pet food,
faces, weather, morphs, ...) are still used. A morph potion is refused
silently. Everything else is silently inert (report c015): the field takes
no key going down (skills, items, faces, macros, attack, jump, pickup, Up
into a portal), the arrows read as released so the player stops walking,
crouching or climbing, a queued macro waits, the field takes no click,
double-click or right-click (NPCs, shop balloons, kites, players), the skill
book's double-click uses nothing, an item dropped on the field stays, and
Gather and Sort do nothing (nor while dead). Windows, menu keys, Enter and
Esc still work.

### Scissors of Karma

The Scissors of Karma (5520000) let an untradeable item change hands once
(report c010). Only items with Item.nx or Character.nx `info/tradeAvailable`
(read into `ItemInfo.trade_available`) can be cut: 148 equips and a few
skill books (2280004-10, 2280013-16). Cutting sets the item's Karma flag
(0x10 on equipment, 0x02 on other items). A trade, a personal shop sale, a
drop, a Duey parcel or taking it out of storage then uses it up
(`Item::use_karma`), and equipment becomes untradeable again.

Double-clicking the scissors in the Cash tab opens their window
(`UIKarmaScissors`, v83 `UIWindow.img/KarmaScissors`). An item dragged in
from the Item window is checked in v83's order: a sealed one gets "You must
break the seal first.", one without `tradeAvailable` "This item cannot be
used.", and one already cut "This item has already been unlocked."
(`Character/Inventory/KarmaScissors.h`). Otherwise it stays in the window,
its icon drawn and its tooltip shown on hover. OK sends USE_CASH_ITEM with
the item's tab and slot, like an Item Guard, plays the cash item sound,
prints "You have used the Scissors of Karma, so 1 time of trading has been
enabled." and closes the window; OK with nothing in it does nothing. Unlike
v83, OK checks again that the scissors and the same item are still in
place. The server (`world/karma.rs`) refuses the same items, and also an
equip bound by a past trade or by wearing it (`FLAG_UNTRADEABLE`), which is
our choice; HeavenClient refuses that one too ("This item cannot be
used."), where v83 would take it. The server then takes the scissors and
re-sends the item, or only unlocks the client. The Item Guards' lock-out
also covers this window.

### Incubator

The Incubator (5060002) hatches a Pigmy Egg (4170000-4170009) (report
c035). The client's Incubator window sends USE_CASH_ITEM with the egg's
inventory type (4) and slot. The server (`world/incubator.rs`) takes the
incubator and one egg and gives one of 20 prizes, each as likely as the
next. The list is Cosmic's, since neither the client nor any server we can
read has the real one (a recommended default, `v83ref/lanes/QUESTIONS.md`).
It answers INCUBATOR_RESULT (`0x45`, item id and count; the client shows
them in the chat log and a popup) and then the inventory changes, which
unlock the client. Unless 1 Equip, 3 Use and 1 Etc slot are free once the
egg and the incubator are gone, the result is item 0 ("Your inventory is
full.") and the client is unlocked with nothing used. Anything else (the DS
Egg 4170012, a slot with no egg, an incubator not held) only unlocks it.
Nothing in our content gives eggs yet.

In HeavenClient, `UIIncubator` draws `UIClassic.nx` `UIWindow.img/Incubator`.
Double-clicking the incubator in the Cash tab opens it empty. Double-clicking
any 417 item in the Etc tab opens it with that item in, for the first
incubator in the Cash tab (or says one must be bought). An egg can also be
dragged in from the Item window; anything outside 4170000-4170009 is refused
there. OK asks the Yes/No from `Character/Inventory/Incubator.h`, then sends
`UseOnItemPacket` and closes. `IncubatorResultHandler` (opcode 69) prints the
gain line and opens a `UINpcTalk` with what was found.

### Reward boxes

Use items with a `reward` node beside `info` (2022309-2022618: the Golden
Pig's eggs, the King Pepe boxes and the like; report c037) hold a list of
entries `{item, count, prob, period}`, which game-data reads into
`ItemInfo::reward`. The v83 client sends them as USE_ITEM_REWARD (`0x70`,
USE slot and item id) instead of USE_ITEM and never reads the entries. The
server (`world/use_item_reward.rs`) picks one entry with a single roll
against the sum of the `prob` weights, takes the box and gives the entry,
expiring after `period` minutes when it has one (a recommended default,
`v83ref/lanes/QUESTIONS.md`), then answers the inventory changes and the
chat gain line. If the entry doesn't fit, or is one of a kind and already
held, it sends "Your inventory is full." and keeps the box. Anything else
(no box in that slot, no reward node, dead, a map that bars the item) only
unlocks the client.

The nine cash bags (5530000-5530008, each one entry: DS Eggs, a medal, a
Golden Compass, Gaga potions or Elixirs; report c040) are the same boxes in
the Cash tab. v83 sends them as USE_CASH_ITEM (`0x4F`, `i16 slot, i32 item,
i32 tick`, its cash type 0x45), never as `0x70`, and the server opens them
with the same `open_box` from the Cash tab. That path has no map check. A
reward box bound to a key or quickslot is sent as `0x70`, the same as a
double-click; cash bags can't be bound. `spec/onlyPickup` (the Dojo buffs
and the King Pepe boxes) has no reader in the v83 client, and the boxes open
from the bag as any other.

An entry may also name an `Effect` (an animation path such as
`Effect/BasicEff/Event1/Best`) and a `worldMsg` (report c038). On a
successful open the server sends user effect mode 14 (`u8 14, i32 item,
u8 has_path, [str path]`; `build_reward_item_effect`) to the opener as
SHOW_ITEM_GAIN_INCHAT and to the rest of the map as SHOW_FOREIGN_EFFECT
with the opener's id first. The item is the prize, which only picks the use
sound. Item.nx writes the path without the `.img` of its image node, so the
server adds it after the image's name (`Effect/BasicEff.img/Event1/Best`),
and an empty path is sent as none. When the
entry has a `worldMsg`, the whole world gets it as a BROADCAST_MSG type 6
item line, with `/name` replaced by the opener's name and `/item` by the
prize's name from String.nx (`ItemNames::item`); if the name can't be found
nothing is sent.

In HeavenClient, `ItemData::has_reward` says whether the item has a
`reward` node, and a double-click on such a Use item sends
`UseItemRewardPacket` (USE_WAIT) right after the sealed check, before any
other kind of Use item is tried, as v83 does (0x4f0372); a key or
quickslot bound to one sends the same (`Player::use_item`, 0x94f882), from
the first slot holding it that isn't sealed (`Inventory::find_unsealed`,
which every item key uses). A double-click on a cash bag sends
`UseCashItemPacket` and plays `UseShopItem` (`use_cash_bag`). User effect 14
(local in `ShowItemGainInChatHandler`, foreign in `ShowForeignEffectHandler`)
calls `Stage::show_reward_effect`: the prize's use sound (`Sound(itemid)`),
and the path once in `ground_effects` where the opener stood, not following
them (0x9382b0). `Stage::effect_node` finds the path's node, as user effect
23 does too: the NX file named first (`UI`, `Effect` or `Map`), then the
node, retried with the image's `.img` put back (`field_effect::with_image`),
since Item.nx writes reward paths without it. A world message's item link
(BROADCAST_MSG type 6) isn't drawn; the line shows as plain blue text.

### Gachapon

A Gachapon Ticket (5220000, from the Cash Shop) does nothing when
double-clicked; only a machine takes it (report c058). The machines are the
NPCs `Map.nx` places in ten towns (`net::gachapon::MACHINES`: 9100100
Henesys to 9100117 Nautilus Harbor). v83 kept the prize lists on the server,
so they come from the pack's `gachapon.json` or our stand-ins
(`agent_docs/server-content.md`).

When a ticket holder clicks a machine, the script
(`npc_scripts::gachapon_machine`) rolls the prize at once
(`gachapon::roll`): 90 in 100 common, 8 uncommon, 2 rare, then one of the
tier's prizes; a tier with none gives way to the one below. Yes applies
`Effect::Gachapon`: the ticket goes, the prize comes, and both inventory
changes and their chat gain lines are sent together, then the next page
names the prize. A ticket holder with any of the Equip, Use, Setup or Etc
tabs full gets the "inventory full" page before anything is rolled: a
prize turned away with the ticket kept would be a free roll, and would show
which tab the prize was for. If the prize still doesn't fit when yes is
pressed, the ticket stays and the same page shows. Without a ticket the
machine only explains itself in a menu. `RUSTMS_GACHAPON_SEED` makes every roll the same (the
harness uses 10, an uncommon prize).

An uncommon or rare prize is announced to the whole world (a recommended
default, `v83ref/lanes/QUESTIONS.md`) as BROADCAST_MSG type 11
(`build_gachapon_message`): `str "<name> : got a(n)", i32 channel, str town`
and the item as in an inventory. The v83 client shows it as a chat line,
"<name> got a(n)  :  <item>  from the <town> Gachapon! Congrats!" (StringPool
0x137d), black text on a green bar (chat type 0x12), with the item's tooltip
on hover; a channel of -1 would name the Cash Shop Surprise. HeavenClient
reads it in `MessagingHandlers.cpp` (case 11), writes the line with
`Megaphones::gachapon` and shows it as `UIChatBar::GACHAPON`, without the
tooltip. The Remote Gachapon (5451000, USE_REMOTE_GACHAPON `0x74`) isn't
handled and isn't sold.

### Item Maker

The Maker skill (`net::skills::maker_skill`: `base*10000+1007`, base being
`job/1000*1000`, or 2001 for Evan), which Meren's lessons in Magatia teach
(quests 6029, 6033 and 6036; see [Quests](./quests.md)), opens the
client's Maker window, which sends ITEM_MAKE (`0x71`,
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
The random make ("???") takes up to six different items, asking "How many
would you like to apply?" (1 to what the player has) for each and showing
"chosen/???", plus any stimulator and up to four gems of any kind; a
seventh item or a second of the same is refused with v83's notices, and a
click takes any of them back out. Create is enabled once one item is in.
After the gauge it always ends in "Production failed." and sends nothing,
as v83 does: its matcher (FUN_00822ce1) only looks at recipes with `hide`,
and there are none.
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

Not done in the window: the drop lists can't be opened while
the gauge fills, which v83 allows, and they don't take the arrow keys and
Enter, which HC sends to the player. The window opens centred, where v83 remembers its
place, and is dragged by its top 20 px, a height not checked against v83.

### NPC item makers

Nine NPCs make equipment from materials and mesos: Serryl (1091003, in the
Nautilus's hallway) makes pirates' knucklers, guns and gloves, Neve
(2010003, Orbis Park) makes and upgrades gloves for each class, Mos
(2080000, Leafre) forges the Dragon weapons, Francois (1032002, Ellinia)
makes magicians' gloves, wands and staffs and upgrades their gloves and
the Bronze Pride, Chrishrama (1061000, Sleepywood) and Gordon (2020002, El
Nath Market) make shoes for each class, Sarah (2040020) and Tara
(2040021), in their house in Ludibrium (220000303), make gloves and shoes,
and Rydole (2040022, the Toy Factory's Aparatus Room) makes weapons from
their manuals. `net::item_makers` keeps their recipes, with items,
materials and prices taken from Cosmic's scripts. The conversation is a
menu of kinds, then a menu of items, then a yes/no question that lists
each material and the price. A yes is one transactional step: `SpendMeso`,
a `TakeItem` for each material, then the item. If anything is missing the
maker says `maker.short` and takes nothing; if the item doesn't fit,
`maker.full`. Upgrades take the glove they upgrade as a material.

Mos, Sarah, Tara and Rydole also offer each class "with a stimulator"
(`stimulator_choices`): it takes the item kind's stimulator
(`item_makers::stimulator`: 4130000 for gloves, 4130001 for shoes,
4130002-4130017 for weapons) as well, and gives the item through
`Effect::GiveVariedItem`, with its stats varied like a drop's
(`items::give_dropped`). One time in ten (`STIMULATOR_LOSS`) the item is
lost and only the materials are taken. The roll is made when the
conversation opens. Tara's and Rydole's stimulated items are their plain
ones, except that Rydole's thieves' menu also upgrades claws; Sarah's are
upgrades of her plain gloves, and only made with a stimulator. Sarah's and
Tara's prices are a tenth below the other makers'. While quest 7301 or
7303 is started, Mos also makes Cornian's Dagger (4001078). No sourced
text exists for these NPCs, so all of it is placeholder (`maker.*`).

### Refiners

Refiners make plates, jewels, crystals, materials and arrows in batches.
`net::refiners` keeps their tables: a `Refine` is an item, how many come
from a batch (15 Screws, a bundle of 800 to 1000 arrows), the materials
and the mesos per batch. Ids, counts and prices come from Cosmic's
scripts (report `v83ref/reports/a103-refiners.md`). The conversation is a
menu of kinds, then a menu of items, then a number prompt (NPC talk type
3, `Page::AskNumber`, 1 to 100 with 1 selected), then a yes/no question
listing the totals. Arrows skip the number prompt: one bundle at a time.
The yes is one transactional step: the item first, then the mesos, then
each material, so a full inventory is told (`maker.full`) before short
mesos or materials (`maker.short`). Nothing is taken unless all of it applies.

- Mr. Thunder (1022003, Perion) refines mineral and jewel ore, and
  upgrades helmets and shields one at a time, as the item makers do.
- Chris (1052003, Kerning City) refines mineral and jewel ore, makes Steel
  Plates from 100 Iron Hog's Metal Hoofs each, and upgrades the Gigantic
  claws.
- Vogen (2020000, El Nath) refines ore, Moon and Star Rocks, crystals
  (Dark Crystals at 1,000,000 mesos each), materials and arrows.
- Pi (2040016, Ludibrium) offers the same except the Dark Crystal, at 90%
  of the price. Muhamad (2100001, Ariant) asks first, then refines mineral
  ore (and Lidium), jewel ore and crystals, also at 90%. The 90% is
  Cosmic's, with no v83 source (`v83ref/lanes/QUESTIONS.md`).
- Spiruna (2032001, Orbis), once quest 3034 is done, asks how many Dark
  Crystals to refine at 500,000 mesos each, and refines them at once.
- The five in Altaire Camp (Perzen, Loha, Sion, Duru and Tess, 2131001 to
  2131007) each take 100 of one item for a Perfect Pitch (4310000). The
  number prompt offers as many as the player's items buy, at most 300.

- Mr. Smith (1022004, Perion) makes and upgrades warriors' gloves, Vicious
  (1012002, Henesys) bows, crossbows and bowmen's gloves, and JM (1052002,
  Kerning City) thieves' gloves and claws, all one at a time; all three
  make materials in batches, and Vicious arrow bundles too.

All of their text is placeholder (`refiner.*`, `altaire.*`).

## NPC shops

Clicking an NPC that keeps a shop (`content::shops()`, from the pack's
`shops.json` plus the shops it lacks in `game-data/shop_additions.toml`;
see `agent_docs/server-content.md`) opens it instead of its speech: `net::shop::Shop::open` builds
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

## Duey

Duey (9010009, in each town's post office) carries mesos and one stack of
an item to another character, online or not (`v83ref` report b028). A
parcel is a `parcels` row with the receiver's id, the sender's name, the
mesos, the item columns as storage keeps them, and for a quick delivery its
message. Equipment stats go in `parcel_equipment_stats`. Both go with the
receiving character. `db::parcel` loads, sends, claims, deletes and announces
parcels. `net::duey` holds the rules and `ClientActor` (`client/duey.rs`)
holds which window is open.

PARCEL (`0x142`) starts with a mode byte:

- **Open** (`0x08`): a receive-only flag, then a count byte and the parcels
  waiting, then a count byte and those that expired. A parcel is a fixed
  record (id, a 13-byte sender, mesos, a FILETIME, a has-message int, a
  200-byte message, quick, has-item) and then the item without a position.
- **Removed** (`0x17`): id and why, 3 deleted or 4 claimed.
- **Added** (`0x18`): one parcel, which joins an open window. A client
  without the window ignores it.
- **Arrived** (`0x19`): the sender's name and quick, for an online receiver.
- **Quick send** (`0x1a`): opens the window from a coupon.
- **Waiting** (`0x1b`): quick, on entering the game.
- **Results** (`0x0a`..`0x16`): `net::duey::result`. Each one also lets
  the window take requests again.

The client throws if an open (`0x08` or `0x1a`) arrives while its window is
up, so the server keeps `duey: Option<DueyWindow>` (Duey's own window, a
receive-only one, or a quick send). It is set on open, so a second open
sends nothing. DUEY_ACTION op 7 clears it, and so do a map change and
entering the Cash Shop: the client destroys the window on any new field or
stage without sending op 7 (`v83ref` report b029). The window isn't modal,
so the player may walk and talk to NPCs while it is up. The client uses a coupon only with no window up, so using one
always opens a quick send. Duey requests in the Cash Shop are ignored. A
send, claim or delete that doesn't fit the open window gets `0x13`, since
the window waits for an answer. With no window open such a request gets
nothing. The client's FILETIME is a day short of the parcel's expiry, which
is 30 days after it was sent. The window counts the days left and calls a
parcel 30 or more days off still "on its way". The day spare keeps a new
parcel claimable on a client whose clock is behind.

DUEY_ACTION (`0x41`) starts with an op byte:

- **Open** (0): from the notifier of a quick delivery. It opens a
  receive-only window. Talking to Duey opens his own, which sends too.
- **Send** (2: bag, slot, quantity, mesos, receiver, quick; if quick, a
  message and the coupon's slot): needs Duey's own window, which has both
  send tabs, or for a quick send the coupon's window. A
  normal send costs 5000 plus a tax on the mesos (`net::duey::tax`: nothing
  below 100,000, then 0.8% rising to 6%). A quick send uses up a Quick
  Delivery Coupon (5330000) found in the Cash bag instead of the 5000. Its
  message is at most 100 characters. Refusals:
  - an unknown receiver gets `0x0c`, and the sender or another character
    of the same account `0x0d`;
  - a receiver with 10 parcels waiting gets `0x0e`, and one with the same
    one-of-a-kind item waiting `0x10`;
  - at level 15 or below, more than 1,000,000 mesos in one send gets
    `0x11`;
  - too few mesos gets `0x0a`;
  - anything else wrong (bad slot or quantity, an item that can't go,
    nothing chosen, no coupon) gets `0x0b`.
  What may go is what storage takes (no cash items), but not items that
  expire or are bound to an account, a party quest or a session. Stars,
  bullets and equipment go whole. Karma is used up on sending.
- **Claim** (4: id) and **delete** (5: id): an id that isn't waiting gets
  `0x13`. A claim may also get a full bag `0x15`, or `0x16` for a
  one-of-a-kind item the player has.
- **Close** (7).

The cost leaves the player, and the claimed parcel comes to them, in one
transaction with saving the player (`db::parcel::send`, `claim`). The send
takes an advisory lock on the receiver, so two senders can't both fill the
last place. It doesn't lock the receiver's row, so two characters sending to
each other at once can't deadlock. If saving fails or is refused, the player
is put back as they were. A failed save gets `0x13` and the player stays
connected. Using a
coupon opens a quick send (`0x1a`); with a window already open it gets an
empty stat update.

Parcels expire lazily. Opening the window deletes any older than 30 days
and lists them as expired. On entering the game the player gets `0x1b` if
any parcel hasn't been announced, and they are then all marked announced.
An online receiver is told at once through the directory, and the parcel is
marked announced. They get `0x18`, then `0x19`, which shows the notifier or
brings an open window to the front. Expired parcels are deleted, not
returned to their sender. Not done: the GM refusal and a per-day limit for
low levels.

The client's window is `UIDuey`, from v83's CParcelDlg (layout in `v83ref`
reports b028 and b030). PARCEL is handled by `ParcelHandler`, and
`Net/Handlers/Parcels.h` reads the records and holds the status and result
texts. Mode `0x08` opens the window unless one is open. The list shows three
of up to ten parcels with a scrollbar: the sender and "Delivering", "%d
day(s)" or "Will expire soon" (red). v83 also writes a Korean word for a
quick parcel, which HC leaves out. Choosing a row shows its sender, mesos,
item with its level limit and count, and its message where Duey's words
were; an empty message reads "No information entered.". The item's tooltip
shows over v83's box, 16 above the icon. RECEIVE refuses a parcel still
being delivered, and DELETE asks first; either waits for the answer before
another request. Each expired parcel gets its own notice, one after
another. The close button and Esc send op 7. A map change drops the window
without telling the server, as v83 does.

Duey's own window has three tabs (`Basic.img/Tab2` pieces at (6,23), as wide
as their `Delivery/Tab` labels): PA STORAGE, SEND PA and QUICK DELIVERY
(report b032). With nothing waiting it opens on SEND PA. A send panel takes
an item dropped from the inventory (`Icon::drop_on_duey`): a sealed item or
one that can't go (`item_drop::sendable`) gets a notice, stars, bullets and
equipment go whole, and other stacks ask how many. Clicking the item asks
before taking it back. Clicking the mesos asks how many, up to
`parcels::most_mesos` (what the purse less 5000 covers with the tax, at most
100,000,000); the normal panel refuses a purse of 5000 or less, and a
character of level 15 or below gets the `0x11` notice over 1,000,000. The
mesos show in the shop's price colours. The fee table below the panel folds
with the arrow button (BtMax2 while it's open, BtMin2 while folded), and a
folded panel lets clicks through below it. SEND asks to confirm the fee
(5000 plus `parcels::tax`, by v83's doubles, so a mesos amount may show one
less than the server takes). The quick panel needs a Quick Delivery Coupon
and a message of up to 100 letters, filtered as chat is, and cuts the item's
name to fit. Return sends from SEND PA. While a send waits for its answer,
the buttons, fields and tabs are off; `0x12` empties both panels.

Modes `0x19` and `0x1b` bring an open window to the front (`UI::raise`);
otherwise they show v83's type-10 notice, a `PARCEL` card in
`UIInviteCard`: `FadeYesNo/backgrnd4` with `Basic.img/icon/delivery`, and
the text from `parcels::arrival`, cut at the card's edge. A plain parcel's
card fades out after six seconds. A quick one's stays until answered, and its
O sends op 0, which opens the receive-only window. A new card replaces the
one before. Mode `0x1a` opens the coupon's window: the quick panel alone,
which closes on `0x12`. Using a Quick Delivery Coupon (533xxxx) from the Cash
tab sends USE_CASH_ITEM unless a dialog is open (Duey's among them; see
"Dialogs that lock the game"), which gets "Not available" in the chat log. Only a send uses the coupon up.

## Rock, Paper, Scissors

The Rock, Paper, Scissor Admin (9000019, `info/rpsGame`) runs a minigame
(report b038 in `v83ref`). No map in v83's Map.nx places him, so the server
does: `net::npc_data::extra_npcs` adds NPCs to a field's own list, and puts
him on Henesys's lower street by Spinel. Talking to him sends RPS_GAME
(0x138) mode 8 with his id, which opens the client's window, and the
ClientActor holds a `net::rps::RpsGame` until the window closes or the map
changes. The window sends RPS_ACTION (0x88), a mode byte: 0 Start, 1 a
hand (one more byte: 0 rock, 1 paper, 2 scissors), 2 out of time, 3
Continue, 4 Exit, 5 Retry. Requests that don't fit the game are ignored.

- Start and Retry cost 1000 mesos and need a free etc. slot; without them
  the reply is mode 6 or 7 and the window shows Start again. Otherwise mode
  9 starts the first round.
- A hand gets mode 11: the admin's random hand and the streak, signed. A
  draw keeps the streak, and the client plays the round again by itself. A
  loss sends -1 and ends the game. The client calls the loss consoled (500
  mesos) only while the streak it last heard is 0, which a new game doesn't
  reset, only a new window or a refusal; the server keeps that streak too
  and pays to match. A win adds one; after it, Continue (mode 12) risks the streak on
  another round, and Exit keeps it.
- Running out of time (mode 10) ends the game with nothing.
- Exit closes the window (mode 13). After a win it gives the certificate for
  the streak, 4031332 for one win up to 4031341 for ten. The tenth win ends
  the game and gives its certificate at once.

The server's game stays until the next talk, which starts a new one, in
case a client closes the window without a packet.

The client's window is `UIRpsGame` (reports b038 and b041), with the game
itself in `Character/RpsGame`, which `rps_game_tests` covers:

- Mode 8 shows the admin's notice as a local NPC dialog first, then opens
  the window centred on the screen. It isn't modal, its top 15 pixels drag
  it, and Exit is the only way out: there is no X, and Esc does nothing.
- The player's hand and the admin's are drawn from the start, rock until
  chosen. Start, Continue and Retry swap in the middle; the three hands show
  while a round waits for a choice, and the player's 30 seconds run out
  into RPS_ACTION 2, with MiniGame.img's Timer in the last ten.
- The admin's hand spins with Reel every 120 ms. After the result it slows
  by a fifth a step until it reaches his hand at 720 ms or more. The banner
  is put up then but shows a second later with the jingle and the avatar's
  face (smile, troubled or cry), and two seconds after that the next state
  comes: a draw plays the round again without asking the server.
- The line under the avatars is centred if it fits in 270 pixels. A wider
  one starts halfway and scrolls left a pixel a frame. It is picked again
  when the middle button changes or shows, and every frame while Start or
  Continue is hidden, which is when it counts down. So the countdown, wider
  than 270 in Arial 12, shows from halfway and never scrolls, and after
  Retry the round keeps Restart's line, as v83 does.
- Refusals 6 and 7 show v83's notices over the window, which shows Start
  again.

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

The blacklist is the User List's Blacklist tab (`UIUserList`, reports
b089 and b090), one list for every character on the machine. HeavenClient
keeps it in the `BlackList` setting, comma separated in the order added;
v83 keeps it in the registry. Add asks for a name (4 to 12 letters) and
refuses one that fails the name filter or a forbidden word (SP `0x2d7`),
the player's own (`0x2d6`), or one listed already in any case (`0x2d8`).
It holds 26, two to a row in byte order, and Delete removes the picked
name with no confirmation. While the player's own shop is open, adding
and deleting also send `0x1f` and `0x20`; a hired merchant's never do.

The client also turns away what a listed name sends it (report b091),
matching the name letter for letter, as the Game Option does when it
blocks the same thing; the player who listed the name sees nothing, and
the sender hears what the server makes of the refusal. A whisper gets
`0x78 {0x22, sender}` unless it is from an admin. Buddy, guild and
alliance chat is dropped; party chat is not. Invitations are declined: a
trade (`MiniRoomDeclinePacket` reason 4), a party (`0x15`, or `0x16`
while a party card shows), a guild (`0x35`, or `0x36`), a family, and a
messenger. A party search's invitation is
dropped without an answer. v83 doesn't check a shop's visitors against
the list; the server does that with the copy the shop got at its opening.

When the last bundle sells, the shop closes: the visitors get LEAVE reason
`0xe` and the owner `{0xa, 0, 0xe}`. The owner leaving (`0xa`), changing
maps or channels, entering the Cash Shop, or logging out also closes it.
The owner gets the unsold stock back, and the visitors get LEAVE reason 3.

### Owl of Minerva

The Owl of Minerva (2310000, the only USE item of class 231) searches the
open personal shops of every channel for an item. The Cash Owl (5230000,
the only Cash item of class 523) does the same. The client lets it be used
only on the Free Market maps (910000000 to 910000022). `net::owl` holds the
rules and `client/owl.rs` the flow.

1. **Opening the search window** sends `0x42 {5}`. The answer is OWL_RESULT
   (`0x46`) mode 7: the 10 items searched for most in the world since it
   started (`DirectoryMessage::OwlPopular`).
2. **A search** (`0x53`: USE slot, owl id, item id, order 0 cheapest first or
   1 dearest, tick; the Cash Owl sends the same fields as USE_CASH_ITEM
   `0x4F` type 0x1d, from its Cash slot) asks every running market room (910000001 to
   910000022, instance 0, all channels, `FieldRegistry::running`) for its
   open shops that still have the item (`FieldMessage::ShopSearch`), waiting
   at most a second. Up to 200 rows go back in mode 6: owner, map, title,
   bundles, per bundle, price, room id, 0-based channel and item kind, with
   the whole item after an equip. The Owl is used up only if a row turns
   up; no rows makes the client say it found nothing. A search by a dead
   player, off the Free Market, in a room, or without the Owl in that slot
   only unlocks the client.
3. **GO** on a row (`0x43`: room id, map) gets OWL_GO_RESULT (`0x47`): 0 go,
   1 closed (no such open shop on the player's channel), 2 full, 3 busy (in
   a room, or the field did not answer), 4 dead, `0x11` banned. On 0 the
   player is warped to the shop's map if needed and visits the shop as if
   they had clicked its balloon.

**The client.** Double-clicking either Owl in the Free Market opens
`UIItemSearch` (v83 FUN_008a0c7b, report c026), which sends `0x42` as it
opens. Its two GO buttons slide a panel out from under its right edge (a
pixel a millisecond, and back the same way) with the 10 items
searched for most (five a page, icon and name; a click asks at once) or a
category tree (Eqp, Use, Etc and Setup, each opening its own categories by
item id range). The search box lists, ten a page, every item whose String
name has the text, ignoring case and spaces (`Data/ItemNames.h`, shared
with the pet's item search). Choosing a row and the panel's SEARCH asks
"Would you like to search for [name]?" (SP 0xe2d, with the order picked by
the panel's SORT BY PRICE box, ticked for the dearest first), then sends `0x53`
(or `0x4F` for the Cash Owl) behind the client's half-second
request gate and closes the window. An empty answer says "Unable to find
the item you have entered."; otherwise `UIItemSearchResult` opens with
eight rows a page. GO is offered only on rows on the player's own channel
(greyed out elsewhere); the others say "Can't Go", with a tooltip naming the
room and channel. The owner, title, column headings and item info have
tooltips too. GO's answer 0 closes the window, 1 marks the row "Store Closed", and
each code shows its notice. Names can repeat: in this data 4000000 is "Blue
Snail Shell" and 4000019 is "Snail Shell".

Not decoded, and approximated: which way the order box's tick means, the
Setup and Summon ranges, the tree's layout once a part is open, and where
the result window opens (centred). A panel's buttons appear once it is all
the way out, and switching panels starts the new one from under the window
instead of sliding both at once. The text colours of the result rows are
not decoded (all black, the status red). Equip tooltips show the listed
equip's own stats. GO after leaving the Free Market gets
answer 1 from the server, so the row shows "Store Closed".

## Hired merchants

A player over level 15 with a hired merchant permit (`503xxxx`, in the
cash bag) can set up a merchant (mini room type 5) on a Free Market map.
It stays and sells after they leave the room, the map or the game. The
wire facts are in v83ref report 165. Unlike a personal shop, the
database holds the merchant (`db::merchant`), and the field never touches
the database itself:

- `runtime::actor::field_merchants::Merchants` is the field's copy, to
  show and sell from. Fields read their merchants back when they start.
  Merchants stand only on the main copy of a map (`instance_id == 0`) and
  only where Map.nx allows shops. A merchant's room id is
  `0x4000_0000 | owner id`.
- The owner's `ClientActor` (`client/hired_merchant.rs`) holds the
  merchant while setting it up, or in for maintenance
  (`MerchantRoom::Setup` and `Maintenance`), and writes every change.

The flow:

1. **Check** (ENTRUSTED_SHOP_REQUEST `0x3F`). The client asks before it
   asks for a title. The answer is ENTRUSTED_SHOP_CHECK_RESULT (`0x32`):
   7 to go on; 8 with `{i32 map, u8 channel}` (0-based) if the player's
   merchant is open; `0xf` if one is closed or left in setup with items
   (Fredrick holds it); `0xb` otherwise. Reasons for `0xb` are: in a room,
   no permit, level 15 or below, not a Free Market map, or too close
   (150 px) to another merchant or a personal shop. A merchant left empty
   in setup by a crash is dropped.
2. **Create** (mode 0, type 5: title, a byte, permit slot, permit id).
   The same checks run again, then the owner gets the empty room
   (`{5, 5, 4, 0, …}`). Nobody else sees it yet, but the field keeps its
   spot, so no other merchant or shop goes up there meanwhile.
3. **Put item** (`0x21`, as `0x16`) and **take back** (`0x26`: index).
   Each change is written, then the owner gets the inventory operation
   and the item list (`0x19`: the merchant's mesos, then the rows).
4. **Open**: the blacklist (`0x1e`), then `{0xb, 1}`, as for a shop. The
   client leaves the room by itself. The merchant shows as
   SPAWN_HIRED_MERCHANT (`0x109`), and UPDATE (`0x10B`) changes its
   balloon. Leaving the room (`0xa`) with something listed also opens
   it; leaving it empty drops it. The permit is not used up
   (`v83ref/lanes/QUESTIONS.md`).
5. **Visit** (4: room id) and **buy** (`0x22`: index, bundles). The field
   reserves the bundles; the buyer's actor checks and writes the sale
   (`db::merchant::sell`) and tells the field, which then credits the
   merchant with the price less the personal shop tax. A reservation not
   settled within 30 seconds is dropped, and the field reads the merchant
   again.
6. **Maintenance**: the owner visits their own merchant. Visitors get
   LEAVE `0x11`, and the owner comes in once every purchase is settled,
   with the sales and the takings. In maintenance the owner can add items,
   take mesos (`0x2b`, answered `0x2c`), organize (`0x28`: strike the
   sold-out rows and take the mesos, answered `{0x28, i32 meso}`), see
   the visitors (`0x2e`) and the blacklist (`0x2f`, `0x30` adds, `0x31`
   removes). `0x27` or leaving opens the merchant again; leaving with
   nothing left in it (the client sends `0xa`, not `0x29`) closes it.
   A sale settled while the owner is inside updates their copy.
7. **Close** (`0x29`): the merchant goes, and the owner gets its items and
   mesos. `0x2a` answers 0 for everything back, 1 if the mesos won't fit
   (nothing comes back), 2 for a one-of-a-kind item they have, or 3 for no
   room (only the mesos come back in both cases). `db::merchant::close`
   locks the merchant's row and gives up if it no longer matches the
   owner's copy; the merchant then opens again as it was.

After 24 hours open, the field closes the merchant: visitors get LEAVE
`0x12`, and what it holds stays in the database for Fredrick.

### Fredrick

Fredrick (`9030000`, Free Market Entrance) keeps what a closed merchant
holds, and what a crash left in one in setup. The check answers `0xf`
for such a merchant, which sends the player to him
(`client/store_bank.rs`, `net::store_bank`):

- **Talk.** STORE_BANK (`0x137`) mode `0x23` opens the Store Bank:
  `i32 npc`, then storage's contents (slots, flags, mesos, the bags).
  With nothing kept, mode `0x25` carries `{i32 npc, i32 999999999, u8
  0xff}`. If the merchant is open, `0x25` carries its map and its 0-based
  channel instead, and the client says where.
- **Get All** (STORE_BANK_REQUEST `0x40` mode `0x1a`). The server reads
  the merchant again and answers `{0x24, i32 days, i32 fee}`. The fee is
  1% a day of the merchant's mesos and the listed price of its unsold
  bundles, up to all of it (the contract, StringPool `0xdaa`). The days
  are the whole days since it closed; a merchant left in setup has none.
- **Yes** (mode `0x1b`). Everything comes back at once, less the fee, or
  nothing does. STORE_BANK_MESSAGE (`0x136`) answers `0x1e` (retrieved),
  `0x1f` (too many mesos), `0x20` (one-of-a-kind item), `0x21` (can't
  pay; the player's mesos and the merchant's together pay the fee) or
  `0x22` (no room). `db::merchant::retrieve` drops the merchant and saves
  the player in one transaction. It gives up if the merchant changed since
  Get All; the client then gets mode `0x26` ("currently unavailable").
- **Close** (mode `0x1c`) forgets the window.

GMS gave away what was kept past 100 days; here the fee just stops at
100%.

HeavenClient's `UIStoreBank` is v83's dialog (`UIWindow.img/StoreBank`,
centred). It lists the kept items in the order the server sent them,
equips first and cash last, five rows at a time. Each row shows the
item's name, its count (use, setup and etc items only) and the cash
mark. The mesos show at the bottom. There is no tooltip. Get All waits
for its answer before it can be pressed again. The fee question is
StringPool `0xda2`, or `0xda3` when the fee is nothing. After `0x1e` the
list empties and Get All is disabled until the window opens again. Mode
`0x25` opens an NPC talk window: Fredrick says where the merchant is open
(`0xda0`, the channel's name and the Free Market room), or that he keeps
nothing (`0xd9f`). Mode `0x26` is a notice, and it also lets a waiting
Get All go; v83 leaves one waiting. Esc and Exit send mode `0x1c`
(`Net/Handlers/StoreBank.h`, `StoreBankHandler`).

### Store Remote Controller

The Store Remote Controller (`547xxxx`, in the cash bag) lets the owner
manage their merchant from anywhere on its channel (report b088). The
item stays, and an expired one gets enter error `0x18`.

- **Use** (REMOTE_SHOP_REQUEST `0x3B`: `i16 slot`). The client sends it
  only when no mini room is open. The answer is ENTRUSTED_SHOP_CHECK_RESULT:
  - `0x11 {i32 room, i16 slot, 8 bytes cash serial}` if the merchant is
    open on this channel;
  - `0x10 {i32 map, u8 channel}` if it is open on another one (the
    client offers to change channel);
  - `0x10` with channel `0xff` if there is none (StringPool `0xd89`).
- **Visit** (PLAYER_INTERACTION `0xe`: `{4, 5, i32 birthday, i32 room,
  u8 1, i16 slot, 8 bytes cash serial}`). Mode `0xe` wraps a request in
  the birthday code (any real date, `cash_shop::birthday_code`); a bad
  code is dropped. The server checks the controller and the merchant
  again, then lets the owner in for maintenance as if they stood by it.
  The owner's actor sends its merchant requests to the merchant's field
  (`ClientActor::merchant_field`), and the field answers through the
  handle it was given (`remote_owners`).
- **Leaving the map** closes the window with LEAVE `0x13` (StringPool
  `0xd8d`); the merchant opens again as usual.

HeavenClient asks before it visits: StringPool `0xd8b` (Yes/No), then
`0x33a` for the birthday code. Code `0x10` shows `0xd90` with the
channel's name, and Yes changes channel; v83 names the channel from a
table by the byte, while HeavenClient reads the world's channel name.
While the player is in alert stance, `0x11` says to catch their breath.

Permits and controllers are sold in the Cash Shop.

Not done yet: the blacklist's storage (it lasts as long as the merchant
stays in memory).

With a cash item listed, the client asks for the birthday (SP `0x33a`)
and opens with `{0xe, 0xb, room type, birthday}` in place of `{0xb, 1}`,
for a shop or a merchant; cancelling sends nothing more, the blacklist
having gone already. The server checks the birthday and opens, or ignores
a wrong one without an answer. v83 leaves the window as it was after a
`0xe` open; HeavenClient, given a real date, treats it like `0xb`,
showing a shop open and leaving a merchant's room. As in v83, the
Blacklist's changes are not sent to a shop opened this way.

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
`net`'s item guard handler covers what may be sealed, for how long, and a
seal running out; `item_guard` (integration) seals an equip for a week,
extends it, makes it lasting and is refused its drop, then has a timed seal
run out at login, and the client's `item_guard` test covers the checks and
the questions. `net`'s karma handler covers what the scissors cut and
refuse; `karma` (integration) cuts an equip, is refused a second cut, and
finds the flag saved; the client's `karma_scissors` test covers the checks.
`net::shop` covers buying, selling, recharging, and their refusals, and
`npc_shops` (integration, needs the pack's `shops.json`) opens the Henesys
grocer, is refused for want of mesos, sells a sword, buys an arrow with it,
and checks both are saved. `net::npc_scripts` covers the item makers'
menus, questions and trades, and `item_makers` (integration) makes Leather
Arms at Serryl, is refused without the Leather, and forges a stimulated
Dragon Carabella at Mos; `makers_of_ellinia_and_ludibrium` (integration)
upgrades a Bronze Pride at Francois and a Bronze Missel with a stimulator
at Sarah. `skill_books` (integration) refuses a mastery
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
`net`'s scripted item handler covers the slot and item checks, and
`net::scripts` what each script does near and far from its portal;
`scripted_items` (integration) starts 2324 with the Secretary of Magic,
uses both items away from and beside their portals, goes through
`obstacle`, completes 2324 with her once the thorns are gone, goes over the
castle wall, and after a relogin through `obstacle` again.
`net`'s Miu Miu handler covers the item and map checks, one taken a call
and Mu Mu's list; `miu_miu` (integration) calls him in Henesys, buys an
apple, is refused in the Pig Farm mini dungeon, and finds one Miu Miu left.
`net::storage` covers each request and its refusals. `storage` (integration)
opens Mr. Lee's storage, stores part of a stack and a whole one for the fee,
is refused too many and too many mesos, moves mesos in, sorts, takes items
out, and after a relogin finds both sides saved. The client's
`storage_contents` test reads the contents of each kind of result.
`net::duey` covers the tax, what may be sent, each send and claim refusal,
and what a send and a claim take and give; `parcel` covers the record
layout and the other modes, and the Duey handler parses every op. `duey`
(integration) has three tests:

- The first is refused a missing receiver and the sender themself. It
  sends potions and mesos less the fee to an offline character, then is
  refused too few mesos. The receiver hears of the parcel on entering the
  game, claims it, and is refused a second claim.
- The second opens a receive-only window from the notifier. A quick
  delivery sent with a coupon joins it, with its message, and the receiver
  hears of it. They delete it, and Duey's own window is then empty.
- The third fills an inbox with ten parcels and is refused an eleventh.

`net::rps` covers each round's outcome, the consolation, the ten-win end,
and requests out of turn. `rps` (integration) is refused a game for want of
mesos, then plays until it wins: a win risked on the next round and lost to
the clock leaves nothing, and a win kept gives the one-win certificate.

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

Fredrick: `net::store_bank` covers the days and the fee, and the build
tests cover the packets. `fredrick` (integration) is told he keeps
nothing, then where an open merchant stands. It takes a closed merchant's
items and mesos back less three days' fee, and is refused when the owner
can't pay the fee or carry the mesos, with the merchant still kept. The
client's `store_bank` test reads the kept items into one list and covers
the counts, the scrolling and the texts.

The Owl of Minerva: `net::owl` covers the market maps, the order and cap of
the rows, and the most searched list; the build tests cover the packets;
`field_shops` covers which shops a search finds and GO's answers.
`owl_of_minerva` (integration) is refused off the Free Market, keeps the Owl
when nothing turns up, finds a shop in another room and uses up one Owl,
does the same with a Cash Owl, sees the item in the most searched list, goes into the shop with GO, and
gets "closed" once the owner logs out.
