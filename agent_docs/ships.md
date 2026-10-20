# Ships

The continent ships (v83 `ContiMove`) carry players between continents on a
fixed timetable. Orbis is the hub: ships run from it to Ellinia, Ludibrium,
Leafre and Ariant, and back. A station's wall clock shows the time of day.

## Timetable

Every channel runs its boats on the same clock, so a trip's state is a
function of the time alone (`net/src/conti.rs`). Any field or client actor
can work it out without shared state.

A ship leaves Ellinia for Orbis and one leaves Orbis for Ellinia every 15
minutes, the first on the hour. Each trip:

| Minutes from departure | State (`conti::State`) | What happens |
| --- | --- | --- |
| 0 to 10 | `Move` (3) | Under way; the waiting room is moved onto the deck at 0 |
| 10 | `End` (6) | Riders are set down at the far station's `come00` |
| 10 to 14 | `Wait` (1) | Boarding |
| 14 to 15 | `Start` (2) | Boarding closed; the ship leaves the platform |

`End` is only ever a change, never a state a client is told. `Dormant` (0),
between an arrival and boarding, takes no time on this route.

The other routes (`ORBIS_BOATS`) leave every 10 minutes: five minutes
boarding, closed for the last minute, and five minutes at sea.

A `Route` names its platform, waiting room, deck, cabin (only the Ellinia
boats have one), arrival map and portal, the platform where it docks at the
other end, its ticket and its fare:

| Route | Boards at | Deck | Arrives at | Fare |
| --- | --- | --- | --- | --- |
| `TO_ORBIS` | 101000300 | 200090010 | 200000100 | 5,000 |
| `TO_ELLINIA` | 200000111 | 200090000 | 101000300 | 5,000 |
| `TO_LUDIBRIUM` | 200000121 | 200090100 | 220000100 | 6,000 |
| `LUDIBRIUM_TO_ORBIS` | 220000110 | 200090110 | 200000100 | 6,000 |
| `TO_LEAFRE` | 200000131 | 200090200 | 240000100 | 30,000 |
| `LEAFRE_TO_ORBIS` | 240000110 | 200090210 | 200000100 | 30,000 |
| `TO_ARIANT` | 200000151 | 200090400 | 260000100 | 6,000 |
| `ARIANT_TO_ORBIS` | 260000100 | 200090410 | 200000100 | 6,000 |

The fares and the 10-minute timetable come from Cosmic and have not been
checked against v83.

`conti::now()` is route time in milliseconds: the wall clock times
`RUSTMS_CONTI_SCALE` (a whole number, default 1). The harness world sets it
to 60, so a 15-minute cycle takes 15 seconds (`tests/ships.rs`).

## Server

- A field on a route map (`conti::has_trips`) reads the timetable on every
  tick (`run_trips` in `runtime/src/actor/field.rs`). `conti::happenings`
  turns the changes since the last reading into what the map does:
  - on the platform, the ship leaving at `Start` (CONTI_MOVE 8, 2);
  - on the arrival platform, the other ship docking at `End`
    (CONTI_MOVE 12, 6). Ellinia's platform is both, so it sees each;
  - in the waiting room at `Move`, and on the deck and in the cabin at
    `End`, everyone is moved on (`ServerMessage::ShipWarp`).
- The client actor (`runtime/src/actor/client/ships.rs`) warps on
  `ShipWarp` only if the player is still on that map. A player in the Cash
  Shop stays where they are.
- A player who logs in on a map with a `forcedReturn`, such as a deck
  under way, starts on that map instead.
- 0xDC (`ContiStateRequestHandler`) is the client asking about its own map's
  ship on entering it. The server answers CONTI_STATE with the state of the
  route whose platform, arrival platform or deck it is, flag 0.
- On joining a map with a `clock` node, the client actor sends CLOCK kind 1
  with the route time of day in UTC (`set_wall_clock`).

The NPCs are in `net/src/npc_scripts.rs`, with placeholder lines under the
`ships.*` content keys:

- Joel (Ellinia), Mel (Ludibrium), Mue (Leafre) and Syras (Ariant) sell
  tickets to Orbis. Agatha sells Orbis's four from one menu.
- Each platform's usher (`USHERS`) takes a ticket and shows the holder to
  the waiting room while the ship boards. A yes that comes after boarding
  closed is refused by the `RequireBoarding` effect.
- The waiting-room guides take people back to the platform of the room they
  are in (`conti::waiting_in`). The ticket is not returned.
- Isa points the way to Orbis Station's four platforms.
- Leafre Station's `dracoout` portal leads back into Leafre.

## Packets

| Op | Direction | Body | Meaning |
| --- | --- | --- | --- |
| 0xDC | client to server | `i32` map id, `u8` ship kind | Where is this map's trip? |
| 147 CLOCK | server to client | `u8` kind 1, `u8` hour, minute, second | Set the wall clock |
| 148 CONTI_MOVE | server to client | `u8` target, `u8` event | The trip changed |
| 149 CONTI_STATE | server to client | `u8` state, `u8` flag | The trip's state, on entry |

The client only checks that its map's ship is of the right kind; the target
is not compared with the map. Other CLOCK kinds (countdowns) are not sent
or drawn yet.

## Client

`Gameplay/MapleMap/MapShip` shows a map's ship. It is built only when
`info/fieldType` is 2 and the map has `shipObj/shipObj`. The path in that
node is resolved in `Map.nx`, with `.img` added to the folder after `Obj/`
(`ship::obj_path`). The rules are in `ShipRules.h` and tested in
`tests/ships.cpp`.

- A docking ship (`shipKind` 0) is drawn behind everything but the
  backgrounds. It starts at `x0`, slides in to `x` on Enter and back out on
  Leave over `tMove` seconds, and whistles (`Sound.nx Object.img/Whistle`).
  CONTI_MOVE (8, 2) is Leave and (12, 6) Enter. CONTI_STATE states 0, 1 and
  6 are Enter, 2 to 5 Leave.
- The Crimson Balrog's ship (`shipKind` 1) is hidden until it appears: it
  fades in sliding from 100 px up and to the side, starts the
  `Bgm04.img/ArabPirate` music, and drifts at random every 1.2 seconds. It
  is drawn above the map. CONTI_MOVE (10, 4) is Appear and (10, 5)
  Disappear; CONTI_STATE 3 or 4 with flag 1 is Appear. The server never
  sends these yet.

`Stage` sends 0xDC after loading a map with a ship. CONTI_MOVE and
CONTI_STATE are ignored while a map loads.

`Gameplay/MapleMap/MapClock` draws a station's wall clock from the map's
`clock` box with `Map.nx Obj/etc.img/clock/fontTime`. It shows AM or PM,
the hour and the minute, and the colon dims on odd seconds. It reads
"AM 00:00" until CLOCK sets it, then counts on by itself, refreshing every
second. As in v83, the AM/PM tag is fixed when the clock is set and does not
flip at noon. A CLOCK that arrives while the map loads is kept and applied
after (`Stage::pending_clock`). The rules are in `ClockRules.h`.

The client's `tMove` is in real seconds, while the harness's route clock
runs 60 times as fast. A ship seen against the harness therefore leaves
again before it has finished docking.

## Not yet

- The Balrog's attack on the Ellinia ship.
- Mu Lung and Ereve in Isa's list; Corba's dragon flight to Leafre.
- Ariant arrivals use the town's first `sp` portal; Cosmic uses the second.
- CLOCK countdowns (kinds 0, 2 and 3).
