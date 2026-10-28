# Showa's Boss

The battle with The Boss in Showa's gang hideout, after Cosmic's
`ShowaBattle`. The analysis and its open questions are in v83ref's
`reports/b272-showa-battle.md`; the defaults shipped are under "b273" in
`lanes/QUESTIONS.md`.

All dialogue is placeholder text (`showa.*` keys in
`game-data/placeholder_text.toml`): no licensed script has any.

## The way in

Konpei (9120015) in Showa Town (801000000, `SHOWA_TOWN`) tells of the
hideout and takes anyone to its first map, Near the Hideout (801040000,
portal `in00`). From there the hideout's own portals lead on to its Armory
(801040004, `SHOWA_ARMORY`). Konpei (9120200) near the hideout takes
players back to town.

The Female Boss (9400121) on 801040003 comes back by its `mobTime` (four
hours, see [combat](./combat.md)) and drops the Lady Boss's Comb (4000138,
`LADY_BOSS_COMB`), the content pack's rate.

## The expedition

The battle is a squad event, `EventKind::Showa` (`net/src/events.rs`,
`SHOWA`), gathered like Zakum's (see [zakum](./zakum.md) and
`net/src/squads.rs`): Konpei (9120201, `npc_scripts::SHOWA_RECRUITER`) in the
Armory opens it for a leader of level 100 or more who holds a comb
(`Recruiter::needs`; the comb is checked, not taken), and others of level
100 join; the leader sets out with 3 to 30 (two in the harness,
`RUSTMS_EXPEDITION_MIN`). One expedition a channel fights at a time. A
member who left may go back in while it lasts.

It goes into its own copy of the arena, The Nightmarish Last Days
(801040100, `SHOWA_ARENA`), for an hour, and lasts until its last member
leaves; when its time is up everyone left goes back to the Armory. Konpei
(9120202) in the arena asks before the win whether to leave, and sends
those who say yes to the Armory; its line warns they can't come back,
but the squad's rejoin lets them.

## The boss

The arena's statue, reactor 8001000 (`shouwaBoss`, `SHOWA_STATUE`), is a
type-100 reactor that takes a comb dropped in its area. It is
`Action::SpawnAt` with no notice and no music: the first bodyguard
(9400112, `SHOWA_BODYGUARD`) comes out on the ground below (420, 160).
Mob.nx's `revive` chain does the rest: Bodyguard A dies into Bodyguard B
(9400113), and that into The Boss (9400300, `THE_BOSS`).

The Boss's death in the copy is `boss_falls` in the field
(`runtime/src/actor/field.rs`), as Papulatus's: the clear effect and jingle,
and the event's one stage cleared, which wins it. Every other mob in the
arena goes with it (`clear_mobs`).

## The win

After the win Konpei in the arena hands each member who talks to it one of
29 prizes at random (`SHOWA_REWARDS`, Cosmic's list with its 2022306
twice) and takes them on to the copy's clear map, Near the Hideout
(Beautiful Sky) (801040101, `SHOWA_CLEAR`). One with no room in Equip, Use
or Etc is told to make some first. Konpei (9120203) there takes them back
to town.

A member's death in the arena or the clear map before the win tells the
event (`EventCommand::ShowaFell`, from `died` in
`runtime/src/actor/client/combat.rs`). If no member died, the win brings
out the Slot Machine (9400114, `SLOT_MACHINE`) on the clear map at
(500, -50) (`EventActor::slot_machine`, `FieldMessage::BringOut`), and the
team reads a pink line (`showa.no_casualties`).

## NPCs

| NPC | Where | Script |
| --- | --- | --- |
| Konpei 9120015 | 801000000 | About the hideout, or to 801040000 |
| Konpei 9120200 | 801040000 | Back to town |
| Konpei 9120201 | 801040004 | The squad |
| Konpei 9120202 | 801040100 | Leave, or after the win the prize and on |
| Konpei 9120203 | 801040101 | Back to town |

## Tests

`net` has unit tests for the event (`events.rs`), the statue
(`reactors.rs`), and the squad and Konpeis (`npc_scripts.rs`). The
harness's `tests/showa_battle.rs` turns away a leader without a comb,
gathers two, has Konpei offer only the way out, drops the comb, beats the
three bodies, and sees the clear, the pink line, the prizes and the Slot
Machine; then gathers again, lets a member die, and sees no Slot Machine
and no line.
