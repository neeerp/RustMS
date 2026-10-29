# Pets

A pet is a cash item (`500xxxx`) bought in the Cash Shop. Taken out to the
Cash tab, it can be summoned to follow its owner, and everyone on the map
sees it. Up to three can be out with Follow the Lead, one without; Bartos
teaches it in Henesys after his pet test (quests 4646 and 4647). It answers
commands said in chat and grows closer when it obeys. It grows hungry, eats pet
food and cash food, goes home when it starves, and can be renamed with a name
tag. It wears pet equipment and loots with a Meso Magnet or Item Pouch,
which this file doesn't cover yet (`v83ref` report 159 has the v83 client's
side). With an HP or MP pouch on, the first pet out gives the potion set
beside it.

## Server

- **Item.** `game::Item.pet` holds `PetData` (name, level, closeness,
  fullness, skill bits, attribute). `net::codec::item` writes it as item type
  3: the 13-byte name, level, closeness, fullness, expiry, attribute, skill
  and remaining life. Quests teach the first pet out skill bits (`petskill`)
  and the owner's speed (attribute bit 1, `petspeed`), through
  `train_first_pet`, which re-sends the item (see [Quests](./quests.md)).
- **Storage.** The `pets` table (`db/src/pet.rs`) is keyed by the item's cash
  serial, so a pet keeps its name and closeness in and out of the Cash
  Inventory. Saving the character upserts the rows of its carried pets, with
  `summoned` holding each one's place in the list of pets out. Loading joins
  them back onto the items.
- **Taking one out.** `net::cash_shop::inventory_item` gives a new pet
  `PetData::new` named after its kind (String.nx `Pet.img/<id>/name`); a pet
  that was out before gets its stored row instead. Putting it back drops it
  from the list of pets out.
- **From a script.** `Effect::GivePet` (the Rune Snail of quest 2230) makes
  a new pet with `net::items::new_pet` and takes its cash serial from the
  Cash Inventory's own sequence (`db::cash_shop::new_serial`), so that it may
  go into the Cash Inventory later. The pet row is written when the
  character is saved.
- **The list.** `Player.pets` holds the summoned serials in the order the
  client lists them. The list closes up when one leaves, as the client's
  does, so the indexes in packets match. `game::pet::toggle` applies a
  summon request: a pet that is out goes back; without Follow the Lead
  (`FOLLOW_THE_LEAD`: 8, 10000018, 20000024) a new pet replaces the one out;
  with it, the lead flag puts the new pet first, and a fourth is refused.
- **SPAWN_PET** (`0x62` in, `net/src/packet/handle/world/pet.rs`): the tick,
  the Cash slot and the lead flag. It answers with an exclusive stat change
  carrying the three serials, and the client actor (`client/pets.rs`) sends
  168 for each change to the owner and the map, with an empty exception list
  (173) for each pet that came out. A slot without a pet gets only the stat
  change, which unlocks the client.
- **Eggs.** A pet that evolves without an item (`info/evol1..n` and
  `evolProb1..n` with `evolReqItemID` 0, read into `ItemInfo::pet_evolutions`)
  is an egg: the Dragon Egg (5000028) and the Robo Egg (5000047). Summoning
  one hatches it instead, as Cosmic has it (no v83 source says when an egg
  hatches; the client has no egg code). The item becomes the evolution its
  weights pick (the Baby Dragon 5000029, the Baby Robo 5000048) in the same
  slot, with the same serial, level, closeness and expiry, re-sent as a
  remove and an add. A pet still called by its egg's kind ("Dragon", "Robo")
  takes the baby's name. Nothing comes out; the stat change unlocks the
  client.
- **Rock of Evolution.** The babies evolve further with the Rock of
  Evolution (5380000, sold in the Cash Shop), through Garnox (9102001, in
  New Leaf City and Singapore). Three repeatable
  quests (`interval 0`) do it, each with an end script (`net::npc_scripts`,
  `pet_evolution`): "Baby Dragon's Evolution" (8185, a Baby Dragon), "Dragon's
  Re-evolution" (8189, any of the evolved dragons, which turn into a
  different one) and "Robo's Upgrade" (4659, a Baby Robo, which also takes 50
  Cheap Batteries 4000111). The quests' `pet` lists and `pettamenessmin`
  (1642) do the checking on both sides (see [Quests](./quests.md)), on the
  first pet out only. The script asks (and if, by the answer, the pet no
  longer leads or the rock or fee are gone, says so and changes nothing), then takes the rock and 10,000 mesos
  (Cosmic's fee; no v83 source gives one), evolves the pet
  (`Effect::EvolvePet`, `net::packet::handle::world::evolve_pet`) and
  completes the quest. The new kind is rolled by the baby's `evol*` weights
  (`rock_evolutions`, `roll_evolution`): the Baby Dragon's four dragons
  33/33/33/1, the Baby Robo's five robos 330/330/330/9/1; a re-evolution
  rolls among the rest of its family. The pet keeps its serial, level,
  closeness and slot; its expiry restarts at the new kind's `info/life` (90
  days), and a pet still called by its old kind takes the new kind's name.
  The item is re-sent as a remove and an add, and a pet out is re-announced
  as a 168 going away and coming back at its index. Pet level 15
  (`evolReqPetLvl`, `pet_evolve_level`) is the level 1642 closeness reaches,
  so the closeness check covers it. Without a pet out, or short of the fee,
  Garnox only says so.
- **Packets.** 168 is `cid, index, 1, replace, body` to show a pet and
  `cid, index, 0, reason` to take it away. The body is `item id, name,
  serial, x, y, stance, foothold, name tag, balloon`; SPAWN_PLAYER repeats it
  after a 1 for each pet out and ends the list with a 0. The name tag byte is
  1 while something (a label ring, or the Pet Collar) is worn in the pet's
  label part (21, 31, 39 by list index), and the balloon byte while a quote
  ring is worn in its part (29, 32, 40) (`Player::pet_equip`). The pet stat bits
  (0x8, 0x80000, 0x100000) are written right after HAIR, 8 bytes each.
  CHAR_INFO lists the pets out, without equipment.
- **Moving.** MOVE_PET (`0xA7` in) is the pet's cash serial and a movement
  path. A pet that isn't out is ignored. The field (`FieldRequest::PetMove`)
  keeps the pet's position for later arrivals and relays 170
  `cid, index, start, path` to the others.
- **Fields.** The field avatar carries the pets, keeping each one's live
  position across avatar updates. After every field join the client actor
  sends the owner a 168 for each pet out, since the client forgets them on
  a map change.
- **Commands.** PET_COMMAND (`0xA9` in) is the pet's serial, whether the
  words began with its name, and the command (`interact/<n>` in Item.nx,
  read into `ItemInfo.pet_commands`). The server rolls `prob` and answers
  the owner alone with 174 `cid, index, 0, command, obeyed, quoted`, the last
  byte set while the pet wears a quote ring (`Player::pet_quoted`). Obeying
  adds `inc` closeness (capped at 30000, levels from Cosmic's table): the
  pet's item is re-sent, and a level up shows effect 4 `0, index` to the
  owner (`0xCE`) and the map (`0xC6`).
- **Food.** PET_FOOD (`0x4C` in) is the tick, the USE slot and the item id
  of a pet food (`212xxxx`). The pet out that is least full eats (the first
  of equals). A pet not yet full gains the food's `spec/inc` fullness (capped
  at 100) and, if its fullness was 75 or less, one closeness. A full pet
  refuses and loses one closeness, which can lower its level. The server
  answers the owner with 174 `cid, index, 1, enjoyed, quoted`, then an exclusive
  inventory operation that takes one food and re-sends the pet's item, then
  the level-up effect if it levelled. With no pet out or the wrong item in
  the slot, it sends only an empty stat change to unlock the client. The
  food's list of pets it suits isn't checked; the v83 client doesn't check
  it either.
- **Cash food.** USE_CASH_ITEM (`0x4F` in, `world/cash_item.rs`) is the Cash
  slot, the item id, the item's own fields and the tick. Cash pet food
  (`524xxxx`), the pet name tag, megaphones, the chalkboard and teleport
  rocks (`inventory.md`) are used so far; any other item, or one not in that slot,
  gets an empty stat change to unlock the client. The first pet out, in list order, whose id is in the
  food's `spec/0..n` eats: fullness 100 and `spec/inc` closeness (100), and
  it always enjoys it. The server answers with 76
  `0, index`, then the 174, inventory operation and level-up effect that
  PET_FOOD sends. When no pet out suits the food it sends 76 `1` and an empty
  stat change. These are Cosmic's rules.
- **Name tag.** The pet name tag (5170000) carries the new name and renames
  the first pet out. The name must be 4 to 12 bytes with no control
  characters or spaces at either end, as the client sends it. The server
  answers with an exclusive inventory operation that takes the tag and
  re-sends the pet's item, then 172 `cid, index, name, 0` to the owner and the
  map; the owner needs it too, since the client hears commands by the pet's
  name. Without a pet out, or with a name the client wouldn't send, it sends
  only an empty stat change. The trailing byte is the name tag flag, as in
  168.
- **Label rings.** Putting a label ring on or taking it off while its pet
  is out re-sends 172 with the pet's name and new flag to the owner and the
  map, since that is the only packet that can restyle another's pet. So
  does sending a pet back: those after it move down the list into the next
  index's label slot. The quote ring's balloon flag goes out only with
  SPAWN_PET, which has no update packet.
- **Hunger.** Every 3 minutes (`client/pets.rs`) each pet out loses its
  `info/hungry` fullness (1 if unset; the Baby Tiger loses 3) and its item is
  re-sent. One left at 5 or less is set to 15 and goes home with 168 reason
  1, followed by the stat change with the remaining serials. These are
  Cosmic's rules, as there is no v83 server code.
- **Chat.** PET_CHAT (`0xA8` in) is what the owner's client showed the pet
  do and say. The server relays it as 171 `cid, index, kind, action, text,
  styled` to the others. 0xA8 doesn't carry the balloon style, so the
  server styles all but the pet's idle lines (kind 3) while it wears a
  quote ring. The owner's client styles them the same way: 174's last byte
  for answers, the stored ring flag for `/pet`, and never for idle lines
  (report c032). Reading 174's last byte as the quote ring, not whether
  the command used the pet's name, is an inference: the client won't put a
  quote ring on a pet without `info/chatBalloon`, and `/pet` uses the ring
  flag. Cosmic always writes 0.
- **Auto potions.** The potions the first pet gives are kept with the
  character in `auto_potions` (`Player.auto_potions`, 0 for none).
  CHANGE_KEYMAP (`0x87` in) mode 1 (HP) or 2 (MP) is followed by the item
  id; any USE item whose spec restores that kind is taken (the Elixir fits
  both), 0 clears, and nothing answers. A v83 client sends zeros each time
  its equipment window redraws without a pet or the pouch. AUTO_HP_POT and
  AUTO_MP_POT (`0x150`, `0x151`) carry the ids on world entry. PET_AUTO_POT
  (`0xAB` in) is the pet's serial, a byte, the tick, the USE slot and the
  item id. The server uses the item as USE_ITEM would only when the pet is
  the first out, the id is the one set for a pouch, it restores that
  pouch's kind, and the pouch (HP 1812002, MP 1812003) is worn in the first
  pet's slot (`-124`, `-125`). It always answers with the exclusive flag,
  an empty stat change when refused, since the client keeps a request
  pending until then. HP the server takes without a hit (a map's `decHP`,
  a Poison tick) is announced first as HP_TAKEN (`0xD4`, the amount), which
  is what makes a v83 client check its HP potion.
- **Not allowed.** Dropping a pet that is out. Trading or storing one isn't
  checked yet.

## Client

- Double-clicking a pet in the Cash tab (`UIItemInventory`) sends SPAWN_PET
  under the USE_WAIT lock, following v83: with Follow the Lead, a pet that
  is out goes back, a fourth gets "You have exceeded the maximum number of
  pets", and one joining another asks whether it should lead (the answer
  sets the lead flag; it is sent either way). `Character/Pets.h` holds these
  rules and the closing-up list.
- 168 adds or removes a pet through that list (index 3, the Water of Life
  question, is ignored). Only the owner reads the reason a pet went back:
  hungry, its magic time ran out, or "You cannot use pet here.", each shown
  as a chat line.
- The player's own pets follow their owner and send their path as MOVE_PET
  on the movement timer. Other players' pets play the paths from 170
  (`PetLook`). On the ground a pet walks as its owner does (shoe physics,
  speed 1), and thinks every 30 ms as v83's CVecCtrlPet does
  (FUN_009c4122, `petfollow` in `Character/Look/PetFollow.h`):
  - Its role comes from the owner's slots: a lone pet 0, two pets 1 and 2,
    three pets 5, 3 and 4. Each role has a spot: at the owner, 160 px ahead
    of them, or 40 px behind. It also has a distance to keep, from 60 to
    80 px.
  - A standing pet starts once farther than that and stops once nearer. A
    walking pet turns back only 80 px past its spot (170 with a Binocular).
  - It hops a rising wall ahead. Below its owner's platforms it jumps up
    toward them.
  - With other pets out, about once every 30 s it hops, and about once
    every 20 s it dashes about at double speed for 700 ms.
  - Once the owner leaves a box around the pet, it is called back to them.
    The box reaches from 300 px left of the pet to 300 px right, and from
    200 px above it to 200 px below. A swimming pet (FLY) keeps
    HeavenClient's old 250 px leash and force-based follow.
- A pet in Wing Boots (1812004) with a Meso Magnet or Item Pouch walks toward
  a drop once its spot near its owner has stayed put for over 100 ms (FUN_00508268,
  `Stage::sweep_pets`, `MapDrops::sweep`). It looks within 124 px to either
  side of the owner (274 with a Binocular) and from 49 px above its feet to
  9 px below, at drops it would loot by the rules of `Drop::pet_lootable`,
  except that mesos draw any such pet. Drop by drop, the nearest pet takes
  each, and walks toward one drop at a time; the loot scan then picks it up
  at its feet. The path goes out as ordinary MOVE_PET.
  v83's sweep differs from its loot scan in two ways that can leave a pet
  standing at a drop it never takes, and both are kept: a pet without a
  Meso Magnet still walks to mesos, and the ignore list holds back only a
  pet that wears an Item Ignore Pendant itself, while the loot scan applies
  every pendant's list to every pet.
- A pet's stance is `action << 1 | left`; `pets::shown` maps the action to
  its animation.
- `Stage::clear` drops the player's pets on a map change; the server sends
  them again.
- `PetParser` reads the 168 body and the SPAWN_PLAYER list.
- **Commands.** Every chat line the flood guard lets through goes to
  `Player::command_pets` first. `pets::hear` follows v83: the line is
  trimmed; a renamed pet whose name starts it has the name and any `!`,
  `,` or `~` cut, and is "named"; the rest must equal one of the words of a
  command within the pet's level range, case and all. Words come from
  String.nx `PetDialog.img/<id>/<command>`, split on `|`. A pet hears
  nothing while a custom action plays, while a command is pending, or until
  it has stood still for 500 ms. A match sends PET_COMMAND.
- **Answers.** 174 picks a random success or fail reaction for the
  command, plays its act and shows a random line, then sends the result as
  PET_CHAT. Others' clients play 171 the same way. Acts map to the base
  actions (move, stand0, stand1, jump, fly, hungry, rest0, rest1, hang:
  0 to 8) or the pet's own animations (9 on); an act the pet lacks does
  nothing. Base actions loop until the pet moves, its own play once, and
  any movement ends them. A command answered with one of the pet's own
  animations also plays its sound from `Sound.nx Pet.img/<id>`.
- **`/pet <text>`.** The chat bar sends this to the pets out, not the
  server (report c030). Text the curse filter won't take (`CurseData::valid`)
  gets the "Cursing is not allowed." box. Otherwise each pet of level 15 or
  more says it as PET_CHAT kind 2, action 0, in its kind's balloon if it
  wears a quote ring (`Player::pets_say`). If the filter starred a word,
  a pet not doing an action shows its `slang` instead: the first `slang/N`
  whose levels hold its level, its act and one of its lines. Only the line
  is sent, still with action 0. Only the Snail's slang reaches level 15, so
  other pets do nothing. v83's chat-ban check on slang isn't kept, as HC
  has no chat ban.
- **Balloons.** Pets speak in `UI ChatBalloon.img/pet/<style>` for 5 s:
  the pet's `info/chatBalloon` when the packet's flag is set, else 0
  (`ChatBalloon::pet`, report c032). A style the file lacks shows nothing.
  The balloon goes at once, without the players' fade. Its width is an odd
  number of `n` tiles around the text, which wraps at 14 tiles; the `arrow`
  replaces the middle bottom tile and a `head` the middle top one. The
  arrow's tip is placed when the pet speaks, 5 px above the top of its
  frame then (its origin's y), and 90 px higher for Sun Wu Kong's
  (5000026) action 20. It follows the pet at that height, and the box is
  centred 3 px right of the pet. v83 draws the text in Arial at the
  `w` piece's height less 2; HC keeps the balloon font.
- **Idle.** Every 5 s a still pet with `randAction` entries has an even
  chance of playing one.
- **Food.** Double-clicking pet food in the USE tab, or its hotkey, sends
  PET_FOOD under the USE_WAIT lock after the usual field and ailment checks.
  As in v83, the client doesn't check that a pet is out. The pet out that is
  least full plays `Sound.nx Pet.img/<id>/eat` if it has one. 174 with mode
  1 picks a success or fail reaction from the first `food/N` entry whose
  levels hold the pet's level, and sends it as PET_CHAT.
- **Cash food.** Double-clicking cash pet food in the Cash tab sends
  USE_CASH_ITEM under the WAIT lock and plays `Sound.nx Game.img/UseShopItem`.
  With no pet out it shows the chat line "This item can only be used with a
  pet." instead. 76 plays the eating pet's eat sound, or shows "Your pet
  cannot consume this food. Please check again." The inventory operation
  after it unlocks the client.
- **Name tag.** Double-clicking a name tag in the Cash tab renames the first
  pet out, following v83; with no pet out it does nothing. A pet with a name
  of its own first asks "The pet already has a name. Will you change it?".
  The name prompt takes up to 12 bytes; `pets::check_name` trims it, asks
  again under 4 ("Please enter at least 4 characters."), refuses control
  characters ("Cursing is not allowed."), stars out curse words, and refuses
  the kind's name in any case or the pet's own ("That name is not
  available."). "Do you want to name your pet 'X'?" then sends USE_CASH_ITEM
  under the WAIT lock and plays `UseShopItem`. 172 sets the pet's name and
  whether it shows its name tag.
- **Name tags.** Every pet has a name tag, its top 2 px under the pet's
  feet (FUN_005f0334 type 0x3eb, origin cy -2). With its flag set (168,
  SPAWN_PLAYER, 172; for the owner's own pets, a label ring worn in its part,
  as FUN_00705e2c re-derives it) the tag is drawn in the kind's
  `info/nameTag` style from `UI NameTag.img/pet/<style>` (UIClassic, else
  the newer UI, which alone has style 42): the left cap `w`, the tile `c`
  repeated under the name, the right cap `e`, and the name in Arial 12 in
  the colour `clr`, laid out as the medal tag is (`StyledTag`,
  `MedalTagLayout`). Without the flag, or for a kind with no style, it is
  the plain translucent box with the name in white (0x5f0e1d).
- **Closeness.** After every inventory operation, a pet out whose item now
  shows different closeness gets the yellow status line "The closeness
  increased (+N)" or "The closeness decreased (-N)".
- **Level up.** Effect 4 (`0xCE` for the player, `0xC6` for others) plays
  `Effect/PetEff.img/Basic/LevelUp` on the pet, with the level-up sound
  for kind 0; kinds 1 to 3 are Teleport, the pet's warp and Evolution.
- **Auto potions.** With the pet panel open, a USE item dropped beside
  the first pet's HP pouch (46, 11) or MP pouch (112, 11) is set when its
  spec restores that kind ("Only usable on Auto HP Potion Pouch." if not):
  CHANGE_KEYMAP mode 1 or 2, a yellow line, and the System Options so the
  alerts can be set. Without the pet or the pouch nothing happens. The slot
  shows the item and how many are left, even none; dragged onto the stage
  it clears, and so does opening the panel without the pet or the pouch
  (not while a field change has the pets away). A potion held on the cursor
  is dropped when its slot empties.
  150 and 151 set the ids on field entry. `Character/AutoPotions.h` holds
  the rules.
- **Giving.** After each TAKE_DAMAGE, and on 212 (HP_TAKEN), the HP check
  runs when the first pet wears the HP pouch, the player isn't seduced, and
  HP less the hit is under the HP alert (a percent in HeavenClient's
  settings). The hit is what HP loses: Achilles, Meso Guard, Power Guard's
  reflection and Magic Guard's share come off. A skill that goes off (one
  refused doesn't; Teleport and Flash Jump never do), Magic Guard taking
  part of a hit, or Power Guard or Meso Guard working arms the MP check
  (Power Guard counts even when the Guardian then blocks the hit), which then
  runs every frame and on each stat change until an MP potion goes. The
  item comes from the first USE slot holding it, as PET_AUTO_POT, when the
  field allows consumables, the player's level meets the item's, nothing is
  pending, 200 ms have passed since the last potion, and (for HP) the
  player is alive; it plays the item's sound. A missing potion prints "You
  are lacking the HP Potion that your pet is supposed to use." each time,
  and pets that speak up on their own (skill bit `0x100`, read from the
  item) say `e_NOHPPotion` for the first three in a row. HP misses reset on
  a give, MP misses on field entry.
- `PetData` reads the pet's commands, food answers, idle actions, own
  animations, balloon and no-potion lines from Item.nx and String.nx.

## Tests

- `game::pet` unit tests cover the list, feeding, cash food, closeness lost
  and starving; `net` tests cover the codecs, which pet eats (cash food
  included), hunger, and the name tag.
- `integration-harness/tests/pets.rs` buys the Baby Tiger Package, takes the
  pet out, summons it, moves it, sends it back, and checks that it is still
  out after a relog and that a watcher sees it. It also commands the pet by
  name until it obeys, checks the closeness update and the level-up effect
  for the owner and the watcher, and the relayed PET_CHAT. It then feeds the
  pet and checks the answer, the food taken and the pet's new fullness and
  closeness, then gives it a cash food that suits it and one that doesn't.
  It renames the pet with a name tag, checks the 172 for the owner and the
  watcher, and that the watcher sees the new name after the relog.
- `integration-harness/tests/mysterious_egg.rs` runs quest 2230 from the
  egg to the Rune Snail and checks the saved pet row and its five hours.
- `integration-harness/tests/pet_eggs.rs` hatches a Dragon Egg and a named
  Robo Egg, summons both babies, and checks they stay babies after a relog.
- `integration-harness/tests/pet_evolution.rs` has Garnox refuse a Baby
  Dragon one point short of 1642 closeness, then evolve it into a dragon for
  the rock and the fee, re-announce it, complete 8185, and keep it a dragon
  after a relog.
- `integration-harness/tests/pet_auto_potion.rs` sets a Red Potion beside a
  worn HP pouch, has the pet give one, checks that a pet not out is refused
  with an empty stat change, and that the potion is kept and cleared across
  relogs. `map_hazards` checks the HP_TAKEN before the cold's stat update.
- `HeavenClient/tests/auto_potions.cpp` covers the alert threshold, the
  three complaints and what setting an item does; `excl_request.cpp` covers
  the potion timer and `mob_damage.cpp` Magic Guard's share.
- `HeavenClient/tests/pets.cpp` covers the closing-up list, the summon
  rules, Follow the Lead's skill ids, the stance mapping, the SPAWN_PLAYER
  list, splitting command words, hearing a command, the action mapping, the
  idle timer, which pet eats, the food answer for a level, the closeness
  lines, the food item ranges and the name checks.
