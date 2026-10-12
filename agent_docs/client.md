# HeavenClient

## What it is

`HeavenClient/` is a vendored copy of HeavenClient, the C++ MapleStory v83 client
by Daniel Allendorf, Ryan Payton, and contributors (continuation of the Journey
client; upstream tested only against HeavenMS). It is a from-scratch client, not
Nexon's binary, so it speaks the v83 wire protocol and renders from NX files.

License: `HeavenClient/LICENSE` is AGPLv3 (the repository root LICENSE is GPLv3).
Anyone who distributes a modified client binary must offer the corresponding
modified source under AGPLv3 and keep the copyright headers. Running it locally
is unrestricted. The NX files are converted Nexon assets, are not covered by this
license, and must not be redistributed with the client.

## Build

```sh
cd HeavenClient
./build-deps.sh                       # clones and builds into libs/ (pinned, see below)
cmake -S . -B cmake-build
cmake --build cmake-build -j"$(nproc)"
```

`build-deps.sh` checks out every dependency at an exact commit (`*_REV`
variables at the top): alure, openal-soft, glad, lz4, NoLifeNx (`nlnx`), glfw,
asio, stb. `build-deps.mac.sh`, `Dockerfile`, and the Vagrant setup still clone
unpinned HEAD (openal-soft and glfw excepted). `libs/` and `nx/` are gitignored
and may be symlinks to another checkout.

`MapleStory.h` must keep `USE_ASIO` and `USE_CRYPTO` enabled. Native logic tests
are described in [client-testing.md](client-testing.md).

## Required NX files

NX files are loaded from the working directory (`Util/NxFiles.cpp`); 18 are
required, and startup fails with `MISSING_FILE` for the first absent one.

| File | Source version | Notes |
| --- | --- | --- |
| Character, Effect, Etc, Item, Mob, Morph, Npc, Quest, Reactor, Skill, Sound, String, TamingMob | v83 GMS | Must match server data |
| Map | v83 GMS | Server shares this file (`assets/game-data/Map.nx`) |
| MapPretty | v167 GMS Map.wz | Higher-quality tiles and backgrounds (~4.5 GB) |
| MapLatest | v209.2 / latest GMS Map.wz | Newer map art (~3.7 GB) |
| Map001 | v209.2 / latest GMS Map001.wz | Login backgrounds and similar (~2.4 GB) |
| UI | v209.2 / latest GMS UI.wz | Post-Big-Bang UI layout (~1.2 GB) |

Total is about 14 GB. In this repo `Map`, `Npc`, `Quest`, and `String` are
symlinks to `assets/game-data/` so client and server read the same v83 data.
`Base.nx` is present locally but not in the required list.

Startup version check: `NxFiles::init` probes `UI.nx` for
`Login.img/WorldSelect/BtChannel/layer:bg`. If it is not a bitmap the UI file is
too old and the client reports `WRONG_UI_FILE`. There is no check on the other
files, so mixing in a mismatched v83 file fails later at runtime.

The mixed UI/Map versions mean the client uses post-v83 art with v83 data, and
some UI nodes may not exist for v83 features.

## Fonts

`fonts/` is copied next to the executable at build time, and `Settings` names
the files (`FontPathNormal`, `FontPathBold`, `FontPathBalloon`). The original
client draws text in Arial, which can't be shipped. Most text uses Roboto
(Apache 2.0), a deviation from the original look. Chat balloons use Arimo
(OFL 1.1), which has Arial's metrics, at 12 px with 14 px lines as in the
original; see `IO/Components/ChatBalloon.h`.

## What works and what is stubbed

This is client-side capability only. The RustMS server currently implements
login, character management, movement, chat, whispers, portals, channel
changes, NPC dialogue, and a few quests; everything else below has no server
side yet.

| Area | Status |
| --- | --- |
| Login, world/channel select, character list/create/delete, PIC | Implemented |
| In-map play: movement, jumping, portals, map change, channel change | Implemented |
| Combat: melee/ranged/magic attacks, mob HP/death, drops, pickup | Implemented |
| Inventory, equip, item use, scroll, sort/gather | Implemented |
| Skills, stat/AP/SP allocation, buffs, keymap | Implemented |
| NPC dialogue and NPC shop | Implemented (`UINpcTalk`, `UIShop`) |
| Quest log and markers | Implemented (`UIQuestLog`); progress arrives via SHOW_STATUS_INFO, quest start/complete is driven by NPC dialogue |
| Chat, whisper, server messages | Implemented; whisper find/buddy modes ignored |
| Character info, minimap, world map | Implemented |
| Buddy / party / guild / boss list (`UIUserList`) | Buttons and layout only; no handlers, party packet exists but is unused |
| Cash shop (`UICashShop`) | Browse/enter/exit; purchase is `// TODO: Purchase item` |
| Storage, trade, player shops, mini-games, messenger, memo/mail | No UI, no handlers |
| Pets, mounts, summons, reactors beyond hit/spawn | Partial or absent (`Reactor` TODOs) |
| Party/guild/boss quests, events, MTS | Absent |

44 outbound opcodes are defined (`Net/OutPacket.h`) and 57 inbound handlers
are registered (`Net/PacketSwitch.cpp`). Anything not listed there is silently ignored by the client.

## Estimated client work for a complete game

| System | Client status | Effort |
| --- | --- | --- |
| Buddy list | UI skeleton, no packets | M |
| Party (create/invite/HP bars) | UI skeleton, `PARTY_OPERATION` packet only | M |
| Guild | Skeleton only, plus guild emblem/BBS UIs missing | L |
| Trade / storage / player shops | Nothing | L |
| Cash shop purchase and inventory | Display only | M |
| Quest accept/complete UI flow | Partial | S |
| Pets and mounts | Partial rendering, no interaction | M |
| Messenger, memo, mail, MTS | Nothing | M each |
| Whisper find/buddy modes | Ignored | S |
| Mini-dungeons, boss/event UI | Nothing | M |

## Protocol deviations fixed

The client had been patched to fit the server instead of speaking canonical v83.
These were corrected:

1. `PLAYER_LOGIN` (opcode 20) wrote `int cid, byte channel`; it now writes only
   `int cid` (`Net/Packets/LoginPackets.h`). The server identifies the channel by
   the port it accepted the connection on. `Configuration::channelid` is still
   kept client-side for UI (channel indicator, channel-change window).
2. `CharlistHandler` treated the first CHARLIST byte as the channel and overwrote
   the user's selection. It is a status byte; it is now skipped. The channel is
   set in `UIWorldSelect` and by `SetFieldHandler`/`UIChannel`.
3. Whisper sub-modes were swapped. Canonical: `0x12` incoming whisper (sender,
   0-based channel byte, admin flag, message) and `0x0A` reply to our whisper
   (target, success byte). Mode `0x09` (find reply) is ignored, not misparsed.
4. `Session::process`/`read` dropped any trailing bytes shorter than a minimum
   packet and ignored small reads, so a 4-byte header split across TCP reads
   desynchronised the stream. Headers are now accumulated across reads, bodies
   are buffered, several packets per read are all processed, empty bodies are
   handled, an oversized length drops the connection, and receive state is
   cleared (and stale input abandoned) when a handler reconnects.
5. The map-change SET_FIELD's HP short (after the portal byte) was not read.
   It now sets the player's HP, which is how a dead player gets back up.
6. Dependency pins in `build-deps.sh` (see Build).
7. SHOW_ITEM_GAIN_INCHAT mode 3 (quest item gains and losses) is a count byte
   followed by an item id and quantity for each. The client read the count as a
   sub-mode and showed nothing unless it was 1. It now shows each item as its
   own grey (`#BBBBBB`) chat line, worded as in period footage: "You have
   gained an item (Roger's Apple)", "You have gained items (3 Green Apple)",
   and "You have lost ..." for losses.
