# Server Architecture

## Crates

| Crate | Role |
| --- | --- |
| `packet` | `Packet` plus little-endian read/write helpers (bounds-checked reads) |
| `crypt` | MapleStory AES/shanda and bcrypt password hashing |
| `game` | Domain model: `Player`, `CharacterStats`, `AvatarLook`, `Inventory`/`Item`, `SkillBook`, `Keymap`, `QuestLog`, `FieldAvatar`. No IO. |
| `game-data` | Reads static data (maps, portals, NPCs, quests) from v83 NX files |
| `db` | Diesel persistence behind a connection pool; converts rows to and from `game` types (`db::player`) |
| `net` | Opcodes, the handler registry, packet handlers, packet builders, and shared encoders (`net::codec`) |
| `runtime` | Tokio actors, socket IO, and the server bootstraps |
| `rust-ms` | The `login` and `world` binaries |

Dependencies point one way: `game` ← `db` ← `net` ← `runtime` ← `rust-ms`.

## Binaries and ports

- `login` listens on `RUSTMS_LOGIN_BIND_ADDR` (default `0.0.0.0:8484`).
- `world` hosts every configured channel in one process. Each channel listens
  on its own advertised port on `RUSTMS_WORLD_BIND_HOST` (default `0.0.0.0`).
- Channels come from `RUSTMS_LOGIN_CHANNELS`
  (`name@host:port@capacity;...`, see `net/src/login_world.rs`). The default is
  three channels on `127.0.0.1:8485`, `8486`, `8487`. Ports must be unique: the
  world identifies a connection's channel by the listener that accepted it,
  because a v83 client's `PLAYER_LOGGEDIN` carries only the character id.

Both binaries apply pending database migrations at startup (embedded in the
`db` crate, serialized with a Postgres advisory lock).

## Session lifecycle and world entry

The `sessions` table is the source of truth for whether an account is online.
A row lives exactly as long as its owning connection, plus a short transition
window between servers:

1. **Login** creates a session (`before_login` until the terms are accepted,
   then `after_login`). Logging in while the account has a live session is
   rejected with status 7.
2. **Character select** checks that the character belongs to the session's
   account, then turns the session into a single-use **transition ticket**
   (`transition`, bound to character, channel, and IP) before sending the
   redirect.
3. **World entry** (`PLAYER_LOGGEDIN`) atomically claims a fresh ticket
   (younger than 60 seconds) for that character, channel, and IP, moving it to
   `in_game`. Without a valid ticket the connection is closed.
4. **Channel change** saves the player, reissues the ticket for the target
   channel, and redirects; the new channel claims it as in step 3.
5. **Disconnect** deletes the session (except during a channel change).

On startup the login server deletes login-stage sessions and expired tickets;
the world server deletes `in_game` sessions for the channels it hosts.

## World runtime

```text
          per channel                         shared
  TcpListener ──accept──► ClientActor ──────► FieldActor  (via FieldRegistry)
                              │   ▲               │
                              │   └── try_send ───┘
                              ▼
                         WorldDirectory ── try_send ──► ClientHandle
```

- **`ClientActor`** (`runtime/src/actor/client.rs`) owns one connection and the
  only copy of its `game::Player`. It runs world handlers, applies their
  actions, talks to its current field directly, and persists the player.
- **`FieldActor`** (`runtime/src/actor/field.rs`) owns one live map copy,
  keyed by `FieldKey { channel_id, map_id, instance_id }`: occupants (as
  `FieldAvatar` projections with live positions) and static NPCs. It runs a
  100 ms tick for future time-driven state (mobs, respawns, drops). See
  [Fields](./fields.md).
- **`FieldRegistry`** (`runtime/src/actor/registry.rs`) maps `FieldKey` to a
  field handle and starts fields on demand. Fields are supervised; if one dies
  its handle is dropped and the next lookup starts a fresh actor (clients
  rejoin automatically).
- **`WorldDirectory`** (`runtime/src/actor/directory.rs`) knows who is online
  under which name and on which channel. It handles whispers, `/find` (asking
  the target's client for its map when it shares the asker's channel), and
  world broadcasts, and kicks the older connection when a character connects
  twice. It owns no game state.

### Backpressure rule

Clients `await` their sends to fields and the directory, which bounds work
per client. Fields and the directory **never await a client**: they use
`ClientHandle::send`, a `try_send` that kicks a client whose 256-message queue
is full. No actor can therefore wait on another actor that is waiting on it,
and one stalled socket cannot stall a map.

A kicked client actor may itself be blocked writing to a peer that stopped
reading. Every socket write races the kick (`ClientActor::send`) and is
bounded by `io::write::WRITE_TIMEOUT` (10 s), so the actor always reaches its
cleanup: it leaves its field, unregisters, saves the player, and closes the
session.

### Persistence

- `db::player::load_player` loads a character (stats, keymap, inventory with
  equipment stats, skills, quests, account GM level) on world entry.
- Handlers mutate the in-memory `Player`; mutators mark it dirty.
- `ClientActor` writes it back with `db::player::save_player` (one
  transaction replacing character columns, keymap, inventory, and skills):
  every 60 seconds if dirty, before a channel change, before a quest
  transition, and on disconnect.
- Quest transitions (`db::quest::transition`) lock the character row and
  commit progress and rewards together; the actor then adopts the committed
  level, EXP, and AP and updates the in-memory quest log.

## Handlers

`net/src/handler.rs` defines the contracts and the routing tables.

- **Login handlers** implement `LoginHandler` and return
  `HandlerResult<LoginAction>` (`Reply`, `CreateSession`, `SetSessionState`,
  `SelectChannel`, `EnterWorld`, `Disconnect`). They may query the database
  and run on the blocking pool.
- **World handlers** implement `WorldHandler` (in game, with
  `WorldContext { client_id, channel_id, player: &mut Player }`) or
  `PreGameHandler` (before entry; only `PLAYER_LOGGEDIN`). They are pure: no
  database or network IO, so they run inline. Side effects are returned as
  `WorldAction`s (`Reply`, `EnterGame`, `ChangeField`, `FieldChat`,
  `FieldMove`, `NpcInteraction`, `Whisper`, `ChangeChannel`, `Disconnect`).
- **Registry**: `login_route(opcode)` and `world_route(opcode)` look up the
  handler and the session stage it requires (`LoginStage::Any`/`Session`/
  `LoggedIn`; `WorldRoute::PreGame`/`InGame`). Opcodes used in the wrong stage
  are ignored; unknown opcodes are logged at debug level and ignored. A
  handler error closes the connection.

Map-id warps (`CHANGE_MAP` with a target) are accepted only for dead players
(sent to the map's return map), accounts with `gm_level > 0`, or when
`RUSTMS_ALLOW_DEBUG_WARPS=1` (development and the harness). Portal warps are
resolved from map data.

## Packet encoding

Structures that recur across packets are encoded once in `net/src/codec/`,
following the HeavenMS reference layout:

- `encode_character_stats` (character list, SET_FIELD)
- `encode_avatar_look` (character list, spawns), derived from equipped items
- `encode_item` / `encode_inventory`
- `encode_character_data` (the full SET_FIELD character block)
- `time::filetime` and the special FILETIME constants

`build::world::stat::build_stat_update` builds STAT_CHANGED from typed
`StatUpdate`s, so mask bits and widths cannot drift. Builders have golden
byte tests using `codec::testing::Expect`.

## Adding a feature

For example, "pick up a drop":

1. Add the recv opcode to `net/src/packet/op/recv.rs` if missing.
2. Write a `WorldHandler` in `net/src/packet/handle/world/` that parses the
   packet, validates it against `ctx.player`, and returns actions.
3. Register it in `world_routes()` in `net/src/handler.rs`.
4. If it needs field state (the drop), add a `FieldMessage` variant and handle
   it in `FieldActor`; the field replies through the client's handle.
5. Encode packets with `net::codec` encoders; add golden tests.
6. Add an integration test under `integration-harness/tests/`.

## Related docs

- [Fields](./fields.md)
- [NPC dialogue](./npc-dialogue.md) and [Quests](./quests.md)
- [Scripting decision](./scripting.md)
- [Client](./client.md)
- [Integration harness usage](./integration-harness-usage.md)
