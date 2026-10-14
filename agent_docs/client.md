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

NX files are loaded from the working directory (`Util/NxFiles.cpp`); 19 are
required, and startup fails with `MISSING_FILE` for the first absent one.

| File | Source version | Notes |
| --- | --- | --- |
| Character, Effect, Etc, Item, Mob, Morph, Npc, Quest, Reactor, Skill, Sound, String, TamingMob | v83 GMS | Must match server data |
| Map | v83 GMS | Server shares this file (`assets/game-data/Map.nx`) |
| MapPretty | v167 GMS Map.wz | Higher-quality tiles and backgrounds (~4.5 GB) |
| MapLatest | v209.2 / latest GMS Map.wz | Newer map art (~3.7 GB) |
| Map001 | v209.2 / latest GMS Map001.wz | Login backgrounds and similar (~2.4 GB) |
| UI | v209.2 / latest GMS UI.wz | Post-Big-Bang UI layout (~1.2 GB) |
| UIClassic | v83 GMS UI.wz | The original UI (~72 MB); see below |

Total is about 14 GB. In this repo `Map`, `Npc`, `Quest`, and `String` are
symlinks to `assets/game-data/` so client and server read the same v83 data.
`Base.nx` is present locally but not in the required list.

Startup version checks: `NxFiles::init` probes `UI.nx` for
`Login.img/WorldSelect/BtChannel/layer:bg`. If it is not a bitmap the UI file is
too old and the client reports `WRONG_UI_FILE`. It probes `UIClassic.nx` for
`UIWindow.img/MiniMap/title`, which the Big Bang UI dropped, and
`UIWindow.img/AranSkillGuide/0`, which v83 added; without both the client
reports `WRONG_UI_CLASSIC_FILE`. There is no check on the other files, so
mixing in a mismatched v83 file fails later at runtime.

### UIClassic.nx

Issue #120 moves the client back to the v83 UI one window at a time. Windows
that have moved read `NxFiles::ui_classic`; the rest still read `nl::nx::ui`
from `UI.nx`. Moved so far:

- **Status bar** (`UIStatusBar`, `StatusBar.img`): gauges, menu buttons, the
  quick slot panel, and the Game Menu and ShortCut popups (`UIWindow.img`).
  Esc opens the Game Menu; the MENU key action opens ShortCut.
- **Chat** (`UIChatBar`): the input strip in the bar's top-left and the log
  above it. The status bar cuts that strip out of its background, so the two
  can overlap in either draw order.
- **NPC dialog** (`UINpcTalk`, `UIWindow.img/UtilDlgEx`): the frame,
  buttons, menu dots, quest list headers, and the `Basic.img/VScr4` scroll
  bar (`Slider::Type::CLASSIC`). See [npc-dialogue.md](npc-dialogue.md).
- **Quest Helper** (`UIQuestHelper`, `UIWindow.img/QuestAlarm`). See
  [quests.md](quests.md).
- **Minimap** (`UIMiniMap`, `UIWindow.img/MiniMap`): folded to its title bar,
  the map alone (`MinMap`), or the map under the map's mark, street, and name
  (`MaxMap`), laid out from period footage. M or the `Basic.img` minus and
  plus buttons step between them; towns open in the largest, maps without a
  `miniMap` stay folded. The window fits the map's `miniMap/canvas` (scaled
  by `2^mag`) and the names. Markers (`Map.nx` `MapHelper.img/minimap`) stand
  bottom-centred on portals to other maps, NPCs, other players, and you.
  Hovering an NPC or portal shows its name. Period canvases had the map's
  background baked in; v83 `Map.nx` canvases are transparent there, so the
  frame shows through.
- **World Map** (`UIWorldMap`, `UIWindow.img/WorldMap`): the frame, title,
  and `Basic.img/BtClose` around the map's 640x470 `BaseImg`, laid out from
  period footage. The Big Bang search panel is gone. Esc steps up to the
  parent map and closes the window from the top.
- **Map tooltips** (`MapTooltip`, `TooltipBox`): the World Map's spots and the
  minimap's NPCs and portals show the name, plus the map's description for
  World Map spots, in the v83 box. `UIClassic.nx` has no bitmaps for it (the
  original drew it), so `TooltipBox` draws the dark edge, light edge, and
  translucent navy fill measured from footage.
- **Stat window** (`UIStatsInfo`, `UIWindow.img/Stat`): the main panel and,
  behind DETAIL, the attack and defence panel (closed with `Basic.img/BtHide`),
  laid out from period footage. The job shows as the `Job/main` and `Job/sub`
  bitmaps for explorers and Cygnus Knights. Beginners get the greyed INT and
  LUK labels and no HP or MP buttons. The AP buttons and AUTO-ASSIGN are
  disabled without AP. EXP shows as "exp (pct%)" and updates as it is gained.
- **Skill window** (`UISkillBook`, `UIWindow.img/Skill`): one tab per job
  advancement (`Basic.img/Tab5`, drawn by `ClassicTabs`), the book and name of
  the selected one, four skill rows with a `VScr4` scroll bar, and the points
  left, laid out from period footage. The job name is 10 px Arimo
  (`Text::Font::A10M`). Rows are grey when a required skill is missing. SP-up
  spends a point at once; v83 had no allocation popup. The MACRO button and the
  `SkillMacro` window are not drawn yet.
- **Quest log** (`UIQuestLog`, `UIWindow.img/Quest`): Available, In Progress,
  and Complete tabs (`Basic.img/Tab2`), quests folded by area, and a detail
  panel with the quest's text and FIND NPC or QUEST HELPER, laid out from
  period footage. See [quest-window.md](quest-window.md).

`UIClassic.nx` is not one of NoLifeNx's standard files, so `NxFiles` opens it
with `nl::file` instead of `nl::nx::load_all`.

Convert it from a v83 GMS `UI.wz` with:

```sh
tools/convert-ui-classic.sh PATH/TO/UI.wz
```

The script builds NoLifeWzToNx at a pinned commit, with fixes for its
`wchar_t` string conversion (4 bytes on Linux, so every string throws) and its
`std::experimental::filesystem` include, then writes `HeavenClient/nx/UIClassic.nx`.
It checks both files against the ones the client was written against:

| File | SHA-256 |
| --- | --- |
| `UI.wz` (input, 28 MB) | `d253934d986cd428ba914c37f1bfae18513e2830ec3ec5d0e055fd2cef05e47c` |
| `UIClassic.nx` | `d2e0e1ce6f1864e99ad6fdd1017e3495e72802744f68e9ffad54e61794b4d98a` |

The converter cannot inflate one bitmap in this `UI.wz`,
`UIWindow.img/FadeYesNo/icon7/icon5` (18x18), and writes it blank.

The mixed UI/Map versions mean the client uses post-v83 art with v83 data, and
some UI nodes may not exist for v83 features. That goes away for the UI as
windows move to `UIClassic.nx`.

## Fonts

`fonts/` is copied next to the executable at build time, and `Settings` names
the files (`FontPathNormal`, `FontPathBold`, `FontPathBalloon`). The original
client draws text in Arial, which can't be shipped, so all text uses Arimo
(OFL 1.1), which has Arial's metrics and widths.

- **Line heights:** the UI fonts keep the line heights the window layouts were
  tuned to under Roboto, which the client used before; they are listed in
  `GraphicsGL::init`.
- **Chat balloons:** these use 12 px Arimo with 14 px lines, as in the
  original; see `IO/Components/ChatBalloon.h`.
- **Old `Settings` files:** a file that still names the old Roboto defaults
  gets the Arimo defaults instead (`Configuration::load`).

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
