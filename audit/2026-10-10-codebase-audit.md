# RustMS codebase audit — 2026-10-10

> **Remediation status (branch `refactor/audit-remediation`).** Everything in
> sections 2–7 was addressed except the items marked *deferred* below. CI was
> out of scope by request.
>
> | Area | Status |
> | --- | --- |
> | Bugs #1–#17 (section 2) | Fixed (#12 and #15 in the client and server) |
> | Protocol conformance (per-channel ports, canonical `PLAYER_LOGGEDIN`, whisper modes, client fixes) | Done |
> | Authority model, direct field handles, `try_send` fan-out, field tick, supervision | Done |
> | DB pool, startup migrations, dirty-tracked autosave and save on disconnect | Done |
> | Shared encoders, opcode registry with session stages, typed per-server actions | Done |
> | Inventory/equipment/skills schema, indexes, starting equipment, real avatar looks | Done |
> | Game data loaded at startup; asset checksums and verification script | Done |
> | Golden-byte tests for packet builders | Done |
> | Docs refresh | Done |
> | Scripting model | Proposed in `agent_docs/scripting.md`; needs an owner decision |
> | Moving quest/dialogue logic out of `net`, EXP table out of `db`, generic quest loader | *Deferred*: a parallel session is actively changing that code; the scripting decision should drive it |
> | Widening `meso`/quest ids | *Not done on purpose*: v83 encodes meso as int32 and quest ids as int16 on the wire, so wider columns would not help |
> | Equipment base stats from item data (Character.nx) | *Deferred*: the server does not load item data yet; starting equips have zero stats |
> | PIC/PIN, rate limiting, auto-account creation | *Deferred* (PIC is still reported as disabled) |
> | Client: BASS/Win binaries, -Wall/ASan, mac/Docker/Vagrant pins | *Deferred* |

Scope: Rust server (`runtime`, `rust-ms`, `net`, `packet`, `crypt`, `db`, `game-data`, `tools`),
`HeavenClient`, client↔server protocol fit, tests/tooling/docs. Read-only audit; findings marked
✅ were re-verified by hand against the code.

Objective signals: `cargo build --workspace` is warning-free; 56 unit tests pass (db: 0);
clippy not installed for the 1.93 toolchain; no CI (only a stub `.travis.yml`).

---

## 1. Verdict

**Server: refactor, not rewrite.** The bones are right — per-field actors, `FieldKey` with
`instance_id`, cancel-safe framing reader, correct crypto, the "handler returns intent"
`HandlerResult` seam, pure dialogue/quiz state machines, NX-backed data, a real e2e harness.
But the things a fuller game needs most — a state-authority model, timers/ticks, persistence
discipline, shared item/avatar encoders, a general data pipeline, and a scripting story — are
absent, and the current shortcuts sit exactly where mobs/inventory/parties will land.

**Client: usable dev tool, unreliable spec, growing liability.** HeavenClient is small and
patchable and already implements more than the server uses (mobs, drops, attacks, inventory,
shops, skills). But social/economy systems are stubs, it depends on mixed v83/v167/v209 NX data,
the build is unpinned and Linux-only, and — most importantly — several patches **bent the client
to fit the server** rather than making the server speak canonical v83. The server today would not
work with a stock v83 client.

---

## 2. Confirmed bugs worth fixing now

| # | Sev | Finding | Location |
|---|-----|---------|----------|
| 1 | Critical ✅ | **Character theft**: char-select attaches client-supplied `cid` without checking it belongs to the session's account. | `net/src/packet/handle/login/char/select.rs:24,50` |
| 2 | Critical ✅ | **Weak world handoff**: world entry trusts client `character_id`/`channel_id`; only check is a `Transition` session row matching char+channel+IP. No one-time token/expiry. | `net/.../world/logged_in.rs:24-25`, `db/src/session/repository.rs:43` |
| 3 | Critical ✅ | **Non-canonical `PLAYER_LOGIN`**: client patched to append a channel byte; real v83 sends only `int cid`. A stock client would fail to enter world. Root cause: all channels share port 8485. | `HeavenClient/Net/Packets/LoginPackets.h:118`, `net/.../logged_in.rs:25` |
| 4 | High ✅ | **Client bug introduced by patch**: `CharlistHandler` reads the charlist *status* byte (always 0) as the channel id, clobbering the user's channel choice. | `HeavenClient/Net/Handlers/LoginHandlers.cpp:159`, `net/src/packet/build/login/char.rs:11` |
| 5 | High ✅ | **Keybind edits never persist**: upsert does `set(key.eq(excluded(key)))` — rewrites the conflict key to itself, never updates `bind_type`/`action`. | `db/src/keybinding/repository.rs:16-25` |
| 6 | High | **Free teleport**: `ChangeMap` with `target != -1` warps to any map. | `net/.../world/change_map.rs:43-56` |
| 7 | High | **Duplicate login**: `register_client` overwrites the entry without kicking the old connection; old actor's later `Disconnected` evicts the new one. `accounts.logged_in` is read but never set ✅. | `runtime/src/actor/world.rs:157-166`, `net/.../login.rs:58` |
| 8 | High | **Nothing saved on disconnect**: `cleanup()` only notifies world. State persists only inside specific handlers. | `runtime/src/actor/client.rs:502-514` |
| 9 | High | **Deadlock risk under load**: client actor awaits `world_tx.send()` (not draining its 32-slot inbox) while world/field await `sender.send()` into that full inbox. One slow TCP writer can stall a field → channel → world. | `client.rs:181-190`, `world.rs:280,303`, `field.rs:221` |
| 10 | Medium ✅ | Negative i16 string length → `vec![0; huge]` panic (pre-auth); connection dropped. | `packet/src/io/read.rs:50-52` |
| 11 | Medium ✅ | `INVALID_OPCODE = 1` == `LoginCredentials`; opcodes ≥ 0x8000 route to the login handler. | `packet/src/model/packet.rs:5` |
| 12 | Medium ✅ | Client `Session::process` discards a 1–5 byte remainder after a packet (split header ⇒ stream desync). Likely the real "garbage" cause; still present. | `HeavenClient/Net/Session.cpp:~110` |
| 13 | Medium ✅ | `schema.rs` declares `buddies`, but no migration creates it (schema drift). | `db/src/schema.rs:40` |
| 14 | Medium ✅ | `Cargo.lock` is gitignored, but the Dockerfile `COPY`s it — fresh clones can't run the harness. | `.gitignore:6`, `Dockerfile.rustms-server:15` |
| 15 | Medium | Whisper modes likely swapped vs v83 (HeavenMS: 0x12 = receive, 0x0A = reply). Both sides were written together so they agree with each other. Verify. | `net/src/packet/build/world/messaging.rs:4-6`, `HeavenClient/.../MessagingHandlers.cpp:29` |
| 16 | Medium | Character creation inserts character then keybinds with no transaction. | `db/src/character/mod.rs:466-474` |
| 17 | Low | 0xDF is `PLAYER_UPDATE` in v83, mislabelled `PartySearch`. | `net/src/packet/op/recv.rs` |

---

## 3. Server: structural issues that block scaling

### 3.1 State authority (Critical)
Character state exists in 4–5 copies: DB row, `ClientActor.session` (`Arc<Mutex<CharacterWrapper>>`),
`WorldServerActor.clients[..].character`, `FieldActor` occupant, plus a DB re-read in quest
rewards. Drift is already visible (world copy keeps login-time 240/190 position; quest level-ups
don't refresh `FieldCharacter.level`; `dialogue.rs:~106` has a comment working around save-over-save
races). There is no path for "stat/meso/inventory changed → client + field + party". The Diesel
row struct (`db::Character`) is the in-game model and is passed straight into packet builders.

### 3.2 Concurrency & topology (Critical/High)
- Every move/chat goes Client → World → Channel → Field (3 hops through a single global World actor).
- No timers anywhere — `FieldActor::run` is a bare `recv()` loop. Mobs, respawns, drop expiry, buffs all need a tick.
- Every packet (including movement) goes through `spawn_blocking`, with the session `mem::replace`d out.
- Sync DB on async threads in places (reattach in `process_actions`; first NX `Map.nx` parse inside `ChannelActor`).
- Actors are fire-and-forget: a panic kills a field silently, no supervision.

### 3.3 Extensibility (High)
Adding one feature touches ~9 places: opcode enum, handler file, `mod.rs`, `get_*_handler` match,
`HandlerAction` variant + `with_*` builder, `ClientEvent`, `ChannelMessage`, `FieldMessage`, and
arms in `process_actions` (~260-line god function), `world.rs`, `channel.rs`. Login and world share
one `HandlerAction` enum, so each server has "ignored" arms. Handlers are sync, see only the session,
and do DB I/O directly — not unit-testable without Postgres. Game logic (quests, dialogue, portal
resolution) lives in `net`; game rules (beginner EXP table, level-up) live in `db/src/quest.rs`.

### 3.4 Packet encoding (High)
No shared encoders for the structures that recur in dozens of v83 packets:
`write_char_meta` is duplicated (`build/world/char.rs:223` vs `build/login/char.rs:556`), avatar look
exists three times with **hard-coded equip ids** (`field.rs:275-277,365`, `login/char.rs:600-638`),
and `write_inventory` / `write_skills` are empty stubs. Only one builder has a golden-byte test.

### 3.5 Persistence (Critical/High)
- `establish_connection()` opens a **new Postgres connection per call** and re-parses config each time; panics on failure (`db/src/lib.rs:25-29`).
- Whole-row `save_changes` on map change = last-writer-wins.
- Schema lacks inventory/equips/skills/buddies/guilds; `meso` is `INTEGER`, quest id is `SMALLINT`; no indexes on `characters.accountid`, `sessions.character_id/account_id`.
- Migrations are applied by hand (diesel CLI) or a psql loop in the harness — not by the server.

### 3.6 Game data & scripting (Critical for scaling)
Reading the client's own NX files is a good choice, and data is loaded once into `&'static`. But:
- Only a thin slice is extracted (portals, NPC life, return maps). No footholds, mobs, items, equips, skills, shops, drops, reactors.
- Quests are special-cased: `for id in 1009..=1015`, NPC `12101` hard-coded (`game-data/src/quests.rs:40,93`, `runtime/src/actor/client/dialogue.rs:34`).
- Loading is lazy on first use (first map change pays the 1.5 GB `Map.nx` walk).
- Two NX parsers (`nx-pkg4` used only as a header check + a hand-written one).
- Assets are unpinned (no fetch script/checksums); asset-dependent unit tests fail on a fresh clone.
- **No scripting layer at all.** Portal `script` fields are parsed and ignored. Thousands of v83 NPC/quest/portal/event behaviours need either an embedded engine (Lua/Rhai/JS) or a data-driven model — decide before more logic is hard-coded.

### 3.7 Channels / multi-server (High)
`instance_id` is always 0; no world id; channels lazily created for any `u8`; default config points
3 channels at the same `127.0.0.1:8485` (which is what forced the non-canonical channel byte).
Whisper/name lookup is process-global — works only because everything is one process. Running
channels as separate processes would need an inter-server bus.

---

## 4. HeavenClient

**What it has (client-side):** login/world/char flows, map rendering (tiles/objs/bg/footholds/portals),
physics, look rendering, mobs, drops & pickup, melee/ranged/magic attacks, skills/buffs/cooldowns,
inventory/equip/skillbook/stats UIs, NPC shop, minimap, world map, keyconfig, chat/whisper, channel change.
~63 recv handlers registered; the server only emits ~21 of them.

**What's missing/stubbed:** buddy/party/guild UI (buttons only, `UIUserList.cpp:41-67`), trade,
storage, minirooms, messenger, family, summons/doors/mists, mob skills/status/attacks, cash-shop
purchase (`UICashShop.cpp:345`), quest start/complete flows, PIC response handling, ping/pong.

**Risks:**
- **Mixed-version data (High):** v83 Character/Item/Mob/Skill/Npc/Quest/String, but v167 `MapPretty` and v209-era `MapLatest`/`Map001`/`UI` (`Util/NxFiles.h:28-37`). Undocumented which UI behaviour is "modern". ~14 GB of NX locally, from a personal Google Drive.
- **Unreproducible build (High):** `build-deps.sh` clones unpinned HEAD for most deps (glad, alure, lz4, asio, stb, NoLifeNx fork); CMake is effectively Linux-only (hard-coded mac Cellar paths, no Windows branch). No `-Wall`, sanitizers, tests, or CI.
- **Robustness (Medium):** inbound reads are bounds-checked and exceptions are caught per packet (good), but handlers mutate state before finishing parsing; server-supplied indices used unchecked; any disconnect quits the client; synchronous map loads with an unclamped fixed-timestep accumulator.
- **Dead/proprietary vendored code:** BASS (proprietary, commented-out) and Win64 GLFW binaries in `includes/` (~32 MB tracked).
- **License:** client is AGPLv3 (root is GPLv3). Separate-process socket communication doesn't infect the server, but distributing a modified client binary requires offering source.
- **Upstream:** appears dormant (deps last pinned 2020, Ubuntu bionic Vagrantfile).

**As a protocol spec:** not trustworthy on its own. It skips fixed blobs and ignores trailing bytes
where it's incomplete, so a server writing zeros/garbage passes locally. The harness builds packets
by hand too, so it validates the server against itself — conformance to real v83 is untested.

**Patch review (since import `195a0a2`):** channel change packet, `SetField` CharEntry caching,
quest status/QuestLog, and the `NpcTalkMorePacket` wire-type fix are legitimate. The `PLAYER_LOGIN`
channel byte and whisper modes are server-fitting deviations; the charlist channel read is a new bug;
the "garbage" fix (921e81b) mostly masked the server-side partial-frame bug later fixed in `ca10b61`
(the build uses Asio, so the Winsock loop change is Windows-only).

---

## 5. Tests, tooling, docs

- Harness: good isolation (random identities, Dockerized Postgres + real migrations, log dump on failure, partial-frame regression test). Weaknesses: timeout-based negative assertions (500 ms), 100 ms quiet-period draining, ~20× copy-pasted `timeout(...).expect().expect()` (no `expect_packet` helper), hard-coded NPC object ids.
- Untested: keybinds, char delete, PIC, db crate (0 tests), almost all packet builders.
- No CI, no rustfmt/clippy config. Editions split 2018/2021; duplicate majors (`rand` 0.7/0.8, `thiserror` 1/2, `toml` 0.5/0.8); stale `config 0.10`.
- Stale docs: README (claims login-only server, `rust-ms-login` crate, threads, `cargo run` at root); `server-architecture.md:140` (map change does move between FieldActors now); `RUSTMS_WORLD_REDIRECT_*` env vars don't exist; redirect is no longer hard-coded to 8485; harness test list/count outdated; broken link to `integration-harness.md`; `fields.md` missing from the CLAUDE.md index.

---

## 6. Keep

Per-field actor ownership and `FieldKey`; cancel-safe framing reader (`runtime/src/io/read.rs`);
crypto crate; `HandlerResult` intent seam (slimmed down); pure `Conversation`/`QuizConversation`
state machines; `quest::transition` (`FOR UPDATE` + transaction) as the model for multi-row writes;
NX as the shared data source; map-data-validator's cross-source idea; the e2e harness; agent_docs.

---

## 7. Recommended order of work

**Phase 0 — hygiene & correctness (small, do first)**
1. Fix bugs #1, #2, #5, #6, #7, #10, #11, #13, #14, #16 above.
2. Track `Cargo.lock`; add a CI job (build, unit tests, clippy, fmt); install clippy.
3. Fix stale docs; add `fields.md` to the index.

**Phase 1 — protocol conformance (decide the client strategy)**
4. Make the server canonical v83: one listener *port per channel* (can stay one process), derive channel from the accepting listener → drop the `PLAYER_LOGIN` channel byte and the charlist hack. Verify whisper modes.
5. Fix `Session::process` remainder handling in the client.
6. Establish a conformance reference: packet captures or the official v83 localhost client for spot checks, plus golden-byte tests for every builder.

**Phase 2 — runtime core**
7. Authority model: `ClientActor` owns a domain `Player`/`CharacterState` (separate from Diesel rows) that emits change events; World/Field hold read-only projections or nothing.
8. Client talks to its `FieldHandle` directly; World becomes a directory (names, whisper, later party/guild).
9. Fan-out via `try_send` (full queue ⇒ slow client ⇒ kick); actor supervision.
10. Field tick (`select!` over `interval` + mailbox) and an object-id allocator — prerequisite for mobs/drops.
11. DB: connection pool, config parsed once, migrations at startup, dirty-tracked periodic + on-disconnect save; stop `spawn_blocking` for pure handlers.

**Phase 3 — scaffolding for content**
12. Shared encoders: `AvatarLook`, `CharStats`, `Item`/`Equip`, `CharacterInfo` with golden tests.
13. Opcode registry (per-server tables with required session state) replacing the dual `match`; per-server typed actions; a `game` crate for logic out of `net`/`db`.
14. Schema for inventory/equips/skills/quest progress/buddies; widen `meso`.
15. Eager, typed `Arc<GameData>` built from NX (maps+footholds+life, mobs, items, quests generic, shops); pin assets with checksums + fetch script.
16. Choose a scripting model (embedded Lua/Rhai/JS vs data-driven) and port Rain's quiz onto it as the first consumer.

**Client, in parallel:** pin `build-deps.sh` to commits, add `-Wall` + an ASan build, drop BASS/Win
binaries, document which NX files are which version, and before committing to "complete" scope,
estimate the client work for party/buddy/guild/trade/storage/cash shop/mob effects. If that list is
too big, plan to use the official v83 client for protocol-heavy features and keep HeavenClient as
the debuggable dev client.
