# Quests

## The v83 flow

Quests run the way the original client ran them:

1. **HeavenClient** reads `Quest.nx` itself. Clicking an NPC that has quests
   lists them in the NPC chat window under its `String/Npc.img` `d0`
   greeting, grouped as v83 groups them (`select_quest_menu`):
   "QUEST THAT CAN BE COMPLETED" (started, and `Check.img/<id>/1` passes
   with this NPC), "QUEST AVAILABLE", then "QUEST IN PROGRESS" (its other
   started quests, such as a hand-in still missing its items), under the
   `UtilDlgEx/list*` headers. Each line is v83's `quest_menu_line`: the
   QuestInfo `type` in red, the name, and "(Pre-completion enabled)" or
   "(In Progress)". If the NPC has a script (`Npc/<id>.img/info/script`,
   the last entry dated for today), an "ETC" line named by
   `Etc/ScriptInfo.img/<script>` (or the script itself) asks the server to
   talk. Party quests (1200-1399) are never listed. With nothing to list,
   an NPC with a script goes straight to the server; one without says `d1`
   if all its quests are done for good, and `d0` otherwise, without the
   server. NPCs without quests always go to the server. Picking a quest
   plays its `Say.img` pages locally
   (`Gameplay/QuestConversation.cpp`); an offer's yes/no page shows ACCEPT
   and DECLINE.
2. When the player accepts or finishes, the client sends `QUEST_ACTION`
   (`0x6B`): action byte (1 start, 2 complete, 3 forfeit, 4/5 scripted),
   quest id, and the NPC, then its position (left out for the quest
   alert's autoStart and autoComplete quests, below). A completion ends
   with the picked reward's index, -1 without one (`FUN_00716fe1`); the
   server reads it from the last four bytes.
3. **The server** checks the request against the quest's `Check.img`
   requirements and the player (`net::quests::check_start`/`check_complete`),
   asks the field whether that NPC is on the map within reach (1200×800
   around the player; not when starting a QuestInfo `autoStart` quest or
   completing an `autoComplete` one), and commits the change and its `Act.img` rewards
   atomically (`db::quest::start`/`complete`/`forfeit`).
4. It answers with the quest record (`SHOW_STATUS_INFO` mode 1), EXP and
   mesos with their stats if any (both shown in the chat log, as mode 3 with
   its in-chat flag and mode 5), and `UPDATE_QUEST_INFO` mode 8 (quest, NPC, next quest).
   Mode 8 is what makes the client show the step's `yes` pages; a nonzero
   next quest (`Act/1/nextQuest`) opens once they have been read to the end
   (straight away when there are none).

The server sends no quest dialogue and keeps no quest conversation state.
Rejected actions (unsupported quest, wrong NPC or state, unmet job, level, or
prerequisite, NPC not nearby, replays) are logged at debug level and ignored;
only missing mesos and a full inventory get an answer (below).

## The quest alert

QuestInfo `autoStart` quests are offered by a bulb over the player's face
instead of an NPC (`Gameplay/QuestAlert.cpp`, rules in
`Gameplay/QuestAlertRules.h`; v83ref report 201-quest-alert). The client
checks the start requirements of those with `Check/0/normalAutoStart` on
entering a field (silently) and on every stat change or quest record (with
the alert), and of those whose `Check/0/fieldEnter` lists the map on entering
it (with the alert). Started `autoPreComplete` quests and `autoComplete`
quests that `Check/1` passes with their end NPC (checked every 10 s) light
the bulb too. A new one shows the bulb's Appear animation, plays
`Sound/Game.img/QuestAlert`, and adds a grey chat line.

Clicking the bulb opens Maple Administrator (9010000) listing them under the
NPC menu's headers. Picking one opens its conversation as its start NPC
would, wherever the player is, so the server starts `autoStart` quests and
completes `autoComplete` ones without the reach check. An `autoAccept` quest opens by itself and
starts without its offer pages. `normalAutoStart` only says when to check;
`fieldEnter` is also a start requirement (below).

## Supported quests

Only requirements and actions the server implements are allowed
(`game-data/src/quests.rs`):

| Node | Allowed keys |
| --- | --- |
| `Check/<id>/0` (start) | `npc`, `job`, `lvmin`, `lvmax`, `quest`, `item`, `skill`, `pop`, `interval`, `startscript`, `equipAllNeed`, `equipSelectNeed`, `fieldEnter`, `start`, `pet`, and the alert trigger `normalAutoStart` |
| `Check/<id>/1` (complete) | `npc`, `item`, `mob`, `quest`, `endscript`, `endmeso`, `level`, `pet` |
| `Act/<id>/0` | `item`, `exp`, `money`, `buffItemID` |
| `Act/<id>/1` | `exp`, `nextQuest`, `item`, `money`, `skill`, `npcAct`, `buffItemID` |
| `Act/<id>/<n>/item/<k>` | `id`, `count`, `job`, `gender`, `prop`, `period`, `var` |
| `Act/<id>/1/skill/<k>` | `id`, `skillLevel`, `masterLevel`, `job`, `onlyMasterLevel` |

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

Start requirements follow v83's own check (FUN_00721163). `item` counts what
the bag holds plus one if a copy is worn: a positive count is a minimum, 0
means none, and a negative count is a maximum. A
`skill` entry with `acquire` 1 needs the skill, one without needs its
absence. "Having" a skill is v83's loose test: any skill not marked
`invisible` counts, for every job and at every level, and a hidden one
counts once it has a record, even at level 0 (that is how the 4th-job skill
book quests, 6110–6325, ask for the book's skill to be unlearned). `pop` is the least fame,
compared unsigned as v83 does, so negative fame passes. Every item listed
under `equipAllNeed` must be worn (the Magatia cape, 1102135, for 3307–3315),
and at least one under `equipSelectNeed`; a copy in the bag doesn't count.
With `fieldEnter`, the player must be on one of the listed maps. `start`
(yyyymmddhh) is the hour before which the quest can't be started; v83
compares it with its clock set to the server's, which for RustMS is UTC
(the four quests with one, such as 3600, date from 2005–2007).

`pet`, on either side, lists pet items (`pet/<n>/id`), and the pet in the
first summon slot must be one of them; another pet out doesn't count, as in
v83's checks (FUN_00721163, FUN_00721d2c). Without a pet out v83 doesn't
offer the quest, and at completion shows "lack of pet" (no pet) or "with the
current pet" (another pet). v83 also reads `pettamenessmin`/`pettamenessmax`
(closeness), which stay unsupported, so the quests with a `pet` list that
qualify (2103 Nella's Dream and Bartos's 4646/4647) accept any closeness.

To complete, `endmeso` is the mesos to hold (it always matches the `money`
taken). v83 checks it after kills and shows `stop/default` (or the NPC's
line) when it fails, since the stop page has no key for it. `level` is the
least level to complete (200 for 29300–29304); the client doesn't check it,
so only the server refuses.

`interval` makes a quest repeatable: once completed, it can be started again
that many minutes after its completion time (0 means at once); without it a
completed quest stays done. Starting it again replaces its completed record,
as in Cosmic, and completing it again sets a new completion time. v83 keeps
both records instead, so HeavenClient counts a quest it has both as started.
The client shows a due repeatable with the NPC's bulb and in its menu, but,
like v83, not in the quest log's Available tab. `dayByDay` (once per calendar
day) is a separate key and stays unsupported.

Item actions (`net::quests::pick_items`/`apply_items`) give positive counts
and take negative ones (capped at what the player holds). An entry without a
count takes all the player holds: the data leaves it out only on the quest
items a completion needs (32 entries, such as Kyrin's test crystals in 2191
and 2192), and v83's reward list skips them. Cosmic gives one instead; taking
them all is our reading of the original. An entry applies
only to its `job` families (v83 bits: 0x1 beginner, 0x2 warrior, 0x4
magician, 0x8 bowman, 0x10 thief, 0x20 pirate, 0x400+ Cygnus) and `gender`
(2 or absent: either). Entries without `prop` always apply; among those with
a positive `prop`, one is picked at random by weight (Todd's beginner hats).
With `prop` -1 the player picks one: the choices are those for the job and
gender, counted in numeric entry order (the file keeps "10" before "2", so
both sides sort), and a completion naming none of them is refused, as in
Hongbu's shoes (3608). `period` lends the item for that many minutes: it
comes dated (v83's client ignores the key and shows the date the item
carries) and on its own, as dated items never stack, and goes when the time
is up (see [Inventory](./inventory.md#expiry)); Rupi's Lost Paper (3085) is
one. `var` is allowed and ignored: neither v83's client nor Cosmic reads
it, and the item comes with its base stats. `dateExpire` and `name` stay
unsupported. If the gifts don't fit, nothing changes and the client gets
UPDATE_QUEST_INFO mode 10 with the quest id; it names the full inventories
itself ("Etc item inventory is full."). `startscript`/
`endscript` are allowed only for scripts the server implements
(`game_data::quests::SCRIPTS`: `q1021s`, `q1021e`, and Meren's Maker
lessons below); those steps go through
QUEST_ACTION 4/5 and the server drives the dialogue (see
[NPC dialogue](./npc-dialogue.md)). `money` pays (or, if negative, takes)
mesos on completion; a quest that takes mesos needs them held, or the
client gets mode 11 ("You do not have enough mesos."). On the start step,
`money` and `exp` do the same when the quest starts: v83's client doesn't
check the fee or show either, so only the server refuses (Nella's 2029
takes 1,000 mesos, where Cosmic takes what the player has). `buffItemID`
gives the named item's effect, as if the player used it (the dragon
potion's transformation for 7301), when its step is done. `pop` gives (or
takes) fame on completion, with a fame stat update and SHOW_STATUS_INFO
mode 4 in the chat log ("You have gained fame. (+n)"). Skill actions
(`net::quests::apply_skills`) teach a skill to a player whose job is exactly
one in the entry's `job` list, as v83's reward list shows them (a 0 there is
the beginner job, not "any"). The level and master level become the entry's
where the player's are lower, as Cosmic does, and the changed skills follow
the completion in one UPDATE_SKILLS. `onlyMasterLevel` marks entries
without a `skillLevel`, which already raise only the master level (Bahamut,
6121–6124); `acquire` stays unsupported there. `npcAct` (always "quest") names an NPC animation that neither
side plays; it is ignored. A quest with
any other key (`infoex`,
`pettamenessmin`, ...) is **unsupported**: it is
never offered, and actions for it are refused. A prerequisite without a
`state` means "not started" (mutually exclusive quests).

Meren's Maker lessons in Magatia are the one exception to the info keys
(`game_data::quests::MEREN_INFO_QUESTS`). "Meren's Lessons" (6029, level
45) and its three classes (6030 Carson's alchemy, 6031 Hughes's science,
6032 Meren's practice, each 10,000 mesos to start) share 6029's quest-info
string, one digit per class (`info`, `infoNumber`, `infoex`). Each class
needs 6029 started and 6029 needs all three completed, so the server keeps
no string and reads past those keys; the classes, which `interval 0` would
make repeatable, stay done once completed, as their digit keeps them in
v83. Only a forfeited and restarted 6029 differs: its classes stay done.
6029's completion teaches the Maker at level 1 (master level 3) through its
skill action. Its Evan entry (20011007) has no `job` list, so it teaches
no one; v83 has no Evan skills, and the crystal test teaches level 2 anyway.
The class end scripts (`q6030e`-`q6032e`) complete the class.
Meren's crystal test (6033, level 75, a level 75 crystal shown) and the
Gold Anvil (6036, level 105, after the paper 6035; the anvil is taken)
carry no rewards in the data; their scripts (`q6033e`, `q6036e`) teach
levels 2 and 3 and give Cosmic's 230,000 and 300,000 EXP. 6034's `Act/0`
repeats its start requirements with a `skill` entry that teaches nothing,
so the server ignores it. The scripts' text is unsourced and stays
placeholder.

The `game-data` asset test compares the server's result with the committed
list `game-data/supported_quests.txt`. HeavenClient doesn't classify quests:
the build copies that list next to the executable, and the client offers
only those quests (`Character/QuestCatalog.cpp`). Supporting a new key means
changing the server's allowlist and regenerating the list. 1390 quests qualify
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

## Exploration medals

The explorer titles (`game_data::quests::EXPLORER_QUESTS`: 29004-29011 and
29014) count places rather than monsters. Their `Check/1` names an info record
(`infoNumber`, e.g. 27010 for Beginner Explorer) and the number of places
(`infoex/0/value`), read into `QuestTemplate::explored`; completion needs at
least that many (`QuestRejection::Explored`), and Dalair (9000040) gives the
medal through the usual `QUEST_ACTION`.

`net::medals::region` picks the medal for a map by id range, as
`explorationPoint` does; the `highposition` touch portals count toward
Touched the Sky (29004). A map counts once, from the medal's `lvmin`, until
the quest is completed (`medals::explore`, `Effect::Explore`). The first
place starts the quest. Each new place sends the info record with the count
(SHOW_STATUS_INFO mode 1), a progress line (SCRIPT_PROGRESS_MESSAGE, 0x7A)
and an info text; the place that reaches the count sends QUEST_CLEAR (0x31),
the title line and the "see Dalair" text instead. The texts are the
`medal.explorer.*` keys (placeholders until the content pack has them).
HeavenClient shows 0x7A as a gold line near the top of the screen
(`UIScriptMessage`) and 0x31 as a Quest Complete! card (`UIQuestComplete`),
and its quest log fills `#R27010#` in the quest text with the record's count.

Places seen are `QuestLog::explored`, keyed by the info record, and are stored
in `explored_maps (character_id, record_id, map_id)`. They outlast a forfeit:
the next region map, even one already seen, starts the medal again with its
old count. At login a started medal's info record is sent among the started
quests with its count, so the client's quest window shows it. The level gate,
the kept places and "at least the count" have no source either way (v83ref
`lanes/QUESTIONS.md`). Ossyria Explorer (29012) and
Maple Explorer (29013), which need other medals and an info record 27020 that
nothing sets, are not supported.

## Conversations in the client

`Character/QuestDialogue.h` is a pure state machine over one side of
`Say.img/<id>`, following v83's `FUN_00716fe1` (reports/182-11 and 182-11b in
the v83ref notes):

- **Offer** (`/0`): numbered pages with Prev/Next, the last one asking
  yes/no. Yes sends start and closes; No shows the `no` pages. No pages at
  all sends start at once.
- **Completion** (`/1`): before anything is said, `Check/1` is checked in
  v83's order (NPC, items, quests, kills; `completion_stop` in
  `QuestMarker.h`). A failure shows `stop/<npc|item|quest|mob>`, else
  `stop/default`, else the NPC's own `String/Npc.img/<npc>/d0`, and sends
  nothing. At the wrong NPC, `lost` replaces `stop/npc` when a quest item
  given at the start (`Act/0/item`) is no longer held or worn. Otherwise
  the pages play with Prev/Next; the last one has only OK, which sends
  complete (closing it sends nothing).
- **Questions** (`ask` 1): every page but the last is a selection without
  Prev. A choice with a `stop/<page>/<choice>` text is wrong: that text shows
  with OK and the conversation ends, so the player talks to the NPC again.
  Any other choice is right; `answer` means nothing to v83. With
  `stop/<page>/flip`, the text before `#L0#` is a page of its own first.
- **Rewards**: the last completion page gets v83's list (`QuestRewards`,
  `Character/QuestRewards.h`): "REWARD!!" with the fixed items for the
  player's job and gender, meso, EXP, and the "?" box when one random item
  is given, unless another quest follows. It sits between `#w` marks, which
  `UINpcTalk` shows at once while the page above types out. Rewards to
  choose from come first, as `#L<n>#` links under their own icon
  (`FUN_0071808a`), and make the page a menu: picking one completes the
  quest with its index.
- **Yes pages** come only with the server's mode 8, as above.

Markers use the same catalog (`QuestLog::supported_rules`) and v83's rules
(`FUN_006d1779`). An NPC's quests are those it starts or finishes, less party
quests (1200-1399). The closed book shows if a started quest can be finished
there (no finishing NPC or this one, kills and items in). Otherwise the bulb
shows if a quest can be started there, and otherwise the open book if a quest
is in progress. So an available quest outranks a started one.
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
The original also lists a quest's `endmeso` as a "Meso" row, which
HeavenClient doesn't show yet. Starting a quest with kills or items to bring
tracks it unless AUTO was switched off; the quest window's helper button
tracks and untracks open quests. Each quest's ⓧ untracks it, the minimize
button or the Quest Helper key (L) folds the helper to its title bar. The key
unfolds it again, and so does a change to what it shows. Close hides it until the
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
- Integration: `maker_quests` (Meren's lessons to the Maker's level 2),
  `scripted_npcs` (Roger's Apple end to end, Robin, the
  Shanks refusing a new beginner, natural recovery limits), `rain_quizzes` (all seven quizzes, rewards once, prerequisites,
  wrong NPC, NPC not on the map, forfeit, relogin, channel change, isolation)
  `beginner_contact_quests` (the route through Sen's house and `nextQuest`
  results), and `quest_objectives` (1034's candy errand, Todd's hunt and
  random hat, Sam's snails persisting across relog and capped at ten, Maria's
  letter to Lucas, refusals before requirements are met), and
  `quest_reward_choice` (Hongbu's shoes, refusing a choice not offered).
