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
have it. The field gives one client control of its NPCs (SPAWN_NPC_C,
`0x103`); that client sends each choice as NPC_ACTION (`0xC5`) and the field
echoes it to the map (`0x104`), so everyone sees the same action. Requests
from other clients are dropped.

Three more packets change NPCs, though no RustMS content sends them yet
(builders in `net/src/packet/build/world/npc.rs`, report b095):

- `0x105` (`build_npc_enabled`): `oid`, then 1 to show the NPC with
  `Effect/MapEff.img/NpcSummon` or 0 to hide it with `NpcReturn`. A hidden
  NPC is not drawn, takes no clicks and picks no idle actions. The spawn's
  last byte (`ForeignNpc::enabled`) is the same flag, for those who come
  later and for a new controller; the field doesn't keep it yet, so content
  that hides an NPC must store it there first.
- `0x106` (`build_npc_special_action`): `oid` and an action name. A client
  that has the action asks for it as an NPC_ACTION request, which the field
  echoes as usual if it comes from the controller; send it to the controller.
  Nothing happens while the NPC plays a `special` action, or if it has no
  such action.
- `0x107` (`build_set_npc_scriptable`): NPC ids with a script name and
  YYYYMMDD start and end days. The client adds them after the NPC's
  `info/script` windows for the rest of the session, and the newest window
  open today is the NPC's script (`Gameplay/NpcScripts.h`). Only an NPC with
  a script asks the server to talk when it has no quests to list (see
  `quests.md`).

## Scripted NPCs

`net::npc_scripts::NpcScript` holds native scripts (Lua is deferred) as a
list of pages: `Say`, `Ask` (effects on yes), `Decide` (a yes/no whose
answers each have their own pages), `Menu` (each choice its own pages), `Style` (each style its own effects), `End` (a failing
choice's last page), `Done` (no page: the conversation ends there with
its effects, as when a menu choice warps at once) and `Act` (a page whose
effects apply as it is shown, as when Cloto hands over a pass while
speaking), plus effects applied after the last page. The effects are
`net::scripts::Effect`, shared with map and portal scripts. The client actor
(`ClientActor::apply_effects` in `client/npc_script.rs`) applies them to a
copy of the player and commits any quest change together with it, or
nothing at all; if a given item doesn't fit, the script's "inventory full"
page shows instead. Scripts are built for the player as they are when the
conversation starts; effects that take items or mesos are checked again when
applied, and apply nothing if the player no longer has them. Answers are matched to the prompt they answer
(`npc_scripts::input`); anything else ends the script.

A portal can open an NPC's conversation without a click
(`Effect::OpenNpc`, `ClientActor::open_npc`): it starts whatever a click
would, and the NPC needn't be on the map, since NPC_DIALOGUE carries the
NPC's template id and HeavenClient reads nothing else. It doesn't open over
a conversation already open.

| Script | Trigger | Behavior |
| --- | --- | --- |
| `q1021s`/`q1021e` | QUEST_ACTION 4/5 for quest 1021 | Roger sets HP to 25 and gives Roger's Apple; finishing needs full HP and no apples left, then pays 3 Apples, 3 Green Apples, 10 EXP |
| `begin5` | Clicking Robin (2003) | 18-topic Q&A menu |
| `tutorialSkip` | Clicking the Maple Administrator (2007), or the `tutoChatNPC` portal for accounts with a level-30 character | Yes warps to Lith Harbor (104000000). Neither is on the original island, so this is unused |
| `begin7` | Clicking Shanks (22000) in Southperry | Yes sails to Lith Harbor (104000000) for Lucas's recommendation letter (4031801), at any level, or for 150 mesos from level 7; otherwise he says why not |
| `fighter`, `magician`, `bowman`, `rogue`, `kairinT` | Clicking Dances with Balrog (1022000), Grendel the Really Old (1032001), Athena Pierce (1012100), the Dark Lord (1052001), or Kyrin (1090000), or a beginner up to level 10 touching the `tutorial` portal in their room | First and second job advancement: see below |
| Job instructors | Clicking an examiner (1072000-1072003) or, inside the test, a proctor (1072004-1072007) | The second job's test: see below |
| `inside_pirate` | Clicking the proctor (1072008) inside Kyrin's tests | Yes warps to Kyrin; see below |
| `hair_*`, `face_*`, `lens_*`, `skin_*` | Clicking one of 53 stylists: hair salons, plastic surgeons, lens shops and skin care (`net::stylists`) | A menu of the NPC's services, each for its coupon. A VIP coupon lets the player pick the look on a preview; a regular or experimental one rolls it after a yes. A cut keeps the hair's colour and a face its lens colour, while a dye or a lens changes only the colour; a style that doesn't come in the player's colour is offered in colour 0. A salon's hair membership card (5420000-06) makes its cuts free and is kept, and a one-time lens (5152100-07) pays for its own colour. Without the coupon the NPC says so and nothing changes. Claudia (9201039) is not done yet |
| `multipet_success` | Clicking the hidden spot (9102100) on the Pet-Walking Road (100000202) | While Bartos's test (4646) is started, yes gives the note (4031921) the test asks for, once; otherwise one page |
| `q4647e` | QUEST_ACTION 5 for quest 4647 (Bartos) | Two pages, then takes a Pet Snack (5460000), completes the quest and teaches Follow the Lead for the player's line (8, 10000018 or 20000024); without a snack, one page |
| Gachapon machines | Clicking a machine (9100100-9100107, 9100109, 9100117) | With a Gachapon Ticket (5220000), yes trades it for the prize rolled when the conversation opened, and the next page names it; without one, a menu explains the machine. See `agent_docs/inventory.md`, "Gachapon" |

The five Explorer instructors (`INSTRUCTORS` in `npc_scripts.rs`) make a
beginner a Warrior, Magician, Bowman, Thief, or Pirate. The beginner needs
level 10 (a Magician 8) and 35 STR, 20 INT, 25 DEX, or for a Pirate 20 DEX; a
weaker one hears why not, and
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
beginner's first advancement if it isn't open.

From level 30 the same instructors offer their first jobs the second job,
following Cosmic's flow. Without the test's proof, the instructor gives a
letter (4031008-4031011) for the line's examiner, and with the letter only
says to go. The examiner sends a letter holder to the test map (Warriors
108000300, Magicians 108000200, Bowmen 108000100, Thieves 108000400). Upstream
takes the letter there and records the entry in a quest the client data lacks
(100003 does not fit a quest id), so here the player keeps the letter until
passing and can come back in with it. The test's proctor trades all of the
player's Dark Marbles (4031013), once there are 30, and the letter for the
Proof of a Hero (4031012); passing or not, yes takes the player back to the
examiner's map at the portal upstream uses (`Effect::WarpToPortal`). With the
proof, the instructor shows a menu of the line's second jobs; each choice
describes the job and asks, and yes takes the proof and applies
`Effect::TakeSecondJob` (`Player::take_second_job`: 1 SP and the job's extra
max HP or MP) and a row more in the Etc bag.

The third job (`THIRD_JOBS`), also after Cosmic, starts with a level-70
second job's chief in El Nath (Tylus 2020008, Robeira 2020009, Rene
2020010, Arec 2020011, Pedro 2020013), who sends the player to their
instructor. The instructor sends them to the Door of Dimension (1061009) in
the line's hunting map, which lets one challenger at a time into the
line's two clone maps (`Effect::RequireEmptyMap` asks the directory
whether anyone is in either on the channel). The clone's room has a
20-minute limit (`npc_scripts::time_limit`), puts its clone back when the
last player leaves (`npc_scripts::resets_when_empty`), and has a Sparkling
Crystal (1061010) back to the door. The clones (9001000-9001004) drop the
Black Charm (4031059) to a player the instructor sent (`[mobs]` in
`reward_overrides.toml`); the instructor trades it for the Necklace of
Strength (4031057), and the chief that for the Holy Stone's assignment.
The Holy Stone (2030006) takes a Dark Crystal (4005004; Spiruna, 2032001,
refines one from 10 ore and 500,000 mesos after her quest 3034) and asks
five questions picked from the text's `holy_stone.q.<n>` keys, with the
answers shuffled. A wrong answer ends the try; all right earns the Necklace
of Wisdom (4031058). The chief takes it for `Effect::TakeThirdJob`
(`Player::take_third_job`: 1 SP, 5 AP and the second job's max HP or MP
again) once the player has no more SP left than 3 for each level past 70.
Upstream tracks the steps in quest records the client data lacks; here
they are script markers (`Effect::Mark`/`Unmark`), Started records at
`quest::SCRIPT_MARKERS` (32000) and up that are saved with the quests but
never sent to the client. The chiefs also approve players for Zakum: a
second job of 70 picks the test or Zakum from a menu (see
[zakum](./zakum.md)).

The fourth job (`FOURTH_JOBS`) is one talk with the line's instructor in
Leafre's 240010501 (Harmonia 2081100, Gritto 2081200, Legor 2081300, Hellin
2081400, Samuel 2081500), once a third job is level 120 and has completed
the line's last quest (6904, 6914, 6924, 6934, 6944; see
[quests](./quests.md)). Yes gives the Maple Hero skill book (2280003, as
Cosmic does; refused with nothing changed if the Use bag is full) and
applies `Effect::TakeFourthJob` (`Player::take_fourth_job`: 3 SP, 5 AP and
the second job's max HP or MP again). It also teaches, at level 0 and
their master level, the new job's skills whose `Skill.nx` node has a
`masterLevel` (`net::skills::fourth_job_skills`), in one UPDATE_SKILLS.
The other fourth-job skills are taught by quests and mastery books.

Kyrin's second jobs are client-driven quests instead (see
[quests](./quests.md)): How to become a Brawler (2191) and How to Become a
Gunslinger (2192), for a level-30 Pirate, one at a time. The client offers them
beside her script. With one started, her script offers its test map
(Brawler 108000502, Gunslinger 108000501), where Octopirates (9001006,
9001005) that only Flash Fist or Double Shot hurt (see
[combat](./combat.md)) drop 15 crystals (4031856, 4031857; `[mobs]` in
`reward_overrides.toml`). The proctor there sees the player back to Kyrin's
map, after a word of praise once the test's crystals are held; the crystals
stay for the quest's completion to take. With the quest completed, Kyrin
describes the job and yes applies `Effect::TakeSecondJob` and the Etc row.
Upstream turns away a player whose test map is occupied and takes the
crystals from one who leaves early; neither is done here.

Amoria's engagement NPCs, after Cosmic, also use script markers (32300
and up). See [marriage](./marriage.md) for the steps. Moony (9201000) offers to make a
ring box or throw one away. Mom and Dad (9201003, in 680000004) want four
different Proofs of Love (4031367 to 4031372). The Nanas (9201001,
9201023 to 9201027) each give one for 50 of their town's drop. A Nana
works only in her own town, and only while the player seeks the blessing;
otherwise she and Mom and Dad say their own asset lines.
Nicole (9201005) and Bonnie (9201008) book weddings, in Amoria only. In
their lobbies they lead guests in instead, which is not done yet. Their
script ends with `Effect::BookWedding`, and the client actor does the
partner round trip after it. Its refusals and its outcome open a one-page
script of their own (`NpcScript::wedding_assistant_says`). Their
`WeddingView` also carries the wedding the player is a guest of, for the
guest's choice, and a Wedding Invitation Ticket adds a choice ending with
`Effect::ExtraInvitations`, handled the same way.

The Event Assistants (`EVENT_ASSISTANTS`: Paul, Jean, Martin and Tony, script
`Event00`) take players to the event a GM opened on their channel. The client
actor asks the directory whether one is open before it builds their menu, and
their "go" choice ends with `Effect::JoinGmEvent`. Pietro, Pietra and Harry
take players back out. See [Coconut Season](./coconut.md), "Opening an event
to players".

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
`03 01 i32` or `0D 01 str`. A non-empty default in a text or number
prompt comes selected, so the first key typed replaces it. The server
sends text prompts (`Page::AskText`, Lenario's alliance name) and number
prompts (`Page::AskNumber`, the refiners in `agent_docs/inventory.md`),
and checks the answer against the bounds it sent: a number out of range
closes the conversation. It sends no box text.

Types 7 and 8 are a choice of styles: after the text come `u8 count` and
that many `i32` style ids. The client shows the v83 `UtilDlgEx_Avatar`
window (`UIAvatarSelect`) instead of the usual dialog: the NPC, the text,
and the player wearing one style at a time, a face if the first id / 10000
is 2, a hairstyle if 3, otherwise a skin. BACK and NEXT go round the
styles, TAKE OFF ALL shows the player without equipment and DEFAULT
SETTINGS brings it back. OK or Enter answers `07 01 u8` (the style's index);
CANCEL, LEAVE STORE or Escape answers `07 00`. The server sends type 7 for
the stylists' looks and applies the chosen one with `Effect::SetStyle`
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
starting with `#` hangs v83 and gives no choices in HeavenClient.

On the server a script's `Page::Slide` becomes `Prompt::Slide`, with a
`#` taken out of any label. Only the Dimensional Mirror (9010022,
`npc_scripts::dimensional_mirror`, in twenty towns) sends one, with art
set 0. Its icons are numbered as in Cosmic, and it offers only the
places this server runs:

| Id | Place | Levels | Saved for the way back | Lands on |
|---|---|---|---|---|
| 1 | Mu Lung Dojo | 25 and up | `DOJO_RETURN` | 925020000, portal 0 |
| 2 | Monster Carnival | 30 to 50 | `CARNIVAL_RETURN` | 980000000, portal 3 |
| 5 | Nett's Pyramid | 40 and up | `PYRAMID_RETURN` | 926010000, portal 4 |

Each place's way out (`dojang_exit`, `mc_out`, the Pyramid lobby's
`out00`) goes back to the saved town. A player who may go nowhere sees
`#-1#` and the `mirror.nowhere` line. Cosmic's Ariant Coliseum (0), its
second Carnival (3) and its Construction Site (6) have no event here.

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
