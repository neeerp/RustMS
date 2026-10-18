# Pets

A pet is a cash item (`500xxxx`) bought in the Cash Shop. Taken out to the
Cash tab, it can be summoned to follow its owner, and everyone on the map
sees it. Up to three can be out with Follow the Lead, one without. Commands,
chat, food, hunger, name tags, pet equipment, looting and auto potions are not
done yet; `v83ref` report 159 has the v83 client's side of each.

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

## Tests

- `game::pet` unit tests cover the list; `net` tests cover the codecs.
- `integration-harness/tests/pets.rs` buys the Baby Tiger Package, takes the
  pet out, summons it, moves it, sends it back, and checks that it is still
  out after a relog and that a watcher sees it.
- `HeavenClient/tests/pets.cpp` covers the closing-up list, the summon
  rules, Follow the Lead's skill ids, the stance mapping and the SPAWN_PLAYER
  list.
