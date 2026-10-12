# Asset-backed map travel

Ordinary exits use `Map.nx/Map/Map*/<id>.img/portal`: `pn` identifies
the source portal, `tm` the destination map, and `tn` the arrival portal.
The server resolves these links; no handwritten map connection table exists.
A nonempty `tn` must match an actual destination portal. Only an empty `tn`
may use the map's default spawn point. The client skips portals without a
travel destination when selecting an exit, including overlapping spawn points.

The active beginner route in these assets is:

New Explorer characters start in `10000`, at the entrance to this route. Existing
characters keep their saved maps. Starting new Explorers in Amherst (`1000000`)
skipped the tutorial quests, whose one-way entrances cannot be reached from town.

`10000 → 20000 → 30000 → 40000 → 50000 → 1000000 → 1010000 → 1020000 → 2000000`

Nina is in `30000`; its `in01` enters Sen's house (`30001`), whose
`out00` returns to Nina. The tutorial segment through `50000` has one-way
entrances. The main path from `50000` onward has reverse exits. These assets
also retain older disconnected map variants (`40001`, `40002`, `50001`,
`60000`, etc.). Do not insert them into the active route or invent reverse
connections based on their presence in the archive.

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

HeavenClient sends CHANGE_MAP_SPECIAL (`0x64`, portal name) when the player
first touches a script portal, and the server answers with the effects and an
enable-actions stat update. An `info*` hint runs once per session after it
was shown; the exit warning repeats. Hint text comes from the
[server content pack](./server-content.md). `tutoChatNPC` (the skip offer)
belongs with scripted NPCs. Explicit direct
warps remain available for existing development/harness workflows.

`beginner_route_and_ninas_house_follow_asset_links` checks the active route,
the house round trip, and exact arrival portal IDs through the network harness.
