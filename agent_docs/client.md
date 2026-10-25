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
  Esc opens the Game Menu; the MENU key action opens ShortCut, whose buttons
  run in v83's order: Item, Equip, Stat, Skill, Community, Quest, Monster
  Book, Messenger. Dropping a
  skill, a USE or SETUP item, or a skill macro on a quick slot binds that
  slot's key and saves the keymap right away; an open Key Config shows the
  new binding. Which key each slot stands for comes from QUICKSLOT_INIT (159)
  after SET_FIELD, which the client echoes back as QUICK_SLOT. Hovering a quick slot shows its skill with levels, its item
  as Item.nx has it, or its macro, with the box 128 px above the cursor;
  cash and other slots leave the last tooltip up, as v83 does. The mouse
  wheel goes only to the window under the cursor (or a modal window), never
  to windows elsewhere on screen. Hovering the job and name shows
  "login ID / channel / name", and hovering the EXP numbers shows the EXP,
  both in the one-line box 20 px right of and below the cursor.
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
- **Chat target** (`UIChatBar`, `IO/Components/ChatTarget.h`,
  `Basic.img/ComboBox2`): the box at the strip's left end opens v83's list
  of eight targets upward, and Tab in the input field steps through them in
  v83's order (FUN_008d536c). Picking Whisper asks for a name, and every
  typed line then goes to that name until another target is picked; the
  name prompt, `/w NAME`, and incoming whispers feed the history `/w` alone
  uses. To Alliance goes to the alliance's members online (`guilds.md`);
  To Spouse goes to the partner (`marriage.md`, "Spouse chat"). Picking To Group opens v83's group dialog
  (`UIGroupPicker`, FUN_008d500b): a combo box of the Default Group,
  which it starts on, and the buddies' other groups, which can also be
  typed in. OK sends lines to that group's online buddies. It does so
  even with the box cleared, and then finds no one. The name prompt has no history list.
- **NPC dialog** (`UINpcTalk`, `UIWindow.img/UtilDlgEx`): the frame,
  buttons, menu dots, quest list headers, and the `Basic.img/VScr4` scroll
  bar (`Slider::Type::CLASSIC`). See [npc-dialogue.md](npc-dialogue.md).
- **Choice of styles** (`UIAvatarSelect`, `UIWindow.img/UtilDlgEx_Avatar`,
  NPC dialogue types 7 and 8). Positions are v83's (FUN_009a92c9,
  FUN_009aa708). See [npc-dialogue.md](npc-dialogue.md).
- **Choice of pets** (`UIPetChooser`, `UIWindow.img/UtilDlgEx_Pet`): which
  pet a piece of pet equipment goes on, and NPC dialogue types 9 and 10.
  Like the NPC dialog, it opens silently. See [npc-dialogue.md](npc-dialogue.md).
- **Quiz and speed quiz** (`UIQuiz`, `UISpeedQuiz`,
  `UIWindow.img/InitialQuiz` and `SpeedQuiz`, NPC dialogue types 5 and 6).
  Positions are v83's (FUN_007e9d54, FUN_007eb7d5). See
  [npc-dialogue.md](npc-dialogue.md).
- **Slide menu** (`UISlideMenu`, `UIWindow.img/SlideMenu`, NPC dialogue
  type 14). Positions are v83's (FUN_0076be7a, FUN_0076c389). See
  [npc-dialogue.md](npc-dialogue.md).
- **Quest Helper** (`UIQuestHelper`, `UIWindow.img/QuestAlarm`). See
  [quests.md](quests.md).
- **Quest Complete! notice** (`UIQuestComplete`, `UIWindow.img/FadeYesNo`),
  for a quest whose objectives the client sees met or one the server names
  in QUEST_CLEAR (0x31). See [quests.md](quests.md).
- **Script lines** (`UIScriptMessage`, SCRIPT_PROGRESS_MESSAGE, 0x7A): up to
  five gold lines at y = 150, 130, ... 70, centred, each shown for 3 s and
  faded over 3 more; with all five busy a line is dropped (FUN_00a13f20,
  FUN_006ce251).
- **Quest alert** (`QuestAlert`, `Effect/BasicEff.img/QuestAlert`): the bulb
  over the player's face for autoStart and autoComplete quests. See
  [quests.md](quests.md).
- **Invitation notices** (`UIInviteCard`, `UIWindow.img/FadeYesNo`): v83's
  fading cards above the status bar for Maple Messenger invitations, buddy
  requests, trade requests, and party invites, each "from '<name>'" beside
  its kind's icon (FUN_0052065d). O accepts and X declines; a card left
  alone fades out after three minutes and sends nothing. A new invitation
  replaces a card of its kind, except buddy requests, which pile up. The
  smaller pink cards (`backgrnd2`, FUN_00523a2c) say "<name> is logged
  in." or "You're on the same ch. with <name>." for six seconds, with only
  an X. On a level-up, each party quest (1200-1399) whose `lvmin` is the
  new level, for the player's job, gets a 51 px tall `backgrnd5` card:
  its `PQuest.img` mark, "Party Quest", and its name cut to 80 px with
  ".." (FUN_00a1fb52, FUN_0052065d type 0xb, reports a074 and a075). These
  pile up too, and O opens the Quest Log's Available tab on the quest
  (`UIQuestLog::show_quest`); neither sends a packet. Each
  card, quest completions included, sits 5 px above the one before it
  (`FadeNotice::raised`), and only the newest can be clicked. The Game
  Option and the Blacklist turn invitations away before a card shows. Not
  done: the cash trade card, and holding cards back while the UI is locked.
- **Minimap** (`UIMiniMap`, `UIWindow.img/MiniMap`): folded to its title bar,
  the map alone (`MinMap`), or the map under the map's mark, street, and name
  (`MaxMap`), laid out from period footage. M or the `Basic.img` minus and
  plus buttons step between them, and other maps keep the last choice
  (`MiniMapType`, the largest until one is made). Towns open in the largest,
  and maps without a `miniMap` stay folded, as do, for all but GMs, the
  `hideMinimap` maps (290 have a `miniMap`, mostly party quests and events)
  and, once the server sends MINIMAP_ON_OFF (86) with 0, the `miniMapOnOff`
  maps. Nothing sends 86 yet. The window fits the map's `miniMap/canvas` (scaled
  by `2^mag`) and the names. Markers (`Map.nx` `MapHelper.img/minimap`) stand
  bottom-centred on the visible portals (types 2 and 7), NPCs, other
  players, and you. Hovering an NPC shows its name. Hovering a portal's
  marker shows, as v83 does, its `ToolTipHelp.img/PortalTooltip` line or
  else where it leads, in a plain navy box 20 right of and below the
  cursor. Period canvases had the map's
  background baked in; v83 `Map.nx` canvases are transparent there, so the
  frame shows through. Its middle is white at alpha 119 over black, as in
  v83, so it comes out grey and the field doesn't show through.
- **World Map** (`UIWorldMap`, `UIWindow.img/WorldMap`): the frame, title,
  and `Basic.img/BtClose` around the map's 640x470 `BaseImg`, laid out from
  period footage. The Big Bang search panel is gone. Esc steps up to the
  parent map and closes the window from the top. It opens on the world map
  v83 picks for the field (`WorldMapLookup.h`): the first of `WorldMap000`,
  `010` to `100` listing it, unless one of the ten after it or `WorldMap140`
  to `142` does too. On a field none of them lists, such as the Free Market,
  the world map key and the minimap's button say the world map is not
  available there instead. FIND NPC picks its world map differently (see
  `quest-window.md`).
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
  under a line, "[Next Level 1]". Skills with a master level (4th job, and
  Evan's 9th and 10th stages) get a "[Master Level : N]" line of their own
  above the description, with the character's master level (0 until one is
  taught), and show a next level only below it. Other skills' descriptions
  already carry the tag and are drawn as they are, flaws included
  (Improved MaxHP Increase's starts "Master Level : 10]").
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
  (`recommended_skill`). Its MACRO button (`Basic.img/BtMacro`) shows or hides
  the skill macro window.
- **Skill macro window** (`UISkillMacro`, `UIWindow.img/SkillMacro`): docked
  at the skill window's right, which closes it and forgets its selection. It
  lists the five macros three at a time with a scroll bar. Skills are dragged
  into a macro's slots from the skill window or from another slot, and taken
  out by dropping them on the field. v83 refuses passive, charged and movement
  skills and ones with a cooldown (`skill_macro::may_hold`). Every change
  sends the whole list (`SkillMacroPacket`). Clicking a row selects it, and
  any other click in the window selects the first macro, as v83 does. The
  selected macro's name (12 bytes) and shout box are saved with OK, which
  stars out curse words in the name. A macro's icon is dragged onto a key in Key Config or
  a quick slot, which then shows it (`Macroicon/<n>/icon`). Hovering the icon,
  or such a key in Key Config, shows the macro's tooltip (`MacroTooltip`). The
  buttons have no tooltips.
- **Item window** (`UIItemInventory`, `UIWindow.img/Item`): five tabs
  (`Item/Tab`, drawn by `ClassicTabs`), six rows of four slots with a `VScr4`
  scroll bar, and the mesos, laid out from period footage. BtFull widens it to
  `FullBackgrnd`, four pages of 24 with no scroll bar; BtSmall goes back.
  Slots past the tab's limit show `disabled`. A gained item pulses its tab
  (`New/Tab0`, or `Tab1` when selected) and its slot (`New/inventory`) until
  the window closes. BtGather and BtSort rearrange the bag, and BtCoin asks
  how many mesos to drop. The full window's BtCashshop enters the Cash Shop,
  as the status bar's button does. It is greyed on the Set-up and Cash tabs
  and on a tab of 96 slots. v83 also has the shop show the tab's slot
  expansion, which our Cash Shop can't take yet.
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
  It shows the keys as they are each time it opens, as v83 builds it anew.
  An effect item (cash 501xxxx such as Sunny Day, or etc 429xxxx) dragged
  from the inventory binds key type 7. It is drawn without a count, here and
  on the quick slots, and the key turns the item's effect on or off while the
  player has the item, as v83 does (0x94f2ed, 0xa24530).
  Esc, Tab, and the other keys printed on the background are fixed. DEFAULT
  restores the server's defaults; DEFAULT and CLEAR ALL ask through the
  in-window `notice` bitmaps. Cancel, X, Esc, the hotkey, and the status bar's
  KeySet button all close it as v83's Cancel does, asking "Save changes?"
  first when keys or quick slots are unsaved. A skill, item or macro dropped
  on a status bar quick slot while it is open is one of its edits, sent with
  OK and taken back on Cancel, and the status bar shows it meanwhile.
  CHANGE QUICK SLOTS opens the quick slot editor (below). Opening Key Config
  and its OK each send QUICK_SLOT (183, eight i32s), and so does a saved close
  if the quick slots changed. DEFAULT also resets the quick slots, and closing
  without saving takes back any change made in the editor.
- **Quick slot editor** (`UIQuickSlotConfig`, `KeyConfig/quickslotConfig`):
  eight cells laid out as the status bar's quick slots, each with its key's
  label over what Key Config has on that key. Click a cell, or Tab to the
  first one, then press a key. A key without a label is refused ("The key you
  just pressed is a key that has been disabled."), and so is one a cell
  already has, the selected cell's own key too ("This key is already being
  used."). Right Shift counts as Shift. Left and Right move inside a row and
  stop at its ends, Up and Down switch rows, and on OK and Cancel Left and
  Right step between them. Tab goes on to OK, Esc lets go of the cells and
  then cancels, and
  Enter is OK. The editor reads every key itself while it is up. OK shows
  the new keys on the status bar at once; Key Config sends them.
  `IO/QuickSlots.h` holds the rules and the keys, which last as long as the
  client runs.
- **Maple Messenger** (`UIMessenger`, `UIWindow.img/Messenger`): three seats
  with the members standing in them, the chat log, and the input line, laid
  out from period footage. The MAPLECHAT key or ShortCut's MESSENGER button
  opens a room, or minimises the open one to its title bar (which blinks on
  unread lines). Clicking an empty seat or typing `/invite name` invites; `/q`
  or the close button leaves. A speaker shows the `chatBalloon` dots for a
  few seconds. Invitations arrive as an invitation notice, whose O does
  nothing while a room is open.
  - The log says "- 'name' has entered." for our own join and for an
    arrival the server announces, and "- 'name' have left the room." when
    someone leaves. A negative seat from the server means we couldn't join:
    we sit in seat 0 and are told so.
  - As in v83 (report b037), every mode but the invitation is dropped while
    no room is open, and a new look for an empty seat is dropped. Mode 8
    resends all three seats silently (0 empty, 1 unchanged, else a member);
    the server doesn't send it.
- **Game Option** (`UIGameOption`, `UIWindow.img/GameOpt`), from MENU: a
  `Basic.img/CheckBox` right of each row's ALLOW, laid out from the
  background. OK saves the boxes as the `Allow*` settings, which the
  handlers for whispers, group chat and invitations check.
- **System Option** (`UISystemOption`, `UIWindow.img/SysOpt`), from MENU:
  `Basic.img/Slider` sliders (`ClassicSlider`) between each row's end labels,
  mute, screen shake, and WINDOW / FULL SCREEN boxes, laid out from the
  background. The BGM and SOUND sliders change the volume as they move, in
  steps of 10, and MUTE greys the slider out; Cancel and Esc put the volume
  back. OK saves `BGMVolume`, `SFXVolume`, `BGMMute`, `SFXMute`,
  `ShakeScreen`, and the stored-only `PictureQuality`, `MouseSpeed`,
  `HPAlert` and `MPAlert`, and switches to the chosen viewing mode. MONSTER INFO has
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
  size shows `BtRecharge` on its row, which asks "Are you sure you want to
  recharge it?" before it recharges (v83 `FUN_00756c28`). The
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
  not read. Excluded items are disabled. Family opens the player's own Family
  window or someone else's Family Tree (`agent_docs/families.md`). REQ PARTY, greyed in the
  player's own window, invites the character when the player leads a party,
  and otherwise does nothing (v83 FUN_008ff5b0). TRADE, also greyed in the
  player's own window, invites the character to a trade. The fame buttons send
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
  party), GUILD INVITE (guild masters and Jr. Masters), ALERT GM, and FAMILY (once
  the family is known and has room for a junior); rows left
  out move up (`IO/Components/ContextMenuLayout.h`). It holds the mouse and
  keys: a press or right release outside, ESC, or Enter closes it, and it
  closes before a row acts. Info opens the character info window, SAVE NAME
  copies the name and opens the chat field, WHISPER puts the chat into
  whisper mode for them, TRADE and PARTY INVITE do what the info window's
  trade and `/party invite` do, GUILD INVITE sends GUILD_OPERATION 5 after
  v83's guild checks, and ALERT GM shows v83's notice that reports are down, and FAMILY sends
  ADD_FAMILY.
  Approximations: v83 also hits the character's second position and a
  morph's own box. The right button acts when it comes up, everywhere, as in
  v83.
- **Family** (`UIFamily`, `UIFamilyTree`, `UIWindow.img/Family` and
  `FamilyTree`): the Family window from the Family key, and the Family Tree
  from it or from someone's info. See `agent_docs/families.md`.
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
  checks it as v83 does (4 to 12 letters or digits, fewer than four `I` and
  `l`, and, except for GMs, no word from `Etc.img/ForbiddenName` or
  `Curse` anywhere in it), then asks the server. Enter on an empty name does
  nothing. The server's answer 1 shows "currently being used", 2 "cannot use
  this name", and anything else "unknown reason". The `charSet`
  board then has nine rows of `avatarSel` with `BtLeft` and `BtRight`
  arrows. Their choices come from `Etc.img/MakeCharInfo`: `Info/CharMale`
  and `CharFemale` for Explorers, `PremiumCharMale` and `PremiumCharFemale`
  for Cygnus, and `OrientCharMale` and `OrientCharFemale` for Aran. Gender is
  the last row. Values too wide for their row, like the Aran's Average
  Musashi Shoes, drop to the 10 px font. Cancel on `charSet` returns to the
  name, and Cancel on `charName` or Back returns to the race select. Once
  the server adds the character, the scene scrolls back to the character
  select with the new character picked. If it refuses (a non-zero result in
  `NewCharacter`), a notice says why and the window stays: 0x1E "cannot use
  this name", 0x0A "too many connection requests", anything else "unknown
  reason" (v83 has its own text for 0x1A, a pending transfer, which the
  server never sends). The character select's New button stays enabled and
  answers "No more characters can be created" when every slot is taken.
  Positions are checked against v0.73 footage.
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
  opens for a PIC, and RustMS disables PICs, so it is untested. A blocked
  account's notice (`Util/BlockReason.h`) follows v83's 0x5f83ee: a reason
  byte and the block's end (a FILETIME) after the result, `Etc/BlockReason.img`'s
  `msg` for the reason (21 to 40 use 12's), then "You can login after
  Oct/25/2026  3:07 pm", or the permanent-block text when the end is three
  or more years away. Reason 99 has its own text. The text shows in a
  `UIOk`; reason 0, a reason without text, or a packet without those bytes
  (RustMS doesn't send them yet) get the `BLOCKED_ID` notice.
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
- **Edits** (`Textfield`): a one-line, left-aligned field shows what fits
  of its text and scrolls sideways to keep the caret shown, as v83's edit
  does (0x4cbc03): past the right edge the caret goes to the edge, 48 px
  further in when text follows it; past the left edge it goes 48 px in.
  Deleting never scrolls back, so a shortened text can leave blank space
  on the right. The text and the selection are clipped to the field's
  bounds, and the scroll keeps the caret inside them; v83 clips only at
  the control's outer edge, a few px wider. Fields with no byte limit (the chat bar) still
  stop 50 px short of the edge, so they never scroll.
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
| Chat, whisper, `/find`, `/?`, `/party`, `/buddy`, `/all`, server messages, megaphones, Maple TV's send dialog | Implemented; the megaphone dialogs are in `inventory.md`; buddy whisper/find modes ignored; `/w NAME` alone whispers every line to NAME, or says "Incorrect ID." for a name no character could have, as v83's `/whisper NAME` does; other commands are dropped, as v83 does for non-GMs |
| Character info, minimap, world map | Implemented |
| UI and game sounds | v83's: GameIn as the server lets a character in, not on map loads; RollDown/RollUp for the channel panel; ScrollUp on every change of login step (`LoginScene::move_to`), not the first; SelectMap alone on world map links and right-click; BuyShopItem on Cash Shop purchase notices; other players' level-up, job, quest and card sounds; DropItem and Tombstone quieter with distance from the player (100 within 250 px, then `120 - 0.08d` down to 40 past 1000 px, `sound_distance::volume`, FUN_009894f3), and anyone's Tombstone heard. QuestAlert and a few rarer sounds are missing |
| User effects (`0xCE` own, `0xC6` others) | The modes both packets share are in `user_effect::show_shared` (FUN_009377d9): level up (0), charm lines (6), recovery numbers (10), ItemEff animations (11), path images and animations (12, 18, 22 only in field type 0x13, 23, 24), ItemLevelUp (15), the Item Maker (16), IncEXP (17, heard only by its owner), Wheel of Destiny and Spirit Stone lines (21, 26) and field sounds (25); 19 and modes past 26 do nothing, and a foreign mode 3 (only its owner gets one) is ignored. Mode 12's delay is not known, so it shows at once |
| Party (`UIUserList` Party tab, `UIPartyHP`, party chat, Party Search) | Implemented; see `parties.md` |
| Buddy list (`UIUserList` Friend tab, groups, blocking, buddy chat) | Implemented; see `buddies.md` |
| Guild (`UIUserList` Guild tab, name tags, guild chat) | Founding, invitations, leaving and expelling, rank changes, rank titles (the GUILD INFO panel), the notice, members' logins, levels and jobs, growing and disbanding at Heracle, Lea's emblem maker (`UIGuildMarkMaker`, opened by 65/0x11, saved with mode 0x0F, applied by 0x42); see `guilds.md` |
| Alliance (`Character/Alliance`, invitation and login cards, alliance chat) | State, cards, chat and the Alliance tab with its rank panel implemented; see `guilds.md` |
| Blacklist tab (`UIUserList`) | Implemented: Add and Delete, kept in the `BlackList` setting and sent with a shop's opening; whispers, chat and invitations from its names are turned away. See "Personal shops" in `inventory.md` |
| Monster Book (`UIMonsterBook`) | Implemented: cards, summary, monster pages, search, cover; see `combat.md` |
| Cash shop (`UICashShop`) | Enter/exit, catalog, buying, gifts, Cash Inventory, wish list, and moving items to and from the bags |
| Maple Messenger (`UIMessenger`) | Implemented |
| Trade (`UITradingRoom`, `UIInviteCard`) | Implemented, without cash trades |
| Chalkboards (`ChalkboardBalloon`) | Implemented: the board, the note prompt and the close button; see `inventory.md` |
| Cash mists (528xxxx) | Implemented: the use gate (`MapMists::item_busy`), the cloud (`MapMists`, type 3), the puff on the user (`Char::show_mist_puff`, user effect mode 20) and the face (ITEM_EMOTION, `Stage::use_item_emotion`); see `inventory.md` |
| Kites (508xxxx) | Implemented: the use gates, the three-line entry window, the kite over the field with its bob, fade, hover line and double-click viewer (`MapKites`, `UIKiteWrite`, `UIKiteView`); see "Kites" in `fields.md` |
| Maplemas Party Potion (2212xxx) | Implemented: the name dialog (`UITransform`, `UIWindow.img/Transform`), USE_TARGET_ITEM and RANDOM_MORPH_RES's chat lines; see "Morphs" in `combat.md` |
| Storage (`UIStorage`) | Implemented; see `inventory.md` |
| Rock, Paper, Scissors (`UIRpsGame`, `Character/RpsGame`) | Implemented: the admin's notice, the window, rounds, results and certificates; see `inventory.md` |
| Duey (`UIDuey`) | The tab strip, receiving, normal and quick sends, the arrival notice, and the Quick Delivery Coupon's window; see `inventory.md` |
| Owl of Minerva (`UIItemSearch`, `UIItemSearchResult`) | The search window with its most-searched and category panels, the search, the results with GO; see `inventory.md` |
| Teleport rocks (`UITeleportRock`) | Implemented: the map list, keeping and dropping maps, and moving to a map or a character; see `inventory.md` |
| Item Maker (`UIMaker`) | Recipes with their stimulator and gems, monster crystals, disassembly and the random make: the drop lists, ingredient counts, fee, gauge, result dialog and chat lines; see `inventory.md` |
| Writs of Solomon, gacha EXP | Implemented: using a writ, the EXP gauge's tooltip and taking the EXP by double-click; see `inventory.md` |
| Continent ships, station wall clocks | Docking ships and the Balrog's ship, CONTI_MOVE/CONTI_STATE, CLOCK kind 1; see `ships.md` |
| Monster Carnival (`Character/Carnival`, `CarnivalHandlers`) | The packets 289 to 296, as v83's `CField_MonsterCarnival` does: START sets the player's side, their CP and the sides', and in the revive room only the side (START and SPAWN_CHAR come before the fade-out loads the new field, so `carnival::enter_field` keeps the new map's field type from the moment the server moves the player, and both go by it); OBTAINED_CP and PARTY_CP keep the CP; SUMMON, MESSAGE and DIED write v83's lines in the chat log in type 7's colour, the summoned thing's name read from the field's `monsterCarnival` lists, and SUMMON and MESSAGE answer the request; LEAVE and RESULT write theirs in type 0xc's. A side's tag, "Maple Red" or "Maple Blue", is drawn instead of the guild's, for the player from START and for others from their spawn's team byte, read only in field types 10, 11 and 15; a new field forgets it. On a battle field (10 or 15) the player's side's mobs can't be hit, neither touch nor attack them, and are drawn half see-through. START on a battle field puts up the CP window (`UIMonsterCarnival`, after v83's `CUIMonsterCarnival`), and leaving the field takes it down. It shows the player's CP and both sides' (the player's side on the line with its own icon), and its tabs list the field's `monsterCarnival` mobs, skills and guardians with their names, costs and F-keys. It starts folded to the CP; a tab, the Tab key, an F-key or BtMax opens it, and an open window left alone folds after 8 seconds unless its lock is on. Clicking a row selects it (drawn white on blue) and shows its description; BtDecide, a double click, or the row's F-key (F1 to F12, which select and open too) sends 218 with the tab and row, once a request is answered, the player alive, and 500 ms gone. While the window is up it takes Tab and F1 to F12. BtSide opens and closes a side panel 157 px right of the window that counts the mobs summoned so far, a row per name: it starts with START's counts of the mobs already out, SUMMON counts a mob up, and the name last summoned is drawn white on blue. It shows 5 rows; its bar scrolls to the summoned row while it is open. Not done: scrolling a list past 12 rows, which no v83 field has |
| Boss sites' closing times | ZAKUM_SHRINE (303) and HORNTAIL_CAVE (302) write v83's chat line (type 0xc, the SKILL line) on when the Zakum Shrine or Horntail's Cave closes: before the boss is summoned, after, or closed at 0 minutes, nothing for a negative count; see `zakum.md` |
| Map clouds (`info/cloud`, 223 maps such as Perion) | Implemented (`MapClouds`): as v83 does, 10 to 59 of `MapHelper.img/weather/cloud`'s canvases scattered over the field's footholds plus 200 px, at 140 to 219 of 255 opacity, drifting right at 17 to 50 px/s for good and wrapping, above the front backgrounds |
| Map tooltips (`ToolTip`, 270 maps such as Henesys) | Implemented (`MapTooltips`, `PlainTooltip`): as v83 does, a numbered zone's `ToolTipHelp.img/Mapobject` title and description show 20 below the cursor while it is over the zone, and stay up at the zone's bottom right while the player stands in its `<N>char` zone, in a translucent navy box without UI art. v83 pairs `<N>char` with the Nth zone in WZ order, which NX loses; HC pairs it with zone N, which differs only in Ellinia and Ariant. Font metrics are assumed (Arial 12). Seven titles in our `String.nx` carry a corrupt Korean prefix from the WZ conversion |
| Button help (`ToolTipHelp.img/Game/Button`) | Implemented (`UIElement::set_button_help`, `PlainTooltip`): as v83 does, hovering a button with help shows its title and description in the plain navy box with a ring cut inside its edge, 270 wide unless the title needs more, hanging from the button's top right; a title alone takes the one-line box. Pressing hides it, and disabled buttons have none. The status bar's Cash Shop, Key Config, quick slot toggle and Claim buttons, the Teleport Rock's buttons (the VIP rock's register help says 10 maps) and the quest log's Quest Helper have it. v83 attaches the Trade button's help to the Cash Shop's by mistake and turns Menu's off, so neither has any. The Stat window's labels and the detail panel's (`Game/UIWnd/Stat` and `StatDetail`) show their help in the same box 20 below the cursor, but a beginner, Noblesse, Legend or Evan of level 10 or less only gets Name to Fame's; the EXP line shows "EXP : current / needed" in the one-line box 20 right of and below it. In the Cash Shop, the avatar's three buttons have help too, and Charge, Check and Coupon's sits on top of them (`UIStateCashShop` draws it). The inventory expansion buttons' help is never read, as in v83 |
| Screenshot key | Implemented: as v83 does, Scroll Lock saves the frame as the next free `MapleNNNN.jpg` (quality 75) in the working directory, where v83 uses the game folder; in game, a fourth press less than a second after the last shows "You cannot take screenshots at this moment..." and later quick presses do nothing |
| Medal name tags (`MedalTag`) | Implemented: as v83 does, a worn medal's name hangs 1 px under the name or guild tag, in the art and text colour of its `info/medalTag` style from `UI NameTag.img/medal`, or in a plain name tag box when the style has no art. v83 writes it in Dotum 12, which HC doesn't have, so the text and the number of tiles follow HC's 12 px font |
| Cash weapons (`CashWeapon`, `CharEquips`) | Implemented, as v83 does (0x41272c, 0x412929, 0x460aa0): a cash weapon (1702xxx, worn at -111) is drawn in the regular weapon's place from `Weapon/<id>.img/<type>`, where the type is the regular weapon's id / 10000 % 100 (30-33, 37-49). A cash weapon with no art for that type, or worn with no regular weapon, shows no weapon at all. The regular weapon keeps its stances, sounds and speed. The player's, other characters' and the character select's avatars all draw it |
| Set effects (`SetEffect`, `SetEffectRules`) | Implemented, as v83 does (FUN_005d925d, FUN_009393fc): a character, the player's or another's, who wears one of each of a set's groups in `Effect.nx SetEff.img` (sets by number, not by name order) shows its `effect`, looping, and its `backgrnd`, which fades out over 500 ms when they don't stand or attack, and comes back from its start after they have stood for 1.5 s. A group's items are looked for in the body parts of its first item, not in the part its name gives: group 0 is the hair, a ring is on any finger, and a cash weapon stands in for the weapon under it, so `CharLook` keeps it. The `multipet` sets need three listed pets out. A `cash` set wins over a plain one, and a plain one over a pets' one, though a second pets' set takes a plain one's place if no plain set came first; a morph or a ghost shows none, and so do the character select and the Cash Shop. The two pets' sets, 63 and 66, also put a `petBackgrnd` behind each pet, on its feet: a random one of the set's, each once before any again, mirrored as the character faced when it came. It hides and returns with the character's background, as another random one; a pet who takes a listed pet's place within the same set has none, as in v83 |
| Boss HP bar (`UIMobGage`) | Implemented: FIELD_EFFECT mode 5 and `HPgaugeHide` mobs' hits show v83's bar right of the minimap, with the mob's name over its icon; see `combat.md` |
| Map traps (`obstacle` objs) | Implemented: the client tests the traps' frame boxes each tick, rolls and reports the hit as TAKE_DAMAGE -3 with v83's push, and follows a `targetField`; trap damage to mobs is not done; see `combat.md` |
| Field shake (`TrembleRules`) | Implemented, as v83's SetTremble does: FIELD_EFFECT mode 1, Zakum's body appearing, Zakum, Horntail and Balrog dying, mob attacks with `tremble`, and Shout, Dragon Roar, Bamboo Rain and Rage of Pharaoh shake the field's view but not the UI. Every 30 ms the view moves up to the force off the player in each direction, and the force fades by 0.92 a tick (0.85 for a heavy shake). Others' skills shake it less the further they are, to nothing at 400 px. The ShakeScreen option turns it off, except for Horntail's deaths |
| Bamboo Rain and Rage of Pharaoh's black (`BlackoutRules`) | Implemented, as v83's FUN_004370cd: a black layer over the whole view, just behind the local player, fades in over 720 ms, stays 3200 ms and fades out over 520 ms, starting at the cast. One's own Bamboo Rain lays two layers at once, which are darker during the fades. One's own Rage of Pharaoh lays one, except the Evan copy 20011020, which lays none. Someone else's lays one. The ShakeScreen option does not turn it off |
| Field effects (`FieldEffectRules`) | Implemented, as v83's FIELD_EFFECT handler does: mode 0 plays `Effect.nx Summon.img/<n>` at a point on the map and its `Sound.nx Summon.img/<n>` louder the nearer the player is; mode 3 plays `Map.nx Effect.img/<path>` once at (400, 260) of an 800x600 view, stacking, and above the windows when the node has a `z`; mode 4 plays `Sound.nx Field.img/<path>` once; mode 6 changes the BGM until the next map; mode 7 spins the reward roulette's three `miro` reels above the windows. A new BGM, a map's included, waits for the last one to fade out over 600 ms. Mode 2 shows a named map object; see "Named map objects" |
| The server's hold on the UI (`UiControl`) | Implemented, as v83 does: OPEN_UI (220) opens one of the player's windows if it isn't open, by id: 0 items, 1 equipment, 2 stats, 3 and 0x21 skills, 5 key config, 6 quest log, 9 monster book, 0xa their own character info, 0xb guild board, 0x14 energy bar, 0x16 party search, 0x17 Maker, 0x1b Family, 0x1c Family Tree; in an Aran tutorial map (fieldType 0x16) only the quest log. 0x1a, the Ranking, has no window in HC; beginners up to level 9 get its refusal notice. LOCK_UI (221) 1 hides the HUD, closes those windows and takes the keys (hotkeys, chat, movement, skills) away until LOCK_UI 0; the quest alert waits for it, and MESSAGE_BOX lines are dropped meanwhile. The fading invitations and Quest Complete! notices stay: one made while locked waits to fade in, and one shown fades out and comes back on unlock with its whole time again, counted from the lock. DISABLE_UI (222) 1 drops other players' spawns (160) and packets 185 to 204 until 0. MESSAGE_BOX (232) is a line in the chat log in type 0xc's colour. OPEN_UI_STRING_LIST (233) opens a slideshow (`UIImageSlides`, the NPC dialog's type 9): a plain white 456 by 386 window showing one image per page, the strings being image paths such as `UI/UIWindow.img/QuestIcon/4/0` (`path_image`, shared with the NPC dialog's `#f` and `#F` images); empty strings are skipped and a path that names no image shows nothing. Prev and Next turn the pages, OK appears on the last, and OK, the close button (End Chat), Esc and Enter close it without telling the server; it takes the keys from an open chat line, and like v83 it first shows Next by the packet's count, empty strings included. While locked, the lower-right gain lines are dropped, and a new map fades in over 300 ms instead of 600. SPAWN_GUIDE (223) 1 puts the tutorial guide at the player's feet (`Guide`, a summon only this client knows of: skill 10001013's fairy for Cygnus jobs, 20001013 for the rest), playing `summoned`; it follows the player, comes along to each new map, and goes on 223 0 or when a level-up reaches 11. TALK_GUIDE (224), ignored without a guide, mode 0 shows a balloon (`ChatBalloon.img/tutorial`) with the text wrapped at the packet's width for its duration in ms; any other mode shows `UI/tutorial.img/<id>` above the guide for its duration. Either replaces the last one, and the guide plays `say` after a balloon, and after a hint for anyone but Aran and Evan. Not done: the lock's muted sounds and map music on unlock, the Monster Carnival, GM board and Title windows. The server sends none of these yet |
| Coconut Season (`Character/Coconut`, `CoconutHandlers`, `UICoconut`) | Implemented, as v83's `CField_Coconut` does (`coconut.md`): COCONUT_HIT (285) shows a nut's state, or every nut's, after its delay; COCONUT_SCORE (286) and CLOCK fill the scoreboard at the top right; FIELD_SPECIFIC_DATA (133) dresses the player in their team's uniform until they leave; a basic attack on the field swings at the nuts and sends COCONUT (213) for the first one hit; a GM's unknown `/` line is sent as chat. Not done: keeping the uniform over equips changed on the field |
| Snowball (`Character/Snowball`, `SnowballHandlers`, `MapSnowball`) | Implemented, as v83's `CField_SnowBall` does (`snowball.md`): SNOWBALL_STATE (281) places the balls and sets their pace, and the client rolls them on and fades out the loser's; HIT_SNOWBALL (282) shows a hit's damage after its delay and plays the snowman's hit; SNOWBALL_MESSAGE (283) adds a yellow line; the snowmen's HP bars; a basic attack on the field swings at the balls and snowmen and sends SNOWBALL (211); standing in a ball sends LEFT_KNOCKBACK (212) and LEFT_KNOCK_BACK (284) throws the player back; a snowman hurts the player who walks into it |
| Named map objects (`ObjStateRules`) | Implemented, as v83 does: an obj with a `name` (party quest gates, OPQ spears, coconuts, the CWKPQ portals and menhirs) has its own node as state 0 and `s1`, `s2`... as further states, each with its own animation, sound and trap. They start hidden, except state 0 of an `autoActive` obj. FIELD_EFFECT mode 2 shows one's current state; FIELD_OBSTACLE_ONOFF (139) and SET_OBJECT_STATE (153) switch one, FIELD_OBSTACLE_ONOFF_LIST (140) several, and FIELD_OBSTACLE_ALL_RESET (141) puts every named trap in state 0. Every obj plays as its `repeat` says: from 0 up it loops back to that frame, -1 plays once and stays, -2 plays once and returns to the first frame. The server sends only mode 2, for party quest gates (`party-quests.md`) |
| Tagged map objects | Implemented, as v83 does: an obj with `hide` starts hidden, and one with `tags` (split on ",") is shown or hidden by tag. A tag that is some quest's `showLayerTag` in Quest.nx QuestInfo.img shows its objs only while that quest is in progress, checked on entering the map, every 5 s and when the quest log changes. These are the arrows to the job instructors' training centers (quests 2400 to 2404) and the Aran arrows in Lith Harbor (21705). SET_MAP_OBJECT_VISIBLE (129) shows or hides a tag too; the server doesn't send it |
| Back fades | Implemented, as v83 does: SET_BACK_EFFECT (128) fades one of the map's backs, by its number under `back`, from its alpha now to full (mode 0) or to nothing (mode 1), linearly over the packet's duration. CLEAR_BACK_EFFECT (130) loads the backs again as the map has them. Backs with `a` 0, such as the Temple of Time's and Ereb's, stay hidden until faded in. The server doesn't send either |
| Curse filter (`CurseData`) | As v83 does (FUN_007a06f3): only text of 0x3ff bytes or more, or with a control character other than a line break, is refused ("Cursing is not allowed."); curse words are starred out and the text goes on. Notes, the plain, super and avatar megaphones, guild BBS threads and replies, macro names, pet names and the weather item's line work so. The triple megaphone's lines go as typed, as nothing shows v83 censoring them |
| General chat while a dialog is open | Refused in the chat log with "You can't chat right now." (FUN_0052c315) while any of v83's dialogs is open: a trade or personal shop (which has its own chat), an NPC shop, storage, Duey, the item megaphone's, Item Guard's, Karma's, the hammer's, Vega's or the kite's window (`UI::dialog_open`, v83's 0xbe7910; report c013) |
| Admin accounts' item rules | Not done: v83 refuses an admin dropping items ("The admin can't drop items.") and some items ("Not available with an Admin account.") by grade bits HeavenClient doesn't tell apart |
| Player shops, mini-games, memo/mail | No UI, no handlers |
| Reactors | Spawning, swings, hits and breaking; no skill hits, touch or quest gates; see `combat.md` |
| Pets | Summoning, following, others' pets, map changes, chat commands, idle actions, level-up effect, food, cash food and hunger; see `pets.md` |
| Mounts, summons | Partial or absent |
| Party/guild/boss quests, events, MTS | Absent |

93 outbound opcodes are defined (`Net/OutPacket.h`) and 120 inbound handlers
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
- **Fly maps.** A map with `info/fly` (and no `info/swim`, which wins) lets
  the player fly whenever they are off a platform (r25 0x9b2ef8, report b108):
  `MapInfo::is_flying` sets `phobj.flymap`, and `Player::is_afloat` puts them
  in the swim state, so they show the `fly` pose, or a morph's `fly`. In the
  air they move sideways as a flier at 200 px/s, whatever their shoe, sink at
  up to 10.05 px/s with no gravity, and drop at 70.35 px/s while holding Down.
  `Physics::fly_flap` beats up at 350 px/s and doubles the sideways speed if
  an arrow is held. A jump from the ground goes at 0.7 of the usual speed, and
  a rope jump as in water. On a platform they walk as on land. In water or
  a fly map v83 takes one jump every 300 ms, from the ground, a rope or the
  air (`Player::may_jump_afloat`), so swim maps have that limit too. The fly
  flag is not a morph's `flying`: that gate is the water's. Not done: how a
  remote flier moves between move packets, the air-attack gates for
  specific skills, whether Flash Jump works in a fly map, and whether pets
  take the flying stance there as they do while their owner swims.
- **Walking.** On flat ground the player accelerates at 1400 px/s² toward
  125 px/s × walkspeed and brakes at 800 px/s². On a slope the push scales by
  1 ± slant², and a slant over 0.9 slides. Positions advance by the average of
  the old and new speeds.
- **Air.** Gravity is 2000 px/s², capped at a fall speed of 670 px/s. A held
  direction accelerates at 200 px/s² toward about 8.9 px/s, so it barely
  steers.
- **Ice.** A map's `info/fs` (0.2 on El Nath's 68 icy maps; missing or 0
  means 1) multiplies, for every shoe, the walking push, the walk drag
  factor before the clamp, the air's drag, steering and steering speed, and
  the drag when flying or swimming (r25 0xa45cd6 into CVecCtrl's field
  attribute). On ice the player takes 0.45 s to reach top speed and slides
  about 1.6 s after letting go. The slide down steep slopes and the top
  walking speed are unchanged.
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
  elsewhere or grabs a rope, `update_fh` skips that platform. As in v83
  (`FUN_0094c4f8`), a platform with `forbidFallDown` refuses the down-jump
  from it, though not onto it, and Down + Jump then does nothing at all.
  A map whose `fieldLimit` has bit 0x20000 refuses the down-jump, and bit
  0x1 the plain jump (`field_limit::may_jump`). v83 lets GMs past both
  bits; HeavenClient doesn't know who is a GM.
- **Knockback.** A hit that does damage, and that the stance roll doesn't
  block, knocks the player away from the mob and up at 270 px/s, or 200 px/s
  under Power Guard. It also knocks the player off ladders and ropes.
  `Physics::knockback` queues the push for the next move. Each axis adds to
  the speed without going past the push, so a fast fall barely slows. The
  next move then lifts the player off the ground and truncates both speeds to
  whole pixels per second. A knock from standing rises about 18 px and lands
  about 73 px away.
  A mob attack with `info/knockback` (Horntail's parts, the Pink Bean
  family, Gatekeeper Nex and others) instead throws the player flat, away
  from the mob whichever way it faced: left when the mob is further right,
  else right (`mob_damage::thrown`, 0x958b0d). The throw goes at 1900 px/s
  from a foothold and 900 px/s from the air, a ladder or a rope, and the
  move path marks it as element 12. On the ground the walk drag slows it at
  1600 px/s², so a throw from standing slides about 1100 px.
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
  0x57 (a bought item), 0x58 (a failed purchase, with the SN when it is sold
  out), 0x59 (a redeemed coupon), 0x5C (another failure), 0x5E (a gift sent), 0x60 (a bag's
  new slot count), 0x62 (the storage's), 0x64 (the account's character
  slots), 0x68 (an item taken out), 0x6A (an
  item put back), 0x89 (a bought package) and 0x8D (bought with mesos) are read. Inventory slots keep the item's cash serial,
  which the item encoding carries, so a cash item can find its way back.
- **Buying:** BUY opens `UICashBuy` (`CashShop.img/CSNotice/0`): the item, its
  price and period, and a checkbox per balance. Only a balance that covers
  the price can be checked, and Maple Points alone for a free item (v83
  `0x46dbe5`); a ring offers NX Credit and NX Prepaid, and Name Change,
  World Transfer, 520000x, 549xxxx, 5221001 and 5222000 NX Prepaid alone
  (`0x46dc4d`). It checks the first balance that pays, trying NX Prepaid,
  NX Credit, then Maple Points. OK
  sends CASHSHOP_OPERATION (229) 0x03 with the cash type (1 Credit, 2 Maple
  Points, 4 Prepaid) and SN, or 0x1E for a package (items 910xxxx) and
  0x06/0x07 mode 1 for an expansion commodity (911x000), or 0x08 for the
  Extra Character Slot Coupon (5430xxx). As in v83, each
  mode is followed by a byte that is 1 for Maple Points, and 0x03 ends with
  a zero (no free trial). The result is a "successful purchase" or a failure
  notice. Failure notices use v83's text for each code (`FUN_0047c17a`),
  except gift-token and character-transfer codes, which get the default.
- **Purchase checks:** before the dialog, as in v83 (`0x46dadd`), BUY says
  "You don't have enough cash." when no balance covers the price, and "You
  don't have enough NX Prepaid" for an item NX Prepaid alone pays for. An item
  the character already has, in the Cash Inventory, its bag or worn
  (`0x4729b5`), can't be bought again when it is free ("You may not purchase
  the item since you already have it."); otherwise BUY asks "You already have
  the item(s) selected. Are you sure you want to buy the item(s) again?". A
  package asks when any of its items (`Etc.nx` `CashPackage.img`) is had
  (`0x46e121`). Buying with mesos refuses an item the character has.
- **Mannequin:** the avatar preview starts as a copy of the player's look.
  Double-clicking a commodity's icon tries it on (v83 `0x4bb37c`,
  `0x4ac3de`): equips only, silently not one whose id's thousands digit is
  the other gender's (0 male, 1 female), nor 16xxxxx. The same item again
  takes it off. An overall takes the pants off, a coat or pants an overall, a
  two-handed weapon the shield and a shield a two-handed weapon; a cash
  weapon (170xxxx) is worn over the weapon in its slot, and a weapon replaces
  it. Rings fill the free ring slots. A package (910xxxx) puts its items on,
  mounts and saddles included, if it is for the player's gender, or for
  either and its first gendered equip is; after its first item anything
  more disables Buy Avatar. Default Avatar goes back to the player's look
  and Take Off takes everything off; both enable Buy Avatar. Tabs don't
  change it. v83 also tries pets and some other cash items on, and tries a
  package on when its row is selected or with Enter; HC doesn't.
- **Buy Avatar:** with a package on, "The option is not available for that
  package."; otherwise the commodities tried on that the player isn't
  wearing (`0x4696b1`). The first one the character has stops the checks:
  a free one can't be bought again, another asks "You already have the
  item(s) selected...". The dialog says "Total N NX.Make purchase?" and offers
  each balance that covers the total, NX Credit only when nothing is free,
  or says "You don't have enough cash.". OK buys them one at a time, the
  last first (`0x46ba55`): each 0x57's balance sends the next, and a 0x58
  goes on to the next unless its code is 0xA2-0xA4. Nothing else can be
  sent meanwhile. At the end it says "You have successfully made the
  purchase.", "The purchase has failed." or names the items bought, "You
  have purchased the following items. (Some items weren't purchased)". v83
  leaves the queue waiting after 0x5C or those codes; HC ends it.
- **Buying with mesos:** v83 has no tab for the commodities that cost mesos
  (SN 80000000-89999999). When the shop opens, each one on sale goes into a
  random sub-tab that has items, at a random place (v83 `0x471f37`;
  `cash_catalog::scatter`). Its price reads "N mesos", and it has no GIFT or
  RESERVE, like anything free (`0x4baa51`). BUY says "You do not have enough
  mesos." when the purse is short, or else asks "Are you sure you want to buy
  this item?" and sends 0x20 with the SN (`0x46e5c5`); an item the character
  has comes first, with "You may not purchase the item since you already have
  it." On 0x8D the Item
  Inventory turns to the item's tab and scrolls to its row, then says "You
  have successfully made the purchase." (`0x47b603`). v83 also selects the
  slot; HC doesn't.
- **Check:** CHECK CASH sends CHECK_CASH (228), as does the game window
  getting the focus back while the shop is open; QUERY_CASH_RESULT answers.
- **Slot buttons:** Equip Slot through Stor. Slot open `UICashBuy` for the
  matching expansion item at 4,000 NX, and OK sends 0x06 mode 0 with the bag,
  or 0x07 mode 0. On 0x60 the bag's slot count changes at once. For 0x60 and
  0x62 the notice says "You have added slots to your item inventory.", as in
  v83.
- **Slot checks:** as in v83 (`FUN_0046c036`, `FUN_0046c55b`,
  `FUN_00470653`), a slot purchase needs some balance that covers the price,
  or it says "You don't have enough cash." first. A bag that would pass 96
  slots says "The requested number of slots can't be added." after OK. The
  storage, whose count comes from 0x4B and 0x62, stops at 48 and says "The
  storage slots can't be extended.", before the dialog for Stor. Slot. A bag
  that the player's next job advancement would widen past 96 asks first
  whether to buy anyway (`FUN_0046c38f`): four Equip, or eight Use or Etc,
  slots for a beginner, and four of the job's bags for a first or second job.
  As in v83 (`0x46c036`, `0x46c55b`), a slot button's dialog asks in words
  instead of naming the item: "Eqp adding inventory slots. (4000 NX) Confirm
  inventory slot purchase?" (Use, Setup and Etc likewise), or "Adding slots
  to the storage (4000 NX) Are you sure you want to add slots to your
  storage?". HC has no character-slot button.
- **Character slots:** the count comes from 0x4B and 0x64. As in v83
  (`FUN_0046c6f8`), the coupon says "You don't have enough cash." without a
  balance that covers it, then "You may not add any more character slots." at
  15; otherwise the dialog asks "Add a character slot (6900 NX) Are you sure
  you want to add another character slot?" in place of the item's name. 0x64
  says "You have added a new character slot." and is ignored unless the count
  grew and is at most 15 (v83 `0x47ac0a` drops it as a bad packet). The
  coupon's tooltip says "Add a character slot (Current : N slots) You may
  have up to 15 slots." in place of its description (`FUN_008f19a8`).
- **Item data:** packages and expansions take their name and icon from
  `Item.nx` `Special`, pets from `Item.nx` `Pet`, and pet equipment from
  `Character.nx` `PetEquip`.
- **Wish list:** RESERVE and REMOVE send the whole list (0x05, ten SNs), and the
  server answers with the list it kept (0x55). As in v83 (FUN_00470d7d,
  FUN_00479844), RESERVE says "Your wish list is full." or "You already have
  this item on your wish list." without sending, and only an added wish is
  announced when the list comes back.
- **Taking out and putting back:** double-clicking a Cash Inventory item
  sends 0x0D with its serial, bag and the bag's first free slot; the server
  answers with the slot and the item (0x68), or 0xBB if the bag is full.
  Double-clicking a cash item in the Item Inventory sends 0x0E, and the server
  answers with the Cash Inventory item (0x6A). As in v83 (`0x472632`,
  `0x472820`), a free commodity's item stays in the Cash Inventory while the
  character has one ("This item cannot be transfered because you already own
  this item."; the Cash Inventory item carries its SN), and an item marked
  `noMoveToLocker` stays out ("This item cannot be moved."). The footage doesn't show
  whether v83 used double-clicks or dragging; dragging isn't supported.
- **Gifts:** as in v83 (`0x46e822`), GIFT says "You don't have enough NX
  Prepaid" when NX Prepaid doesn't cover the price, or else asks "Would you
  like to gift this item to someone? [ ITEM ]". Yes opens `UICashGift` on `CashShop.img/CSGift/backgrnd`, with
  a To field and a message box that wraps over its two ruled rows (73
  characters). OK, once both are filled, swaps the art for `CSNotice/1` and asks
  for the birthday code (YYYYMMDD). Its OK sends 0x04 with the code, SN, name
  and message, after asking "This contains an item that will disappear once
  the expiration date has passed. Do you want to give it as a gift?" for an
  item with a period (`0x46ec85`). On 0x5E the sender sees v83's "COUNT [ ITEM ] was sent to
  NAME. PRICE NX Prepaid were spent in the process.", or a failure notice (0xA8 own account, 0xA9 unknown name, 0xC4 birthday code).
  For each gift in 0x4D the receiver gets v83's dialog (`UIGiftNotice`,
  `CSNotice/3`, FUN_0047959e): the item's icon and name, the sender, their
  message, and a note to send back. OK or Enter sends NOTE_ACTION (0x83) and
  shows "The note has successfully been sent"; Esc sends nothing. The next
  gift follows either way. The buddy and guild buttons that open the wider
  `backgrnd1` aren't drawn, and only one name is sent.
- **Leaving:** EXIT sends CHANGE_MAP with no body at once. Esc first asks
  "Are you sure you want to leave the Cash Shop?", as v83 does
  (`FUN_004bcc86`).
- **Coupons:** COUPON opens `UICouponDialog` (`UIWindow.img/Coupon`, v83
  0x471240) unless a request is out. A "-" typed at the end is dropped, as
  the art asks. OK or Enter trims the code and sends COUPON_CODE (230) with
  an empty string and the code; an empty code says "Please enter the coupon
  code." instead. 0x59 adds the cash items to the Cash Inventory and says
  what was received, glued together as v83 does (`cash_coupon::notice`,
  report b073); unlike other modes it leaves the request out until the
  balances come. Each coupon failure code has v83's text, and the third
  wrong code (0xB1) shows the default notice, then "You are disconnected
  from Cash Shop", then leaves the shop.
- **Not served yet:** CHARGE does nothing.

## Estimated client work for a complete game

| System | Client status | Effort |
| --- | --- | --- |
| Guild | Window, founding, invitations, leaving and expelling, ranks, titles, the notice, growing and disbanding, the emblem maker, the board (reading, writing, editing, the notice, replying and deleting) | M |
| Player shops | Nothing | L |
| Cash shop | No charging or storage window | S |
| Quest accept/complete UI flow | Partial | S |
| Pets | Summon, follow, chat commands, food, cash food, name tags, equipment, looting, the exception list and auto potions | M |
| Mounts | Partial rendering, no interaction | M |
| Memo, mail, MTS | Nothing | M each |
| Buddy whisper/find modes | Ignored | S |
| Mini-dungeons | Copies load their body through `info/link`; the field clock shows the time limit | - |
| Boss/event UI | Nothing | M |

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
14. The client read the stat change's SKIN as a short; v83 reads a byte
    (FUN_004e2fba), as the server writes it.
