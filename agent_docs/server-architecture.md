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
- `RUSTMS_LOGIN_WORLD_POPULATED` (0, 1 or 2; default 0) is the banner the
  client's channel list shows beside the world's name: 1 is "Highly
  Populated" and 2 "Over-Populated" (`Login.img/WorldNotice/Populated`). It's
  the second byte of the answer to a world pick (`SERVERSTATUS`); RustMS
  counts no population, so it's off unless set.

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
   `in_game`. Without a valid ticket the connection is closed. The client
   actor then sends SET_FIELD (the character block) before the key map,
   macros, auto potions and quick slots: a v83 client handles only the login stage's
   packets until its first SET_FIELD and drops the rest.
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
  under which name and on which channel. It handles whispers, `/find`,
  world broadcasts, and duplicate connections. It also owns Maple Messenger
  rooms (`runtime/src/actor/messenger.rs`); a member changing channel keeps
  their seat for `CHANNEL_CHANGE_GRACE`.

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
  transaction replacing character columns, keymap, quick slots, inventory,
  and skills):
  every 60 seconds if dirty, before a channel change, before a quest
  transition, and on disconnect.
- Quest transitions (`db::quest::transition`) lock the character row and
  commit progress and rewards together; the actor then adopts the committed
  level, EXP, and AP and updates the in-memory quest log.
- An account's storage (`db::storage`) is loaded when a storage NPC opens it.
  Every change is saved together with the player (`db::storage::save`) in one
  transaction, outside the dirty-flag cycle (`agent_docs/inventory.md`).

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
  `ReplyLater { after, packet }` sends a packet after a delay; the client
  actor keeps it in a queue ordered by due time.
- **Registry**: `login_route(opcode)` and `world_route(opcode)` look up the
  handler and the session stage it requires (`LoginStage::Any`/`Session`/
  `LoggedIn`; `WorldRoute::PreGame`/`InGame`). Opcodes used in the wrong stage
  are ignored; unknown opcodes are logged at debug level and ignored. A
  handler error closes the connection.

Map-id warps (`CHANGE_MAP` with a target) are accepted only for dead players
(sent to the map's return map), accounts with `gm_level > 0`, or when
`RUSTMS_ALLOW_DEBUG_WARPS=1` (development and the harness). Portal warps are
resolved from map data.

### The client's grade

The login result carries two grade bytes, which v83 keeps for the session
(`build_successful_login_packet`, reports b156 and a006b):

- The grade has bit 0 (a GM's account) when `gm_level` is 3 or more
  (`policy::client_gm`). The client then has its GM slash commands (below),
  sends the `/` lines it doesn't know as chat (GM events' `/start`), and
  refuses to trade or open a store ("Not available with an Admin
  account.").
- With `accounts.test_admin` the grade is 0x11 as well: v83's admin account,
  which can't drop items ("The admin can't drop items.") or mesos either.
  Only SQL sets it (`UPDATE accounts SET test_admin = true WHERE ...`);
  `gm_level` never does.
- The sub-grade stays 0. Its 0x80 would refuse every attack ("Not available
  with an MWLB account.") and its 0x20 and 0x40 only rank a GM the grade
  already ranks.

`gm_level` is set by SQL only, as `test_admin` is:
`UPDATE accounts SET gm_level = 2 WHERE user_name = '...'`, then log in
again.

### Chat commands

A chat line starting with `!` is a GM's command and one starting with `@`
is anyone's (`net::gm_commands`, run from `AllChatHandler` before chat).
v83 has neither, so the commands and their text are ours. A command needs a
tier of `gm_level` (`gm_commands::tier`, levels above 3 count as 3):

| Tier | Commands |
|---|---|
| 0, anyone | `@help`, `@dispose` (closes the server's dialog, shop or storage and lets a stuck client act again) |
| 1, helper | `!help`, `!whereami`, `!warp <map id> [portal]`, `!hide`, `!unhide`, `!heal` |
| 2, GM | `!item <item id> [quantity]` (no cash items), `!meso <amount>`, `!exp <amount>`, `!level <level>`, `!job <job id>`, `!ap <amount>`, `!sp <amount>`, `!skill <skill id> [level]` (its highest level by default; 0 removes it), `!givenx [nx\|mp\|np] [name] <amount>`, `!coupon <reward>... [days=N] [code=CODE]`, `!gp <amount>` (to the GM's guild), `!notice <text>`, `!dc <name>`, `!jail <name> [minutes]`, `!unjail <name>`, `!mute <name> [minutes] [reason]`, `!unmute <name>` |
| 3, admin | `!ban <name> [days] [reason] [note]`, `!unban <name>` |

- A GM's `!` line never reaches the map. A player's `!` line is ordinary
  chat, as is an `@` line that names no command (`@_@`). A command the sender may not run gets the same "Unknown command"
  line as one that doesn't exist.
- Replies are pink lines in the sender's chat.
- `!warp` and `!heal` refuse the dead, who revive through the client.
- `!hide` (or `/h 1`) lasts the session. Those not GMs lose the GM, with
  the GM's pets and summons, and everything the GM does: chat, moves,
  attacks, chairs, effects and buffs. v83 drops a packet about a character
  it doesn't know, so the field sends these only to those who see the
  sender (`broadcast_by`, `broadcast_about`; report b177). The map's own
  packets still reach everyone: mobs, drops, mists, doors, notices and a
  jukebox's song (`FieldRequest::BroadcastAll`). GMs see hidden GMs.
  Mobs pass a hidden GM by: they don't take the GM as their controller,
  hiding hands the GM's mobs to someone else (`FieldActor::mob_candidate`),
  and nothing hurts the GM. To those not GMs, a hidden GM isn't online for
  `/find`, whispers, the Teleport Rock, the Party Potion or the family's
  teleport (`Presence::hidden`). Buddy lists, guilds and parties still
  show the GM.
- `!level` and `!job` set only the level or job. Others see the level-up or
  job-change effect (`WorldAction::AvatarChanged`). `!exp` gains EXP as a
  kill does.
- `!givenx` adds to an account's NX Credit, or its Maple Points (`mp`) or
  NX Prepaid (`np`), within 0..=i32::MAX; a negative amount takes NX away.
  It goes to the account of the character named, online or not, or else
  the GM's own (`db::cash_shop::grant`, from the runtime's
  `WorldAction::GiveCash`). The Cash Shop reads the balances on entry and
  on every Check, so nothing is pushed. Tests can use it instead of setting
  `nx_credit` by SQL.
- `!coupon` makes a Cash Shop coupon (see "Coupons" in `fields.md`). Its
  rewards are `nx=N`, `mp=N`, `np=N`, `meso=N` and `<item id>[xN]`, cash
  items included, which nothing else gives. Without `code=` the code is 16
  random letters and digits (no 0, O, 1 or I). Without `days=` it never
  expires. The reply names the code; a code already made is refused.
- `!ban` blocks the account of the character named, for good unless a
  number of days comes first (at most 36,500). The reason is a type from
  Etc/BlockReason.img (`hack`, `bot`, `ad`, `harass`, `curse`, `scam`,
  `misconduct`, `sell`, `icash`, `temp`, `gm`, `iprogram`, `megaphone`) or
  its number 1..=13; the rest is a note kept in `accounts.ban_msg`. Every
  character of the account that is online is disconnected
  (`DirectoryMessage::Kick`), and a login is refused with result 2, the
  reason and the end (`build_blocked_packet`), which the client shows as
  BlockReason.img's text and the date; with no reason it shows its plain
  notice. A block that has ended no longer counts (`Account::blocked_at`).
  The world refuses an entry by a blocked account too, from the character
  select or a channel change. `!unban` lifts it, and `!dc` only
  disconnects. None of them may touch a GM of the sender's level or
  higher (`ClientActor::discipline`).
- `!jail` puts the character named in the jail map, 300000012 ("Cellar:
  Camp Conference Room", which has no portals out), for 5 minutes or the
  number given (at most a year), added to any time they have left. The
  time is kept in the `jails` table (`db::jail`), so it holds for a player
  who is offline, and across a logout or a channel change: entering the
  game puts a jailed player there. An online player is told through
  `DirectoryMessage::JailChanged`. Any map change while jailed (a return
  scroll, a warp, a revive) ends with a warp back (`hold_in_jail`): the
  other map is joined first, since its SET_FIELD has already gone out. A
  dead player isn't moved; their revival is a map change. When the time
  is up (checked again in the database) or at `!unjail`, they come out at
  300000010's `in01` (Cosmic's `jail_out.js`) and hear "You are free to
  go.". One freed while offline or in the Cash Shop logs in at 300000010,
  the jail map's `forcedReturn`. This is ours, as v83 has no jail.
- `!mute` blocks the character's account from chatting, for an hour or
  the minutes given, for one of Etc/ChatBlockReason.img's reasons: 1
  `foul` (the default), 2 `advertising`, 3 `hack` (fake GM), 4
  `account`, 5 `trading` and 6 `penalty`, by number or name. A second
  `!mute` replaces the first. v83 keeps the block per account, and only
  the login result sets it: the byte after the account name's flag is the
  reason and the long after it the end (report b180). The client then
  refuses all-chat, whispers, multi-chat, spouse chat and the Messenger's
  and mini-rooms' chat itself, says "You are currently blocked from
  chatting.", and shows the reason on entering a field. It never checks
  the end, so the server sends 0 once it has passed. The columns are
  `accounts.chat_block_reason` and `chat_block_until`. Online characters
  of the account hear of it through `DirectoryMessage::ChatBlockChanged`.
  Until their next login the world drops their chat
  (`net::chat_block::is_chat`, which also covers the megaphones, Maple TV,
  notes, weather, the chalkboard and the kites, which the client doesn't
  refuse) with the client's own line.
  `!unmute` lifts it; a client that logged in blocked keeps refusing
  until it logs in again, as its line says.
- Every `!` line from a GM is logged at info under the `gm_audit` target,
  with the character, the line, and whether it ran. The world server always
  shows that target. `!givenx` and `!coupon` log their results there too.
- `RUSTMS_GM_COMMANDS=0` turns every command off, so each line is chat.

### The client's slash commands

v83's client has GM commands of its own, read from the chat bar before the
players' commands (report b175) and only for the grades that rank a GM:
grade bit 0, or a sub-grade of 0x20, 0x40 or 0x80. The rank (0 may do the
most, 5 is a player's) sets which: `/h` from 4, `/ch`, `/u`, `/w`, `/ban`
and `/uclip` from 3, the broadcasts from 2. Our GMs, with grade bit 0, are
rank 1 and have them all. Each sends its packet, and then ADMIN_LOG (0x81)
with the line as typed, which the server logs under `gm_audit`.

| Command | Packet | Server |
|---|---|---|
| `/h N` | ADMIN_COMMAND 0x10, hidden when N isn't 0 | helpers: hides or shows the GM, ADMIN_RESULT 0x10 |
| `/w NAME MSG` | ADMIN_COMMAND 0x1E | helpers: an alert (BROADCAST_MSG 1) to NAME, wherever they are; ADMIN_RESULT 0x1E 1, or 0 for no one by that name |
| `/u`, `/uclip` | ADMIN_COMMAND 0x11 0 and 0x0C | nothing yet |
| `/ban NAME` | ADMIN_COMMAND 0x03 | admins: `!ban NAME`, for good and with no reason; ADMIN_RESULT 4 0 ("You have successfully blocked access."), or 6 1 ("invalid character name") for no one by that name or a GM of the sender's level or higher |
| `/block NAME DAYS REASON`, then its dialog (Admin/Block: the days, the reason by BlockReason.img `type`, as typed, and a note; no days or no such reason opens nothing) | ADMIN_COMMAND 0x04: name, reason (1..=13), days, note | admins: `!ban NAME DAYS REASON NOTE`, with the same replies; the client never sends no days, and the server ignores it |
| `/alertall`, `/alertch`, `/alertmap`, `/noticeall`, `/noticech`, `/noticemap`, `/slideall`, `/slidech`, `/slidemap` TEXT | ADMIN_BROADCAST (0x76): scope (0 world, 1 channel, 2 map), type (0 notice, 1 alert, 4 slide), text | GMs: BROADCAST_MSG of that type to that scope |
| `/slideallx`, `/slidechx`, `/slidemapx` | ADMIN_BROADCAST with no text | GMs: takes the slide down |
| `/slideax` | 0x86 | nothing |
| `/ch N` | the channel menu's request | |

`/flag` (0x72, rank 4 only), `/c` and `/clip` aren't in HeavenClient.

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
