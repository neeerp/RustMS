# Asset-backed map travel

Ordinary exits use `Map.nx/Map/Map*/<id>.img/portal`: `pn` identifies
the source portal, `tm` the destination map, and `tn` the arrival portal.
The server resolves these links; no handwritten map connection table exists.
A nonempty `tn` must match an actual destination portal. Only an empty `tn`
may use the map's default spawn point. The client skips portals without a
travel destination when selecting an exit, including overlapping spawn points.

The ships between continents run on a timetable instead; see `ships.md`.

## Maple Island

The island is the original one, as in 2009 footage. v83 GMS already ships the
revamped island, so `assets/game-data` replaces its 12 maps, their names, and
the island's world map with the ones from GMS v68
(`tools/graft-original-island.sh`, see `assets/game-data/README.md`). The
client loads the same files.

New Explorer characters start in `0`, outside the Training Camp, with Heena
and Sera. Existing characters keep their saved maps. Other jobs start in
Amherst (`1010000`). The active route is:

`0 → 10000 → 20000 → 30000 → 40000 → 40001 → 40002 → 50000 → 1000000 → 1010000 → 1020000`

Roger is in `10000`. Nina is in `20000`; its `in01` enters Sen's house
(`20001`), whose `out00` returns to Nina. Todd and Peter are in `30000`, with
the Jr. Sentinels; Robin and Sam are in `40000`, Maria in `50000`, and Lucas,
Rain, and Pio in Amherst. The exits up to `50000` are one-way. From `50000`
the island's paths have reverse exits: east to Amherst, and `east01` to
Southperry (`50001`, `60000`). The Training Camp's own maps (`1`-`3`), the
revamp's Sen house (`30001`), and the Adventurer Training Centers
(`1010100`-`1010400`) are in the assets but not linked from the route. Do not
insert them or invent reverse connections.

## Taxis

Taxi NPCs are scripted in `net::npc_scripts` (fares from v83 server
scripts). Every ride asks Yes or No first. Yes with too few mesos takes
nothing and shows the cab's "not enough mesos" line.

| NPC | Where | Ride |
| --- | --- | --- |
| Regular Cab (`1002007`, `1012000`, `1022001`, `1032000`, `1052016`, `1092014`) | Lith Harbor, Henesys, Perion, Ellinia, Kerning City, Nautilus | Menu of the other five towns: 1000 mesos, 800 to Ellinia and Nautilus. Arrives at the town's first spawn point |
| VIP Cab (`1002004`, `1032005`) | Lith Harbor, Ellinia | Ant Tunnel Park (`105070001`) for 10000 |
| Danger Zone Taxi (`2023000`) | El Nath, Ludibrium, Omega Sector, Leafre | El Nath to `211040200` (10000), Ludibrium to `220050300` portal 1 (25000), Omega Sector to Ludibrium (25000), Leafre to `240030000` (65000) |
| Camel Cab (`2110005`) | Outside Ariant and Magatia | The other end for 1500 |
| NLC Taxi (`9201056`) | New Leaf City, the Haunted House's grounds (`682000000`) | The other end, portal 0, for 15000 |
| Dolphin (`2060009`) | Aquarium, Herb Town pier | Menu: down to Sharp Unknown (`230030200` portal 2) for 1000 or one Dolphin Taxi Coupon (`4031242`), or across between the two for 10000 |

Beginners (job 0) pay a tenth on the Regular and VIP cabs, and the Regular
Cab's menu opens with the discount line. In the original, choosing a
Dolphin ride warps at once; here each choice asks Yes or No, since a menu
choice opens pages and cannot take mesos itself.

Only the Regular Cab's text is in the content pack (`taxi.*`). The VIP Cab,
Danger Zone Taxi, Camel Cab, Dolphin and NLC Taxi keys are `unsourced` and show
placeholder text. `taxis.rs` checks a beginner's Regular Cab ride and the
VIP Cab's fare through the network harness.
`subway.rs` rides the NLC Taxi and walks through `halloween_enter`. The
Kerning City-New Leaf City subway runs on the ship clock (`ships.md`), as
does the plane between Kerning City and Singapore (`plane.rs`).

## Trips with a way back

Some trips end at an NPC who takes the player back where they came from.
The map they left is saved with the character (`saved_locations`), so the
way back outlasts a relogin, and taken once. The fares are Cosmic's.

| NPC | Where | Trip |
| --- | --- | --- |
| Spinel (`9000020`, `world_trip`) | Victoria Island's five towns, Orbis, Ludibrium, Leafre, Mu Lung, Ariant, Boat Quay Town, Amoria and the Mushroom Shrine (`800000000`) | From a town to the Mushroom Shrine, portal 0, for 3000; from Boat Quay Town (`541000000`) to Trend Zone Metropolis (`550000000`), portal 0, for 10000. Saved as kind 1 (`WORLD_TOUR_RETURN`). At the shrine, a menu: back to the saved map, else Lith Harbor, or stay |
| Pason (`1002002`), Shuri (`2010005`), Nara (`2040048`), `florina2` | Lith Harbor, Orbis, Ludibrium | Menu: to Florina Beach (`110000000`, `st00`, id 4) for 1500, 2000 or 1800, or by showing the VIP Ticket to Florina Beach (`4031134`), which is kept; or two pages about the ticket. Saved as kind 2 (`FLORINA_RETURN`) |
| Pison (`1081001`, `florina1`) | Florina Beach | Back to the saved map, else Lith Harbor |
| Thomas Swift (`9201022`) | Henesys, Amoria | Free, to Amoria's portal 0 or back to Henesys's portal 5. Saves nothing |

Spinel's tour has no Yes or No: the fare is paid as the last page turns,
and a player short of it is told so after the first page about the trip.
Spinel doesn't stand in Malaysia; Audrey (`9201135`), who does, isn't
scripted yet, so the way back from Malaysia is on foot. All of this text is
`unsourced` placeholder text (`world_trip.*`, `florina.*`, `amoria.*`).
`world_tour.rs` checks the tour over a relogin, Florina Beach both ways
and Thomas Swift.

## Guards

Jeff (`2030000`, `goDungeon`) stands in Ice Valley II (`211040200`), where
the El Nath Danger Zone Taxi stops. After a first page he asks a player of
level 50 or more whether to go down, and Yes warps to Sharp Cliff I
(`211040300`) portal 5 (`under00`); below level 50 he refuses. A holder of
the Orihalcon Hammer (`4031450`) goes straight to Ice Valley (`921100100`)
portal 1 (`west00`) without a word, though nothing hands the hammer out yet.
His keys (`jeff.*`) are `unsourced`. `el_nath_dungeon.rs` checks all three
through the network harness.

Shane (`1032003`, `herb_in`) in Ellinia lets players from level 25 into the
Forest of Patience for 5000 mesos (Yes/No, then a refusal if short). They
start at Step 1 (`101000100`) while on Sabitrama's first herb quest (2050),
else at Step 3 (`101000102`) while on the second (2051), else at Step 1 below
level 50 and Step 3 from it, arriving at portal 0. Louis (`1032004`,
`herb_out`) on each step asks, and Yes goes back to Ellinia portal 0. Their
keys (`shane.*`, `louis.ask`) are `unsourced`. `forest_of_patience.rs` checks
them through the network harness.

## Menu warps

These NPCs warp straight off a menu choice, with no question after it
(`Page::Done`):

- The Mysterious Statue (`1061006`, `flower_in`) in the Sleepywood forest
  (`105040300`) speaks a first page, then offers the Deep Forest's steps that
  John's quests have opened, each taken or done: 2052 opens Step 1
  (`105040310`), 2053 Step 3 (`105040312`) and 2054 Step 5 (`105040314`),
  arriving at portal 0. With none, it only speaks. The Crumbling Statue
  (`1061007`, `flower_out`) on each step asks, and Yes goes back to
  `105040300` portal 0.
- The Time Gate (`2083006`, `TD_neoCity_enter`) in the Tera Forest
  (`240070000`) sleeps until Andy the Time Traveler (3718) is completed. It
  then offers each era in turn whose Guardian Nex quest (3719, 3724, 3730,
  3736, 3742, 3748 for Years 2021 to 2503) is completed, stopping at the
  first that isn't, and arrives at the era's portal 1 (`left00`). With none,
  it says to beat Guardian Nex first.

Their keys (`statue.*`, `crumbling.ask`, `time_gate.*`) are `unsourced`;
menu labels are map names (`#m<map id>#`). `statues_and_time_gate.rs` checks
them through the network harness.

## Map and portal scripts

Map entry (`info/onUserEnter`) and touch portals (`pt` 9) run native scripts
from `net::scripts`, keyed by the script names in `Map.nx`; unknown names do
nothing. A script returns effects that the client actor sends:

| Script | Effect |
| --- | --- |
| `go<map id>` (entry) | Map name banner, FIELD_EFFECT `maplemap/enter/<map id>` |
| `infoMinimap` | `UI/tutorial.img/25` once 1031 is started |
| `infoAttack`, `infoPickup` | `/20`, `/21` once 1035 is started |
| `infoSkill` | `/23` once 1035 is completed |
| `infoWorldmap` | `/26` |
| `infoReactor` | `/22` after 1008, else `/27` after 1020 |
| `glTutoMsg0` | PLAYER_HINT balloon, text key `glTutoMsg0.message` |
| `market01`-`market22` | A town's Free Market entrance: save the town, portal sound, warp to `910000000` portal `out00` |
| `market00` | The Free Market's exit: portal sound, back to the saved town's entrance (else Henesys), forget the town |
| `halloween_enter` | The Haunted House's gate: portal sound, warp to `682000100` portal `st00` |
| `photoOut`, `StudioZone_Out`, `outBabyBird` | An event map's exit: portal sound, back to where its invitation was used (else Henesys), forget it |
| `TD_MC_first` | The Empty House's (`100000002`) way to the Mushroom Kingdom: portal sound, warp to `106020000`, once 2260 is completed or any of 2300-2310 is taken; else a pink line |
| `obstacle` | The poisonous mushrooms on `106020300`: with a Poison Spore (`4000507`), a pink line, portal sound, take one, warp to `106020400` portal `left00`; else a pink line |
| `gotocastle` | Split Road of Destiny's way over the castle wall: portal sound, warp to `106020501`, once 2324 is completed; else a pink line |
| `TD_MC_jump` | Portal sound, warp to `106020501` |
| `TD_MC_Egate` | Portal sound, warp to `106021300` portal `right00` |
| `subway_in2` | Kerning City's ticketing booth (`103000100` `in00`) into the tunnels: portal sound, warp to `103000101` portal `out00` |
| `met_in` | The construction site's tunnel (`102040000` `subway00`): portal sound, warp to `103000103` portal 1, as in Cosmic (the original's Abandoned Subway Station needs Mr. Lim's instances) |
| `rankRoom` | A job hall's (or Ereve's, or the Dangerous Forest's) way into its room: portal sound, warp to the room's portal back (`RANK_ROOMS`) |

Only the `go<map id>` banners are used by the current maps, on the Adventurer
Training Centers and the revamp's Southperry (`2000000`). The other scripts,
and `tutoChatNPC`, were for the revamped island's maps and are unused. The
original island's touch portals run `advice00`-`advice09` and `adviceMap`,
which have no implementation yet, so they do nothing.

HeavenClient sends CHANGE_MAP_SPECIAL (`0x64`, portal name, player x and y)
while the player stands in a script portal's box, and the server answers with
the effects and an enable-actions stat update. As in the original, the box is
the portal's `hRange` by `vRange` (100 by default) centred on it, so the
triggers that sit 30-40 px above the floor fire as the player walks past. The
client sends again once that answer arrives and the portal's `delay` (ms) has
passed, unless the portal is `onlyOnce` and was the last to run in the map.
Pressing Up on a type 7, 8 or 11 script portal with no destination sends the
same packet, at most once per 500 ms. The client plays no portal sound for
these; a script that warps sends it, SHOW_ITEM_GAIN_INCHAT mode 7.

The town a player entered the Free Market from is saved with the character
(`saved_locations`, kind 0; NPC trips use kinds 1 and 2, above), so the
exit still leads back after a relogin.
`free_market.rs` checks the round trip through the network harness. An event
invitation saves the map it was used in the same way, under its
`returnMapQR` (see "Return scrolls" in `inventory.md`).

Touch portals and jump pads test where the player was at the start of the
physics step, not where it ended. v83 tests the drawn position, which lies
between its last two 30 ms steps and so trails the player. A player landing
is therefore still tested a few pixels above the floor.

Jump pads (`pt` 12 and 13) throw a grounded, non-climbing player into the air
and play `Effect/BasicEff.img/VerticalJump` where they took off:

- Type 12 throws at 2300 px/s up and 200 px/s the way the player faces, plus
  100 px/s for each arrow key held.
- Type 13 uses the portal's `horizontalImpact` and `verticalImpact`.

Most pads sit 30-56 px above their floor. Those more than 49 px up, such as
the Haunted House chimney's (`682010200`), fire only when the player lands on
them, not when walking. An `info*` hint runs once per session after it
was shown; the exit warning repeats. Hint text comes from the
[server content pack](./server-content.md). `tutoChatNPC` (the skip offer)
belongs with scripted NPCs. Explicit direct
warps remain available for existing development/harness workflows.

`beginner_route_and_ninas_house_follow_asset_links` checks the active route,
the house round trip, and exact arrival portal IDs through the network harness.
