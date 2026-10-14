# Asset-backed map travel

Ordinary exits use `Map.nx/Map/Map*/<id>.img/portal`: `pn` identifies
the source portal, `tm` the destination map, and `tn` the arrival portal.
The server resolves these links; no handwritten map connection table exists.
A nonempty `tn` must match an actual destination portal. Only an empty `tn`
may use the map's default spawn point. The client skips portals without a
travel destination when selecting an exit, including overlapping spawn points.

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

Only the `go<map id>` banners are used by the current maps, on the Adventurer
Training Centers and the revamp's Southperry (`2000000`). The other scripts,
and `tutoChatNPC`, were for the revamped island's maps and are unused. The
original island's touch portals run `advice00`-`advice09` and `adviceMap`,
which have no implementation yet, so they do nothing.

HeavenClient sends CHANGE_MAP_SPECIAL (`0x64`, portal name) when the player's
feet first enter a script portal's box, and the server answers with the effects
and an enable-actions stat update. As in the original, the box is the portal's
`hRange` by `vRange` (100 by default) centred on it, so the triggers that sit
30-40 px above the floor fire as the player walks past. An `info*` hint runs once per session after it
was shown; the exit warning repeats. Hint text comes from the
[server content pack](./server-content.md). `tutoChatNPC` (the skip offer)
belongs with scripted NPCs. Explicit direct
warps remain available for existing development/harness workflows.

`beginner_route_and_ninas_house_follow_asset_links` checks the active route,
the house round trip, and exact arrival portal IDs through the network harness.
