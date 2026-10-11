# NPC dialogue

Clicking an NPC opens asset-backed dialogue. Pio and other NPCs with normal
speech use Next, Back, and OK. Rain (12101) offers her seven-quiz quest chain
when the character is eligible, then falls back to normal speech after it is
completed. Close and Escape cancel. See [Quests](./quests.md).

## Content and quest boundary

`NpcDialogueData` in `game-data` resolves the numbered references under
`Npc.nx/<NPC ID>.img/info/speak` against `String.nx/Npc.img/<NPC ID>`.
This uses existing asset text without copying it into the repository. NPCs
without those references currently have no conversation.

`Quest.nx` contains conversation text (`Say.img`), descriptions
(`QuestInfo.img`), requirements (`Check.img`), and actions (`Act.img`). The Rain
quiz implementation uses those definitions. Normal speech still has no quest
or reward effects.

`net::dialogue::Conversation` handles ordinary pages independently of transport.
`net::quests::QuizConversation` handles offers, tutorial pages, choices, feedback,
and effect intents. The runtime commits quest effects before confirming them
with packets. Persisted progress is separate from transient conversation state.

## Runtime ownership

TalkToNpc (0x3A) and NpcTalkMore (0x3C) are parsed into `NpcInteraction` actions.
Start requests pass through WorldServerActor and ChannelActor to the player's
FieldActor, which resolves the object ID against its own NPC list. The field
returns the validated template ID, location, and a server-generated request ID.
ClientActor rejects late replies after another NPC click or a map transfer.

ClientActor owns at most one normal or quiz conversation. It handles responses
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
