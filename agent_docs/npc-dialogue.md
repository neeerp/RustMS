# NPC dialogue

Clicking an NPC opens asset-backed dialogue. Pio and other NPCs with normal
speech use Next, Back, and OK. Close and Escape cancel. When the NPC has
quests for the player, HeavenClient shows its quest menu first and renders
quest conversations itself; picking "Misc." asks the server for normal speech.
See [Quests](./quests.md).

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

NPCs also say their `info/speak` lines now and then in an `npc`-style chat
balloon (client only).

## Scripted NPCs

`net::npc_scripts::NpcScript` holds native scripts (Lua is deferred) as a
list of pages: `Say`, `Ask` (effects on yes), and `Menu` (each choice its
own pages), plus effects applied after the last page. The effects are
`net::scripts::Effect`, shared with map and portal scripts. The client actor
(`ClientActor::apply_effects` in `client/npc_script.rs`) applies them to a
copy of the player and commits any quest change together with it, or
nothing at all; if a given item doesn't fit, the script's "inventory full"
page shows instead. Answers are matched to the prompt they answer
(`npc_scripts::input`); anything else ends the script.

| Script | Trigger | Behavior |
| --- | --- | --- |
| `q1021s`/`q1021e` | QUEST_ACTION 4/5 for quest 1021 | Roger sets HP to 25 and gives Roger's Apple; finishing needs full HP and no apples left, then pays 3 Apples, 3 Green Apples, 10 EXP |
| `begin5` | Clicking Robin (2003) | 18-topic Q&A menu |
| `tutorialSkip` | Clicking the Maple Administrator (2007), or the `tutoChatNPC` portal for accounts with a level-30 character | Yes warps to Lith Harbor (104000000) |

Text comes from the [server content pack](./server-content.md) with
placeholders. HeavenClient expands NPC text markup (`Util/NpcTextMarkup.h`:
`#h`, `#p`, `#t`/`#z`, `#o`, `#m`; image tags `#v`/`#i`/`#f`/`#s` are dropped
until inline icons exist) and clips long menus to the dialog.

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
speaker portrait flag, text, and separate Back/Next bytes. Normal text uses
message type 0. Response 1 advances or finishes, 0 goes back, and -1 cancels.

HeavenClient uses the Back/Next bytes to choose buttons while retaining the
wire message type for responses. Escape sends cancellation rather than Back.

## Assets and validation

Place `Map.nx`, `Npc.nx`, `String.nx`, and `Quest.nx` in `assets/game-data/`. The server's
NPC paths can be overridden with `RUSTMS_NPC_NX_PATH` and
`RUSTMS_STRING_NX_PATH`. These assets stay untracked. When running the Docker
harness, files or symlink targets must live within its mounted `assets/` tree.

Validate with `cargo run -p integration-harness --bin harnessctl -- test`.
`npc_dialogue` integration tests cover asset text, progression, cancellation,
restarting, invalid requests, stale replies across warps, and player isolation.
Unit tests cover the conversation transitions and asset-key resolution.
