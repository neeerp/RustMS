# Papulatus

The fight with Papulatus at the bottom of Ludibrium's Clocktower, after
Cosmic. Built so far: the way in at Deep Inside the Clocktower, its gates,
the way out, the summon at the cracked dimension, and the win. The slice
plan and its open questions are in v83ref's `reports/a110-papulatus.md`;
the defaults shipped are under "a110" in `lanes/QUESTIONS.md`.

All dialogue is placeholder text (`papulatus.*` keys in
`game-data/placeholder_text.toml`): no licensed script has any.

## The event

The fight is a party event, `EventKind::Papulatus` (`net/src/events.rs`,
`PAPULATUS`), with the machinery of [party-quests](./party-quests.md): its
lobby is Deep Inside the Clocktower (220080000, `CLOCKTOWER_DEPTHS`), its one
map the Origin of Clocktower (220080001, `ORIGIN_OF_CLOCKTOWER`), played in a
copy of its own, and it lasts 45 minutes. A party of one to six may go in,
those of its members on the lobby with the leader, at any level. One party
a channel fights at a time.

Unlike a party quest it has no NPC at the door: the leader starts it by the
`in00` portal (script `Populatus00`, `scripts::PAPULATUS_IN`). The client
actor asks the directory (`papulatus_portal` in
`runtime/src/actor/client/events.rs`, `DirectoryMessage::EventEntry`), and
`scripts::papulatus_portal` turns the answer into a pink line for each
refusal (no party, not the leader, too few or too many there, another party
inside) or the start.

It ends only when its last member leaves (`Event::until_empty`, as an
expedition's), not when its leader does: a member who leaves the party
inside stays, and passing the lead on changes nothing. Leaving is by the
Machine Apparatus (2041025) on 220080001, which asks and sends the player
back to 220080000, by dying (the respawn goes to the map's return map,
Ludibrium), or by logging out (logging back in on 220080001 lands on
220080000, its `forcedReturn`).

The Ludibrium Medal (4031172) for quest 7103 needs nothing of its own: the
content pack's Gatekeeper (8160000) and Thanatos (8170000) drop it, and a
quest item drops only for those on its quest
([combat](./combat.md)).

## The gates

Three reactors stand before the portal on 220080000, `ludigate1` to
`ludigate3` (2208001-2208003; `Event::gate`). When a party goes in the event
actor sets them to state 1 (`shut_gate`), and when the event ends to state 0
(`free_lobby`), each by `FieldMessage::SetReactor`, so everyone on the lobby
sees them close and open.

## The boss

On the Origin of Clocktower the cracked dimension, reactor 2201004 (`boss`,
`reactors::PAPULATUS_CRACK`), takes a Piece of Cracked Dimension (4031179,
from quest 7103) dropped on the ledge above it (Reactor.nx's area). It is
`Action::SpawnAt`: everyone there reads a pink line (`papulatus.summoned`),
the music turns to `Bgm09/TimeAttack`, and the Time Sphere (8500000,
`mobs::TIME_SPHERE`) comes out on the ground below (-410, -400). The reactor
is spent after the one item. Mob.nx's `revive` chain does the rest: the
sphere dies into the Papulatus Clock (8500001), and the clock into Papulatus
(8500002, `mobs::PAPULATUS_LAST`).

Papulatus's death on 220080001 is `boss_falls` in the field
(`runtime/src/actor/field.rs`, shared with Zakum's `zakum_falls`): the
map sees the party quests' clear effect and hears their jingle, and in the
event's copy each player's client actor clears the event's one stage,
which wins it. There is no stage EXP and no world notice. The win stops the
clock and frees the lobby (`free_lobby`), so the gates open and the next
party may go in while the winners loot.

## NPCs

| NPC | Where | Script |
| --- | --- | --- |
| Tombstone 2041024 | 220080000 | A line on the way in |
| Machine Apparatus 2041025 | 220080001 | Asks, then back to 220080000 |

## Tests

`net` has unit tests for the event's end (`events.rs`), the portal
(`scripts.rs`), the NPCs (`npc_scripts.rs`) and the reactor
(`reactors.rs`). The harness's `tests/papulatus_gate.rs` walks a party in,
sees the gates shut, turns the next party away while any member of the
first is inside, and sees them open once the last leaves.
`tests/papulatus_boss.rs` drops the piece, sees the line, music and sphere,
beats the three bodies, and sees the clear and the gates open.
