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

Scripted portals and map-entry scripts need separate implementations; the
presence of a script name alone does not provide its behavior. Explicit direct
warps remain available for existing development/harness workflows.

`beginner_route_and_ninas_house_follow_asset_links` checks the active route,
the house round trip, and exact arrival portal IDs through the network harness.
