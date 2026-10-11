# Rain's Maple Quiz quests

Rain (NPC 12101 in Amherst) supports the seven asset-defined quests 1009–1015.
Click her to accept the first quiz, read the tutorial, and select an answer.
Wrong answers display the asset's explanation and allow a retry. Correct
answers complete the current quest and award its EXP. Click Rain again for the
next quiz. Progress is retained if the dialogue is cancelled or the player
changes maps, changes channels, disconnects, or logs in again.

## Assets and eligibility

`game-data::quests::QuizQuestData` reads the Rain definitions from `Quest.nx`:

- `Check.img`: NPC, beginner job requirement, completed-quest prerequisites
- `Say.img`: offer, acceptance/decline, question, choices, explanations
- `Say.img/<id>/1/stop/0/answer`: one-based answer, converted to a choice ID
- `Act.img`: EXP reward
- `QuestInfo.img`: quest name

No questions or answers are copied into the server source. The executable
quiz set is deliberately limited to this chain. Other quests must not be
activated without implementations of their objectives and actions.

Place `Quest.nx` alongside `Map.nx`, `Npc.nx`, and `String.nx` in the ignored
`assets/game-data/` directory. `RUSTMS_QUEST_NX_PATH` overrides its server path.
Files must be visible through the harness's assets mount.

## Persistence and rewards

`character_quests` is keyed by `(character_id, quest_id)`:

- no row: not started
- status 1: started
- status 2: completed, with a completion timestamp

`db::quest::transition` locks the character row, validates job and prerequisite
progress, and commits quest completion and EXP together. Only a started quest
can complete. Duplicate completion attempts cannot award the EXP again.

Rain awards 2, 4, 6, 10, 10, 10, and 20 EXP (62 total). Starting at level 1
with zero EXP yields level 3, 13 remaining EXP, and 10 AP. Beginner EXP thresholds
through level 9 are implemented using the client's table. Full leveling stat
growth (HP/MP, class-specific rules) remains separate future work.

After a reward, ClientActor refreshes its character snapshot before processing
other actions. This prevents a later map save from overwriting rewarded EXP.
Quest records are serialized at world entry and sent live through status-info
packets. The client updates its started/completed quest log and displays EXP
notifications. The quest window lists started and completed quests; available
quest discovery and objective/detail panes are not implemented yet.

## Conversations

`QuizConversation` is a pure state machine. Offers, tutorial pages, questions,
wrong-answer feedback, and correct-answer feedback have distinct states. It
emits start/complete intents; the runtime persists them before sending the next
prompt. A correct answer pays once even if the player closes its feedback.

NPC identity is validated by the field, while the client actor owns transient
conversation state. Wrong prompt types, invalid choice IDs, and stale replies
are discarded. A new NPC request or map transition invalidates pending field
validation replies with a request generation ID.

The client supports yes/no prompts (type 1) and clickable choices (type 4),
including the selected integer payload. It normalizes literal newline characters
to the text renderer's escaped line breaks. Clicking the text reveals a page
immediately; long text scrolls in 32-pixel increments.

## NPC quest markers

Rain displays the existing animated `UI.nx/UIWindow2.img/QuestIcon` graphics:
light bulb (0) for an available quiz, open book (1) for unmet objectives, and
closed book (2) for a quest that can be resolved through NPC dialogue. Accepted
Rain quizzes use the closed book because they have no external objectives;
the dialogue still validates the answer before awarding anything. Completion
unlocks the next quiz's light bulb. Finishing all seven removes the marker.

`QuestLog::npc_marker` reads the supported chain's NPC, job, and prerequisite
requirements from `Quest.nx`, then combines them with the local quest records.
It advertises only quests implemented by the server. Marker priority is ready
turn-in, in progress, then available. Each NPC refreshes its marker when quest
records or the player's job change, rather than scanning assets every frame.
The markers are drawn above the standing sprite and are clickable.

Logic tests cover all seven transitions, prerequisites, job/NPC filtering,
marker priority, and incomplete objectives. Enable the lightweight C++ tests
with `-DHEAVENCLIENT_BUILD_TESTS=ON`, then run CTest in the client build directory.

## Validation

Fast logic tests: `cargo test -p net -p runtime -p integration-harness --lib`.
Asset checks: `cargo test -p game-data --lib`.
Full suite: `cargo run -p integration-harness --bin harnessctl -- test`.

`rain_quizzes` plays all seven quests, tests wrong answers and replayed rewards,
checks persisted EXP/AP/level after map saves and relogin, and checks decline,
cancellation, invalid selections, channel migration, and isolation between characters. Ordinary
dialogue tests use Pio so they continue to exercise non-quest speech.

Character creation now translates client class-family flags to real job IDs.
The separate correction migration repairs invalid legacy Explorer job 1 rows.
