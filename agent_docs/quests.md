# Quests

## The v83 flow

Quests run the way the original client ran them:

1. **HeavenClient** reads `Quest.nx` itself. Clicking an NPC lists its quests
   in the NPC chat window under the v83 `UtilDlgEx/list*` headers ("QUEST
   THAT CAN BE COMPLETED", "QUEST IN PROGRESS", "QUEST AVAILABLE"), with
   "ETC: I have something to tell you." for the NPC's normal speech, as in
   period footage. Picking a quest plays its `Say.img` pages locally
   (`Gameplay/QuestConversation.cpp`); an offer's yes/no page shows ACCEPT
   and DECLINE.
2. When the player accepts or finishes, the client sends `QUEST_ACTION`
   (`0x6B`): action byte (1 start, 2 complete, 3 forfeit, 4/5 scripted),
   quest id, and the NPC, then its position.
3. **The server** checks the request against the quest's `Check.img`
   requirements and the player (`net::quests::check_start`/`check_complete`),
   asks the field whether that NPC is on the map within reach (1200×800
   around the player), and commits the change and its `Act.img` rewards
   atomically (`db::quest::start`/`complete`/`forfeit`).
4. It answers with the quest record (`SHOW_STATUS_INFO` mode 1), EXP and
   mesos with their stats if any (both shown in the chat log, as mode 3 with
   its in-chat flag and mode 5), and `UPDATE_QUEST_INFO` mode 8 (quest, NPC, next quest). A
   nonzero next quest (`Act/1/nextQuest`) makes the client continue straight
   into that quest's offer once the current conversation ends.

The server sends no quest dialogue and keeps no quest conversation state.
Rejected actions (unsupported quest, wrong NPC or state, unmet job, level, or
prerequisite, NPC not nearby, replays) are logged at debug level and ignored.

## Supported quests

Only requirements and actions the server implements are allowed
(`game-data/src/quests.rs`):

| Node | Allowed keys |
| --- | --- |
| `Check/<id>/0` (start) | `npc`, `job`, `lvmin`, `lvmax`, `quest`, `startscript` |
| `Check/<id>/1` (complete) | `npc`, `item`, `mob`, `quest`, `endscript` |
| `Act/<id>/0` | `item` |
| `Act/<id>/1` | `exp`, `nextQuest`, `item`, `money` |
| `Act/<id>/<n>/item/<k>` | `id`, `count`, `job`, `gender`, `prop` (not -1) |

`Act` nodes may repeat speech under numbered keys and `yes`/`no`; that is not
an action. `item` requirements are counts the player must hold; a count of 0
means none may be left (Roger's apples must all be eaten). `mob` requirements
are kill counts (below). `quest` requirements, on either side, name other
quests that must be in a given state: Chief's Introduction (1040) completes
only once Mai's, Sam's, Rain's and Yoona's quests (1041–1044) are done.
A quest without a `Check/1` `npc` is completed at
its start NPC if `Check/1` asks for items or kills (Pio's 1008); one with
neither is unsupported, which keeps out the event records that only a
server script completes.

Item actions (`net::quests::pick_items`/`apply_items`) give positive counts
and take negative ones (capped at what the player holds). An entry applies
only to its `job` families (v83 bits: 0x1 beginner, 0x2 warrior, 0x4
magician, 0x8 bowman, 0x10 thief, 0x20 pirate, 0x400+ Cygnus) and `gender`
(2 or absent: either). Entries without `prop` always apply; among those with
a positive `prop`, one is picked at random by weight (Todd's beginner hats).
`prop` -1 is a reward the player chooses, which needs UI and stays
unsupported, as do `period`, `dateExpire`, `var`, and `name`. If the gifts
don't fit, nothing changes and the client gets "You can't get anymore
items." `startscript`/
`endscript` are allowed only for scripts the server implements
(`game_data::quests::SCRIPTS`: `q1021s`, `q1021e`); those steps go through
QUEST_ACTION 4/5 and the server drives the dialogue (see
[NPC dialogue](./npc-dialogue.md)). `money` pays (or, if negative, takes)
mesos on completion; a quest that takes mesos needs them held. A quest with
any other key (`pop`, `skill`, `infoex`,
`pet`, ...) is **unsupported**: it is
never offered, and actions for it are refused. A prerequisite without a
`state` means "not started" (mutually exclusive quests).

The `game-data` asset test compares the server's result with the committed
list `game-data/supported_quests.txt`. HeavenClient doesn't classify quests:
the build copies that list next to the executable, and the client offers
only those quests (`Character/QuestCatalog.cpp`). Supporting a new key means
changing the server's allowlist and regenerating the list. 960 quests qualify
with the current rules, including the beginner ones:

| Quest | Start → Finish | EXP |
| --- | --- | --- |
| 1009–1015 Rain's Maple Quiz | Rain (12101) | 2, 4, 6, 10, 10, 10, 20 |
| 1031 Heena and Sera | Heena (2101) → Sera (2100) | 5 |
| 1032 Nina's Brother Sen | Nina (2102) → Sen (2001) | 7, next 1033 |
| 1033 What Sen Wants | Sen (2001) → Nina (2102) | 7, next 1034 |
| 1034 Tasty Mushroom Candy | Nina (2102) → Sen (2001) | 20; gives 10 candies for Sen, then 2 edible ones (2022253) for beginners |
| 1035 Todd's Hunting Method | Todd (2004) → Peter (2002) | 30; one Jr. Sentinel and its shellpiece for a random beginner hat |
| 1036 Robin the Walking Encyclopedia | Robin (2003) | 40 |
| 1037 Help Hunt the Snails | Sam (2005) → Maria (2103) | 60 for 10 snails, next 1038 |
| 1038 Maria's Letter | Maria (2103) → Lucas (12000) | 40; the letter for 5 Red and 5 Blue Potions |
| 1040 Chief's Introduction | Lucas (12000) | 500 and Lucas's letter, after 1041–1044; opens those four |
| 1021 Roger's Apple (scripted) | Roger (2000) | 10, 3 Apples, 3 Green Apples |
| 1008 Pio's Collecting Recycled Goods | Pio (10000) | 100 and 200 mesos for the two materials from Amherst's boxes |

## Kill counts

`character_quests.progress` holds a started quest's v83 progress string:
three digits per `Check/1/mob` entry, in order ("003012" for 3 and 12).
Starting a hunting quest sets every count to 0; each kill a player takes part
in (`ServerMessage::MobKilled`) adds one for every started quest that needs
that mob, up to the required count (`net::quests::count_kill`), and sends the
quest record again with the new string. Counts live in the quest's
`QuestStatus::Started { kills }` and are saved with the player. Completion
requires every count.

Completing a quest also plays the v83 "Quest Clear" effect
(`effect::build_own_effect` and `effect::build_foreign_effect` with
`CharEffect::QuestClear`).

## Conversations in the client

`Character/QuestDialogue.h` is a pure state machine over one side of
`Say.img/<id>`:

- **Offer** (`/0`): numbered pages, the last one asking yes/no; `yes` pages
  follow acceptance and `no` pages a decline. Accepting sends start. An offer
  ending in a question starts when answered correctly.
- **Completion** (`/1`): numbered pages, then `yes` pages. Quiz pages contain
  `#L` choices and `stop/<page>/answer` (one-based); a wrong choice shows
  `stop/<page>/<choice>` and asks again. Passing the last question, or leaving
  the last page of a talk-only completion, sends complete. There is no going
  back to an answered question.
- **Reminder**: `stop/npc` pages when the start NPC is clicked while the
  quest is open; at the finishing NPC, `stop/mob` while kills are missing and
  `stop/item` while items are.
- No pages at all: the action is sent immediately (Sen's completion of 1032).

Markers use the same catalog (`QuestLog::supported_rules`): bulb for an
available quest, open book while in progress at the start NPC, closed book at
the finishing NPC once its kills and items are in (open book until then).
The marker is the v83 `UIWindow.img/QuestIcon` bubble. As in footage, it
sits right of the NPC's centre with its tail resting on the head, and it
holds still while the bulb inside glows.

The **Quest Helper** (`IO/UITypes/UIQuestHelper.cpp`) is the v83
`UIWindow.img/QuestAlarm` from `UIClassic.nx`, laid out from period footage.
It sits flush with the top-right corner, below any party HP bars, even with
nothing tracked (then it shows only its title bar), until it is dragged by its top 25 pixels (not its buttons),
as the original's window base allows. From then on it stays where it was
put, which is saved as `PosQUESTHELPER`. It
lists up to five tracked quests with what each still needs ("Snail 3/10").
As in the original's font choice (`FUN_00704c2c`), the count you have is red
under a third of the way, pink under two thirds, orange until done, then
green; the rest of the line is black. As in the original's layout
(`FUN_007052e6`), a blank 18-pixel row separates one quest from the next.
The original also lists a quest's `endmeso` as a "Meso" row, but no
supported quest has one. Starting a quest with kills or items to bring
tracks it unless AUTO was switched off; the quest window's helper button
tracks and untracks open quests. Each quest's ⓧ untracks it, the minimize
button folds the helper to its title bar, and close hides it until the
quest window tracks a quest again. Footage doesn't show what AUTO and close
did, so those two are best guesses.

The **Quest Complete! notice** (`IO/UITypes/UIQuestComplete.cpp`) is the red
`UIWindow.img/FadeYesNo/backgrnd4` box with the envelope `icon7`, above the
status bar (FRS4wNJWWIE, 2:00). Each update it reads every started quest's
objectives (kills and items), and `QuestCompletion` reports those whose last
objective was just met. A quest is judged from when it is first seen, so one
already met when started or on login doesn't get a notice. O opens the quest
on the quest log's In Progress tab (`UIQuestLog::show_quest`) and X dismisses
it. As in the v83 client (type 7 of its fading Yes/No windows, `FUN_00523408`),
it plays UI.img/Invite and fades in over a second. It fades out over a second
once a button is pressed or 30 s after it appeared, whichever comes first.
`IO/Components/FadeNotice.h` keeps that timing. Each newer notice sits 5 px
above the last, over it, and when one goes, those above it move down 5 px.
Handing in or forfeiting the quest also takes its notice away.

## Persistence and rewards

`character_quests` is keyed by `(character_id, quest_id)`: no row (not
started), status 1 (started), status 2 (completed, with a timestamp).
Forfeiting deletes a started row. A start or completion writes the quest row
and the whole player snapshot (EXP, items, HP) in one transaction that locks
the character row first: rewards are applied to a copy of the player, and the
actor adopts it only after the commit, so a replay cannot pay twice.
Prerequisites are rechecked inside the transaction.

EXP uses `game::exp` (HeavenClient's table) through `Player::gain_exp`
(+5 AP per level). HP/MP growth and SP belong with combat. If the level
changed, the client actor refreshes its field avatar.

## Assets

`Quest.nx` lives in the ignored `assets/game-data/` with `Map.nx`, `Npc.nx`,
and `String.nx`; `RUSTMS_QUEST_NX_PATH` overrides the server's path. The
client reads its own `Quest.nx` (a symlink to the same file). No quest text is
copied into the source.

## Validation

- Unit: `cargo test -p game-data -p net -p runtime -p game --lib`
  (classification, golden list, eligibility, packet parsing and building,
  NPC reach). Set `RUSTMS_REQUIRE_ASSETS=1` so asset tests cannot skip.
- Client: build with `-DHEAVENCLIENT_BUILD_TESTS=ON` and run CTest
  (`quest_dialogue`, `quest_catalog`, `quest_markers`).
- Integration: `scripted_npcs` (Roger's Apple end to end, Robin, the
  Shanks refusing a new beginner, natural recovery limits), `rain_quizzes` (all seven quizzes, rewards once, prerequisites,
  wrong NPC, NPC not on the map, forfeit, relogin, channel change, isolation)
  `beginner_contact_quests` (the route through Sen's house and `nextQuest`
  results), and `quest_objectives` (1034's candy errand, Todd's hunt and
  random hat, Sam's snails persisting across relog and capped at ten, Maria's
  letter to Lucas, refusals before requirements are met).
