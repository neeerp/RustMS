# Asset-backed map travel

Ordinary exits use `Map.nx/Map/Map*/<id>.img/portal`: `pn` identifies
the source portal, `tm` the destination map, and `tn` the arrival portal.
The server resolves these links; no handwritten map connection table exists.
A nonempty `tn` must match an actual destination portal. Only an empty `tn`
may use the map's default spawn point. The exceptions are `reactors::lost_link`'s exits,
whose `tn` the target lacks: the Bart room's way out and Ardin's room's
(926000000, `st00`) go to the target's default spawn instead of closing. The client skips portals without a
travel destination when selecting an exit, including overlapping spawn points.

The ships between continents run on a timetable instead; see `ships.md`.

## Maple Island

The island is the original one, as in 2009 footage. v83 GMS already ships the
revamped island, so `assets/game-data` replaces its 12 maps, their names, and
the island's world map with the ones from GMS v68
(`tools/graft-original-island.sh`, see `assets/game-data/README.md`). The
client loads the same files.

New Explorer characters start in `0`, outside the Training Camp, with Heena
and Sera. Existing characters keep their saved maps. A Noblesse starts on
Ereve's tutorial path (`130030000`, below) and an Aran in Amherst
(`1010000`). The active route is:

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
`subway.rs` rides the subway to New Leaf City and back on a new
character's Basic tickets, once leaving the
waiting room by Bell (the ticket isn't returned), then rides the NLC Taxi
and walks through `halloween_enter`. The
Kerning City-New Leaf City subway runs on the ship clock (`ships.md`), as
does the plane between Kerning City and Singapore (`plane.rs`).

## Trips with a way back

Some trips end at an NPC who takes the player back where they came from.
The map they left is saved with the character (`saved_locations`), so the
way back outlasts a relogin; going back clears it. The fares are Cosmic's.

| NPC | Where | Trip |
| --- | --- | --- |
| Spinel (`9000020`, `world_trip`) | Victoria Island's five towns, Orbis, Ludibrium, Leafre, Mu Lung, Ariant, Boat Quay Town, Amoria and the Mushroom Shrine (`800000000`) | From a town to the Mushroom Shrine, portal 0, for 3000; from Boat Quay Town (`541000000`) to Trend Zone Metropolis (`550000000`), portal 0, for 10000. Saved as kind 1 (`WORLD_TOUR_RETURN`). At the shrine, a menu: back to the saved map, else Lith Harbor, or stay |
| Pason (`1002002`), Shuri (`2010005`), Nara (`2040048`), `florina2` | Lith Harbor, Orbis, Ludibrium | Menu: to Florina Beach (`110000000`, `st00`, id 4) for 1500, 2000 or 1800, or by showing the VIP Ticket to Florina Beach (`4031134`), which is kept; or two pages about the ticket. Saved as kind 2 (`FLORINA_RETURN`) |
| Pison (`1081001`, `florina1`) | Florina Beach | Back to the saved map, else Lith Harbor |
| Thomas Swift (`9201022`) | Henesys, Amoria | Free, to Amoria's portal 0 or back to Henesys's portal 5. Saves nothing |
| Audrey (`9201135`, `Malay_Warp2`) | The CBD (`540000000`, where the plane lands), Trend Zone Metropolis, Kampung Village (`551000000`) | Menu, then a Yes or No on the fare: from the CBD to Trend Zone, portal 0, for 42000, saved as kind 1; from Trend Zone to Kampung Village, portal 2, for 10000; from Kampung Village to Trend Zone, portal 4, for 10000. From Trend Zone also free back to the saved map, else Boat Quay Town, at portal 4 |

Spinel's tour has no Yes or No: the fare is paid as the last page turns,
and a player short of it is told so after the first page about the trip.
Spinel doesn't stand in Malaysia; Audrey does, and shares Spinel's kind,
so a tour of Malaysia begun with either ends with her. Only the CBD and
Boat Quay Town are saved on the way there; any other saved map is warped
to without a portal. All of this text is `unsourced` placeholder text
(`world_trip.*`, `florina.*`, `amoria.*`, `malaysia.*`). `world_tour.rs`
checks the tour over a relogin, Florina Beach both ways, Thomas Swift and
Audrey's rides.

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

At the end of each forest a pile (`PATIENCE_PILES`) hands something out and
sends the player out at once, without a word:
- The pile of flowers (`1043000`, Step 2) and the pile of herbs (`1043001`,
  Step 5) give Sabitrama's quest item (`4031020`, `4031032`) to a player on
  quest 2050 or 2051 who lacks it. Everyone also gets a random piece of
  clothing, weighted, for their gender or for both, some of them NX. They
  return the player to Ellinia.
- The pink, blue and white flowers (`1063000`-`1063002`) at the ends of the
  Deep Forest's steps give John's 10, 20 or 30 flowers to a player on
  quest 2052, 2053 or 2054 who has fewer. Everyone else gets a few ores.
  They return the player to the Mysterious Statue's forest (`105040300`).

The prize lists and weights come from Cosmic's scripts, and the shipped default is
recorded in `v83ref/lanes/QUESTIONS.md`. With no room, `patience.full`
replaces the visit and the player stays.

## Line 3's construction sites

Under Kerning City's station (`103000100`) are three construction sites,
B1 to B3, one for each of Shumi's quests (2055 to 2057, each asking for
an item lost down there):
- Jake (`1052006`) greets the player and then sells a ticket for 1000 mesos
  (`4031036`-`4031038`) to each site whose quest the player has begun or
  finished, in order. Short of the price he only says so.
- The ticket gate (`1052007`) asks first: the construction site or the
  subway. The site choice lists the site tickets held, takes the one picked,
  and warps to the site's first map (`103000900`, `103000903` or `103000906`)
  at `st00`. With none it only speaks.
- The way out (`1052011`) on each site's maps speaks, asks, and Yes goes back
  to the ticketing booth's portal 0; No gets a word.
- The box at each site's end (`1052008`-`1052010`, a `PATIENCE_PILES` entry)
  gives the quest's item (`4031039`-`4031041`) to a player on it who lacks
  it, else one random ore. It then returns the player to the ticketing
  booth without a word, or says `line3.chest.full` if there is no room.

The `line3.*` keys are `unsourced`, and the ids, price and prizes come from
Cosmic's scripts. `line3_construction_site.rs` checks B1 through the network
harness.

### Stunned and seduced players

A Stun or Seduce takes the client's input (HeavenClient's `disease::steer`),
so an honest client enters no portal by the Up key then. The server answers
CHANGE_MAP through such a portal, and CHANGE_MAP_SPECIAL, with only an empty
STAT_CHANGED that frees the client (`change_map::held_at`). Touch portals
(types 3 and 9) still work: v83 checks them every update, so a Seduce can
walk the player into one. Map-id warps and revivals aren't affected.

### Level limits

The v83 client reads neither `info/lvLimit` (672 maps) nor
`info/lvForceMove` (150), so the server applies them (`net::map_rules`).
Nothing shows how a v83 server did; these are the recommended defaults
from `v83ref/lanes/QUESTIONS.md` (row 196, 2026-10-22). Walking through a
portal into a map whose `lvLimit` the player hasn't reached is refused with
BLOCKED_MAP 2 ("You cannot go to that place."), as at Leafre's `west00`
(level 15). NPCs, scripts, rocks and map-id warps don't check it. A player of
level 20 or more who enters one of the five Training Centers (`910060000`,
`910120000`, `910220000`, `910310000`, `912030000`, five maps each,
`lvForceMove` 20) is sent at once to its `returnMap`, portal 0, by an
expired map timer. That happens on every entry, a login or a return from
the Cash Shop included; a player who reaches level 20 inside stays until
they next enter. The other `lvForceMove` maps (Kerning Square's 51,
the Mushroom Castle's 39) are left alone. GMs pass both checks.
`map_level_rules.rs` checks them through the network harness.

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
  it says to beat Guardian Nex first. Up on the `TD_neo` portal beside it
  (`TD_chat_enter`) opens the same conversation.
  The way to Guardian Nex is the forest's `in00` (`TD_neo_inTree`,
  "Guardian Nex's rooms" in `party-quests.md`). Each era but the first
  ends at a door to its boss (`TD_Boss_enter`, "Neo City's era bosses"
  there).

Their keys (`statue.*`, `crumbling.ask`, `time_gate.*`) are `unsourced`;
menu labels are map names (`#m<map id>#`). `statues_and_time_gate.rs` checks
them through the network harness.

## Tower rocks

Magic rocks in two towers move the player between floors, and each move
uses up one rock scroll. The scrolls are quest rewards: quest 3038 gives
the Orbis Rock Scroll (`4001019`) and quest 3205 the Eos Rock Scroll
(`4001020`).

- **Orbis Tower:** the Orbis Magic Spot (`2012014`, `200080200`) and the
  El Nath Magic Spot (`2012015`, `200082100`) lead to each other, arriving
  at portal 0.
- **Eos Tower:** there are four Eos Rocks. The First (`2040024`, 100th
  floor) goes down to the 71st, and the Fourth (`2040027`, 1st floor) up to
  the 41st. The Second (`2040025`, 71st floor) offers a menu with the 100th
  and the 41st. The Third (`2040026`, 41st floor) offers the 71st and the
  1st. Players arrive by the `go00` portal beside the target rock.

Without its scroll, a rock only speaks. Otherwise it asks before the move,
and a No keeps the scroll. The `tower_rock.*` keys are `unsourced`, and the
ids and targets come from Cosmic's scripts. `TOWER_ROCKS` in
`net/src/npc_scripts.rs` holds the table.

## Map and portal scripts

Map entry (`info/onUserEnter`, `scripts::map_enter`, which sees the
entering player) and touch portals (`pt` 9) run native scripts
from `net::scripts`, keyed by the script names in `Map.nx`; unknown names do
nothing. A script returns effects that the client actor sends:

| Script | Effect |
| --- | --- |
| `go<map id>` (entry) | Map name banner, FIELD_EFFECT `maplemap/enter/<map id>` |
| `TD_NC_title`, `TD_MC_title` (entry) | The Time Gate's and the Mushroom Kingdom's theme banners, FIELD_EFFECT `temaD/enter/<name>`: `teraForest` on `240070000`, `neoCity1`-`neoCity6` on `240070100`-`240070600`, `mushCatle` on `106020000`. Neither script is in Cosmic; the pairing is by name and by the maps' years |
| `TD_MC_Openning` (entry) | The Mushroom Kingdom's opening scene (`106020001`): sets record 2311 to "1", which The Test (2312) needs to start (see `quests.md`), locks the UI and sends the ten parts of `Effect.nx` `Direction2.img/open` (`Effect::Scene`, user effect 0x12, only to the player). The scene's `out` part makes the client ask for `106020000` by id 13 s in; that request is let through until the player's next change of map (`Player::scene_exit`), and LOCK_UI 0 follows the warp. If the client never asks (v83 drops a transfer sent within 500 ms of another request), the player stays in the scene's map; there is no server-side timer |
| `sealGarden` (entry) | The Sealed Garden (`920030001`): brings out its guardian (`9300348`) at (591, -34) for an Aran on 21739 who hasn't beaten it, unless one is out already. No source; Cosmic brings it out from `enterNepenthes`. The garden and its way are one player's maps (`SOLO_MAPS`) |
| `dollCave02` (entry) | The Puppeteer's arena (`910510202`): brings out the Puppeteer (`9300346`) at (95, 200) for an Aran on 21734 who hasn't beaten it, unless one is out already. No source; Cosmic brings it out from the passage's end statue (`1063017`) |
| `findvioleta` (entry) | The Wedding Hall (`106021600`): sets 2332's record to 1 while it is taken, and brings out the Prime Minister (`3300008`) at (292, 143) for one on 2333 who has not beaten him (`Effect::BringOutMob`) |
| `explorationPoint` (entry) | Lith Harbor's (`104000000`) name banner; in the Disciples' hideout (`100000006`), while 2175 is taken, the Disciple (`9300156`) comes out at (-1027, 216) unless one is out already (`Effect::BringOutMob`, under the player's control, never respawning). It also counts the map toward its region's exploration medal (`net::medals`, see "Exploration medals" in `quests.md`) |
| (entry, no `onUserEnter`) | `scripts::unscripted_map_enter`, keyed by map id: the Puppeteer's hideout (`910510000`) brings out the Puppeteer being hunted (see `enterDollcave` below), and Lith Harbor's Info Shop (`104000004`) brings out the Puppeteer (`9300345`) at (0, 120) for an Aran on 21733 who hasn't beaten it, unless one is out already (Cosmic `enterInfo`). Its fall sets record `21762` to `2` (`net::quests::kill_records`), which 21733's completion reads. The Sealed Shrine (`925040100`, solo) brings out the Shadow Knight (`9300351`) at (897, 51) for an Aran on 21747 who hasn't beaten it. Ellin's warehouse (`300000011`), once 21751 is completed, sets record `21764` to `1` and brings out the Gentleman (`9300353`) at (200, 100), once. The Bowman Instructional School (`100000201`), once 21753 is completed, sets record `21765` to `2`. No source sets either record; they are what 21753 and 21754 need to start |
| `highposition` (touch) | Counts the map toward Touched the Sky (29004); it takes the player nowhere |
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
| `TD_MC_first` | The Empty House's (`100000002`) way to the Mushroom Kingdom, from level 30 (else a pink line): portal sound, then the opening scene's map (`106020001`) until record 2311 says it was seen, and `106020000` after that (Edelstein's script) |
| `obstacle` | The poisonous mushrooms on `106020300`: once a Killer Mushroom Spore opened them (marker 32500, see `inventory.md`, "Scripted items"), portal sound, warp to `106020400` portal `left00`; with a Poison Spore (`4000507`), a pink line, portal sound, take one, the same warp; else a pink line |
| `gotocastle` | Split Road of Destiny's way over the castle wall: portal sound, warp to `106020501`, once 2324 is completed (with the Secretary of Magic, after the Thorn Remover); while 2322 is under way, to the Thorny Wall's Edge (`106020500`), where the wall is looked at (no source names that map's way in; assumed); else a pink line |
| `investigate1`, `investigate2` | The barrier on `106020300` and the castle wall on `106020500`: while 2314 or 2322 is under way and its record is not yet 1, set it to 1 (`Effect::QuestRecord`), show the player's thought (`Effect::Think`, the speaker's 0x2 bit) and the `mushroomcastle/chatBalloon1` or `2` effect; else nothing |
| `TD_MC_jump` | Portal sound, warp to `106020501` |
| `TD_MC_faild` | The trap (`trap00`) on the guards' watch (`106020601`): the guard's look (`Effect::ScenePart`, user effect 0x12 `Direction2.img/mushCatle/nugu`), a chat-log line (`Effect::InfoText`), and two seconds later a warp to the foot of the Shadow Cliffs (`106020403`; `Effect::WarpAfter`, the client actor's map timer, so leaving the map first cancels it). Not in Cosmic; after Kinoko's `MushroomCastle.java` |
| `find_james` | The Central Castle Tower's (`106021201`): until James's Whereabouts (1) (2325) is completed, James calls out under the `mushroomcastle/chatBalloon2` effect; else nothing. After Kinoko |
| `TD_MC_Egate` | Portal sound, warp to `106021300` portal `right00` |
| `subway_in2` | Kerning City's ticketing booth (`103000100` `in00`) into the tunnels: portal sound, warp to `103000101` portal `out00` |
| `met_in` | The construction site's tunnel (`102040000` `subway00`): portal sound, warp to the Abandoned Subway Station (`910320000`) portal 2 (Cosmic's commented line; `agent_docs/pyramid.md`) |
| `met_out` | The station's `out00`: forget `DOJO_RETURN`, portal sound, warp to the town it saved, or else to `102040000` portal 12 |
| `metro_in00` | The station's `metro00`: opens Mr. Lim (1052115) |
| `metro_Chat00` | The station's `chat00`: the `Direction2.img/metro/Im` effect (`Effect::ScenePart`), which Cosmic lacks |
| `inERShip` | Ellinia's (`101000000` `in04`) way down to its Sky Ferry dock: portal sound, warp to `101000400` portal `out00` |
| `enterPort` | Rien's pier (`140020200` `east00`) onto the dock: portal sound, warp to `140020300` portal `west00`. While 21301 is taken with no Thief Crow kill, into the thief's solo room (`108010700` `west00`) instead, with a pink line when someone is in it |
| `rankRoom` | A job hall's (or Ereve's, or the Dangerous Forest's) way into its room: portal sound, warp to the room's portal back (`RANK_ROOMS`) |
| `timeQuest` | A Temple of Time lane's way on (`TIME_LANES`, 16 portals `in00` on `270010100`-`270040000`): portal sound, warp to the next map's `out00` once the lane's quest (3501-3504, 3507-3511, 3514-3519) is completed; the Broken Corridor's with a Marble of Chaos (`4032002`) or 3522 completed. Else portal sound, a pink line, back to `270010000`/`270020000`/`270030000` `in00`. The Temple itself is only reached by Corba's flight, which isn't in |
| `enterAchter`, `enterMagiclibrar`, `balogTemple`, `enterNepenthes` | Henesys Park's way into the Bowman Instructional School (`100000201` `out02`), Ellinia's into the Magic Library (`101000003` `jobout00`; while 20718 is taken and its shadow not yet found, into the hidden Magic Library `910110000`, one player at a time, or a pink line while it's held), the dungeon's into the Underground Temple's stairway (`105100000` `top00`), Orbis's into the Neglected Strolling Path (`200060001` `out00`): portal sound and warp. On the last, an Aran on To the Sealed Garden (21739) goes to the way into the Sealed Garden (`920030000` `out00`) instead, while it and the garden (`920030001`) are both empty, or gets a pink line |
| `outMagiclib` | The hidden Magic Library's way out (`910110000` `out00`): a pink line while any mob there stands; else portal sound, warp to Ellinia `101000000` `jobin00`. Its black shadow (1032109), with 20718 taken, sets record 20732 to "1" and brings out 30 Blue Mushrooms |
| `ariant_castle` | Ariant Castle's door: with an Entry Pass to the Palace (`4031582`, kept), portal sound, warp to `260000301` `out00`; else a pink line |
| `enterDisguise0` | Ereve's Crossroads into the Training Forest (`130010000` `east00`): portal sound and warp, except while a Master of Disguise quest (20301-20305) is taken without an Ereve Investigation Permit (`4032179`): a pink line |
| `enterSecondDH` | The Entrance to the Drill Hall's way in (`130020000` `in01`): for a player on a Knighthood Exam (20201-20205), takes the exams' tokens held (4032096-4032100), then portal sound and warp to one of the 2nd Drill Halls (`108000600`-`108000602`, portal 0) at random; else nothing (Cosmic's). Kiriko (1102001) in each hall takes the player back |
| `enterWitch` | Leafre's nest into the Cave of Black Witches once 20404 is completed: `924010200` after 20407, `924010100` after 20406, else `924010000`, portal `out00`; else a pink line |
| `TD_MC_enterboss1` | The castle tower's way up: while 2330 is taken and a Yeti brother is still unbeaten, portal sound and into King Pepe's room (`EventKind::KingPepe`, see [party quests](./party-quests.md#king-pepes-room)), or a pink line while another player is in it; else portal sound, warp to the Wedding Hall entrance `106021401` `out00` |
| `TD_MC_enterboss2` | The Wedding Hall's door (`106021402`): locked with a pink line before 2332; from 2332 on until 2333 is done, portal sound and into the player's own copy of the hall (`EventKind::WeddingHall`, see [party quests](./party-quests.md#the-wedding-hall)), or a pink line while another player is in it; after 2333, portal sound, warp to `106021600` `left00`; after "The Seal of the Empire" (2331), the Blocked Entrance (`1300013`)'s menu for the Prime Minister's refight (`EventKind::PrimeMinister`, see [party quests](./party-quests.md#the-wedding-hall)) |
| `out_pepeking` | King Pepe's room's way out: the Wedding Hall key (4032388) and a pink line once 2330's three brothers are beaten and none is held (shut with a pink line if there's no ETC slot), then portal sound, warp to `106021400` `TD_MC_enterboss1` |
| `ludi021` | The Secret Passage's way out (`922000009` `out00`): takes any Machine Parts (`4031092`) left from the Toy Factory's Sector 4, then portal sound and warp to `220020600` |
| `secretDoor` | Magatia's lab doors (`261010000`, `261020200` `secret00`): while 3360 is taken and its info is Parwen's password, the Secret Passage (2111024) asks for it, and the right one sets the info to `1`, then portal sound and warp to `261030000` `sp_jenu` (from Zenumist's) or `sp_alca`; a wrong one is "Wrong!" (Cosmic `MagatiaPassword.js`). Once given, or with 3360 done, the door just leads there; else a pink line |
| `magatia_dark0` | The dark pipe on `261020600` (`up00`) into Yulete's lab: portal sound, warp to `926130000` `out00`, once Yulete's traces in the Romeo and Juliet party quest have shown the player the way (marker 32600, `party-quests.md`; Cosmic completes quest 7770, which isn't in `Quest.nx`); else a pink line. The lab's quest rooms are below |
| `q3366in`, `q3367in`, `q3368in` | The Lab Entrance's doors (`926130100` `in00`-`in02`) into Yulete's quest rooms, as Cosmic's: a player on Making the Reagent (3366), Organizing the Lab Report (3367) or Eliminating Obstacles (3368) goes with the portal sound to Lab 1, 2 or 3 (`926130101`-`926130103`, portal 0); anyone else gets a pink line. Lab 2's door first gives back the Yulete's Reports (`4031797`) a player has found (`quests.md`) but no longer holds, or with no room for them gives a pink line instead |
| `q3366out`, `q3367out`, `q3368out` | The rooms' exits (`926130200`, `926130201`, `926130203`, `out00`) lead back to the Lab Entrance's `in00`, `in01` and `in02`, with the portal sound |
| `curseforest` | The Forest of Evil I's hidden portal (`100040105` `secret00`) into the Cursed Forest, for players with 2224 or 2226 taken or 2227 done: portal sound, warp to `910100001` (after 2227) or `910100000`, portal `out00`, from 17:00 until 7:00 by the stations' clock (route time, UTC); by day, or off those quests, a pink line |
| `elevator` | The Helios Tower's floors (`222020100`, `222020200`, `in00`): portal sound and into the floor's waiting room (`222020110`, `222020210`, portal 0) while the elevator's car is docked there, else a pink line; the car takes the waiting room up or down (`ships.md`) |
| `rienTutor8` | Rien's way out (`140000000` `west00`): portal sound, warp to `140010000` `east00`; an Aran beginner (job 2000) on Basic Fitness Training 1-3 (21015-21017) stays, with a chat line (SHOW_STATUS_INFO 9, `Effect::InfoText`) saying which way the training lies |
| `tutorHelper` | Ereve's tutorial's first map (`130030000` `scr00`): brings out the guide, Mimo (SPAWN_GUIDE `0xDF`), who says the `portal.ereve_tutor.guide_welcome` line (TALK_GUIDE `0xE0` mode 0, 200 px wide, 4 s), once a session |
| `tutorMinimap`, `tutorWorldmap` | The tutorial's touch portals on `130030001` and `130030006`: the guide points at the minimap and the world map (TALK_GUIDE mode 1, `UI/tutorial.img` 1 and 11, 7 s), once a session |
| `tutorquest` | The doors of `130030001`-`130030004` (`west00`): warp to the next map's portal 0 once Kimu's Welcome to Ereve (20010) is taken, or once Kizan's, Kinu's or Kia's quest (20011-20013) is done; otherwise a pink line saying to take or finish the quest. |
| `enterRienFirst`, `rienTutor7` | The Dangerous Forest's (`140010000`) ways out: `east00` back to Rien (`140000000` `west00`, or `st00` for an Aran beginner who hasn't completed 21014), `west00` on to `140010100` portal 2; an Aran beginner before 21014 isn't let on, with an `Effect::InfoText` line pointing back to town |
| `OutPerrion_1`, `OutPerrion_2` | The Bottom of the Temple's (`105100100`) ways back up the stairway (`105100000`): `OutPerrion` with a pink line to portal 2, `top001`/`top002` to portal 0 |
| `foxLaidy_map` | Fox Ridge's (`222010300`) `west00`: to Tiger Ridge (`222010200` `east00`), or with 3647 taken and an Old Fox's Tail (`4031793`), into the Gloomy Forest (`922220000` `east00`). Cosmic also starts quest 23647, which isn't in `Quest.nx` |
| `move_elin`, `minar_job4`, `gryphius`, `mayong`, `Pianus`, `hontale_morph2`, `eliza_Garden`, `catPriest_map`, `enterInfo` | Ways through (`PLAIN_WARPS`): the Time Control Room's `in01` into Ellin Forest (`300000100` `out00`, with a pink line), Leafre's to the Forest of the Priest, Griffey's and Manon's forests (`240010501`, `240020101`, `240020401` `out00`), Aqua Road's to the Cave of Pianus (`230040420` `out00`), the Horntail cave's back to the Peak of the Big Nest (`240040600` portal 4), Eliza's Garden (`920020000` portal 2), the Forest of the Ghost Priest (`925000000` portal 2), Lith Harbor's Info Shop (`104000004` portal 1; 21733's Puppeteer comes out on entry, see the unscripted entry row) |
| `enterGym`, `rienCaveEnter` | The second Dangerous Forest's (`140010100`) way to Puo's training grounds (`914010000`/`914010100`/`914010200` portal 1 while 21701/21702/21703 is taken), and the Mirror Cave's (`140030000` portal 1, while 21201 or 21302 is taken); else a pink line |
| `enterMCave`, `moveNext`, `moveBefore` | The Mirror Cave's (`140030000`) `in00`, while 21201 is taken: into the first Mirror of Desire room (`108000700` to `108000708` `out00`) that, with its Scarred Bears' room (`108000710` to `108000718`), nobody is in, setting record 21203 to "0"; a pink line when every pair is taken, or without the quest. While 21302 is taken and 21303 isn't done, into Yeti's room (`108010701` `out00`) when it and the thief's solo room (`108010702`) are empty, setting 21203 to "1". Each room's `west00` leads to its bears' room's `east00` and that back to the room's `west00` |
| `outMaha` | Maha's battle's (`914020000`) `out00`: portal sound, warp to Rien (`140000000`) portal 0 |
| `dragonNest`, `go_secretroom`, `tristanEnter`, `q2073`, `enterDollWay`, `ariant_Agit`, `s4resur_enter`, `s4rush` | Doors a quest opens, else a pink line: the Nine Spirit's Nest (`240040612` `out00`, 3706 completed; before that, with the egg or after placing one, the egg's nest alone, see [horntail](./horntail.md#the-nine-spirits-nest)), the Mushroom Castle's Security Room (`106021001` portal 1, 2335 completed, or taken with its key `4032405`, which Violetta's `q2335s` gives and which completes it, gives 5000 EXP and takes the key), Tristan's Resting Place (`105100101` `in00`, 2238 completed), Utah's Pig Farm (`900000000`, 2073 taken), the Cave of Evil Eye III (`105070300` portal 3, 20730 or 21734 completed) or the Puppeteer's Secret Passage (`910510100`, 21734 taken; its end statue `1063017` asks and lets one challenger at a time into the Puppeteer's arena `910510202` portal 0, a solo map, and its start statue `1063016` asks and leads back to `105040201` portal 2), the Sand Bandits' house (`260000201` portal 1, 3928, 3931 and 3934 completed), the Forgotten Darkness (`922020000`, 6134 taken), the Secret Shrine (`910500100`, 6110 taken). Several of these quests aren't supported yet (2238), so their doors stay shut for now |
| `tutorialNPC`, `TD_chat_enter` | Open an NPC's conversation without a click (`Effect::OpenNpc`), unless one is open already: the `tutorial` portal in each job instructor's room (`101000003`, `100000201`, `102000003`, `103000003`, `120000101`) has a beginner up to level 10 hear the instructor (`1032001`, `1012100`, `1022000`, `1052001`, `1090000`); Leafre's `TD_neo` (`240070000`) opens the Time Gate (`2083006`) |
| `enterBackStreet`, `outSpecialSchool`, `enterWarehouse` | The Mu Lung Dojo hall's (`925020000`) `back00`: for an Aran on 21744 with 21745 completed, or from 21744's completion until 21747 is completed (Cosmic opens it only on 21747, which shuts out an Aran on Mu Gong's Test), portal sound and Mu Gong's back street (`925040000` portal 0); else a pink line. Mu Gong's arena (`925040001`, solo, entered through `q21746s`) leads back to the back street's `st00`. Ellin's Conference Hall (`300000010`) `in00` leads into the warehouse (`300000011` portal 0). The Sealed Shrine's pillar (`2091009`, in `250020300`) asks for a password, in any case and with or without its ".". Right, it lets an Aran on 21747 who hasn't beaten the Shadow Knight into the shrine (`925040100` portal 0, solo); anyone else hears it's right in pink, and a wrong one is told so |
| `enterRider`, `clearRider`, `outtestWolf` | An Aran's wolf runs, each solo (`SOLO_MAPS`) with a 3-minute clock back to the map's `forcedReturn` (Cosmic's event time). The Wolf-Shaped Rock's (`211050000`) `in00`: on "Scadur's Advice" (21610) before record 21619 reads "0", into the Field of Wolves (`921110000` `st00`), else a pink line. Its touch portal `clear00` at the far end (`clearRider`, no source) sets 21619 to "0" for the quest's end, with a pink line; `clear01` leads back out. The wolves' test room (`914030000`, entered through `q21613s` or the pack's wolf 1202008 while 21613 is unpassed) has 30 wolves (`9300354`, spawned once); its `out00` stays shut with a pink line while one is left, then sets record 21620 to "0" on 21613 and leads to `140010210` portal 0. Cosmic gives a token (`4001193`) for both instead of the records |
| `enterDollcave` | The Cave of Evil Eye III's (`105070300`) `in00`: with 20730 or 21734 completed, portal sound and back to `105040201` `in00`; else the Cave Wall of Evil Eye (`1063011`) asks for the Puppeteer's password, as 20730 spells it out (any case, with or without its "!"). Right, it lets a player hunting the Puppeteer (20730 or 21731 started, none killed) into Francis's room (`910510001` portal 1); anyone else hears it's right in pink, and a wrong one is told so. With 21728 taken, the wall is searched instead, which sets record `21761` to `0`, as its completion needs. Francis (`1104000`) sends the intruder to the Puppeteer's hideout (`910510000` portal 1, solo, no timer), where the Puppeteer (`9300285` for 20730, `9300344` for 21731) is brought out at (680, 258) on entry. |
| `thief_in1` | The Red Scorpions' door in the Scorching Desert (`260010401` `center00`): the Mysterious Voice (`2103008`, also when clicked) asks for the password. "Open Sesame" (any case, with or without its "!") from one who has done True or False (`3925`) gives the portal sound and leads into the hideout (`260010402` portal 1), where the Treasure Box starts `3926`; anyone else hears it's right in pink, and a wrong one is told so. Sirin only says the player knows it already; the word is Cosmic's (`ThiefPassword`) |
| `s4resur_out` | The Forgotten Darkness's way out (`922020000` `out00`) gives the portal sound and leads to `220070400` portal 3. A Bishop on Resurrection (`6134`, taken) is first given its proof `4031448`, which the quest's end takes; with no room in the Etc tab, a pink line and the portal stays shut |
| `inNix1`, `inNix2`, `outNix1`, `outNix2` | Griffey's Forest (`240020101`, `240020102` `in00`) and Manon's Forest (`240020401`, `240020402` `in00`) lead into the Isolated Forest (`240020600` `out00` and `out01`), and its two ways lead back to `240020101` and `240020401` `in00`, each with the portal sound |
| `tamepig_out2` | The Watch Hog's room's exit's way out (`923010100` `out00`): a player on Monster Riding (`6002`, started) holding five Pheromones (`4031507`) and five Kenta's Reports (`4031508`) keeps five of each. Any others, and all of them for anyone else, are taken. Then the portal sound and the warp to the Aquarium's zoo (`230000003` `out00`). See [party quests](./party-quests.md), "The Watch Hog's room" |
| `s4firehawk`, `s4iceeagle`, `s4tornado_enter`, `s4resurrection`, `s4nest`, `s4hitman`, `s4berserk`, `s4berserk_move` | The fourth job's solo rooms (`scripts::SOLO_MAPS`), portal 0 unless said, one player at a time (a pink line while someone is there), each starting afresh when left: the Fire Hawk's nest (`211042200` `in00` to `921100200`, 6240 taken) and the Ice Eagle's (`211040700` `in00` to `921100210`, 6242 taken), else a pink line; the Tornado room (`220010001` `pt00` to `922020200`, 6230 or 6231 taken, or holding `4001110`), else nothing; the resurrection mound (`230040300` `in01` to `923000100`, holding a Cold Flame `4001108`, quest or not, as in Cosmic) and the nest of the eggs (`240010700` `pt00` to `924000100`, 6241 or 6243 taken), else a pink line. Their reactors take items (`combat.md`). The Hitman's hideout (`101030104` `pt00` to `910200000`, 6201 taken), else a pink line; its boxes lead up its floors. The Berserk room (`105090800` `in00` to `910500200` `out01`, 6153 taken and holding its key `4031475`), else a pink line; its way up to the statues, `s4berserk_move` (`in00` to `out00` within the map), stays shut with a pink line while a monster is left (`Effect::RequireNoMobs`, asking the field for its live mob count) |
| `skyrom` | Ariant's Treasure Room of Queen (`926000010`): with 3935 taken and no Queen's jewel (`4031574`), portal sound and in, one player at a time (a pink line while someone is there); else nothing |
| `enterDisguise1`-`enterDisguise5` | Ereve's Training Forests' ways on to the next (`130010010`, `130010020`, `130010110`, `130010120` `out00`, `130020000` `east00`); while a Master of Disguise quest (20301-20305) is taken, into the Knight's hideout instead (`108010600` + 10 per job, `out00`, the fifth `east00`), one player at a time, with a pink line if someone is there or the player holds his proof (`4032101` + job) already |
| `minar_elli` | The shortcut between the Field Up North of Ellinia (`101010000` `minar00`) and Leafre's Minar Forest West Border (`240010100` `elli00`): spends a Magic Seed (`4031346`) with a pink line, portal sound, and on to the other side's portal; without one, a pink line |
| `enter_earth00`, `enter_earth01` | The Nautilus's Navigation Room (`120000101` `earth01`) to Omega Sector's Command Center (`221000300` `earth00`) and back, for a holder of a Warp Card (`4031890`, kept); else a pink line. Cosmic sends that line as type 6; it's pink here, as Edelstein's scripts send theirs |
| `Masteria_U2_1`, `Masteria_U3_1`, `Masteria_U5_1`, `Masteria_U5_2`, `Masteria_U6_1`, `Masteria_B1_1`-`Masteria_B3_1`, `Masteria_CM1_A`-`Masteria_CM1_D`, `Masteria_CM2_B`-`Masteria_CM2_E`, `Masteria_CC1_A`, `Masteria_CC6_A` | The Phantom Forest's hidden paths (`610010001`-`610010005`, `610010200`-`610010202`, `s` portals) lead a holder of its map (`3992040`, from Jack on Finding Jack) on, as Cosmic's do: `sU2_1` and `sU5_2` to Dead Man's Gorge's `U5_1` (`610010004`), `sU5_1` back to `610010001` `sU2_1`, `sU3_1` to `610010201` `sB2_1`, `sU6_1` to `610010002` `sU3_1`, and the Crossroads' `sB1_1`-`sB3_1` to `610010005` `sU6_1`. The Valley of Heroes' statues (`610020000` `CM1_A`-`CM1_D`, `610020001` `CM2_B`-`CM2_E`) carry a holder of the Marker of Heroism (`3992039`) up to the next one, the last to `CM1_E` and `CM2_F`. Neither item is taken; without it nothing happens. The Cavern of Fear's `CC1_A` (`610020010`) and the Cavern of Valor's `CC6_A` (`610020015`) lead to each other for anyone. All with the portal sound |
| `ariant_queens` | Ariant's palace rooms (`260000301`-`260000303`, `ring00`-`ring06`): nothing for a player in Tigun's disguise (the morph of item `2210005`, a buff of id `-2210005`); anyone else is turned out to Ariant Castle's `R001` (`260000300`) with portal sound and a pink line |
| `undodraco`, `templeenter`, `outTemple`, `undomorphdarco` | Corba's dragon flight (report a014). Corba (`2082003`, Leafre Station `240000110`) puts item `2210016`'s effect on the player without taking one (Morph 16, a buff of id `-2210016`, `Effect::ApplyItemBuff`) and warps them to the flight `200090500`. Its `downNN` portals (`undodraco`) end the morph (`Effect::CancelItemBuff`) and drop them at the station's `po00`; Minar's flight `200090510` `in00` (`templeenter`) ends it and goes on to the Temple of Time's road (`270000100` `out00`), whose `out00` (`outTemple`) puts it back on for the flight home (`200090510` portal 0). The station's map-entry script `undomorphdarco` has no source; it ends the morph (QUESTIONS 2026-10-21 row 198, question 3) |
| `advice00`-`advice09`, `adviceMap` | The original Maple Island's instruction balloons (moving, Heena, jumping, climbing, Sera, the quest window, portals, the World Map, stats, jumping down), each a PLAYER_HINT with Cosmic's width and height; no warp |
| `nets_in`, `nets_out` | Nett's Pyramid's lobby, the Pyramid Dunes (`926010000` `out00`), from Sahel 3's `piramid00` (`260020500`), saving the map in `saved_locations` kind 3 (`PYRAMID_RETURN`); its `out00` goes back there (Sahel 3 by its `piramid00`, else the map's spawn point), or to Sahel 3 when nothing was saved. The party quest itself isn't in |
| `MD_pig`, `MD_golem`, `MD_mushroom`, `MD_drakeroom`, `MD_rabbit`, `MD_roundTable`, `MD_remember`, `MD_protect`, `MD_treasure`, `MD_sand`, `MD_error`, `MD_high` | A mini-dungeon's way in (the base map's `MD00`) and out (each copy's `out00`); see below |

A portal that takes one player at a time puts `Effect::RequireEmptyMap`
first; when someone is there, nothing applies and the client actor says the
outcome's `occupied` line in pink instead. Leafre Station's `reundodraco`
portals do nothing in Cosmic either, so they're left unserved.

### Mini-dungeons

Twelve maps have a mini-dungeon behind their `MD00` portal
(`net::dungeons::DUNGEONS`: `100020000`, `105040304`, `105050100`,
`105090311`, `221023400`, `240020500`, `240040511`, `240040520`,
`251010402`, `260020600`, `261020300`, `551030000`). Each
dungeon has 19 to 34 copies, maps of their own (`base`'s `first` and on)
that take their body from one map through `info/link` (see `fields.md`)
and carry their own `forcedReturn` (the base map) and `timeLimit` (7200 s).

On the base map, `MD00` asks the directory for a copy
(`DirectoryMessage::EnterDungeon`). A party member who doesn't lead their
party is refused with a pink line, and so is anyone when every copy is
taken. Otherwise the directory picks the first copy on the player's
channel with nobody in it and not kept for a team on its way (10 s), keeps
it for the player, who goes in with the portal sound. The party members
who were beside the leader follow to the copy's `out00` once the leader is
in (`ServerMessage::ShipWarp`). A copy's `out00`, like `MD00` anywhere but the base map,
leads back to the base map's `MD00`. When the player who took the copy
leaves it (by map change, logout or logging in again),
everyone still in it is sent to the base map's portal 0. If the leader
hands the party to a member in the copy, the copy goes with it; if the new
leader is anywhere else, the run ends the same way, as in Cosmic
(`WorldDirectory::dungeon_leader_changed`). Nobody in a copy may change channel or
enter the Cash Shop: both get the client's own refusal (BLOCKED_SERVER 1
and 2). An empty copy puts its mobs back, as a
clone's room does (`npc_scripts::resets_when_empty`). The clock is each
player's own `MapTimer`, and a login into a copy lands
on its forced return map.

Not yet: the time limit runs per player rather than per copy (the members
follow the leader in within seconds, so the clocks hardly differ).

Only the `go<map id>` banners are used by the current maps, on the Adventurer
Training Centers and the revamp's Southperry (`2000000`). The other scripts,
and `tutoChatNPC`, were for the revamped island's maps and are unused. The
original island's touch portals run `advice00`-`advice09` and `adviceMap`
(see the table above).

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
