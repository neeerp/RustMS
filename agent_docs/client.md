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
  Esc opens the Game Menu; the MENU key action opens ShortCut. Dropping a
  skill, or a USE or SETUP item, on a quick slot binds that slot's key and
  saves the keymap right away; an open Key Config shows the new binding.
  Status messages (`UIStatusMessenger`) stack upward from just above the
  quick slot panel, right-aligned with it; `UIStatusBar::QUICKSLOT_TOP` is
  the shared anchor.
- **Chat** (`UIChatBar`): the input strip in the bar's top-left and the log
  above it. The status bar cuts that strip out of its background, so the two
  can overlap in either draw order.
- **NPC dialog** (`UINpcTalk`, `UIWindow.img/UtilDlgEx`): the frame,
  buttons, menu dots, quest list headers, and the `Basic.img/VScr4` scroll
  bar (`Slider::Type::CLASSIC`). See [npc-dialogue.md](npc-dialogue.md).
- **Quest Helper** (`UIQuestHelper`, `UIWindow.img/QuestAlarm`). See
  [quests.md](quests.md).
- **Quest Complete! notice** (`UIQuestComplete`, `UIWindow.img/FadeYesNo`).
  See [quests.md](quests.md).
- **Minimap** (`UIMiniMap`, `UIWindow.img/MiniMap`): folded to its title bar,
  the map alone (`MinMap`), or the map under the map's mark, street, and name
  (`MaxMap`), laid out from period footage. M or the `Basic.img` minus and
  plus buttons step between them, and other maps keep the last choice
  (`MiniMapType`, the largest until one is made). Towns open in the largest,
  and maps without a `miniMap` stay folded. The window fits the map's `miniMap/canvas` (scaled
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
- **Item window** (`UIItemInventory`, `UIWindow.img/Item`): five tabs
  (`Item/Tab`, drawn by `ClassicTabs`), six rows of four slots with a `VScr4`
  scroll bar, and the mesos, laid out from period footage. BtFull widens it to
  `FullBackgrnd`, four pages of 24 with no scroll bar; BtSmall goes back.
  Slots past the tab's limit show `disabled`. A gained item pulses its tab
  (`New/Tab0`, or `Tab1` when selected) and its slot (`New/inventory`) until
  the window closes. BtGather and BtSort rearrange the bag, and BtCoin asks
  how many mesos to drop.
- **Equipment window** (`UIEquipInventory`, `UIWindow.img/Equip`): the labelled
  `backgrnd` grid (33 px cells from (5,35)) with an icon per worn slot, and the
  `pet` panel that PET EQUIP opens beside it, laid out from period footage.
  Cells that cannot be used are tinted red as in the footage: PANTS under an
  overall, SHIELD beside a two-handed weapon, and the riding cells without
  Monster Riding. RustMS has no pets, so the pet tabs are disabled and the pet
  and MOB EQUIP cells hold nothing.
- **Channel window** (`UIChannel`, `UIWindow.img/Channel`): the `t`, tiled
  `c` and `s` pieces around a six-column grid of 68×20 cells (70×21 apart from
  (9,67)), with a row per six channels, laid out from period footage. Every
  cell in those rows is drawn, `channel0` where there is no channel. The
  selected channel is `channel2` and the current one `channel1`. The world
  name is `world/<world id>`.
- **Key config** (`UIKeyConfig`, `UIWindow.img/KeyConfig`): the keyboard with a
  `key` label over each bindable key's icon, and a three-row palette of unbound
  functions (`icon/N`, by v83 function id), laid out from the background and
  period footage. Drag an icon between keys or onto the palette to unbind it.
  Esc, Tab, and the other keys printed on the background are fixed. DEFAULT
  restores the server's defaults; DEFAULT, CLEAR ALL, and Esc with unsaved
  changes ask through the in-window `notice` bitmaps. CHANGE QUICK SLOTS is
  disabled until the quick slot is configurable.
- **Maple Messenger** (`UIMessenger`, `UIWindow.img/Messenger`): three seats
  with the members standing in them, the chat log, and the input line, laid
  out from period footage. The MAPLECHAT key or ShortCut's MESSENGER button
  opens a room, or minimises the open one to its title bar (which blinks on
  unread lines). Clicking an empty seat or typing `/invite name` invites; `/q`
  or the close button leaves. A speaker shows the `chatBalloon` dots for a
  few seconds. Invitations arrive as a yes/no notice.
- **Game Option** (`UIGameOption`, `UIWindow.img/GameOpt`), from MENU: a
  `Basic.img/CheckBox` right of each row's ALLOW, laid out from the
  background. OK saves the boxes as the `Allow*` settings; the client doesn't
  act on them yet.
- **System Option** (`UISystemOption`, `UIWindow.img/SysOpt`), from MENU:
  `Basic.img/Slider` sliders (`ClassicSlider`) between each row's end labels,
  mute, screen shake, and WINDOW / FULL SCREEN boxes, laid out from the
  background. The BGM and SOUND sliders change the volume as they move, in
  steps of 10, and MUTE greys the slider out; Cancel and Esc put the volume
  back. OK saves `BGMVolume`, `SFXVolume`, `BGMMute`, `SFXMute`, and the
  stored-only `PictureQuality`, `MouseSpeed`, `HPAlert`, `MPAlert` and
  `ShakeScreen`, and switches to the chosen viewing mode. MONSTER INFO has
  no control, and the post-Big Bang Options window's resolution picker is
  gone; set `Width` and `Height` in `Settings` instead.
- **Notices** (`UIOk`, `UIYesNo`, `UIEnterNumber`, `Basic.img/Notice2` and
  `YesNo2`): the silver bolted boxes. `t`, then `c` repeated to fit the black
  message, then `s` with one button frame (`BtOK`) or two (`BtYes`/`BtNo`, or
  `BtOK`/`BtCancel` for number entry). Number entry draws a white field under
  the message. The click that opens it no longer takes the focus from its
  field, so typed amounts count. `UIEnterNumber` has its own element type, so the notice about a
  bad number opens over it, and OK goes back to the field.
- **Quit** (`ask_to_quit`): MENU's QUIT GAME and closing the window ask
  "Are you sure you want to quit?" in a `UIYesNo`; yes goes back to the login
  screen. The later playtime and EXP summary (`UIWindow6.img/askReward`) is
  gone.
- **NPC shop** (`UIShop`, `UIWindow.img/Shop`): the shop's list on the left
  under the NPC, your bag on the right under you and your mesos, five 40 px
  rows a side with a `VScr4` scroll bar each, laid out from the background
  and period footage. The sell side has the five `Basic.img/Tab5` bag tabs;
  the buy side shows only the "All" tab. A star or bullet set below its full
  size shows `BtRecharge` on its row, which recharges it at once. The
  right-click-to-sell checkbox is gone; right-click still sells.
- **Character info** (`UICharInfo`, `UIWindow.img/UserInfo`): opened by
  double-clicking a character on the map, the player included
  (`Stage::character_at`, a body-sized box above the feet in
  `MapleMap/CharClick.h`; NPCs under the cursor take the click first). It shows
  a copy of the character's look and name, and the level, job, fame, guild, and
  alliance from CHAR_INFO, laid out from the background and the v83 client.
  Double-clicking someone else replaces the window. TAMING MOB, PET INFO, and
  COLLECTION lengthen it to `backgrnd5`, `backgrnd2`, and `backgrnd7`, and ITEM
  and WISHLIST open `backgrnd4` and `backgrnd3` beside it; all are empty, as
  RustMS has no mounts, pets, monster book, or cash shop wish list. Family,
  party, trade, fame, and excluded items are disabled.
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
| Chat, whisper, `/find`, server messages | Implemented; buddy whisper/find modes ignored |
| Character info, minimap, world map | Implemented |
| Party (`UIUserList` Party tab, `UIPartyHP`, party chat) | Implemented; see `parties.md` |
| Buddy list (`UIUserList` Friend tab, buddy chat) | Implemented without groups; see `buddies.md` |
| Guild / alliance / blacklist tabs (`UIUserList`) | Tab strip only |
| Monster Book (`UIMonsterBook`) | Implemented: cards, summary, monster pages, search, cover; see `combat.md` |
| Cash shop (`UICashShop`) | Enter/exit, catalog, buying, gifts, Cash Inventory, wish list, and moving items to and from the bags |
| Maple Messenger (`UIMessenger`) | Implemented |
| Storage, trade, player shops, mini-games, memo/mail | No UI, no handlers |
| Pets, mounts, summons, reactors beyond hit/spawn | Partial or absent (`Reactor` TODOs) |
| Party/guild/boss quests, events, MTS | Absent |

51 outbound opcodes are defined (`Net/OutPacket.h`) and 63 inbound handlers
are registered (`Net/PacketSwitch.cpp`). Anything not listed there is silently ignored by the client.

## Cash Shop

`UICashShop` draws the 800x600 v83 Cash Shop from `UIClassic.nx`
`CashShop.img`, matched against 2008–2010 footage. The window doesn't change
the screen size.

- **Catalog:** `Etc.nx` `Commodity.img` is the catalog and `Category.img`
  names the sub-tabs. A commodity's SN holds its tab and sub-tab (50200003 is
  tab 5, ETC, sub-tab 2, Game). `Data/CashCatalog.h` sorts a sub-tab by
  `Priority` (highest first), then SN. HOT items have the highest priorities,
  so a sub-tab's first page is all HOT, as in the footage. `Class` picks the
  NEW/SALE/HOT/EVENT label, and unlabelled Event > New items show NEW.
- **Gender:** items for the other gender are hidden. Commodities whose item has
  no data are skipped.
- **Background:** the preview background follows the job: Cygnus and Aran have
  their own.
- **Packets:** SET_CASH_SHOP gives the account name and the Best Item SNs for
  the player's gender. QUERY_CASH_RESULT (324) gives the NX Credit, Maple Point
  and NX Prepaid balances. CASHSHOP_OPERATION (325) modes 0x4B (Cash
  Inventory), 0x4D (gifts received since the last visit), 0x4F and 0x55 (wish list),
  0x57 (a bought item), 0x5C (a failure), 0x5E (a gift sent), 0x68 (an item
  taken out) and 0x6A (an item put back) are read. Inventory slots keep the item's cash serial,
  which the item encoding carries, so a cash item can find its way back.
- **Buying:** BUY opens `UICashBuy` (`CashShop.img/CSNotice/0`): the item, its
  price and period, and a checkbox per balance. It checks the first balance
  that covers the price, trying NX Prepaid, NX Credit, then Maple Points. OK
  sends CASHSHOP_OPERATION (229) 0x03 with the cash type (1 Credit, 2 Maple
  Points, 4 Prepaid) and SN. The result is a "successful purchase" or a
  failure notice.
- **Wish list:** RESERVE and REMOVE send the whole list (0x05, ten SNs), and the
  server answers with the list it kept.
- **Taking out and putting back:** double-clicking a Cash Inventory item
  sends 0x0D with its serial, bag and the bag's first free slot; the server
  answers with the slot and the item (0x68), or 0xBB if the bag is full.
  Double-clicking a cash item in the Item Inventory sends 0x0E, and the server
  answers with the Cash Inventory item (0x6A). The footage doesn't show
  whether v83 used double-clicks or dragging; dragging isn't supported.
- **Gifts:** GIFT opens `UICashGift` on `CashShop.img/CSGift/backgrnd`, with
  a To field and a message box that wraps over its two ruled rows (73
  characters). OK, once both are filled, swaps the art for `CSNotice/1` and asks
  for the birthday code (YYYYMMDD). Its OK sends 0x04 with the code, SN, name
  and message. The sender sees "You have sent ITEM to NAME." on 0x5E, or a
  failure notice (0xA8 own account, 0xA9 unknown name, 0xC4 birthday code).
  The receiver gets one notice listing each gift's sender, item and message
  when 0x4D arrives. The buddy and guild buttons that open the wider
  `backgrnd1` aren't drawn, and only one name is sent.
- **Not served yet:** the slot expansions, CHARGE, CHECK CASH and COUPON do
  nothing.

## Estimated client work for a complete game

| System | Client status | Effort |
| --- | --- | --- |
| Buddy groups | Default group only | S |
| Guild | Skeleton only, plus guild emblem/BBS UIs missing | L |
| Trade / storage / player shops | Nothing | L |
| Cash shop packages | Buying, gifts, the wish list and moving items only | M |
| Quest accept/complete UI flow | Partial | S |
| Pets and mounts | Partial rendering, no interaction | M |
| Memo, mail, MTS | Nothing | M each |
| Buddy whisper/find modes | Ignored | S |
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
8. The stat window's HP and MP buttons sent SPEND_AP with the current HP and
   MP codes (0x400, 0x1000). They now send MaxHP and MaxMP (0x800, 0x2000),
   which is what AP raises.
8. Keymap keys are v83 scan codes, where the number row runs 1 to 9 then 0 from
   key 2. `Keyboard::Keytable` had 0 to 9 from key 2, so every number-row
   binding sat one key off. The `v83_number_row_keys` migration moves saved
   bindings, and the server's default party chat is on key 3 (the 2 key).
9. OPEN_NPC_SHOP told stars and bullets from other items by whether the short
   after the row's zero ints was 1, then read their 8-byte recharge rate as a
   skip and a short. It now decides by item id (`207xxxx`, `233xxxx`) and
   reads the double (`Net/Handlers/ShopList.h`). Rows without a price, which
   are there only to recharge, are not shown for sale but give the full set
   for the Rech. button, and buying sends the row's index in the server's
   list rather than its place on screen. Stars are bought one
   set at a time and sold as a whole set, both after a yes/no. The new
   CONFIRM_SHOP_TRANSACTION handler shows "You do not have enough mesos." and
   "Please check if your inventory is full or not."; codes 0 and 8 need no
   notice.
