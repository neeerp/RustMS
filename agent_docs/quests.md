# Quests

## The v83 flow

Quests run the way the original client ran them:

1. **HeavenClient** reads `Quest.nx` itself. Clicking an NPC lists its quests
   in the NPC chat window under the original `UtilDlgEx/list*` headers
   ("Quests to turn in", "Quests in progress", "Available quests"), with
   "Misc." for the NPC's normal speech. Picking a quest plays its `Say.img`
   pages locally (`Gameplay/QuestConversation.cpp`).
2. When the player accepts or finishes, the client sends `QUEST_ACTION`
   (`0x6B`): action byte (1 start, 2 complete, 3 forfeit, 4/5 scripted),
   quest id, and the NPC, then its position.
3. **The server** checks the request against the quest's `Check.img`
   requirements and the player (`net::quests::check_start`/`check_complete`),
   asks the field whether that NPC is on the map within reach (1200×800
   around the player), and commits the change and its `Act.img` rewards
   atomically (`db::quest::start`/`complete`/`forfeit`).
4. It answers with the quest record (`SHOW_STATUS_INFO` mode 1), EXP gain and
   stats if any, and `UPDATE_QUEST_INFO` mode 8 (quest, NPC, next quest). A
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
| `Check/<id>/0` (start) | `npc`, `job`, `lvmin`, `lvmax`, `quest` |
| `Check/<id>/1` (complete) | `npc` |
| `Act/<id>/0` | none |
| `Act/<id>/1` | `exp`, `nextQuest` |

`Act` nodes may repeat speech under numbered keys and `yes`/`no`; that is not
an action. A quest with any other key (`item`, `mob`, `money`, `pop`,
`skill`, `startscript`, `endscript`, `infoex`, ...) is **unsupported**: it is
never offered, and actions for it are refused. A prerequisite without a
`state` means "not started" (mutually exclusive quests).

The `game-data` asset test compares the server's result with the committed
list `game-data/supported_quests.txt`. HeavenClient doesn't classify quests:
the build copies that list next to the executable, and the client offers
only those quests (`Character/QuestCatalog.cpp`). Supporting a new key means
changing the server's allowlist and regenerating the list. 209 quests qualify with the current
rules, including the beginner ones:

| Quest | Start → Finish | EXP |
| --- | --- | --- |
| 1009–1015 Rain's Maple Quiz | Rain (12101) | 2, 4, 6, 10, 10, 10, 20 |
| 1031 Heena and Sera | Heena (2101) → Sera (2100) | 5 |
| 1032 Nina's Brother Sen | Nina (2102) → Sen (2001) | 7, next 1033 |
| 1033 What Sen Wants | Sen (2001) → Nina (2102) | 7 |
| 1036 Robin the Walking Encyclopedia | Robin (2003) | 40 |

1034 (Mushroom Candy), 1035 (Todd's hunting), and 1021 (Roger's Apple) need
items, kills, or scripts and stay unsupported for now.

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
  quest is open.
- No pages at all: the action is sent immediately (Sen's completion of 1032).

Markers use the same catalog (`QuestLog::supported_rules`): bulb for an
available quest, open book while in progress at the start NPC, closed book at
the finishing NPC.

## Persistence and rewards

`character_quests` is keyed by `(character_id, quest_id)`: no row (not
started), status 1 (started), status 2 (completed, with a timestamp).
Forfeiting deletes a started row. Every change locks the character row first;
prerequisites are rechecked inside the transaction, and a completion moves
1 → 2 and adds EXP in one transaction, so a replay cannot pay twice.

EXP uses the beginner table through level 9 (+5 AP per level). Full leveling
(HP/MP growth, SP, other jobs) belongs with combat. After a completion the
client actor adopts the committed level, EXP, and AP and refreshes its field
avatar if the level changed.

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
- Integration: `rain_quizzes` (all seven quizzes, rewards once, prerequisites,
  wrong NPC, NPC not on the map, forfeit, relogin, channel change, isolation)
  and `beginner_contact_quests` (the route through Sen's house, `nextQuest`
  results, and unsupported 1034).
