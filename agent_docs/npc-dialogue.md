# NPC dialogue

Clicking an NPC opens normal speech loaded from the local v83 assets. For
example, Rain (12101) in Amherst has three pages with Next, Back, and OK.
Close and Escape cancel. Pio and other NPCs with normal speech use the same flow.

## Content and quest boundary

`NpcDialogueData` in `game-data` resolves the numbered references under
`Npc.nx/<NPC ID>.img/info/speak` against `String.nx/Npc.img/<NPC ID>`.
This uses existing asset text without copying it into the repository. NPCs
without those references currently have no conversation.

`Quest.nx` also contains conversation text (`Say.img`), descriptions
(`QuestInfo.img`), requirements (`Check.img`), and actions (`Act.img`). Those
are not used yet. Normal speech is not a quest script and reading it does not
accept or complete quests or give rewards.

`net::dialogue::Conversation` handles progression independently of packets,
NPC lookup, and asset loading. It consumes a `NpcDialogueTemplate` definition
and produces a `DialoguePage`. A future quest service should select dialogue
using the character's persisted quest state and the NPC's quest/script metadata.
Branching prompts and explicit quest effects belong in that conversation flow;
quest progress belongs in character persistence, not in a field occupant.
Reward/quest actions will need server-side eligibility validation and an atomic,
idempotent persistence operation before confirming success to the client.

## Runtime ownership

TalkToNpc (0x3A) and NpcTalkMore (0x3C) are parsed by handlers into typed
`NpcInteraction` actions. ClientActor forwards them through WorldServerActor
and ChannelActor to the player's current FieldActor.

The field resolves NPC object IDs against its own NPC list. Template IDs and
unknown object IDs cannot start a conversation. Each occupant owns at most one
conversation; clicking another NPC replaces it. Replies go only to that player.
Finishing, cancellation, or invalid response types clear the conversation.
Leaving a field drops its occupant state, so map transfers, channel changes,
and disconnects also discard conversation progress. A reply without an active
conversation is ignored.

## Wire format and client

NpcDialogue (0x130) carries speaker category, NPC template ID, message type,
speaker portrait flag, text, and separate Back/Next bytes. Normal text uses
message type 0. Response 1 advances or finishes, 0 goes back, and -1 cancels.

HeavenClient uses the Back/Next bytes to choose buttons while retaining the
wire message type for responses. Escape sends cancellation rather than Back.

## Assets and validation

Place `Map.nx`, `Npc.nx`, and `String.nx` in `assets/game-data/`. The server's
NPC paths can be overridden with `RUSTMS_NPC_NX_PATH` and
`RUSTMS_STRING_NX_PATH`. These assets stay untracked. When running the Docker
harness, files or symlink targets must live within its mounted `assets/` tree.

Validate with `cargo run -p integration-harness --bin harnessctl -- test`.
`npc_dialogue` integration tests cover asset text, progression, cancellation,
restarting, invalid requests, stale replies across warps, and player isolation.
Unit tests cover the conversation transitions and asset-key resolution.
