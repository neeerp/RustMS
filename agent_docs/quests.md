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
   "(In Progress)". If the NPC has a script (`Npc/<id>.img/info/script`
   and SET_NPC_SCRIPTABLE's windows, the newest dated for today), an
   "ETC" line named by `Etc/ScriptInfo.img/<script>` (or the script
   itself) asks the server to talk. Party quests (1200-1399) are never listed. With nothing to list,
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
| `Check/<id>/0` (start) | `npc`, `job`, `lvmin`, `lvmax`, `quest`, `item`, `skill`, `pop`, `interval`, `dayByDay`, `startscript`, `equipAllNeed`, `equipSelectNeed`, `fieldEnter`, `start`, `end`, `pet`, `pettamenessmin`, `pettamenessmax`, `dayOfWeek`, and the alert trigger `normalAutoStart` |
| `Check/<id>/1` (complete) | `npc`, `item`, `mob`, `quest`, `endscript`, `endmeso`, `level`, `pet`, `pettamenessmin`, `pettamenessmax`, and `start`/`end`, which v83 doesn't check on completion (only 4306 and 4309 have them, set to 1 to 2 January 2000) |
| `Act/<id>/0` | `item`, `exp`, `money`, `buffItemID` |
| `Act/<id>/1` | `exp`, `nextQuest`, `item`, `money`, `pop`, `pettameness`, `skill`, `npcAct`, `buffItemID` |
| `Act/<id>/<n>/item/<k>` | `id`, `count`, `job`, `gender`, `prop`, `period`, `var` |
| `Act/<id>/1/skill/<k>` | `id`, `skillLevel`, `masterLevel`, `job`, `onlyMasterLevel` |

`Act` nodes may repeat speech under numbered keys and `yes`/`no`, and an
offer's questions (`ask`) with their answers (`stop`), as an old copy of
`Say.img` (Act/0 of 7300, the first of the Horntail quests, is in Korean);
that is not an action. They may also copy requirements from `Check`
(`npc`, `lvmin`, `lvmax`, `job`, `quest`, `interval`, `start`, `end`;
`INERT_ACTIONS`). v83's client reads none of these under `Act`, nor does
Cosmic, so only the `Check` copy counts: 3081 and 3082 have an `Act/0/lvmin`
but no level requirement, and 3075's `Act/0/npc` names its completion NPC. `item` requirements are counts the player must hold; a count of 0
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
(the four quests with one, such as 3600, date from 2005–2007). `end` is the
last hour it can be (v83 offers it from `start` to `end` inclusive). Nearly
every quest with an `end` is an event that ended in 2005–2010, so, as in the
client, neither side offers it (9625's one hour is on 31 December 2030). v83
lets a server move a quest's window with SET_QUEST_TIME (151), which RustMS
doesn't send. `dayOfWeek` lists the days (`sun` to `sat`, each "1") on which
the quest can be started: 8248 on Saturdays, 8249 and 8251 on Sundays, 8254
Monday to Friday. A day left out counts as 0, so a `dayOfWeek` with no day
set opens on none. v83 tests it against the PC's own weekday (GetLocalTime,
not the server's clock), so the client's guide follows the player's time
zone, while the server, which can't know it, keeps to UTC's weekday; near
midnight the two can differ, and the server's refusal wins.

`pet`, on either side, lists pet items (`pet/<n>/id`), and the pet in the
first summon slot must be one of them; another pet out doesn't count, as in
v83's checks (FUN_00721163, FUN_00721d2c). Without a pet out v83 doesn't
offer the quest, and at completion shows "lack of pet" (no pet) or "with the
current pet" (another pet). `pettamenessmin`/`pettamenessmax` (0 and 30000
when unset) bound the closeness of that first pet out; v83 reads them only
inside the `pet` check, and a pet outside the range counts as the wrong pet.
HeavenClient checks them too. Garnox's 8184 (a Baby Dragon
at 1642) and the pet medal quest 29507 (any pet at 1400) qualify through
them.

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
like v83, not in the quest log's Available tab. Without an interval,
`dayByDay` makes a quest repeatable once a day: from the day (UTC, the
server's clock) after its completion, not 24 hours later. Every quest with
it also has an `end` that has passed, so none repeats in practice.

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
(`game_data::quests::SCRIPTS`: `q1021s`, `q1021e`, Bartos's secret
`q4647e`, Garnox's pet evolutions `q8185e`, `q8189e` and `q4659e` (see
[Pets](./pets.md)), Meren's Maker lessons, Dyle's informants, Reef, Mu Young, the baby dragon and Captain Al's family lessons below); those steps go through
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
mode 4 in the chat log ("You have gained fame. (+n)"). `pettameness`
makes the first pet out that much closer on completion, as Cosmic does
(Fanzy's 2104 gives 15); with no pet out it is lost. The pet's item is
re-sent, so the client's pet says "The closeness increased (+n)", and a
pet that levels up shows it to its owner and the map. v83 lists it last in
the rewards, after fame, as QuestIcon 9 and the number. Skill actions
(`net::quests::apply_skills`) teach a skill to a player whose job is exactly
one in the entry's `job` list, as v83's reward list shows them (a 0 there is
the beginner job, not "any"). The level and master level become the entry's
where the player's are lower, as Cosmic does, and the changed skills follow
the completion in one UPDATE_SKILLS. `onlyMasterLevel` marks entries
without a `skillLevel`, which already raise only the master level (Bahamut,
6121–6124); `acquire` stays unsupported there. `npcAct` (always "quest") names an NPC animation that neither
side plays; it is ignored. A quest with
any other key (`infoex`,
`petRecallLimit`, ...) is **unsupported**: it is
never offered, and actions for it are refused. A prerequisite without a
`state` means "not started" (mutually exclusive quests).

The info quests (`game::quest::INFO_QUESTS`, below) and Meren's Maker
lessons in Magatia are the exceptions to the info keys. Meren's are
`game_data::quests::MEREN_INFO_QUESTS`: "Meren's Lessons" (6029, level
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

"Over the Castle Walls (4)" (2324, `OWN_RECORD_QUESTS`) needs its own record
to reach `infoex/0/value` (cond 1). The server keeps that count as the
record's one "kill" (`QuestTemplate::end_record`, `QuestRejection::Record`),
so it is saved and sent as its progress string ("001"). The Thorn Remover the
quest gives sets it (`Effect::QuestRecord`, see `inventory.md`, "Scripted
items"), and the Secretary of Magic (1300004) then completes the quest.
"Find Fresh Milk" (2180) uses the same one-"kill" record only to remember
the last cow milked; its completion needs just the full bottle. While it is started, Tangyoon (1092000) gives an empty bottle
(4031847) and sends the player to the stable (912000100). Each mother cow
(1092090 records 1, 1092091 records 2) fills the first bottle held by a
third (4031848, 4031849, 4031850), but not twice running; a calf (1092094,
1092095) drinks any milk in it, back to empty. The stable's way out
(`end_cow`) keeps a player with a bottle that isn't full, and leads to
Tangyoon's kitchen (120000103), where the full bottle completes the quest.
The cows' and Tangyoon's text is unsourced and stays placeholder.
Meren's crystal test (6033, level 75, a level 75 crystal shown) and the
Gold Anvil (6036, level 105, after the paper 6035; the anvil is taken)
carry no rewards in the data; their scripts (`q6033e`, `q6036e`) teach
levels 2 and 3 and give Cosmic's 230,000 and 300,000 EXP. 6034's `Act/0`
repeats its start requirements with a `skill` entry that teaches nothing,
so the server ignores it. The scripts' text is unsourced and stays
placeholder.

Dyle's informants in Kerning City (2214-2219, before JM's 2220). The Knocked
Trash Can (1052108) in the Hut in the Swamp (107000301) ends 2214 only from 5 to 8 p.m. on the
stations' wall clock (`conti::time_of_day`, as the ships keep it): its
script (`q2214e`) gives JM's list of informants (4031894) and Cosmic's
20,000 EXP; at other hours the can is empty. A player who lost the
list gets another there on 2215 (`q2215e`) for 2,000 mesos. Each informant
(Mr. Pickall 9000008, Shumi 1052102, Nella 1052103 and Jake 1052006 for
2216-2219) has a start script and no end: talking to them through
QUEST_ACTION 4 starts and completes their quest at once
(`Effect::FinishQuest`, saved by `db::quest::finish` with the start's
checks) and gives Cosmic's 7,000 EXP, and the last of the four takes the
list. `game_data::quests::FINISHED_BY_START_SCRIPT` lets these four through
without a completion NPC. The can's and the informants' lines are
unsourced and stay placeholder (`dyle.*`).

"Reef's Gratitude" (2228) works the same way: once Faust is beaten (2227),
Reef (1032108, in 910100001) thanks the player through a start script
(`q2228s`) that completes the quest at once and gives the data's 8 fame
(`Act/1 pop`, through `Effect::GainFame`, since `FinishQuest` gives no
`Act/1` rewards). Its line is a placeholder (`reef.thanks`).

Mu Young (1061014, in 105100100) has two start scripts, both silent as in
Cosmic. `q2238s` starts "Who is the Owner of the Mysterious Note?" (2238,
once 2237 is done). `q2245s` ("To Tristan's Tomb", for a player on 2238)
completes 2245 at once, so that its `interval 0` lets it be taken again,
and takes the player alone into the unknown Jr. Balrog's room
(`EventKind::Tristan`, see [party quests](./party-quests.md#jr-balrogs-room)).
Jr. Balrog's fall brings out Tristan's Spirit (1061015), where 2238 ends
for the data's 50,000 EXP; with it done, the `tristanEnter` portal opens.

"The Remnants of Horned Tail" (3714) works the same way. Nine Spirit's Baby
Dragon (2081011), in the nest past Leafre's `dragonNest` portal (240040612,
open once 3706 is done), has a start script (`q3714s`) and no end. It
takes a Nine Spirit's Egg (4001094) and gives a Dragon Stone (2041200) and
Cosmic's 42,000 EXP, completing the quest at once. The quest repeats
(`interval 0`), once for each egg, but not while a Dragon Stone is held,
as in Cosmic. Its lines are placeholders (`nine_spirit.baby.*`). 3706's
`Act/1/map` and `message` (and 2282-2284's `Act/0/message`) are read by
neither v83's client nor Cosmic, so they are ignored (`INERT_ACTIONS`).

Captain Al's lessons in leading a family (2232-2234, "Find a Junior!",
"Raise the Rep!" and "Enjoy the Entitlement!") start at Captain Al
(1002103, in Lith Harbor and Orbis Park) and have an end script
(`q2232e`-`q2234e`) and no end NPC, so they end at him too: `classify`
lets a quest whose `Check/1` names an end script through without a
completion NPC, and both sides default the end NPC to the start NPC. His
scripts read the Family window's numbers (`db::family::info`, see
[Families](./families.md)): 2232 needs a junior and gives Cosmic's 3,000
EXP; 2233 needs 1,000 Rep earned in all; 2234 needs 2,000 earned and less
than 500 left to spend. The data gives no rewards, and Cosmic has a
script for 2232 only. The lines are placeholders (`family.captain_al.*`).

The `game-data` asset test compares the server's result with the committed
list `game-data/supported_quests.txt`. HeavenClient doesn't classify quests:
the build copies that list next to the executable, and the client offers
only those quests (`Character/QuestCatalog.cpp`). Supporting a new key means
changing the server's allowlist and regenerating the list. 1401 quests qualify
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

## Timed quests

A quest whose `QuestInfo` has a `timeLimit` (3458, Jr. Officer Medin's wall plan, 2262,
3951, 9999, ...) gives that many seconds once started
(`QuestTemplate::time_limit`). The server keeps one clock per started timed
quest in the `ClientActor` (`runtime/src/actor/client/timed_quest.rs`) and
tells the client with `UPDATE_QUEST_INFO`:

- starting it sends mode 6 (`u16 1, u16 quest, i32 ms`) before the quest
  record;
- completing or forfeiting it sends mode 7 (`u16 1, u16 quest`) first;
- when the time is up the quest is forfeited: mode 7, the record back to 0,
  then mode 15 (`u16 quest`), on which the client writes "The [name] quest
  expired because the time limit ended" in the chat log.

The clock goes with the player to another channel: `change_channel` hands
the remaining time over in `WorldServices::quest_clocks`, keyed by the
session, before it sends the redirect, and only the login that carries that
session on picks it up. Mode 6 is not sent again, as the client keeps its
clocks. Logging out, disconnecting and going to the
Cash Shop forfeit the quest, as Cosmic does; a started timed quest found at
login without a handed-over clock is forfeited silently. Deviations: the
clock's length is shortened with `RUSTMS_EVENT_SCALE` like the events', and a
forfeit deletes the row (no forfeited count).

The client (`IO/UITypes/UITimedQuests`, logic in
`Character/TimedQuests.h`, CTest `timed_quests`) draws v83's widgets from
`UIWindow.img/Quest/TimeQuest`: a 52x12 bar at (700, 50) of the 800 px
screen, each later one 50 px under the last, whose 48-step gauge empties
from the left, and an alarm clock to its right whose ring frames spread over
the whole time. Only a `timerUI` of "default" plays `questCount` over the last
frame and `clockBoom` when taken down. Hovering over a bar shows the
quest's name over "Time Left h:m:s". Mode 16 resets a clock to its whole
time. The clocks stay up across map and channel changes, and a new login
clears them. The gauge measures against `timeLimit`, so with
`RUSTMS_EVENT_SCALE` the harness's clocks start nearly empty.

## Info strings

Some quests keep a string of their own instead of kill counts: `Act/0/info`
sets it on starting and `Check/1/infoex` says what it must read to complete
(`QuestTemplate::start_info`, `end_info`). v83's completion check passes
when any entry matches: cond 0 (or none) is string equality, cond 1 and 2
compare the numbers at the start of both strings (at least, at most), and
a quest without a string fails (`game_data::quests::info_met`,
`QuestRejection::Info`). HeavenClient doesn't check it, so its quest marker
may call such a quest completable early; the server just refuses.

Only the quests whose strings the server's NPCs can set are supported
(`game::quest::INFO_QUESTS`), because no other could ever complete. A
string kept in another record (`infoNumber`) is supported only where a
script sets that record (`game_data::quests::RECORD_QUESTS`, read through
`QuestTemplate::end_info_record`). Such records are no quest of their own:
`game::quest::RECORDS` lists them, `QuestLog::set_record` sets them whole,
and they are saved as started rows like any other info string; forfeiting
the quest deletes them with it (`db::quest::forfeit`). They are `QuestStatus::Started { info: Some(..) }`, stored in
`character_quests.progress` as is and sent wherever kill counts would be
(`QuestStatus::record_string`). `Effect::SetQuestChar` changes one
character if it reads what it should, sending the quest record again;
otherwise nothing in its script applies.

| Quest | String | Set by |
| --- | --- | --- |
| 3926 Screwing the Red Scorpions | "2222" to "3333" | four cupboards in Ariant's houses (2103009-2103012), each taking one Jewel (4031579) |
| 3929 Sejan's Test | "2222" to "3333" | four houses (2103003-2103006), each taking one parcel of food (4031580) |
| 3900 Learning the Culture of Ariant | "4" to "5" | the Palace Oasis (2103000) |
| 3927 Existence of the Secret Organization | "0" to "1" | the secret wall (2103001) |
| 3311 Clue | "4" to "5" | the wall in the missing alchemist's home (2111011), after a yes |

Cosmic's scripts gave the slots and digits. The houses are silent, as
there; the walls' and the oasis's words are placeholders (`quest_info.*`).
Not supported: 3114 (a seven-NPC puzzle), 3345 (a chain of four NPCs),
2236 and the other quests reading another record that no script sets.

| Quest | Record | Set by |
| --- | --- | --- |
| 6400 Jonathan's Court | 6401 to "q3" | hitting the real Bart (reactor 1209000) in Lord Jonathan's test room (912020000), with a pink line |
| 6330 How to Improve Your Strength (Marauder) | 6331 to "2" | Kyrin (1090000) past the test room 912010000 |
| 6370 How to Improve Your Strength (Outlaw) | 6371 to "2" | Kyrin past the test room 912010100 |
| 6363 A New Revelation | 6364 to "2" | the past Papulatus (2043000) sending the Buccaneer back from 922020300 |
| 6410 Mission Free the Monsters | 6411 to "p2" | the way out of Save Delli! (925010300) once Willi is saved |
| 29932 Protector of Pharaoh | 7760, a count | each of the player's own kills and cool kills in Nett's Pyramid; Duarte (2103013) gives medal 1142142 at 50000 |

Lord Jonathan (1092019, in the Navigation Room 120000102) asks a Corsair on
6400 without the record whether to take his test, then sends them into the
room if nobody is in it (`scripts::SOLO_MAPS`) for two minutes
(`npc_scripts::time_limit`), after which the map's `forcedReturn` takes them
back. Bart and his nine look-alikes (1200000, which do nothing) break in one
hit and are shuffled on each fill. The room's way out names a spawn portal
the Navigation Room lacks, so `reactors::lost_link` sends it to the map's
default spawn instead of closing it. Cosmic asks a riddle before the test;
its text has no source, so it is left out. His lines are placeholders
(`jonathan.*`).

Kyrin, in her room by the Nautilus (120000101), asks a Pirate on 6330 or
6370 without its record whether to take her test, and sends them into its
room if nobody is in it (`scripts::KYRIN_TESTS`) for four minutes. The room's
way out (`s4super_out`, `s4ship_out`) holds them for the first minute with a
pink line, takes them back to her room before two, and after two leads past
the room (912010200), where Kyrin sets the record (`Effect::SetRecord`) and
sends them back to complete the quest with her. The times are Cosmic's and,
as there, run at the event pace (`RUSTMS_EVENT_SCALE`), measured from
entering the room (`MapTimer::started`). Her lines are placeholders
(`kyrin.test.*`).

"Junior Balrog in a Different World" (6108) counts 300 kills of the
Junior Balrog (9300085) in the other world (910500000). The Insignificant
Being (1061012, at the Sleepywood Dungeon's Another Entrance 105090200)
takes a Bowmaster or Marksman (or GM) of level 120 on 6107 or 6108 straight
in, one at a time (`scripts::SOLO_MAPS`), for twenty minutes, after which
the map's `forcedReturn` brings them back; its way out does too. Cosmic lets
in the leader's whole party standing by; here each Bowman goes alone, and
needs no party. Its lines are placeholders (`balrog_world.*`).

The Buccaneers' Time Leap quests (6360-6363) go into the past: with
Papulatus's Key (4031870) on 6361, and until 6363 is done, the way to
Papulatus leads to the past Origin of the Clocktower (922020300; see
`papulatus.md`). There the past Papulatus takes the key (6361), and gives
6362 and 6363; talking to him sends the Buccaneer back to Deep Inside the
Clocktower (220080000, as Cosmic), and with 6363 started sets its record
too, so Rolonay (1092012) completes it with Time Leap. No source says where
v83 set the record; his line is a placeholder (`papulatus.past.send_back`).

The Corsairs' "Mission Free the Monsters" (6410) is Delli's rescue, a
party event (see `party-quests.md`): won, the way out of Save Delli! sets
6411 to "p2" for a player on the quest and leads to Delli (2095000), who
completes it there (925010400).

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
  v83's order (NPC, items, quests, info, kills; `completion_stop` in
  `QuestMarker.h`). The info check (`end_info_met`) reads the started
  record `infoNumber` names and needs one `infoex` entry to hold (cond 0
  equal, 1 at least, 2 at most, as numbers), so an explorer medal short of
  its places or 2324 before the Thorn Remover shows `stop/info`; entries
  without an `infoNumber` (6029) are left to the server. A failure shows
  `stop/<npc|item|quest|info|mob>`, else
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
A quest's `endmeso` is its last row, "Meso", with the amount needed in
red or green. Starting a quest with kills or items to bring
tracks it unless AUTO was switched off; the quest window's helper button
tracks and untracks open quests. Each quest's ⓧ untracks it and puts
v83's grey line in the chat log: "[name] It has been excluded from the auto
alarm and it will not be automatically reigstered until you re log-on"
(0x1047, FUN_00887ee8). The minimize
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
  (`quest_dialogue`, `quest_catalog`, `quest_markers`, `timed_quests`).
- Integration: `maker_quests` (Meren's lessons to the Maker's level 2),
  `scripted_npcs` (Roger's Apple end to end, Robin, the
  Shanks refusing a new beginner, natural recovery limits), `rain_quizzes` (all seven quizzes, rewards once, prerequisites,
  wrong NPC, NPC not on the map, forfeit, relogin, channel change, isolation)
  `beginner_contact_quests` (the route through Sen's house and `nextQuest`
  results), and `quest_objectives` (1034's candy errand, Todd's hunt and
  random hat, Sam's snails persisting across relog and capped at ten, Maria's
  letter to Lucas, refusals before requirements are met), and
  `quest_reward_choice` (Hongbu's shoes, refusing a choice not offered),
  and `quest_info` (Sejan's Test through Ariant's four houses, kept across
  a relogin, and a string one delivery short refused),
  and `timed_quests` (Medin's clock: shown on start, taken down on
  forfeit, running out, kept through a channel change, given up on logout).
