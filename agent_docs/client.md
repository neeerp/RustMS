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
  the shared anchor. Each fades out over 6 s. SHOW_STATUS_INFO is worded and
  coloured as in the v83 client (`Net/Handlers/StatusMessages.h`): pickups
  ("You have gained an item in the Etc tab (Snail Shell)", names cut to
  120 px, or 96 px when a count follows), mesos, and kill EXP with a yellow
  line per bonus ("Bonus EXP for PARTY (+N)") show here; quest EXP (mode 3
  with its in-chat flag), mode 4 fame and mode 5 mesos go to the chat log as
  grey lines. So do mode 6 guild points and mode 7 (an item's description).
  Expired cash items, items, seals and skills (modes 2, 8, 11, 13) and
  server lines (9, 12) go to the chat log in the colour v83 uses for chat
  type 0xc. Mode 10, a quest's extra record, shows nothing, and other modes
  are ignored, as in v83.
- **Chat** (`UIChatBar`): the input strip in the bar's top-left and the log
  above it. The status bar cuts that strip out of its background, so the two
  can overlap in either draw order. As in the original, the log has three
  layouts: collapsed, the latest line sits in the strip; collapsed while
  typing, it moves just above the strip; expanded, five lines sit above the
  strip and the input field stays shown. Each is a uniform half-black panel
  with a `Basic.img/VScr5` scroll bar. Its thumb is placed as the original
  places it, which hit-tests only an arrow's height of it. As in the original
  (`FUN_0073ecb5`), dragging the expanded log's top edge resizes it a 13-pixel
  row at a time, from 2 to 39 rows, once the cursor is more than a row from
  where the edge last stopped. The height is saved as `ChatHeight`, and logs
  under four rows tall hide the thumb.
- **NPC dialog** (`UINpcTalk`, `UIWindow.img/UtilDlgEx`): the frame,
  buttons, menu dots, quest list headers, and the `Basic.img/VScr4` scroll
  bar (`Slider::Type::CLASSIC`). See [npc-dialogue.md](npc-dialogue.md).
- **Quest Helper** (`UIQuestHelper`, `UIWindow.img/QuestAlarm`). See
  [quests.md](quests.md).
- **Quest Complete! notice** (`UIQuestComplete`, `UIWindow.img/FadeYesNo`).
  See [quests.md](quests.md).
- **Invitation notices** (`UIInviteCard`, `UIWindow.img/FadeYesNo`): v83's
  fading cards above the status bar for Maple Messenger invitations, buddy
  requests, trade requests, and party invites, each "from '<name>'" beside
  its kind's icon (FUN_0052065d). O accepts and X declines; a card left
  alone fades out after three minutes and sends nothing. A new invitation
  replaces a card of its kind, except buddy requests, which pile up. The
  smaller pink cards (`backgrnd2`, FUN_00523a2c) say "<name> is logged
  in." or "You're on the same ch. with <name>." for six seconds, with only
  an X. Each
  card, quest completions included, sits 5 px above the one before it
  (`FadeNotice::raised`), and only the newest can be clicked. Not done: the
  cash trade card, holding cards back while the UI is locked, and the Game
  Option and blacklist refusals.
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
  frame shows through. Its middle is white at alpha 119 over black, as in
  v83, so it comes out grey and the field doesn't show through.
- **World Map** (`UIWorldMap`, `UIWindow.img/WorldMap`): the frame, title,
  and `Basic.img/BtClose` around the map's 640x470 `BaseImg`, laid out from
  period footage. The Big Bang search panel is gone. Esc steps up to the
  parent map and closes the window from the top.
- **Map tooltips** (`MapTooltip`, `TooltipBox`): the World Map's spots and the
  minimap's NPCs and portals show the name, plus the map's description for
  World Map spots, in the v83 box. `UIClassic.nx` has no bitmaps for it (the
  original drew it), so `TooltipBox` draws the dark edge, light edge, and
  translucent navy fill measured from footage.
- **Item, equip, skill, and text tooltips** (`ItemTooltip`, `EquipTooltip`,
  `SkillTooltip`, `TextTooltip`): the same box, with the layouts measured from
  period footage in place of the Big Bang `UIToolTip.img` frames. Items, equips,
  and skills show a blue dot before the name and the icon at twice its size in
  a light box. Equips have the required stats and the job row from
  `UIWindow.img/ToolTip/Equip` (white and orange when met, red and grey when
  not), then a line and one row per stat ("CATEGORY :OVERALL", "LUK : +11").
  Upgraded equips' names are light blue with the upgrades and gender run
  together, "(+4Male)", as in footage. Skills show the current and next levels
  under a line, "[Next Level 1]".
- **Stat window** (`UIStatsInfo`, `UIWindow.img/Stat`): the main panel and,
  behind DETAIL, the attack and defence panel (closed with `Basic.img/BtHide`),
  laid out from period footage. The job shows as the `Job/main` and `Job/sub`
  bitmaps for explorers and Cygnus Knights. Beginners get the greyed INT and
  LUK labels and no HP or MP buttons. The AP buttons are disabled without AP.
  v83 had no AP auto-assign: the window creates only the six AP-up buttons and
  DETAIL. EXP shows as "exp (pct%)" and updates as it is gained.
- **Skill window** (`UISkillBook`, `UIWindow.img/Skill`): one tab per job
  advancement (`Basic.img/Tab5`, drawn by `ClassicTabs`), the book and name of
  the selected one, four skill rows with a `VScr4` scroll bar, and the points
  left, laid out from period footage. The job name is 10 px Arimo
  (`Text::Font::A10M`). Rows are grey when a required skill is missing. SP-up
  spends a point at once; v83 had no allocation popup. A `recommend/0` frame
  marks the skill `Etc/RecommendSkill.img` names for the SP spent in the tab
  (`recommended_skill`). The MACRO button and the `SkillMacro` window are not
  drawn yet.
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
  Monster Riding. Pets wear nothing yet, so the pet tabs are disabled and the
  pet and MOB EQUIP cells hold nothing.
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
  few seconds. Invitations arrive as an invitation notice, whose O does
  nothing while a room is open.
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
  bad number opens over it, and OK goes back to the field. As in v83, the notice names the box's real
  minimum, which is 10 for dropping mesos. Prompts use v83's words.
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
  RustMS has no mounts or monster book, and the pet and wish list pages are
  not read. Family,
  party, and excluded items are disabled; TRADE invites the character to a
  trade. The fame buttons send
  GIVE_FAME (`GiveFamePacket`) unless the window is the player's own or the
  player is below level 15. As in v83 (FUN_00a23eb5), the client refuses a
  character who isn't on the map ("Unable to find the character.") and a
  second try within 5 minutes ("Please try it again later.",
  `Gameplay/FameCooldown.h`). FAME_RESPONSE (`FameResponseHandler`) prints its
  v83 lines in the skill chat colour and updates the open window's fame.
- **Menu on someone** (`UIContextMenu`, `UIWindow.img/ContextMenu`): opened
  by releasing the right button over another character (`Stage::other_character_at`,
  v83's 44 by 65 box above the feet, or 46 by 31 toward the head when prone,
  `char_click::menu_hits`). It opens at the cursor, pushed back inside the
  view, with their name in the top cap. The rows are PERSONAL INFO, SAVE NAME,
  TRADE REQUEST, WHISPER, PARTY INVITE (party leaders, or players in no
  party), GUILD INVITE (guild masters and Jr. Masters), and ALERT GM; rows left
  out move up (`IO/Components/ContextMenuLayout.h`). It holds the mouse and
  keys: a press or right release outside, ESC, or Enter closes it, and it
  closes before a row acts. Info opens the character info window, SAVE NAME
  copies the name and opens the chat field, WHISPER fills in "/w name ",
  TRADE and PARTY INVITE do what the info window's trade and `/party invite`
  do, GUILD INVITE sends GUILD_OPERATION 5 after v83's guild checks, and
  ALERT GM shows v83's notice that reports are down. Approximations: v83
  also hits the character's second position and a morph's own box; it puts
  the chat into whisper mode rather than filling in a command; and it offers
  Add Junior to family members, which HC doesn't have. The right button
  acts when it comes up, everywhere, as in v83.
- **Quest log** (`UIQuestLog`, `UIWindow.img/Quest`): Available, In Progress,
  and Complete tabs (`Basic.img/Tab2`), quests folded by area, and a detail
  panel with the quest's text and FIND NPC or QUEST HELPER, laid out from
  period footage. See [quest-window.md](quest-window.md).
- **Login scene** (`LoginScene`, `MapLogin.img`): v83 draws the login steps
  as one map, with a stop for each step stacked 600 pixels apart going up:
  title at y 0, world select at -600, character select at -1200, race select
  at -1800, then Cygnus, Explorer, and Aran creation at -2400, -3000, and
  -3600. The camera centre sits at x 28 and 8 pixels above each stop; fitting
  renders of the map to 2009 title and v83 character select footage agrees on
  that offset to the pixel. Its backgrounds and objects come from `Map.nx` `Back/login.img` and
  `Obj/login.img`. `UIStateLogin` loads the scene and draws it, then the book
  frame (`Login.img/Common/frame`) and the version, under the login windows.
  Each classic window calls `LoginScene::move_to` with its step. The first
  call cuts straight to that step. Later calls follow v83
  `CLogin::ChangeStep` (0x564909): the camera flies from the old step's stop
  to the new one in 300 ms per step plus 500 ms, on a Hermite curve that
  leaves at 3.5 times the distance per flight and arrives at rest, so it
  overshoots by about 1% before it settles. The old step's sign fades out in
  place over 300 ms. The step's windows stay hidden until the camera arrives
  (`UIStateLogin` skips drawing, updating, and input for them), since v83
  creates them only then. On arrival, the step's sign (`Login.img/Common/step`:
  1 world, 2 character, 3 race select and creation) slides in from x -149 to
  rest at (0, 33) over 300 ms. From the character select on, the world box
  (`Common/selectWorld`) slides in to (0, 96) over 400 ms with the world and
  channel names on it. It stays up through creation and fades out on the
  way back to the world select or title. Once a sign or the box is in place,
  its `Common/shadow` fades in under it over a second.
- **Title** (`UILogin`, `Login.img/Title`): the ID and password fields and
  the buttons sit on the planks of the scene's signboard. The labels are part
  of the signboard art, and v83 places everything else in code, so the
  positions are measured from the art and checked against 2009 footage. The version reads "Ver. 0.83".
- **World select** (`UIWorldSelect`, `Login.img/WorldSelect`): the worlds
  hang in packet order as planks (`BtWorld/<world id>`) from the scene's rope
  bar. Fifteen slots start at x 232, 27 apart, and blank planks fill the
  unused ones. Picking a world asks the server for its status
  (SERVERSTATUS_REQUEST), and no world or channel takes clicks until the
  answer comes. Status 0 plays `scroll/0` over the scene's closed scroll; 1
  does too, under a notice that the world is busy; 2 says the world is full
  and leaves the list as it was. RustMS answers 2 only when no world has a
  channel. The banner byte after the status is read but not drawn. When the
  scroll opens, it shows the world's name and its channels in four
  columns, each with a load gauge. The gauge treats a load of 1000 as full,
  and RustMS sends each channel's configured capacity (700 by default) as its
  load. The red `chSelect` circle marks the chosen channel. Clicking it again,
  Enter Channel, or Enter goes in. Escape rolls the scroll up with `scroll/1`.
  Back reconnects and returns to the title. View Recommended and View All
  Characters stay disabled, since the server recommends no worlds and can't
  list every character. The balloon is `WorldNotice/Balloon`. Positions are
  checked against footage from 2009 (v80) and v55.
- **Character select** (`UICharSelect`, `Login.img/CharSelect`): up to three
  characters stand on the scene's rope bridge, 125 pixels apart. Each one
  stands under a signpost for its job family (`adventure`, `knight`, or
  `aran`), which the stats scroll covers. The v83 client lays out each
  `nameTag` (0x571ba9, 0x5732f8): its top is level with the feet, its centre
  6 px right of them, and it is the name's width plus 18 px, at least 58 px.
  The name is grey (`#999999`) on tag `0` and white on the picked
  character's tag `1`. Picking a character plays the light from above
  (`effect/1`) and the sparkle (`effect/0`), and the character walks in
  place. It also unrolls `scroll/0` over the character, showing the
  `charInfo` table with
  job, level, fame, stats, and world ranking. `charInfo1` and `scroll/2` add
  the job ranking, but RustMS sends no rankings, so the short scroll is
  always used. The `pageL` and `pageR` arrows appear when the account has
  more than three slots. Tab toggles a focus hand (`keyFocused`) on the
  Start, Create, and Delete signs. Those three buttons are boards that hang
  on the map's `CharSelect/signboard` post, at the v83 offsets (see
  `MENU_POS`). Up and Down move the hand, Left and Right
  move between characters, Enter starts, and Escape goes back. It gives the
  login scene the world and channel names for the world box. Positions are checked
  against footage from v55 and v1.0.
- **Race select** (`UIRaceSelect`, `Login.img/RaceSelect`): Cygnus Knights
  (`knight`), Explorers (`normal`), and Aran stand under the `textGL` banner.
  Explorers start picked. The picked race's `OnAnimation` is drawn in colour
  over its faded button, which keeps the name plate under it. Its `text`
  board sits below, with Select on it. Left and Right change the race, and
  Enter goes on to that race's creation, with no confirmation, as in v80
  footage. Escape and Back return to the character select. `aran1` is the
  same art as `aran`.
- **Character creation** (`UICharCreation`, `Login.img/NewCharKnight`,
  `NewChar`, and `NewCharAran`): one class for all three races, with a
  `Layout` per race for its board and button positions. The new character
  stands in the race's scene. A `charName` board asks for the name first. OK
  checks it against `Etc.img/ForbiddenName` and the server. The `charSet`
  board then has nine rows of `avatarSel` with `BtLeft` and `BtRight`
  arrows. Their choices come from `Etc.img/MakeCharInfo`: `Info/CharMale`
  and `CharFemale` for Explorers, `PremiumCharMale` and `PremiumCharFemale`
  for Cygnus, and `OrientCharMale` and `OrientCharFemale` for Aran. Gender is
  the last row. Values too wide for their row, like the Aran's Average
  Musashi Shoes, drop to the 10 px font. Cancel on `charSet` returns to the
  name, and Cancel on `charName` or Back returns to the race select. Once
  the server adds the character, the scene scrolls back to the character
  select with the new character picked. Positions are checked against v0.73
  footage.
- **Login dialogs** (`UILoginNotice`, `UIQuitConfirm`, `UILoginWait`,
  `UITermsOfService`, `UIGender`): notices are a 362x219 `Notice/backgrnd`
  scroll at (219, 170). Errors use `backgrnd/1`, with the "no" stamp.
  Confirmations and "name OK" use `backgrnd/0`. The message's `text` image has the parchment baked
  in and sits at (119, 21). A lone OK is at (162, 150); OK and Cancel pairs,
  as in the quit confirmation, are at (106, 150) and (195, 150). These are
  checked against v0.50 and v0.55 footage. "Connecting to server"
  (`Notice/Loading`) is at (259, 226). Its `bar` animation fills the LOADING
  slot at (131, 98), and `BtCancel` sits right of the caption at (189, 44),
  as in v0.59 footage. `TOS` and `Gender` keep their layouts, which already
  fit the v83 art. The gender `DropBox` has no normal image, since its box
  is drawn in `text/0`, so `MapleComboBox` has a mode that draws one image
  per state over a box in the parent's background. No footage shows these
  two, since they appear once per account. `UISoftKey` reads the v83
  `Common/SoftKey`, whose `Tab` and `BtNext` the modern file lacks. It only
  opens for a PIC, and RustMS disables PICs, so it is untested.
- **Shared art**: the cursor (`Basic.img/Cursor`), item counts
  (`Basic.img/ItemNo`), and chat balloons (`ChatBalloon.img`) read
  `UIClassic.nx`. States and styles added after v83 fall back to `UI.nx`:
  the `LEAF` and chat bar cursors, and balloon styles above 39.
- **Startup logos** (`UILogo`, `Logo.img`): v83's 137-frame Nexon intro, then
  Wizet. Both are centred through their origins. `SKIP_STARTUP_LOGOS` in
  `Configuration.h` skips them by default.
- **Sliders** (`Slider`): the numbered `Basic.img/VScr` types read
  `UIClassic.nx`, which has up to `VScr8`. Later types, such as
  `DEFAULT_SILVER`, read `UI.nx`.

- **Combo boxes** (`MapleComboBox`): the typed boxes read `Basic.img/ComboBox`
  to `ComboBox5` from `UIClassic.nx`; later types keep the modern art. No
  window uses one yet.
- **Cash Shop stage** (`Stage::load_map(-1)`): v83's `CashShopPreview.img`.
  The full-screen Cash Shop hides the stage, whose preview window draws its
  own art. The v83 stage names no music, so the Cash Shop plays
  `BgmUI.img/ShopBgm` instead of the modern stage's `Bgm00/FloralLife`.

`UI.nx` is now only a fallback for art added after v83. The event list,
joypad, and region select windows came after v83 and had no art in
`UIClassic.nx`. Nothing opened them, so they were removed.

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

## Scroll bars

`Slider` behaves like v83's `CScrollBar`. A press steps once right away: an
arrow by one row, and the track by `max(16, positions / 16)` rows. While the
button stays down, `Slider::update_held` (called from `UI::update`) repeats
that step every 50 ms, as long as the cursor is over the part it went down
on. Only one slider can be held at a time.

## What works and what is stubbed

This is client-side capability only. The RustMS server currently implements
login, character management, movement, chat, whispers, portals, channel
changes, NPC dialogue, and a few quests; everything else below has no server
side yet.

| Area | Status |
| --- | --- |
| Login, world/channel select, character list/create/delete, PIC | Implemented |
| In-map play: movement, jumping, portals, map change, channel change | Implemented |
| Combat: melee/ranged/magic attacks, mob HP/death, mob attacks and skills, drops, pickup | Implemented, with diseases; see `combat.md` |
| Inventory, equip, item use, scroll, sort/gather | Implemented |
| Skills, stat/AP/SP allocation, buffs, keymap | Implemented |
| NPC dialogue and NPC shop | Implemented (`UINpcTalk`, `UIShop`) |
| Quest log and markers | Implemented (`UIQuestLog`); progress arrives via SHOW_STATUS_INFO, quest start/complete is driven by NPC dialogue; UPDATE_QUEST_INFO refusals 9–11, 13 and 14 show v83's notices (10 names the full inventories from `Act.img`), 15 the expiry chat line, and the timer modes are ignored |
| Chat, whisper, `/find`, `/?`, `/party`, `/buddy`, `/all`, server messages | Implemented; buddy whisper/find modes ignored; `/w NAME` alone fills in a whisper to NAME, or says "Incorrect ID." for a name no character could have, as v83's `/whisper NAME` does; other commands are dropped, as v83 does for non-GMs |
| Character info, minimap, world map | Implemented |
| Party (`UIUserList` Party tab, `UIPartyHP`, party chat) | Implemented; see `parties.md` |
| Buddy list (`UIUserList` Friend tab, buddy chat) | Implemented without groups; see `buddies.md` |
| Guild (`UIUserList` Guild tab, name tags, guild chat) | Founding, invitations, leaving and expelling; see `guilds.md` |
| Alliance / blacklist tabs (`UIUserList`) | Tab strip only |
| Monster Book (`UIMonsterBook`) | Implemented: cards, summary, monster pages, search, cover; see `combat.md` |
| Cash shop (`UICashShop`) | Enter/exit, catalog, buying, gifts, Cash Inventory, wish list, and moving items to and from the bags |
| Maple Messenger (`UIMessenger`) | Implemented |
| Trade (`UITradingRoom`, `UIInviteCard`) | Implemented, without cash trades |
| Storage (`UIStorage`) | Implemented; see `inventory.md` |
| Continent ships, station wall clocks | Docking ships and the Balrog's ship, CONTI_MOVE/CONTI_STATE, CLOCK kind 1; see `ships.md` |
| Player shops, mini-games, memo/mail | No UI, no handlers |
| Reactors | Spawning, swings, hits and breaking; no skill hits, touch or quest gates; see `combat.md` |
| Pets | Summoning, following, others' pets, map changes, chat commands, idle actions, level-up effect, food, cash food and hunger; see `pets.md` |
| Mounts, summons | Partial or absent |
| Party/guild/boss quests, events, MTS | Absent |

81 outbound opcodes are defined (`Net/OutPacket.h`) and 116 inbound handlers
are registered (`Net/PacketSwitch.cpp`). Anything not listed there is silently ignored by the client.

## Movement physics

The player follows v83's CVecCtrl rules (MapleLegends client, decompiled).
`Player` and mobs set `phobj.shoe`, and `Physics::move_shoe` handles it.
Everything else, including NPCs, pets and drops, keeps HeavenClient's older
force-and-friction model (`move_normal`).

- **Units.** The constants in `Physics.cpp` come from v83 `Map.wz/Physics.img`
  and are in pixels and seconds. One 8 ms tick is 1/125 s. `hspeed` and
  `vspeed` are still stored per tick.
- **Steering.** Player states set `phobj.walkdir` (-1, 0 or 1) each tick
  instead of pushing with `hforce`. `phobj.walkspeed` is the shoe's speed
  multiplier, and `walkforce` and `walkdrag` scale its ground acceleration
  and drag. The drag factor is clamped to 0.05–2 and halved below 1.
- **Mobs.** A walking mob's shoe (FUN_005c55cd) has walkspeed
  clamp(`info/speed` + 100, 10, 140)/100, or 0 without a `move` animation,
  and `info/fs` (default 1) as both walkforce and walkdrag. Chasing, a mob
  with a `chase` animation walks at `info/chaseSpeed` instead. A snail (speed
  -65, fs 10) reaches 44 px/s in one tick and stops within four. A jump
  takes off at 555 px/s and keeps the walking speed. While a knocked-back mob
  slides (`NOFRICTION`), `Mob` slows it itself. A mob that can also jump
  takes off like the player, holding its walking direction. It keeps within
  its foothold groups (`JUMPER`); see `agent_docs/combat.md`.
- **Flying.** A flier's shoe sets `phobj.flyspeed` to `info/flySpeed` plus
  100, clamped to 10-140 percent, and the flier is never on the ground. It passes through
  platforms and walls, and `FootholdTree::limit_flight` keeps it within v83's
  physical space: the platforms' extent, 30 px in from the sides, 300 px
  above and 10 px below. It speeds up at 1200 px/s² toward 200 px/s ×
  flyspeed, and slows at 1000 px/s² when not steering or when faster than
  that. `phobj.flydir` -1 glides down at 0.3 of its top speed, and 0 dives at
  1200 px/s². `Physics::fly_jump` beats up at its full top speed. Fliers are
  drawn on the top layer.
- **Walking.** On flat ground the player accelerates at 1400 px/s² toward
  125 px/s × walkspeed and brakes at 800 px/s². On a slope the push scales by
  1 ± slant², and a slant over 0.9 slides. Positions advance by the average of
  the old and new speeds.
- **Air.** Gravity is 2000 px/s², capped at a fall speed of 670 px/s. A held
  direction accelerates at 200 px/s² toward about 8.9 px/s, so it barely
  steers.
- **Stats.** `get_walkspeed` is clamp(SPEED, 80, 140)/100. `get_jumpforce`
  uses clamp(JUMP, 80, 123)/100 of the 555 px/s takeoff speed.
- **Jumping.** `Physics::jump` sets the takeoff speed. A held direction adds
  80% of the top walking speed, capped at the top speed. A rope jump uses half
  the jump speed and 1.3 times the walking speed.
- **Climbing.** Ladders and ropes climb at 100 px/s whatever the SPEED, and Up
  and Down together hold still. Past the top, the player stops there, or
  steps 5 px above it onto a platform if the ladder has `uf`. Past the bottom,
  they let go 1 px below it.
- **Grabbing.** `Stage::catch_ladders` runs after every move while an arrow
  is held, unless the player is climbing or attacking. On a foothold, Up
  grabs a ladder that reaches within 20 px above the feet, and the feet rise
  to its bottom. Down grabs one whose top is 0 to 10 px below the feet, and
  the feet move to its top. In the air, Up grabs only while falling: it
  grabs any ladder within 10 px of the path since the last move, at the
  current height. `MapInfo::findladder` checks ladders in number order and
  returns the first one that matches.
- **Down-jump.** Allowed when the lowest platform within 300 px below, or
  else the highest one 300 to 600 px below, is at least 5 px under the feet
  (`FootholdTree::can_jump_down`). `Physics::jump_down` hops up at 196 px/s
  and stores the platform in `phobj.fallstart`. Until the player lands
  elsewhere or grabs a rope, `update_fh` skips that platform.
- **Knockback.** A hit that does damage, and that the stance roll doesn't
  block, knocks the player away from the mob and up at 270 px/s, or 200 px/s
  under Power Guard. It also knocks the player off ladders and ropes.
  `Physics::knockback` queues the push for the next move. Each axis adds to
  the speed without going past the push, so a fast fall barely slows. The
  next move then lifts the player off the ground and truncates both speeds to
  whole pixels per second. A knock from standing rises about 18 px and lands
  about 73 px away.
- **Walls.** A wall linked to the floor's end, or the foothold after it,
  blocks if it reaches between 1 and 50 px above the feet
  (`FootholdTree::get_wall`). Maps draw walls both top to bottom and bottom
  to top, so `Foothold::is_blocking` sorts the ends first.
- **Map bounds.** `FootholdTree` keeps objects 30 px inside the outermost
  footholds and at most 300 px above the highest. A map with `info/VRLimit`
  pulls those in to its `VRLeft + 20`, `VRRight - 20` and `VRTop + 65`,
  skipping missing or zero edges as v83 does. A flying mob's lower bound
  is also limited by a nonzero `VRBottom`.
  v83 never stops a fall; HeavenClient stops it 100 px below the lowest
  foothold.
- **View range.** `MapInfo` reads each `VR*` edge on its own; zero is a real
  camera edge. A missing edge
  sits 20 px outside the bounds at the sides, 60 px above the top, and 110 px
  below the lowest foothold. The camera centres a range smaller than the
  screen.
- **Camera follow.** The view aims 50 px above the player's feet. On each
  axis it stays still while within 28 px of that point. Further off, it
  closes the gap d at d² / 100 px/s, so a walking player leads the view by
  about 110 px (v83 sets this up in `FUN_0042935a` as a Shape2D vector with
  looseness 100). The followed point isn't held inside the view range; only
  the view is.
- **Not ported yet:**
  - Mob attacks' own knockback speeds (900 or 1900 px/s, flat).
  - Swimming knockback: impacts remain queued until normal movement resumes;
    tracked in [#341](https://github.com/neeerp/RustMS/issues/341).

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
  0x57 (a bought item), 0x5C (a failure), 0x5E (a gift sent), 0x60 (a bag's
  new slot count), 0x62 (the storage's), 0x68 (an item taken out), 0x6A (an
  item put back) and 0xA4 (a bought package) are read. Inventory slots keep the item's cash serial,
  which the item encoding carries, so a cash item can find its way back.
- **Buying:** BUY opens `UICashBuy` (`CashShop.img/CSNotice/0`): the item, its
  price and period, and a checkbox per balance. It checks the first balance
  that covers the price, trying NX Prepaid, NX Credit, then Maple Points. OK
  sends CASHSHOP_OPERATION (229) 0x03 with the cash type (1 Credit, 2 Maple
  Points, 4 Prepaid) and SN, or 0x1E for a package (items 910xxxx) and
  0x06/0x07 mode 1 for an expansion commodity (911x000). As in v83, each
  mode is followed by a byte that is 1 for Maple Points, and 0x03 ends with
  a zero (no free trial). The result is a "successful purchase" or a failure
  notice. Failure notices use v83's text for each code (`FUN_0047c17a`),
  except coupon, gift-token and character-transfer codes, which get the
  default.
- **Check:** CHECK CASH sends CHECK_CASH (228), as does the game window
  getting the focus back while the shop is open; QUERY_CASH_RESULT answers.
- **Slot buttons:** Equip Slot through Stor. Slot open `UICashBuy` for the
  matching expansion item at 4,000 NX, and OK sends 0x06 mode 0 with the bag,
  or 0x07 mode 0. On 0x60 the bag's slot count changes at once. For 0x60 and
  0x62 the notice says "You have added slots to your item inventory.", as in
  v83.
- **Item data:** packages and expansions take their name and icon from
  `Item.nx` `Special`, pets from `Item.nx` `Pet`, and pet equipment from
  `Character.nx` `PetEquip`.
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
  and message. On 0x5E the sender sees v83's "COUNT [ ITEM ] was sent to
  NAME. PRICE NX Prepaid were spent in the process.", or a failure notice (0xA8 own account, 0xA9 unknown name, 0xC4 birthday code).
  The receiver gets one notice listing each gift's sender, item and message
  when 0x4D arrives. The buddy and guild buttons that open the wider
  `backgrnd1` aren't drawn, and only one name is sent.
- **Not served yet:** CHARGE and COUPON do nothing.

## Estimated client work for a complete game

| System | Client status | Effort |
| --- | --- | --- |
| Buddy groups | Default group only | S |
| Guild | Window, founding, invitations, leaving and expelling; ranks, emblem and BBS UIs missing | M |
| Player shops | Nothing | L |
| Cash shop | No coupons, charging, or storage window | S |
| Quest accept/complete UI flow | Partial | S |
| Pets | Summon, follow, chat commands, food and cash food only: no name tags, equipment or looting | M |
| Mounts | Partial rendering, no interaction | M |
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
   own grey (`#BBBBBB`) chat line, worded as the 2025 MapleRoyals client does:
   "You have gained an item in the Use tab (Roger's Apple)", "You have gained
   items in the Use tab (Green Apple 3)", "You have lost an item (X)" and "You
   have lost items (X 3)". A count of 0 instead carries a message and an
   `Effect/Quest.img` effect number; the message is a `#64B4F0` status line.
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
   CONFIRM_SHOP_TRANSACTION handler shows v83's notice for each code: codes
   0, 4 and 8 need none, and a code v83 doesn't know is "Due to an error, the
   trade did not happen.". TRUNK_RESULT had swapped `0x0b`, which is out of
   mesos to take an item out, and `0x0c`, which is a one-of-a-kind item the
   player already has.
10. v83 keeps one binding each for Ctrl, Alt, and Shift: the game reads a
    key's scan code without the extended bit, so Right Ctrl and Right Alt
    arrive as the left keys, and it folds Right Shift (54) onto Left Shift
    (42). Key Config's right-hand faces store under the left keys
    (`KeyConfig::fold`). The client had given the right-hand keys their own
    keys at 85 to 87, so Right Ctrl did nothing, and had put F11 and F12 at 69
    and 70 rather than v83's 87 and 88. The `v83_modifier_and_f_keys`
    migration drops the old right-hand copies and moves F11 and F12.
11. SET_FIELD's first byte after the channel is the new field's key, and the
    second says whether character data follows. The client took the
    change-map branch only when both were 0. It now branches on the second
    byte and keeps the key (`Stage::get_field_key`). As in v83, every
    CHANGE_MAP and CHANGE_MAP_SPECIAL starts with that key. Before, they
    started with 0, or with 1 for "died" on the death respawn.
12. DAMAGE_REACTOR (`0xCD`) wrote the player's position, stance and four
    zero bytes after the object id. v83 writes the kind of hit (0 a swing,
    1 a skill), the swing's flags (bit 0 facing left, bit 1 on a foothold)
    and the delay before the hit lands. HIT_REACTOR (`0x115`) was read as
    a stance and frame delay; it carries the delay, the event of the old
    state, and a cooldown. An attack also sent `0xCD` for the nearest reactor
    it reached, as well as hitting mobs; it now swings at a reactor instead
    of the mobs. See `combat.md`.
13. Pets: SPAWN_PLAYER read a byte before each pet that v83 doesn't send, and
    dropped the pets it read. The stat change read the three pet bits as one
    short after MESO; v83 sends 8 bytes for each bit set, right after HAIR.
    Items were taken for pets by their id range rather than item type 3. A
    pet's stance byte is `action << 1 | left`, but HeavenClient read the
    action as its own animation index and odd as facing right. 168 put pets
    at a fixed index; v83's list closes up. See `pets.md`.
