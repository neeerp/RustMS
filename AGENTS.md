# RustMS Agent Guide

This repository is a Rust workspace for a MapleStory server plus a nested C++ client (`HeavenClient`).

## Documentation index

- `agent_docs/server-architecture.md`: crates, actor topology, state ownership, persistence, sessions, handler registry, and how to add a feature
- `agent_docs/fields.md`: field actors, occupants, map changes, and field packets
- `agent_docs/npc-dialogue.md`: asset-backed conversations, runtime ownership, and future quest integration
- `agent_docs/quests.md`: the v83 client-driven quest flow, supported requirements/actions, persistence, and rewards
- `agent_docs/map-travel.md`: asset portal links, active beginner route, legacy map variants, and map/portal scripts
- `agent_docs/party-quests.md`: instanced party events (the party quests of Kerning City, Henesys, Ludibrium, Orbis and Horntail's Trial Grounds, Delli's rescue, the Ludibrium Maze, Protecting Tylus, the Forest of Poison Haze, the Pirate Ship, Romeo and Juliet, Amoria's Amorian Challenge, Crimsonwood Keep's, Happyville's Holiday PQ, and MV's Lair): entry, tickets, copies of maps, leaving and ending
- `agent_docs/guild-quest.md`: the Guild Quest to Sharenian: Shuang's registration, each channel's line of guilds, the waiting room and its gathering, the ends, and the waitlist packet
- `agent_docs/zakum.md`: the way to Zakum: the chiefs' approval, Adobis's trials, the Breath of Lava, and the gate
- `agent_docs/papulatus.md`: the fight with Papulatus in Ludibrium's Clocktower: the party's portal, the gates, the way out, the summon, and the win
- `agent_docs/pianus.md`: Pianus in Aqua Road's Cave of Pianus: the two spawn points, their skills and Bloody Booms, and the drops
- `agent_docs/dojo.md`: the Mu Lung Dojo: the notice board, So Gong's solo and party runs, its floors and bosses, the door's warp up (0xCF), and the ways on and out
- `agent_docs/ariant.md`: the Ariant Coliseum's Battle Arena: the lobby and the arenas' rosters, the battle, its scores and results, the jewels, and the King's Room's Areda and Abdullah VIII
- `agent_docs/pyramid.md`: Nett's Pyramid: its stages and their copies, the gauge and its rules, cool kills and misses, and the packets
- `agent_docs/carnival.md`: the Monster Carnival: the Carnival Field and Spiegelmann, the rooms and their battle fields, CP, and the packets
- `agent_docs/coconut.md`: Coconut Season: its fields, the teams, a GM's `/start` and `/reset`, opening a GM's event to players through the Event Assistants, the nuts' hits and the score, the end, and the packets
- `agent_docs/snowball.md`: Snowball: its fields, the teams, the balls and the snowmen, the hits, the pace, the end, and the packets
- `agent_docs/ox-quiz.md`: the OX Quiz: its field and rooms, the questions, a GM's `/ox` and `/start`, the judging, the end, and the packets
- `agent_docs/timed-events.md`: GM events run on a clock, starting with the Physical Fitness Test: its maps, the lobby's gate, the clock and notices, and the time up
- `agent_docs/horntail.md`: the way to Horntail in Leafre's Cave of Life: the Dragon Squad's quests, the doors at Keroben and the slate, the ways out, the expedition, its trials, the fight, and the win
- `agent_docs/showa.md`: The Boss in Showa's gang hideout: the way in, Konpei's expedition and the comb, the statue and the bodyguards, the win, the prizes, and the Slot Machine
- `agent_docs/balrog.md`: the Balrog's tomb under Sleepywood: Mu Young's expedition, the arms and the claw that shield the body, the win, the sealed Balrog, and the leather
- `agent_docs/latanica.md`: Capt. Latanica on the Ghost Ship off Singapore: the White Essence and the party's portal, the reactor's summon, Bob, and the win
- `agent_docs/pink-bean.md`: Pink Bean in the Temple of Time: the gate's expedition, the copy and its revival room, the Temple Keeper, the ways out, and the statue's guardians' waves
- `agent_docs/ships.md`: the Ellinia–Orbis ship's timetable, boarding NPCs, CONTI packets, and the station wall clock
- `agent_docs/combat.md`: mob spawning and control, melee attacks, EXP and level-ups, spending AP and SP, drops and pickup, touch damage and death
- `agent_docs/parties.md`: party state and flows, EXP sharing, party chat, and the client's party window and HP displays
- `agent_docs/buddies.md`: buddy list storage, requests, channel visibility, buddy chat, and the client's Friend tab
- `agent_docs/notes.md`: notes (memos) left for offline characters, gift thank-yous, and the client's inbox and send dialog
- `agent_docs/reports.md`: reporting a player to a GM (CLAIM): the hours and status sent on entry, the fee, the weekly limit, the `reports` table, and the result modes
- `agent_docs/families.md`: family storage, juniors and seniors, the Family window's numbers, the Family Tree, and login notices
- `agent_docs/marriage.md`: engagement with a ring box, the marriages table, proposals and their refusals, the wedding ring look, crush and friendship rings, and the client's dialogs
- `agent_docs/guilds.md`: guild storage, founding at Heracle, invitations, leaving and expelling, ranks, titles and the notice, guild chat, name tags, and the client's Guild tab
- `agent_docs/pets.md`: pet items and storage, summoning, the list of pets out, their packets, and the client's side
- `agent_docs/inventory.md`: inventory operations, item data, item moves, NPC shops, storage, Duey parcels, the Rock, Paper, Scissors game, and their packets
- `agent_docs/server-content.md`: the ignored content pack (script text, drops), placeholders, and the importer
- `agent_docs/client-testing.md`: fast client logic tests and private-display GUI verification
- `agent_docs/quest-window.md`: original quest UI assets, layout, controls, and visual validation
- `agent_docs/client.md`: HeavenClient origin, license, NX file versions, feature status, and fixed protocol deviations
- `agent_docs/scripting.md`: proposed decision for data-driven quests plus Lua scripts
- `agent_docs/integration-harness-usage.md`: commands, prerequisites, and day-to-day harness workflow
- `agent_docs/integration-harness-architecture.md`: harness design, lifecycle, and test isolation model
- `agent_docs/integration-harness-troubleshooting.md`: common harness failures and targeted fixes

## Workspace layout

- `rust-ms`: binaries (`login`, `world`)
- `runtime`: actor runtime and message types
- `net`: packet handlers/builders, the opcode registry, and shared encoders (`net::codec`)
- `game`: domain model (`Player`, stats, look, inventory, skills, keymap, quests)
- `game-data`: static data read from NX files (see `assets/game-data/README.md`)
- `db`: Diesel-backed persistence and repositories
- `packet`: packet type plus read/write helpers
- `crypt`: MapleStory crypto helpers
- `integration-harness`: async end-to-end test harness
- `HeavenClient`: standalone C++ client used for local manual testing

## Build and launch the servers

Run everything from the workspace root unless noted otherwise.

During feature work, the normal rebuild loop is:

```sh
cargo build --workspace
```

If you only changed server code and want a quick rebuild of the binaries:

```sh
cargo build -p rust-ms
```

### Launch the login server

```sh
RUST_LOG=runtime=info cargo run -p rust-ms --bin login
```

### Launch the world server

In a second terminal:

```sh
RUST_LOG=runtime=info cargo run -p rust-ms --bin world
```

## Build and launch the client

During feature work, rebuild from `HeavenClient` with:

```sh
cd HeavenClient
cmake --build cmake-build -j"$(nproc)"
```

Notes:

- `HeavenClient/MapleStory.h` has `USE_CRYPTO` enabled; keep that on for RustMS.
- The generated executable is `HeavenClient/cmake-build/HeavenClient`.
- The client reads host/port from a `Settings` file. Defaults are `ServerIP = 127.0.0.1` and `ServerPort = 8484`.
- By default the world server hosts three channels on `127.0.0.1:8485`, `8486`, and `8487` (one port per channel; configure with `RUSTMS_LOGIN_CHANNELS`), so the standard setup is fully local.
- Set `RUSTMS_ALLOW_DEBUG_WARPS=1` on the world server to allow map-id warps for every player during local testing.

Run the client:

```sh
cd HeavenClient/cmake-build
./HeavenClient
```

For automated native-client interaction, prefer a private Xvfb display and
separate Settings directory so tests do not steal the user's desktop focus.
See `agent_docs/client-testing.md` for setup and input/screenshot commands.

## Validate changes

Run the integration harness when validating server behavior changes:

```sh
cargo run -p integration-harness --bin harnessctl -- test
```

Use `harnessctl` as the default integration-harness entrypoint because it brings up the isolated Docker stack, runs the tests against the harness endpoints, and tears the stack down.

If another checkout (for example a worktree used by a parallel session) may be running the harness, give yours its own stack first; see "Running several stacks at once" in `agent_docs/integration-harness-usage.md`.

If you need to run a specific integration test while reusing an already-running harness stack, use:

```sh
cargo run -p integration-harness --bin harnessctl -- up
HARNESS_LOGIN_ADDR=127.0.0.1:18484 HARNESS_WORLD_ADDR=127.0.0.1:18485 cargo test -p integration-harness --test presence_same_map -- --nocapture
cargo run -p integration-harness --bin harnessctl -- down
```

After changing Rust server code, rebuild and restart the affected server binaries before trusting live test or manual client results.

For packet mismatch bugs, inspect both the Rust server packet path and the corresponding `HeavenClient` packet handler.
