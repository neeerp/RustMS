# NPC dialogue

Clicking an NPC opens asset-backed dialogue. Pio and other NPCs with normal
speech use Next, Back, and OK. Close and Escape cancel. When the NPC has
quests for the player, HeavenClient shows its quest menu first and renders
quest conversations itself; picking the "ETC" line (the NPC's script) asks
the server to talk. See [Quests](./quests.md) for when the client talks
instead of the server.

The NPC chat key (Space by default, action 55) talks to the nearest NPC
within 150 px horizontally and 50 px vertically of the player's feet
(`HeavenClient/Gameplay/MapleMap/NpcReach.h`). While a conversation is open,
it also does what Enter does: Next, OK, Yes, the focused answer, or the
selected choice. Migrations give earlier characters this binding unless
Space is already taken, and normalize it to an action binding.

## Content and quest boundary

`NpcDialogueData` in `game-data` resolves the numbered references under
`Npc.nx/<NPC ID>.img/info/speak` against `String.nx/Npc.img/<NPC ID>`.
This uses existing asset text without copying it into the repository. NPCs
without those references currently have no conversation.

`Quest.nx` contains conversation text (`Say.img`), descriptions
(`QuestInfo.img`), requirements (`Check.img`), and actions (`Act.img`). The
client renders `Say.img`; the server only reads `Check.img`/`Act.img` to
validate `QUEST_ACTION`. Normal speech has no quest or reward effects.

`net::dialogue::Conversation` handles ordinary pages independently of
transport. `net::dialogue::Prompt` and `build_prompt` also cover yes/no and
menu prompts for server-driven NPCs (scripts).

NPCs also act on their own in HeavenClient (`npc_idle`, after the v83
client). Every 3 to 9 s a standing NPC either plays one of its actions once
(any animation but `stand` and `move`, perhaps saying one of that action's
`speak` lines) or says one of its `info/speak` lines, in an `npc`-style chat
balloon. NPCs never walk: v83 moves only those with `info/move`, and none
have it. In v83 the controlling client sends each choice as NPC_ACTION
(`0xA6`) for the server to echo to the map; RustMS doesn't control NPCs, so
each client chooses locally and players see different actions.

## Scripted NPCs

`net::npc_scripts::NpcScript` holds native scripts (Lua is deferred) as a
list of pages: `Say`, `Ask` (effects on yes), `Menu` (each choice its
own pages), and `Style` (each style its own effects), plus effects applied
after the last page. The effects are
`net::scripts::Effect`, shared with map and portal scripts. The client actor
(`ClientActor::apply_effects` in `client/npc_script.rs`) applies them to a
copy of the player and commits any quest change together with it, or
nothing at all; if a given item doesn't fit, the script's "inventory full"
page shows instead. Scripts are built for the player as they are when the
conversation starts; effects that take items or mesos are checked again when
applied, and apply nothing if the player no longer has them. Answers are matched to the prompt they answer
(`npc_scripts::input`); anything else ends the script.

| Script | Trigger | Behavior |
| --- | --- | --- |
| `q1021s`/`q1021e` | QUEST_ACTION 4/5 for quest 1021 | Roger sets HP to 25 and gives Roger's Apple; finishing needs full HP and no apples left, then pays 3 Apples, 3 Green Apples, 10 EXP |
| `begin5` | Clicking Robin (2003) | 18-topic Q&A menu |
| `tutorialSkip` | Clicking the Maple Administrator (2007), or the `tutoChatNPC` portal for accounts with a level-30 character | Yes warps to Lith Harbor (104000000). Neither is on the original island, so this is unused |
| `begin7` | Clicking Shanks (22000) in Southperry | Yes sails to Lith Harbor (104000000) for Lucas's recommendation letter (4031801), at any level, or for 150 mesos from level 7; otherwise he says why not |
| `fighter`, `magician`, `bowman`, `rogue` | Clicking Dances with Balrog (1022000), Grendel the Really Old (1032001), Athena Pierce (1012100), or the Dark Lord (1052001) | First job advancement: see below |
| `hair_henesys1` | Clicking Natalie (1012103) in Henesys's hair salon (100000104) | A haircut (5150001) or a dye (5151001) coupon buys one of her styles, picked on a preview of the player; without a coupon she says so and nothing changes |

The four Explorer instructors (`INSTRUCTORS` in `npc_scripts.rs`) make a
beginner a Warrior, Magician, Bowman, or Thief. The beginner needs level 10
(a Magician 8) and 35 STR, 20 INT, or 25 DEX; a weaker one hears why not, and
anyone else gets one page. Yes applies `Effect::TakeFirstJob`
(`Player::take_first_job`: 3 SP for each level past the minimum plus 1, and
the job's extra max HP and MP, without a stat reset or heal), then
`Effect::AddSlotRow` for the bags the job widens, then the job's weapon and
ammunition. The rules and grants follow Cosmic, not yet checked against GMS
footage. The player gets STAT_CHANGED (job, max HP, max MP, SP) and
INVENTORY_GROW (`0x1E`: bag type, new slot count) per bag; others on the
field see the job-change effect (SHOW_FOREIGN_EFFECT 8). The player's own
client plays it, and the job-change sound, from the job bit of STAT_CHANGED.
As in v83 (`FUN_00a1fb52`), HeavenClient also opens the Stat window after a
beginner's first advancement if it isn't open. Pirates (Kyrin) and later
advancements are not scripted yet.

Text comes from the [server content pack](./server-content.md) with
placeholders. HeavenClient expands NPC text markup (`Util/NpcTextMarkup.h`:
`#h`, `#p`, `#t`/`#z`, `#o`, `#m`). The text layout (`Graphics/TextLayout.cpp`)
handles the colours (`#k`, `#b`, `#g`, `#r`, `#c`, `#d`) and bold: `#e` switches
to the bold of the text's font and `#n` back.

The dialog (`UINpcTalk`) is the v83 `UtilDlgEx` window from `UIClassic.nx`,
laid out from period footage. It is centred on the screen, and its middle
grows with the text, from 120 px to 240 px; longer text scrolls by line.
NEXT and BACK sit in the text area; END CHAT, OK, YES/NO, and ACCEPT/DECLINE
sit on the bottom bar. A server menu's `#L` choices stay in the text where
the script puts them, each with a dot. The client's own quest menus list
their choices in 32 px rows under the question instead (`UINpcTalk::Menu`).
The text types out at about 35 characters a second, but a menu's choices
show at once, as in footage.

The first choice starts selected (red arrow, dotted underline), and Up and
Down move the selection. Clicking a choice or pressing Enter answers with
it. A server menu has no NEXT, as in v83 footage; a quest list keeps NEXT,
which also answers. On YES/NO and ACCEPT/DECLINE pages, the first Left or
Right outlines YES or ACCEPT with its `keyFocused` frame, as in the original;
then Left and Right move the outline, and Enter presses it. With
nothing outlined, as when the page opens, Enter presses YES or ACCEPT. On
other pages Enter presses NEXT or OK. Enter works even while the text types
out.

Image tags draw inline: `#v`/`#i` item icons and `#s` skill icons in a 32x32
slot, `#f`/`#F` any image by NX path (`UI/UIWindow.img/QuestIcon/4/0`). `UI/`
paths are read from `UIClassic.nx`, then from `UI.nx`, whose
`UIWindow2.img` holds the quest icons. Each image becomes a `Text::BOX` character that the text layout sizes
like the image, bottom on the baseline; a taller image pushes its line
down. As in 2008 GMS footage, the images show as soon as a page opens and
only the text types out. Images inside `#L` menu choices are still dropped.

## Runtime ownership

TalkToNpc (0x3A) and NpcTalkMore (0x3C) are parsed into `NpcInteraction` actions.
Start requests go straight from the ClientActor to the player's FieldActor,
which resolves the object ID against its own NPC list. The field
returns the validated template ID, location, and a server-generated request ID.
ClientActor rejects late replies after another NPC click or a map transfer.

ClientActor owns at most one normal conversation. It handles responses
and awaits database work on the blocking pool, serializing reward updates with
other character operations. A new click replaces the conversation. Finishing,
cancellation, invalid responses, map changes, channel changes, and disconnects
clear transient state. Responses without an active conversation are ignored.

## Wire format and client

NpcDialogue (0x130) carries speaker category, NPC template ID, message type,
speaker byte, text, and separate Back/Next bytes. Normal text uses message
type 0. Response 1 advances or finishes, 0 goes back, and -1 cancels. The
speaker byte's bits, as v83 reads them: 0x1 leaves out END CHAT, 0x2 shows
the player instead of the NPC, 0x6 (either bit) puts the speaker on the
right with the `UtilDlgEx/it`, `ic` and `is` frame and the text on the left,
and 0x8 mirrors the NPC. The server always sends 0. HeavenClient's quest
dialogs use 2 when the player answers a flipped question
(`Say.img/<id>/<n>/stop/<page>/flip`).

HeavenClient uses the Back/Next bytes to choose buttons while retaining the
wire message type for responses. Escape and END CHAT send -1 on say, yes/no
and accept/decline pages, and 0 on menus and prompts; without END CHAT,
Escape does nothing. Message type 1 shows YES and NO; type 12 shows ACCEPT
and DECLINE, which the client's quest dialogs use for an offer. A menu
(type 4) answers with response 1 and the chosen `#L` id. A say page with
speaker bit 0x4 carries an i32 before its text: the NPC it speaks for.

Types 2, 3 and 13 are prompts with an input box under the text. Their
bodies follow the text: a text prompt (2) has `str default, i16 min,
i16 max` (bytes, 0 for no most), a number prompt (3) `i32 default, i32 min,
i32 max`, and a box text (13) `str default, i16 cols, i16 rows`. The
client checks a text's trimmed length and a number's range, shows v83's
notice and stays open when they are wrong, and answers `02 01 str`,
`03 01 i32` or `0D 01 str`. The server sends none of them yet.

Types 7 and 8 are a choice of styles: after the text come `u8 count` and
that many `i32` style ids. The client shows the v83 `UtilDlgEx_Avatar`
window (`UIAvatarSelect`) instead of the usual dialog: the NPC, the text,
and the player wearing one style at a time, a face if the first id / 10000
is 2, a hairstyle if 3, otherwise a skin. BACK and NEXT go round the
styles, TAKE OFF ALL shows the player without equipment and DEFAULT
SETTINGS brings it back. OK or Enter answers `07 01 u8` (the style's index);
CANCEL, LEAVE STORE or Escape answers `07 00`. The server sends type 7 for
Natalie's hairstyles and applies the chosen one with `Effect::SetStyle`
(a stat change and UPDATE_CHAR_LOOK for others). The window has no scroll
bar for text taller than its 100 px box, drops the text's images, and does
not type the text out.

Types 9 and 10 are a choice of pets: after the text come `u8 count`, for
type 10 a flag byte, then that many entries of an 8-byte pet serial and a
byte the client ignores. Pets not in the player's Cash tab are left out.
Type 9 needs one pet left, type 10 two when its flag is 0 and one
otherwise; with fewer the client answers `t 02` and shows nothing.
Otherwise it shows the pet chooser (`UIPetChooser`, as for pet equipment)
with the text as its question. OK or Enter answers `t 01` and the pet's
serial; CANCEL, the corner button or Escape answers `t 00`. The server
sends neither type yet.

Types 5 and 6 are quizzes. Neither has text: after the speaker byte comes
`u8 flag`, where 1 closes an open quiz without an answer. With 0, a quiz
(5) has `str title, str question, str clue, i32 min, i32 max, i32
seconds`, and a speed quiz (6) `i32 subject type, i32 subject id, i32
points, i32 number, i32 seconds`. The subject is an NPC (0), a monster (1)
or otherwise an item, shown by its picture. While one is open, another
show is dropped, so the server hides the last before sending the next.
Both windows (`UIQuiz` and `UISpeedQuiz`, rules in `IO/UITypes/Quiz.h`)
count down and stay open beside the game: they are not modal, and Escape
does not close them. The answer has no response byte: `05 str` or `06
str`.
- A quiz checks the answer's bytes against twice `min` and `max`, two
  bytes a Korean letter, and otherwise shows v83's "You must enter
  atleast/less than %d letters. (Korean)" notice and stays open. An empty
  answer does nothing.
- A speed quiz answers two letters or more. NEXT answers `""`, GIVE UP
  `"__GIVEUP__"`, and its last two minutes tick, louder as they run out.
- When the time is up, either answers `""` and closes.
- While one is open, the Cash Shop and the game menu's channel change
  refuse with "You can't do this while taking the quiz."

The server sends neither type yet.

Type 14 is a slide menu, such as the Dimensional Mirror's. It has no text:
after the speaker byte come an `i32` art set
(`UIWindow.img/SlideMenu/<set>`) and a string of `#<id>#<label>` pairs;
the last label needs no closing `#`. The client
(`UISlideMenu`, `IO/UITypes/SlideMenu.h`) sorts the choices by id and
shows three icons (`BtMain/<id>`) at a time, the middle one framed and
labelled, starting from the second choice when there are two or more. The
arrows, Left and Right slide along. MOVE, Enter or the middle icon answers
`0E 01 i32` with its id; CANCEL or Escape answers `0E 00`. An id of -1
means nowhere to go: alone, it shows its label with no icons and MOVE
disabled; among other choices, it leaves nothing at all. Text not
starting with `#` hangs v83 and gives no choices in HeavenClient. The
server sends none yet.

## Assets and validation

Place `Map.nx`, `Npc.nx`, `String.nx`, and `Quest.nx` in `assets/game-data/`. The server's
NPC paths can be overridden with `RUSTMS_NPC_NX_PATH` and
`RUSTMS_STRING_NX_PATH`. These assets stay untracked. When running the Docker
harness, files or symlink targets must live within its mounted `assets/` tree.

Validate with `cargo run -p integration-harness --bin harnessctl -- test`.
`npc_dialogue` integration tests cover asset text, progression, cancellation,
restarting, invalid requests, stale replies across warps, and player isolation.
`first_job` covers Dances with Balrog refusing a weak beginner, then making a
strong one a Warrior, and what is saved.
Unit tests cover the conversation transitions and asset-key resolution.
