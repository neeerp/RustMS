# Pets

A pet is a cash item (`500xxxx`) bought in the Cash Shop. Taken out to the
Cash tab, it can be summoned to follow its owner, and everyone on the map
sees it. Up to three can be out with Follow the Lead, one without. It answers
commands said in chat and grows closer when it obeys. It grows hungry, eats pet
food and cash food, and goes home when it starves. Name tags, pet equipment,
looting and auto potions are not done yet; `v83ref` report 159 has the v83
client's side of each.

## Server

- **Item.** `game::Item.pet` holds `PetData` (name, level, closeness,
  fullness, skill bits, attribute). `net::codec::item` writes it as item type
  3: the 13-byte name, level, closeness, fullness, expiry, attribute, skill
  and remaining life.
- **Storage.** The `pets` table (`db/src/pet.rs`) is keyed by the item's cash
  serial, so a pet keeps its name and closeness in and out of the Cash
  Inventory. Saving the character upserts the rows of its carried pets, with
  `summoned` holding each one's place in the list of pets out. Loading joins
  them back onto the items.
- **Taking one out.** `net::cash_shop::inventory_item` gives a new pet
  `PetData::new` named after its kind (String.nx `Pet.img/<id>/name`); a pet
  that was out before gets its stored row instead. Putting it back drops it
  from the list of pets out.
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
- **Packets.** 168 is `cid, index, 1, replace, body` to show a pet and
  `cid, index, 0, reason` to take it away. The body is `item id, name,
  serial, x, y, stance, foothold, name tag, balloon`; SPAWN_PLAYER repeats it
  after a 1 for each pet out and ends the list with a 0. The pet stat bits
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
  the owner alone with 174 `cid, index, 0, command, obeyed, named`. Obeying
  adds `inc` closeness (capped at 30000, levels from Cosmic's table): the
  pet's item is re-sent, and a level up shows effect 4 `0, index` to the
  owner (`0xCE`) and the map (`0xC6`).
- **Food.** PET_FOOD (`0x4C` in) is the tick, the USE slot and the item id
  of a pet food (`212xxxx`). The pet out that is least full eats (the first
  of equals). A pet not yet full gains the food's `spec/inc` fullness (capped
  at 100) and, if its fullness was 75 or less, one closeness. A full pet
  refuses and loses one closeness, which can lower its level. The server
  answers the owner with 174 `cid, index, 1, enjoyed, 0`, then an exclusive
  inventory operation that takes one food and re-sends the pet's item, then
  the level-up effect if it levelled. With no pet out or the wrong item in
  the slot, it sends only an empty stat change to unlock the client. The
  food's list of pets it suits isn't checked; the v83 client doesn't check
  it either.
- **Cash food.** USE_CASH_ITEM (`0x4F` in, `world/cash_item.rs`) is the Cash
  slot, the item id, the item's own fields and the tick. Only cash pet food
  (`524xxxx`) is used so far; any other item, or one not in that slot, gets
  an empty stat change to unlock the client. The first pet out, in list
  order, whose id is in the food's `spec/0..n` eats: fullness 100 and `spec/inc`
  closeness (100), and it always enjoys it. The server answers with 76
  `0, index`, then the 174, inventory operation and level-up effect that
  PET_FOOD sends. When no pet out suits the food it sends 76 `1` and an empty
  stat change. These are Cosmic's rules.
- **Hunger.** Every 3 minutes (`client/pets.rs`) each pet out loses its
  `info/hungry` fullness (1 if unset; the Baby Tiger loses 3) and its item is
  re-sent. One left at 5 or less is set to 15 and goes home with 168 reason
  1, followed by the stat change with the remaining serials. These are
  Cosmic's rules, as there is no v83 server code.
- **Chat.** PET_CHAT (`0xA8` in) is what the owner's client showed the pet
  do and say. The server relays it as 171 `cid, index, kind, action, text,
  styled` to the others. 0xA8 doesn't carry the balloon style, so the
  server remembers which pets the last command called by name
  (`Player.pets_named`, not saved) and styles their answers. Echoing the
  name flag is an inference; Cosmic always writes 0.
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
- The player's own pets follow with HeavenClient's follow physics and send
  their path as MOVE_PET on the movement timer. Other players' pets play
  the paths from 170 (`PetLook`).
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
- **Balloons.** Pets speak in `UI ChatBalloon.img/pet/<style>` for 5 s:
  the pet's `info/chatBalloon` when the packet's flag is set, else 0.
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
- **Closeness.** After every inventory operation, a pet out whose item now
  shows different closeness gets the yellow status line "The closeness
  increased (+N)" or "The closeness decreased (-N)".
- **Level up.** Effect 4 (`0xCE` for the player, `0xC6` for others) plays
  `Effect/PetEff.img/Basic/LevelUp` on the pet, with the level-up sound
  for kind 0; kinds 1 to 3 are Teleport, the pet's warp and Evolution.
- `PetData` reads the pet's commands, food answers, idle actions, own
  animations and balloon from Item.nx and String.nx.

## Tests

- `game::pet` unit tests cover the list, feeding, cash food, closeness lost
  and starving; `net` tests cover the codecs, which pet eats (cash food
  included), and hunger.
- `integration-harness/tests/pets.rs` buys the Baby Tiger Package, takes the
  pet out, summons it, moves it, sends it back, and checks that it is still
  out after a relog and that a watcher sees it. It also commands the pet by
  name until it obeys, checks the closeness update and the level-up effect
  for the owner and the watcher, and the relayed PET_CHAT. It then feeds the
  pet and checks the answer, the food taken and the pet's new fullness and
  closeness, then gives it a cash food that suits it and one that doesn't.
- `HeavenClient/tests/pets.cpp` covers the closing-up list, the summon
  rules, Follow the Lead's skill ids, the stance mapping, the SPAWN_PLAYER
  list, splitting command words, hearing a command, the action mapping, the
  idle timer, which pet eats, the food answer for a level, the closeness
  lines and the food item ranges.
