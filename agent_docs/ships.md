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

### The subway

The subway between Kerning City and New Leaf City runs on the same clock
as a `Route` with `ship: false`. A train leaves each way every 5 minutes:
boarding opens 4 minutes before, closes 1 minute before, and the ride takes
1 minute (`SUBWAY`, Edelstein's Continent 8 and 9). Its stations have no
`shipObj`, so they get no CONTI_MOVE and `ship_state` answers nothing for
them; the waiting room is still moved onto the train at departure and the
riders set down at the far station's first `sp`.

| Route | Boards at | Waiting room | Train | Arrives at | Ticket | Fare |
| --- | --- | --- | --- | --- | --- | --- |
| `TO_NLC` | 103000100 | 600010004 | 600010005 | 600010001 | 4031711 | 5,000 |
| `TO_KERNING_CITY` | 600010001 | 600010002 | 600010003 | 103000100 | 4031713 | 5,000 |

The fare is Cosmic's. The Basic tickets (4031710, 4031712) that Edelstein
sells to level 10 and under are not sold or accepted.

### The plane

The plane between Kerning City and Singapore's Changi Airport is another
`Route` without a ship. One leaves each way every 6 minutes: boarding
opens 5 minutes before, closes 1 minute before, and the flight takes 1
minute (`PLANE`, Cosmic's `AirPlane`). Kerning City has no station of its
own: the plane boards from the town itself (103000000).

| Route | Boards at | Waiting room | Plane | Arrives at | Ticket | Fare |
| --- | --- | --- | --- | --- | --- | --- |
| `TO_CHANGI` | 103000000 | 540010100 | 540010101 | 540010000 `sp` | 4031731 | 5,000 |
| `AIR_TO_KERNING_CITY` | 540010000 | 540010001 | 540010002 | 103000000 portal 7 | 4031732 | 5,000 |

The plane home sets its passengers down by Irene, at Kerning City's portal
7, which has no name of its own. So an arrival is a `conti::Portal`: a name,
or an id (`warp_to_spawn_point`). The fare and the timetable are Cosmic's.

Its NPCs, with placeholder lines under the `plane.*` keys:

- Irene (9270041, Kerning City) and Shalon (9270038, Changi) each have a
  menu (`PLANE_BOOTHS`): buy the ticket, or board. They sell one ticket at
  a time, to a player who has none, and board its holder while the plane
  boards.
- Xinga (9270017) in Kerning City's waiting room and Kerny (9270018) in
  Changi's let passengers back out to the airport; the ticket is gone. On
  either plane, Kerny only says it lands soon (`conti::riding_on`).

### The Helios Tower elevator

Ludibrium's Helios Tower has an elevator between its 2nd floor (222020100)
and its 99th (222020200), made of two more shipless routes. The car docks on
a floor for a minute, rides for one, and leaves the other floor two minutes
after it left this one: a 4-minute term, the way up 2 minutes behind the way
down (`Timetable::offset`; Edelstein's Continent 4 and 5, Cosmic's
`Elevator.js`). It is free: no ticket, no fare, no NPC.

| Route | Boards at | Waiting room | Car | Arrives at | Leaves at |
| --- | --- | --- | --- | --- | --- |
| `ELEVATOR_UP` | 222020100 | 222020110 | 222020111 | 222020200 portal 0 | :02 and every 4 minutes |
| `ELEVATOR_DOWN` | 222020200 | 222020210 | 222020211 | 222020100 `st00` | :00 and every 4 minutes |

- The floors' `in00` (pt 8, the `elevator` portal script) lets a player into
  the waiting room, portal 0, with the portal sound while that floor's route
  boards (the car is there), else says so in pink
  (`portal.elevator.away`, sourced from Server.Scripts' `elevator.lua`).
- Each floor has the reactor `elevator` (2208004): state 0 the car with its
  door open (`conti::CAR_HERE`), 1 the door shut (`CAR_AWAY`). `Route::car`
  names it. At `Move` the departure floor's reactor goes to 1, at `End` the
  arrival floor's to 0 (`Happening::Reactor`, HIT_REACTOR with delay 0 and
  event 0, `ReactorSet::set_state`). A field that starts on a floor places
  the car from the clock (`conti::car_state`). Its events are item events
  (type 100), so nobody can hit it.
- The waiting rooms and cars have no time limit and no NPC; their
  `forcedReturn` is the floor. Players walk out by `out00` or fall out.

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
    `End`, everyone is moved on (`ServerMessage::ShipWarp`);
  - on an Ellinia-Orbis deck, the Crimson Balrog's attack (below);
  - on the Helios Tower's floors, the elevator's door shutting at `Move`
    and opening at `End`.
- The client actor (`runtime/src/actor/client/ships.rs`) warps on
  `ShipWarp` only if the player is still on that map. A player in the Cash
  Shop stays where they are.
- A player who logs in on a map with a `forcedReturn`, such as a deck
  under way, starts on that map instead.
- 0xDC (`ContiStateRequestHandler`) is the client asking about its own map's
  ship on entering it. The server answers CONTI_STATE with the state of the
  route whose platform, arrival platform or deck it is. The flag is 1 on a
  deck while the Balrog's ship is out, else 0.
- On joining a map with a `clock` node, the client actor sends CLOCK kind 1
  with the route time of day in UTC (`set_wall_clock`).

### The Crimson Balrog

`conti::invasion` decides from the departure time alone whether a trip
between Ellinia and Orbis is attacked and when. Both decks and every
channel therefore agree, as with Cosmic's single roll. The cabins are safe.

- Cosmic attacks 42% of trips; `RUSTMS_CONTI_BALROG` sets the percentage,
  and the harness world uses 100.
- 3 to 4 minutes after departure (Cosmic) the ship comes alongside: CONTI_MOVE
  (10, 4). The client starts the ArabPirate music itself.
- 5 seconds later two Crimson Balrogs (8150000) board on the ground below
  Cosmic's points, (339, 148) on 200090010 and (-538, 143) on 200090000.
  The first player there controls them, or the next to come.
- A minute before arrival (`tEventEnd` 9 in a later version's
  Continent.img) the ship sails off: CONTI_MOVE (10, 5). Every mob on the
  deck leaves with it (KILL_MONSTER 1, no EXP or drops). Cosmic keeps them
  until arrival, after everyone has been moved off.

None of the odds, times or points can be checked against the v83 client,
which only shows the ship.

The NPCs are in `net/src/npc_scripts.rs`, with placeholder lines under the
`ships.*` content keys:

- Joel (Ellinia), Mel (Ludibrium), Mue (Leafre) and Syras (Ariant) sell
  tickets to Orbis. Agatha sells Orbis's four from one menu.
- Each platform's usher (`USHERS`) takes a ticket and shows the holder to
  the waiting room while the ship boards. A yes that comes after boarding
  closed is refused by the `RequireBoarding` effect.
- The waiting-room guides take people back to the platform of the room they
  are in (`conti::waiting_in`). The ticket is not returned.
- Isa points the way to Orbis Station's six platforms, the crane's and
  Ereve's ferry's among them.
- Leafre Station's `dracoout` portal leads back into Leafre.
- The subway's ticket gates (`SUBWAY_GATES`: 1052007 in Kerning City,
  9201068 in New Leaf City) ask first, as `NLC_Move` does, and only then
  look for the ticket. Kerning City's gate only serves the subway; its
  original menu also offered Kerning Square and the Construction Site.
- Bell (9201057, `NLC_ticketing`) sells the ticket for the line leaving her
  platform from a one-choice menu, and in a waiting room lets the rider back
  out to its platform. The text is under the `subway.*` keys, from
  Server.Scripts.

## Rides on a countdown

The ferries to Ereve and Rien and the crane between Orbis and Mu Lung keep
no timetable: a player boards when they like and rides alone
(`net/src/rides.rs`). The pilot or the crane sells the ride from a
one-choice menu and a Yes or No on the fare, and the yes warps the player
onto the ride map. Joining a ride map starts the client actor's map timer
(`time_limit.rs`) with the ride's length and its arrival, overriding the
map's `forcedReturn`, which is the dock the ride began at. The countdown is
CLOCK kind 2, scaled like the route clock (`RUSTMS_CONTI_SCALE`).

| Ride map | Sold by | Fare | Ride | Arrives |
| --- | --- | --- | --- | --- |
| `200090020` | Kiru `1100008`, Orbis Station `200000161` | 1000 | 8 min | Ereve's dock `130000210`, id 0 |
| `200090021` | Kiru `1100004`, Ereve | 1000 | 8 min | `200000161`, id 0 |
| `200090030` | Kiriru `1100007`, Ellinia's dock `101000400` | 1000 | 2 min | `130000210`, id 0 |
| `200090031` | Kiriru `1100003`, Ereve | 1000 | 2 min | `101000400`, id 0 |
| `200090060` | Puro `1200004`, Lith Harbor | 800 | 1 min | Rien's dock `140020300`, id 0 |
| `200090070` | Puro `1200003`, Rien | 800 | 1 min | Lith Harbor, id 3 |
| `200090300` | The crane `2090005`, Orbis Station `200000141` | 1500 | 1 min | Mu Lung Temple `250000100`, id 0 |
| `200090310` | The crane, Mu Lung Temple | 1500 | 1 min | `200000141`, id 0 |

The crane also goes straight between Mu Lung Temple and Herb Town
(`251000000`) for 500; from Herb Town it is a bare Yes or No. Falling off a
ride map (its `hd*` portals) lands back on the dock it left. Logging out
lands on its `forcedReturn`, which is that dock too, except on the two Rien
ferries, whose `forcedReturn` (Map.nx) is where they arrive. The fares and lengths are Cosmic's. The
text is under the unsourced `ferry.*` keys. Ellinia's `inERShip` and Rien's
`enterPort` lead down to the docks (`map-travel.md`). `ferries.rs` checks
Rien both ways, the crane and Herb Town, and a logout mid-ride.

## Packets

| Op | Direction | Body | Meaning |
| --- | --- | --- | --- |
| 0xDC | client to server | `i32` map id, `u8` ship kind | Where is this map's trip? |
| 147 CLOCK | server to client | `u8` kind 1, `u8` hour, minute, second | Set the wall clock |
| 148 CONTI_MOVE | server to client | `u8` target, `u8` event | The trip changed |
| 149 CONTI_STATE | server to client | `u8` state, `u8` flag | The trip's state, on entry |

The client only checks that its map's ship is of the right kind; the target
is not compared with the map. Ride maps and timed fields send CLOCK kind 2,
a countdown (see "Rides on a countdown").

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
  Disappear; CONTI_STATE 3 or 4 with flag 1 is Appear.

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

- The subway's Basic tickets, and Kerning City gate's Kerning Square and
  Construction Site choices.

- 8150000's drops, which belong in the content pack.
- Corba's dragon flight to Leafre.
- Ariant arrivals use the town's first `sp` portal; Cosmic uses the second.
- CLOCK countdown kinds 0 and 3.
