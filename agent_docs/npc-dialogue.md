# NPC dialogue

Clicking an NPC opens asset-backed dialogue. Pio and other NPCs with normal
speech use Next, Back, and OK. Close and Escape cancel. When the NPC has
quests for the player, HeavenClient shows its quest menu first and renders
quest conversations itself; picking "Misc." asks the server for normal speech.
See [Quests](./quests.md).

The Interact key (Space by default, action 55) does the same as clicking the
nearest NPC within 150 px horizontally and 50 px vertically of the player's
feet (`HeavenClient/Gameplay/MapleMap/NpcReach.h`). v83 had no such key; it is
a convenience from the later client. Characters created before the binding was
added need to bind it in the key config.

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
| `tutorialSkip` | Clicking the Maple Administrator (2007), or the `tutoChatNPC` portal for accounts with a level-30 character | Yes warps to Lith Harbor (104000000). Neither is on the original island, so this is unused |

Text comes from the [server content pack](./server-content.md) with
placeholders. HeavenClient expands NPC text markup (`Util/NpcTextMarkup.h`:
`#h`, `#p`, `#t`/`#z`, `#o`, `#m`).

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
speaker portrait flag, text, and separate Back/Next bytes. Normal text uses
message type 0. Response 1 advances or finishes, 0 goes back, and -1 cancels.

HeavenClient uses the Back/Next bytes to choose buttons while retaining the
wire message type for responses. Escape sends cancellation rather than Back.
Message type 1 shows YES and NO; type 12 shows ACCEPT and DECLINE, which the
client's quest dialogs use for an offer. A menu (type 4) answers with
response 1 and the chosen `#L` id.

## Assets and validation

Place `Map.nx`, `Npc.nx`, `String.nx`, and `Quest.nx` in `assets/game-data/`. The server's
NPC paths can be overridden with `RUSTMS_NPC_NX_PATH` and
`RUSTMS_STRING_NX_PATH`. These assets stay untracked. When running the Docker
harness, files or symlink targets must live within its mounted `assets/` tree.

Validate with `cargo run -p integration-harness --bin harnessctl -- test`.
`npc_dialogue` integration tests cover asset text, progression, cancellation,
restarting, invalid requests, stale replies across warps, and player isolation.
Unit tests cover the conversation transitions and asset-key resolution.
