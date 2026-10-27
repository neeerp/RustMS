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
   completing an `autoComplete` or `autoPreComplete` one), and commits the change and its `Act.img` rewards
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
completes `autoComplete` and `autoPreComplete` ones without the reach check. An `autoAccept` quest opens by itself and
starts without its offer pages. `normalAutoStart` only says when to check;
`fieldEnter` is also a start requirement (below).

## Supported quests

Only requirements and actions the server implements are allowed
(`game-data/src/quests.rs`):

| Node | Allowed keys |
| --- | --- |
| `Check/<id>/0` (start) | `npc`, `job`, `lvmin`, `lvmax`, `quest`, `item`, `skill`, `pop`, `interval`, `dayByDay`, `startscript`, `equipAllNeed`, `equipSelectNeed`, `fieldEnter`, `start`, `end`, `pet`, `pettamenessmin`, `pettamenessmax`, `petRecallLimit`, `petAutoSpeakingLimit`, `tamingmoblevelmin`, `tamingmoblevelmax`, `dayOfWeek`, `buff`, `exceptbuff`, and the alert trigger `normalAutoStart` |
| `Check/<id>/1` (complete) | `npc`, `item`, `mob`, `quest`, `endscript`, `endmeso`, `level`, `pet`, `pettamenessmin`, `pettamenessmax`, `petRecallLimit`, `petAutoSpeakingLimit`, `buff`, `exceptbuff`, `mbmin`, `mbmax`, `mbcard`, `questComplete`, and `start`/`end`, which v83 doesn't check on completion (only 4306 and 4309 have them, set to 1 to 2 January 2000) |
| `Act/<id>/0` | `item`, `exp`, `money`, `buffItemID` |
| `Act/<id>/1` | `exp`, `nextQuest`, `item`, `money`, `pop`, `pettameness`, `petskill`, `petspeed`, `skill`, `npcAct`, `buffItemID` |
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
server script completes. An `autoPreComplete` quest is the exception: the
quest alert completes it (Maria's Map Reading, 1030, whose `Check/1` is
empty, for 11 EXP).

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

`petRecallLimit` and `petAutoSpeakingLimit`, on either side, need the first
pet out to lack a pet skill: recall (bit 0x80 of the pet's skill) or auto
speaking (0x100). v83 tests them after the `pet` list and closeness, so
without a pet out the quest isn't offered and completion shows "lack of
pet", and a pet that already has the skill counts as the wrong pet.
HeavenClient checks them too. On completion `petskill` ORs its bits into
that pet's skill and `petspeed` 1 sets bit 1 of its attribute (the owner's
speed); v83 never reads either, so the server re-sends the pet's item.
Trainer Bartos's 4660 and 4661 (Robo pets, in Henesys's pet park) use the
limits and `petskill`; Kenta's 3083 and 3096 (Aquarium) use `petspeed`.

`tamingmoblevelmin` and `tamingmoblevelmax` (0 and 30 when unset) bound the
level of the player's mount for a start; v83 (FUN_00721163) compares them
with the level the last SET_TAMING_MOB_INFO gave, which the server sends on
entering a field with a mount worn. HeavenClient checks them too. Only
Kenta's Silver Mane (6006) has one, a least of 3.

`buff` and `exceptbuff`, on either side, name an item whose effect, as if
used, must be on the player or must not be. An item's effect runs under its
negated id, whether the item was used or a quest's `buffItemID` gave it, as
Kinoko tests it. v83 checks both last (FUN_00721163, FUN_00721d2c): it
doesn't offer a quest that fails them, and at completion it shows the stop
page's default. HeavenClient checks them too. Only Kerning's scents use
them (2282-2284, from Lala, 1052119, to Lana, 1052121, both in Kerning Square's lobby).
Each start gives its scent's slowing effect for 40 seconds, which
completion needs. The next scent can't start while the last scent's effect
is on.

`mbmin` and `mbmax` bound the different cards the Monster Book holds, and
each `mbcard` entry bounds how many of one card it holds (`min`, `max`;
either may be missing). v83 checks them on completion right after the
mesos (FUN_00721d2c), and a quest that fails them stops on the items page,
as HeavenClient does too. Dalair's Monster Expert (29512) wants 30 kinds
of card, and the Spirit titles from The Rememberer (1061011; 29016-29019) want one of each
of five cards on top of their kills.

`questComplete` is the fewest quests the player must have completed to
complete this one. v83's client doesn't read it, so only the server checks
it, counting the completed quests as Cosmic does; a refusal sends nothing.
Dalair's Quest Specialist (29001) wants 800.

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
An Aran's "Scadur's Advice" (21610) and "The Wolves' Test" (21613) end on
records 21619 and 21620 reading "0" (`RECORD_QUESTS`), which reaching the
end of the Field of Wolves and leaving the wolves' test room with every wolf
down set (`map-travel.md`). Nothing in the sources sets either; Cosmic's
data asks for a token (4001193) instead. Cosmic has no way back into the
test room once its clock runs out; here the pack's wolf (1202008) offers
the test again while 21613 is started and unpassed. Maha's end script
(`q21618e`) swaps the wolf (1902017) for its grown form (1902018) only when
it is in the Equip inventory, not ridden.
"The Test" (2312) has a start check of the same kind: its `Check.img/0`
`infoNumber` names record 2311, which must be "1" (cond 0 is string equality,
1 at least, 2 at most, as in the end check; v83 FUN_00721163). Such pairs are
in `START_RECORD_QUESTS` (`StartRequirements::record`). The Mushroom Kingdom's
opening scene sets that record (`TD_MC_Openning`, see `map-travel.md`), so the
officer (1300005) in the Mushroom Forest Field offers 2312 only after it.
An Aran's "The Lost Seal Stone of Ellin Forest" (21753) likewise waits for
record 21764 to be "1", set on entering Ellin's warehouse after 21751, and
"Info on Another Seal Stone" (21754) for 21765 to be "2", set on entering the
Bowman Instructional School after 21753 (see `map-travel.md`). Nothing in the
sources sets either record, so the server's setters are a guess.
"The Puppeteer's Warning" (21720) waits for record 21760 to be "0". The
shadow's invitation (21719, `q21719s`) sets it as it sends the Aran to the
Puppeteer's cave (910510200), where Francis (1204001) sets it again,
completes 21719 and sends the Aran back to Deep Forest II, so Tru offers
21720 afterwards. Nothing in the sources sets 21760 either; Cosmic
completes 21719 on accepting.
HeavenClient checks the same record before offering the quest, unless the
record is itself a quest in `QuestInfo.img` (Meren's 6030-6032 read 6029's
string, which the server doesn't keep, so the client leaves those to it).
"Endangered Mushking Empire" (2300-2304, levels 30-38) leads an Explorer to
the kingdom: each branch's instructor (Dances with Balrog, Grendel, the Dark
Lord, Athena Pierce, Kyrin) offers it through the quest alert, and the start
scripts `q2300s`-`q2304s` (Edelstein's `q2300s`) give the recommendation
letter (4032375, needing an ETC slot), which starts the quest, then offer to
send the player straight to the opening scene (106020001). The Head Security
Officer's (1300005) end scripts `q2300e`-`q2304e` take the letter, complete
the quest and give 1,200 EXP; the data has no rewards. The player answers
him on one page (`Page::Reply`, speaker bit 0x2). The Cygnus Knights'
2305-2310 are left out with their other quests.
"James's Whereabouts" (2325-2327) follows the officer's brother James
(1300008, in 106021201): his end script `q2325e` has the player answer him
too and gives the 6,000 EXP of its `Act` (a script's completion applies no
`Act`), and his start script `q2327s` starts 2327 after the Mushroom
Disguise of 2326. Scarrs's (1300007) `q2338s`, "Killer Mushroom Spores,
Again", gives a spare spore (2430014) to one holding none before 2322
starts, completing the quest as it starts. Their text is unsourced; the
`find_james` portal in James's room has no source either and does nothing.
"About the Mushroom Castle..." leads there from Sleepywood and the desert. The
Rememberer's (1061011) end script `q2251e` takes the 20 Recording Charms of
"Zombie Mushroom Signal 3" (4032399, a quest drop of the Zombie and Annoyed
Zombie Mushrooms in `reward_overrides.toml`) and gives its 8,000 EXP; his
start script `q2254s` sends the player to Karcasa (2101013), who ends 2254
and, after his own 2255 and 2256, starts 2257 from the data. The Camel Cab
(2110005) ends 2257 (`q2257e`), starts 2258 after asking (`q2258s`; its
five-minute `timeLimit` clock starts with it) and ends it once the 40 Meerkats
are down (`q2258e`), then starts and ends 2259 and 2260. He ends 2259
(`q2259e`) only at Sahel 1 (260020700), not outside Ariant (260020000) where
the Meerkats listen. 2260 ends at level 30, the data's check; Cosmic also asks
for a second job, which neither the data nor `QuestInfo` says. It only tells
the way; the Empty House's `TD_MC_first` checks level 30, not this quest (see
`map-travel.md`). No source has `q2254s`, and the text of all of them is
unsourced.
Magatia's societies are joined through Han the Broker (2111007). After his
"A Deal with the Broker" (3300), his end scripts `q3301e` and `q3303e` offer a
choice of the jewel ores (4020000–4020008) the player holds two of, take two of
the one picked and complete "Test from the Head of Zenumist/Alcadno Society"
(Cosmic `3301.js`, `3303.js`); without any he refuses. The data's Korean start
pages ask 30,000 and 50,000 mesos, which no end script takes. The capes that
follow (3302, 3304) are the data's. Carson (2111000) and Maed (2111001) make a
lost cape again: their start scripts `q3305s` and `q3306s` ask before starting
"Re-acquiring Zenumist/Alcadno Cape", whose start and end checks and rewards
(5 Emeralds or Topazes, 10 Leather, 10,000 mesos, every 4,320 minutes) are the
data's. Their text is unsourced.
Dr. De Lang (2111002) hides in Dran's Lab (926120200), which has no portal;
its forced return is Parwen's map (261020401). Parwen's (2111006) start scripts
`q3320s` and `q3353s` ask, then start "What Parwen Knows" or "What De Lang
Wants" and send the player there (Cosmic `3320.js`, `3353.js`), and once 3320
is taken a click on Parwen does the same (Cosmic `2111006.js`). De Lang's
`q3321s` and `q3354s` ask and start "Dr. De Lang, the Missing Alchemist" and
"De Lang's Potion"; the ends are the data's. Parwen's `q3360s` makes up a
password of ten digits and capitals, says it once, and starts "Verifying the
Password" (an info quest) with it as the info (Cosmic `3360.js`): a script's
`SetQuestInfo` after its `StartQuest` of the same quest sets the string the
quest starts with. The Secret Passage (2111024) asks for it at the secret
door, and the right one sets the info to `1`, which the data's end checks (see
`map-travel.md`). Their text is unsourced.
"Life Alchemy, and the Missing Alchemist" (3314) starts from the data, with
Alcadno's cape (1102136) worn. The desk in Alcadno's upper lab (2111015,
261020200) gives one of Russellon's Pills (2022198) to a player on 3314 who
holds none and isn't under one, in the player's own words (Cosmic
`2111015.js`). The pill brings on its `specEx` disease, Poison (see
`combat.md`). Russellon's `q3314e` checks for that poison: without it he sends
the player back to the desk; with it he completes the quest (the data's 12,500
EXP and one weighted stack of potions) and cures them with an All Cure
Potion's effect (2050004; `QuestInfo` says he gives an antidote). Cosmic gives
10 All Cure and 10 random potions after a buff of its own instead.
A won Romeo and Juliet run sets record 7072 to `1` (see `party-quests.md`),
which "Yulete's Request" (3365) and "Romeo's Present" (3369) start on
(`START_RECORD_QUESTS`). Nothing in the data or Cosmic sets it. Their ends are
the data's. "Yulete's Reward" (3382), after 3365, can be taken again at once
(`interval` 0). Yulete's (2112014, 926130000) `q3382e` takes 25 of each marble
(4001159, 4001160) for Horus' Eye (1122010) from a player who owns none, worn
or carried, or else 10 of each for a Rock of Wisdom (2041212), and with fewer
asks for more (Cosmic `3382.js`). Their text is unsourced.
Three end scripts take what their quest asks for and give rewards the data
leaves out. Dr. Kim's (2050001) `q3452e` takes the Bloctopus Key Chain and
gives 50 Mana Elixir Pills and 8,000 EXP (Cosmic `3452.js`). Mr. Ku's
(2092000) `q3833e` gives by the Bellflowers held: the largest of 1,000, 600,
500, 100, 50 or 1 that the player holds is taken, for that tier's items and
EXP in Cosmic `3833.js` (from a scroll, 50 Power Elixirs and 54,000 EXP down
to a Red Potion and 10 EXP). Muhamad (2100001) is convinced in `q3953e` after four answers, each the
only choice of a menu (Cosmic `3953.js`); then a Lidium is taken for 20,000
EXP. If the rewards don't fit, the script says so and nothing is taken. Their
text is unsourced.
Ardin's (2101003) start script `q3933s` for "Ardin's Sand Bandits" asks,
then, if nobody is in Ardin's room (926000000), starts the quest and sends
the player to its `st00`, beside the other self (9100013) that the data's
end asks to be killed (Cosmic `3933.js`); with someone there, it says so. The
room's spawn has a negative `mobTime`, so the other self appears once. The
room is one of `scripts::SOLO_MAPS`, so it fills again when empty. Its way
out is a lost link (see `map-travel.md`). The text is unsourced.
"The Sorcerer Who Sells Emotions" (3514) starts from the data: it gives the
Sorcerer's Potion (2022337) for 1,000,000 mesos. The potion's `hpR` and
`mpR` are -100, so drinking it leaves 1 HP and no MP (see `inventory.md`).
The end needs the potion gone. The Sorcerer's (2140002) `q3514e` says it
worked, then gives the end's 891,500 EXP and completes the quest. Cosmic
`3514.js` instead checks for its own potion buff, which v83's potion has no
`time` for, so there is no buff to check. The text is unsourced.
Dr. Kim's "Wave Translator" (3454, repeatable) gives a working translator
(4031928) or a faulty one (4031927) at even odds (`prop` 1 each). Cosmic
`3454.js` always gives the working one. The working one starts 3456 at the
Alien Gray (2050002). The faulty one starts "The Endangered Lives of Grays"
(3455), also repeatable, whose start script `q3455s` says the translator gave
only noise, takes it, and starts the quest. The quest then ends at the Gray
with nothing, and Dr. Kim can make another translator. No script for 3455
survives, so its text is unsourced.
Manji's (1022002) "To the Desert..." (2127) ends in his script `q2127e`,
which only completes it. That opens "A Special Assignment" (2123), whose
end at Moppie (2012019) leads to "A Supply from the Sand Crew" (2124).
Eleska's (2101002) `q2124e` and `q2126e` take Moppie's box (4031619 or
4031624, as each quest's data checks) and complete the quest. Cosmic's
`2126.js` takes 2124's box instead. The text is unsourced.
In "Help Me Find My Glasses" (2186), Abel (1094001) in Nautilus Harbor
(120000000) has lost his glasses in the five bushes there (1094002 to
1094006). While the quest is under way, searching a bush with no glasses
held gives one of three pairs at even odds: Abel's own (4031853) or one of
two others (4031854, 4031855). With any pair held, the bushes give nothing
more. The data's end checks for Abel's own pair, and his `q2186e` takes it
for 1,700 EXP and ten Nautilus Return Scrolls (2030019). If the player
clicks Abel holding another pair instead, Abel takes it for 1,000 EXP and
five scrolls and completes the quest anyway (Cosmic `2186.js` and the
bushes' scripts). The text is unsourced.
In "Truth of the Rumor" (2144), Winston (1022006) wants to hear what five
people know about the old tree: Blackbull (1020000), Manji (1022002),
Ayan (1022007) and Dances with Balrog (1022000) in Perion, and Betty
(1032104) in Ellinia. Each has a quest of their own (2148 to 2152) whose start
script tells the player one thing and completes the quest as it starts,
with the data's EXP (10, or 15 for Betty). Winston's end asks for all five.
The text is unsourced.
Tienk (2006) in Lith Harbor greets a new Beginner with "Tienk, the Monster
Book Salesman" (2197); the end script `q2197e` completes it with nothing
given. "The Last Song" (2293) follows "Say NO to Plagiarism" (2288):
Maestro Rho's (1052120) start script `q2293s` asks about the last song the
Spirit of Rock played and starts the quest. The data's end has no end script
and asks for and gives nothing, so Cosmic's guessing game and its 32,500 EXP
are left out. The text is unsourced.
New Leaf City's Masteria quests start from scripts that ask whether the
player is in; a yes says what to bring or hunt and starts the quest, and a
no ends it. The data's end checks and rewards each one. They are John
Barricade's "The Mark of Heroism" (8221), Lukan's "Storming the Castle"
(8223), Taggrin's Raven Ninja Clan tasks (8224 to 8226), and Lita Lawless's
bounties (8231 to 8238), where every second one is a repeatable follow-up
(Cosmic `8221.js` to `8238.js`). The text is unsourced.
John Barricade's brother Jack takes the story further. "Finding Jack"
(8219) starts the same way at John, and Jack's end script `q8219e` gives
the Map of the Phantom Forest (3992040) and 175,000 EXP. In "Lost in
Translation" (8227 to 8229), Jack's start gives a Coded Communique
(4032032), and the data's end at John completes 8227. John then sends the
letter to Elpam Gorlab (8228, given again if lost), whose end script takes
it. Next John hands over the Translated Communique (4032018, 8229), and
Jack's end script takes it for 50,000 EXP. In "The Brewing Storm" (8222),
Lukan's end script takes the ten Stormbreaker Badges and gives nothing, as
the data does. In "Stemming the Tide" (8230), Jack completes the quest when
the player has the Crimsonwood Keystone and leaves the key with them.
Without it, Jack says where to turn (Cosmic `8219.js` and `8227.js` to
`8230.js`). Each end script that takes an item first checks it is there,
and a missing item gets its own line. The text is unsourced.
"Exploring Mushroom Forest (1)" (2314) and "Over the Castle Walls (2)" (2322)
work the same way as 2312. The Domestic Secretary's (1300003) start scripts `q2314s`
and `q2322s` send the player off; looking at the barrier (`investigate1`) or
the wall (`investigate2`, reached by `gotocastle` while 2322 is under way)
sets the record, and she then completes the quest (see `map-travel.md`).
"Where's Violetta?" (2332) works the same way: entering the Wedding Hall
(`findvioleta`) sets it, and Violetta's end script `q2332e` then completes
it (see [party quests](./party-quests.md#the-wedding-hall), with her other
scripts `q2333s`, `q2333e`, `q2334s`, `q2335s` and `q2342s`).
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

"Baby Steps" (21704) is another: Lilin's start script (`q21704s`) completes
it at once with Cosmic's 500 EXP, as the data has no end rewards.

"Dollhouse" (3230) has no completion NPC either: its `Check/1` is only an
`infoex` ("dummy") on an info string nothing sets, so neither the client nor
the server completes it through the quest window. Mark the Toy Soldier's
script does, for the pendulum (`Effect::CompleteQuest`; see
[party quests](./party-quests.md), "Eos Tower's dollhouse room").
"The Missing Mechanical Parts" (3239) is the same, completed by Cheng the
Assistant for ten parts ("The Toy Factory's Sector 4").
"Eliminating the Rest" (2335) reads its own record (`infoNumber` 2335,
at least 1), which nothing sets either. Violetta's start script `q2335s`
gives the Security Room's key (4032405) with it, and the room's door near
Skyscraper 3 (`go_secretroom`, see [map travel](./map-travel.md)) completes
it for 5000 EXP and takes the key.
`game_data::quests::COMPLETED_BY_NPC_SCRIPT` lets them through and keeps
the `infoex`.

Mu Young (1061014, in 105100100) has two start scripts, both silent as in
Cosmic. `q2238s` starts "Who is the Owner of the Mysterious Note?" (2238,
once 2237 is done). `q2245s` ("To Tristan's Tomb", for a player on 2238)
completes 2245 at once, so that its `interval 0` lets it be taken again,
and takes the player alone into the unknown Jr. Balrog's room
(`EventKind::Tristan`, see [party quests](./party-quests.md#jr-balrogs-room)).
Jr. Balrog's fall brings out Tristan's Spirit (1061015), where 2238 ends
for the data's 50,000 EXP; with it done, the `tristanEnter` portal opens.

"Snowman's Rage - Found a clue" (3108) works the same way too: for a
player on "Snowman's Rage" (3107), the broken Spirit of Snow Statue
(2020012, in the Valley of Snowmen 211040102) has a start script
(`q3108s`) that completes 3108 at once with the data's 1,000 EXP
(`Act/1 exp`, given by the script). 3107, which asks for 3108 done, then
ends at Scadur. Its line is a placeholder (`snow_statue.clue`).

"In Search for the Lost Memory" (3523–3527, 3529, 3539) works the same way,
one quest per first-job instructor: Dances with Balrog (3523), Grendel the
Really Old (3524), Athena Pierce (3525), the Dark Lord (3526), Kyrin (3527),
for the Cygnus Knights Neinheart (3529), and for the Arans Lilin (3539, in
Rien). For a player of that class on "Memory Keeper" (3507), the
instructor's start script (`q3523s` etc.) completes the quest at once and
sets record 7081 to `1`. 3507 then ends at the Memory Keeper (2140001, on
270010111), whose completion checks that record (`RECORD_QUESTS`). The line
is a placeholder (`lost_memory.regained`).

"Stealing Queen's Order of Silk" (3941), on the way to Jano's "Mission
Complete!" (3940), repeats (`interval` 0) while the player holds no Queen's
silk (4031571). Karcasa (2101013, on 260010600) only deals with a player
disguised by Tigun's potion (2210005, the Morph buff that the palace's
`ariant_queens` portal also checks, `scripts::disguised_as_tigun`). In the
disguise, `q3941s` starts the quest and `q3941e` gives the silk (the data's
`Act/1`, given by the script) and completes it. Anyone else is sent off at
either end. The lines are placeholders (`karcasa.silk.*`).

Adonis (9201106, El Nath) tells "Lost Spirits" (8255): `q8255s` completes
it as it starts, with the data's 500 EXP. Its sequel (8256, `q8256s`) is
the Liberation Ritual, which powers up a relic of El Nath (iTCG items such
as the Crystal Blade) with a Zakum Diamond. It isn't in: nothing in v83
gives the relics, and the recipe has no source.

"The Seed That Swallow Lost" (3609) starts from the data at Swallow
(2071006, Korean Folk Town). That needs 3606 done, neither of the gourd
quests (3607, 3608) started, and no Magic Seed (4031225) held. Its
`Check/1` is empty (`COMPLETED_BY_NPC_SCRIPT`), so talking to Swallow
again while it is started gives a new seed (the data's `Act/1`) and
completes it. The lines are placeholders (`adonis.spirits.*`,
`swallow.new_seed*`).

"A rainbow snail shell that makes wishes come true!?" (2156) starts from
the data at Pia (1012102, Henesys Park) after Snail Hunt (2155). Our data
ends it on Pia's `q2156e` and has no `Check/1` items and no `Act/1`. The
script asks for the Rainbow-colored Snail Shell (2210006). Mano drops it
every time (`reward_overrides.toml`, from Mano's monster book). Pia takes
it and gives the GMS v83 data's 7,500 EXP, 30,000 mesos and 3 fame. Then
Pia plays the shell's effect, the special action `act2156`
(`Effect::NpcSpecialAction`). The lines are placeholders (`pia.shell.*`).

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
| 1028 To Lith Harbor! (scripted start) | Shanks (22000) → Olaf (1002101) | 130, 10 Meat and 10 Lemons; after Mai's Request (1027), Shanks's `q1028s` sails to Lith Harbor for free and starts it, and a no stays in Southperry (no Cosmic script, so the ask is unsourced). The start also asks for the Wizet Plain Suit (1042003), a GM item, which is how the data shelves the quest for players |
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
otherwise nothing in its script applies. `Effect::SetQuestInfo`
(`QuestLog::set_info`) sets a started quest's whole string, and does
nothing if it already reads so; after a `StartQuest` of the same quest in
the same script, it is the string the quest starts with (`commit_player`
keeps it). A quest whose start sets no string
(`Act/0/info`) starts with an empty one.

| Quest | String | Set by |
| --- | --- | --- |
| 3926 Screwing the Red Scorpions | "2222" to "3333" | four cupboards in Ariant's houses (2103009-2103012), each taking one Jewel (4031579) |
| 3929 Sejan's Test | "2222" to "3333" | four houses (2103003-2103006), each taking one parcel of food (4031580) |
| 3900 Learning the Culture of Ariant | "4" to "5" | the Palace Oasis (2103000) |
| 3927 Existence of the Secret Organization | "0" to "1" | the secret wall (2103001) |
| 3311 Clue | "4" to "5" | the wall in the missing alchemist's home (2111011), after a yes |
| 2166 Shining Stone | "" to "5" | the Shiny Stone (1092016, `nautil_stone`) in the Nautilus's Generator Room (120000301); Sharyl (1092003) completes it |
| 2236 How to Shoo Away the Evil | "000000" to "111111" | the six Shaman Rocks (1063012 on 105050200, 105060000, 105070000; 1063013 twice on 105090000 and once on 105090100), each taking one charm (4032263) |
| 3360 Verifying the Password | Parwen's password, set with the start, to "1" | Parwen's start script (`q3360s`); the Secret Passage (2111024) at the secret door |
| 3421 Collecting Meteorite Samples | "000000" to "111111" | the six meteorites in Kulan Field (2050014 on 221040000, 2050015 on 221040200, 2050016 on 221040300, 2050017 on 221040100, 2050018 on 221040201, 2050019 on 221040400), each giving one Meteorite Sample (4031117) |
| 3114 Eliza's Eternal Sleep | "0" to "42" | the seven strings of the goddess's harp (2012027-2012033, do to si) in Eliza's Garden (920020000): each sounds its note to the map (`Effect::SoundToMap`, `Sound.nx/Field.img/orbis/<note>`) and, while the quest is under way, counts the notes of Twinkle Twinkle Little Star played right; a wrong one sets "0" again with the party quest's miss effect and sound |
| 3345 The Black Magician's Magic Pentagram | "0" to "4" | the four circles of the pentagram in the Black Magician's lab (2111020-2111023 on 261040000), touched in turn; the last takes the three Magic Stones (4031739 from the start, 4031740 from Homunscullos, 4031741 from D. Roys, `reward_overrides.toml`). One out of turn, or the last without the stones, sets "0" |

Cosmic's scripts gave the slots and digits. The houses are silent, as
there; the walls', the oasis's, the stone's and the rocks' words are
placeholders (`quest_info.*`). Cosmic's stone completes 2166 itself;
here it only sets the string, since Quest.nx has Sharyl complete it.
2166 and 2236 name their own records (`infoNumber`), so they are
read as info quests. 2236's two rocks in the same swamp are the same NPC;
the field tells them apart by `nth`, which of the map's NPCs with that id
was clicked, in Map.nx order (`ServerMessage::NpcConversation`,
`net::npc_scripts::shaman_rock`). A rock already charmed, or clicked
without a charm, gives its usual speech. The meteorites are silent, as
in Cosmic, whose bitmask record is the info string here; once all six
have given theirs, any meteorite gives another sample while fewer than
six are held, so a lost one can be replaced. 3421's completion only
counts the six samples (it has no `infoex`). With no Etc slot free, a
meteorite says so (`omega_meteorite.full`) and keeps its sample.
Not supported: the quests reading another record that no script sets.

| Quest | Record | Set by |
| --- | --- | --- |
| 6400 Jonathan's Court | 6401 to "q3" | hitting the real Bart (reactor 1209000) in Lord Jonathan's test room (912020000), with a pink line |
| 6330 How to Improve Your Strength (Marauder) | 6331 to "2" | Kyrin (1090000) past the test room 912010000 |
| 6370 How to Improve Your Strength (Outlaw) | 6371 to "2" | Kyrin past the test room 912010100 |
| 6363 A New Revelation | 6364 to "2" | the past Papulatus (2043000) sending the Buccaneer back from 922020300 |
| 6410 Mission Free the Monsters | 6411 to "p2" | the way out of Save Delli! (925010300) once Willi is saved |
| 29932 Protector of Pharaoh | 7760, a count | each of the player's own kills and cool kills in Nett's Pyramid; Duarte (2103013) gives medal 1142142 at 50000 |
| 3250 A Cute Baby Bird | 7067 to "3000" | Bird Feed (4031992) dropped on the Timer's Egg's raise window, 100 each (`world/raise.rs`, see "Raise items" in `inventory.md`) |
| 21742 The Black Scroll | 21763 to "800" | Special Ink (4032342) dropped on Mr. Do's Hanging Scroll's (4220151) raise window, 100 each, likewise |

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

## Adventurer titles

The Explorer titles are Beginner, Junior, Veteran and Master Adventurer
(29900 to 29903). Each has `normalAutoStart`, so the client offers the
quest once the player reaches its job advancement and level and does not
hold its medal. Starting it runs the start script anywhere: there is no
dialogue, the quest starts, and a line in the chat log (SHOW_STATUS_INFO 9)
names the title. Dalair's (9000040) end script gives the title's medal
(1142107 to 1142110) and completes the quest. If the Equip tab is full,
it says so and does nothing. This follows Cosmic's `29900.js` to `29903.js`.
The `adventurer_medal.*` text is unsourced. The Knight and Aran titles
(29904 to 29909 and 29924 to 29928) wait for the Cygnus and Aran jobs.

Some titles go with medals won elsewhere: the job medals at level 180
(29910 to 29914), the Mu Lung Dojo Vanquisher (29915), and the town donor
medals (29916 to 29921 and 29933). Each quest's start needs the medal held,
and the data starts it by itself (`autoStart`, `autoAccept`). The start
script runs anywhere and starts and completes the quest at once, with a line
in the chat log naming the title, as Cosmic's `29924.js` does for the Aran
titles. The medal stays with the player. The `medal_title` text is
unsourced. Veteran Hunter and Legendary Hunter (29922 and 29923) have no
completion NPC in the data, so the catalog leaves them out.

Title Challenge - Outstanding Citizen (29508) starts at Dalair from the
data, from level 10. Dalair's end script gives the Outstanding Citizen Medal
(1142081, the quest's `viewMedalItem`) and completes the quest once the
player is married, is in a guild, and has a Junior, as the quest's
`demandSummary` asks. Until then Dalair says what is missing and does
nothing. The runtime loads the Family window's numbers for this script, as
it does for Captain Al's (`FAMILY_QUEST_SCRIPTS`). Cosmic has no script for
it, and the text is unsourced. The other title challenges count fame,
kills or rankings that the server does not keep.

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
  a relogin, and a string one delivery short refused), `shaman_rocks`
  (2236's six rocks, the swamp's two told apart, each charmed once),
  `shining_stone` (2166's stone, once, then Sharyl),
  `mushroom_forest` (2314's barrier and 2322's wall, each looked at once),
  `mushroom_opening` (the opening scene, its way out, then The Test),
  `mushroom_letter` (2300's letter, the way to the scene, the officer),
  `mushroom_james` (James found, disguised and escaping; Scarrs's spare),
  `mushroom_karcasa` (2251's charms, Karcasa, the Camel Cab's Meerkats and
  clock, Sahel 1, level 30),
  `magatia_societies` (Han's ore choice for both tests, Carson's cape again,
  Maed's ask and its no),
  `magatia_de_lang` (Parwen's and De Lang's starts and the lab, Parwen's
  password at the secret door, wrong and right),
  `quest_end_rewards` (Dr. Kim's pills, Mr. Ku's tier for 99 Bellflowers,
  Muhamad's four answers),
  `ardins_sand_bandits` (one player at a time in Ardin's room, the other
  self's kill, the way out, the room fresh for the next),
  `sorcerers_potion` (3514 sold, refused while the potion is held, drunk
  to 1 HP, then completed),
  `meteorite_samples` (3421's six meteorites, each giving once and
  marking its character, then the completion),
  `grays_translator` (3455's faulty translator used up at the Gray and the
  quest ended, then 3456 started with the working one),
  `sand_crew` (Manji's 2127 end script, 2123 through Moppie, and Eleska
  taking 2124's box),
  `abels_glasses` (2186: a bush gives one pair at a time, then Abel's end
  script or his click, depending on the pair),
  `truth_of_the_rumor` (2144 through its five tellers' start scripts),
  `tienk_and_the_last_song` (2197's end script, 2293's start and data end),
  `new_leaf_city_starts` (Lita's bounty turned down, taken and completed, its
  follow-up started, and Taggrin's first task),
  `finding_jack` (8219 from John to Jack, the three letters, the badges, and
  the keystone kept),
  `adventurer_medals` (two titles started away from Dalair, a third
  refused below its level, and both medals given in Perion),
  `to_lith_harbor` (1028: Shanks turned down, then sailing for free, and
  Olaf's reward),
  `outstanding_citizen` (29508: refused with no marriage, guild or Junior,
  then the medal once all three are set),
  `medal_titles` (two titles finished away from their NPCs with the medals
  held, not twice, and a third refused without its medal),
  and `timed_quests` (Medin's clock: shown on start, taken down on
  forfeit, running out, kept through a channel change, given up on logout).
