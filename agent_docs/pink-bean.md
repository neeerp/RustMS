# Pink Bean

The battle with Pink Bean in the Temple of Time, after Cosmic's
`PinkBeanBattle`. The analysis and its open questions are in v83ref's
`reports/b274-pink-bean.md`; the defaults shipped are under "b283" and
"b284" in `lanes/QUESTIONS.md`.

All dialogue is placeholder text (`pink_bean.*` keys in
`game-data/placeholder_text.toml`): no licensed script has any.

So far the way in and out (slice 1) and the statue's guardians (slice 2)
are done. Pink Bean's own fight, the win, the deaths count and the drops
are still to come.

## The expedition

The battle is a squad event, `EventKind::PinkBean` (`net/src/events.rs`,
`PINK_BEAN`), gathered like Zakum's (see [zakum](./zakum.md) and
`net/src/squads.rs`). The gatherer (2141001, `npc_scripts::PINK_BEAN_RECRUITER`,
no name in String.nx) at the temple's last gate (270050000,
`PINK_BEAN_GATE`) opens it for a leader of level 120 or more, with no item.
Others of level 120 join, and the leader sets out with 6 to 30 (two in the
harness, `RUSTMS_EXPEDITION_MIN`). One expedition a channel fights at a
time. A member who left may go back in while it lasts.

It goes into its own copy of the Twilight of Gods (270050100,
`PINK_BEAN_ARENA`) for two hours and twenty minutes, and lasts until its
last member leaves. When its time is up, everyone left goes to Between
Twilight and Daybreak (270050300, `PINK_BEAN_OUT`), Cosmic's exit map and
every map's `forcedReturn`.

## Death and the ways out

The copy also holds the Forgotten Twilight (270050200, `PINK_BEAN_REVIVE`).
A member who dies in the copy comes back there (`EventTicket::revive_to` in
`runtime/src/actor/event.rs`), which is Cosmic's wz `returnMap`. Royals's
Map.nx gives 270050300 instead, which would leave the dead no way back.
The Forgotten Twilight's `in00` (portal script `Pinkin`) leads back to the
fight, in the same copy.

The expedition counts its dead (Cosmic `PinkBeanBattle.playerDead`): each
death in either of the copy's maps before the win tells the event
(`EventCommand::Fell`, from `died` in `runtime/src/actor/client/combat.rs`).
The third and fourth deaths warn every member in pink, and the fifth ends
the expedition, which sends everyone to Between Twilight and Daybreak
(`events::pink_bean_fallen`, `pink_bean.fallen.*`).

The Forgotten Temple Keeper (2141002, `npc_scripts::TEMPLE_KEEPER`) stands
in both of the copy's maps. Before the win and after it, the keeper asks
whether to leave and takes those who say yes to the gate. As in Cosmic,
there is no reward from the keeper.

Between Twilight and Daybreak's `in00` (`PPinkOut`) leads to the gate.

## The statue and its guardians

Every copy of the Twilight of Gods opens with Pink Bean's statue (8820000,
`mobs::PINK_BEAN_STATUE`) at (0,-42) (`events::opening_mobs`). The map's own
life holds the five guardians' statues (8820019-23), which come with the
field. The statue is warded (`FieldMobs::ward` in
`runtime/src/actor/field_mobs.rs`): unlike Zakum's or the Balrog's shields,
a ward keeps the statue's controller, so it still casts its skills, but no
attack or poison hurts it.

The first to arrive in the copy hears, in pink, that the first wave wakes
in 15 seconds (`pink_bean.wave.first`, `FieldMessage::GuardiansWake`).
Wave n wakes the first n guardians of `events::PINK_BEAN_GUARDIANS`, in
Cosmic's order: Solomon the Wise (8820003), Rex the Wise (8820004), Hugin
(8820005), Munin (8820006) and Ariel (8820002). Each comes out of one of
its statues, which goes, or at (5,-42) if none is left. When a wave's last
guardian falls, the next stirs 15 seconds later (`pink_bean.wave.next`).
A guardian's death leaves a statue behind (its `revive` in Mob.nx), which
the next wave wakes again.

When the fifth wave falls, the ward lifts and a 4001193 drops for anyone
on the statue's reactor (2708000, `reactors::PINK_BEAN_LIGHTNING`), with an
item line in the chat log (`pink_bean.wave.last`). The reactor takes it as
it takes any item dropped on it (`ITEM_TAKEN_AFTER` in the field, five
seconds), which strikes the statue dead (`reactors::Action::Lightning`).
However the statue dies, Pink Bean (8820001, `mobs::PINK_BEAN`) comes out
on the ground where it stood. The wave lines reach only those in the
fight, not the dead waiting in the Forgotten Twilight.

## The win

When Pink Bean dies in the copy, the map shows the party clear effect and
plays its sound, which wins the expedition (`boss_falls`), and every other
mob there goes, the guardians' statues too (`clear_mobs`). The world reads
an item line naming the channel whose expedition won (`pink_bean.victory`,
Cosmic `broadcastPinkBeanVictory`). The keeper then offers the way out
(`pink_bean.keeper.won`).

## Their skills

The skills are Mob.nx's, cast through the usual mob skill path (see
`combat.md`). The statue (8820000) heals, powers up, guards and summons
six 8820007 (200/102) while warded: a ward keeps its controller, so its
skills are still granted. Pink Bean (8820001) summons twelve 8820007
(200/101) at 80% HP, turns attacks back (145/2) and makes players Undead
(133/1). 8820007 is immune to weapon attacks (140/12), which then deal
at most 1 a line. Munin (8820006) Seduces up to `count` players (128/8-10)
and banishes one (129/2) to the copy's Forgotten Twilight (270050200,
portal `sp`), since a banish lands in the copy when the event's ticket
holds its map. Ariel (8820002) heals, dispels, lays Poison Mist, makes
players Undead, and turns weapons (140/9) or spells (141/8) aside. The picks are b286 in v83ref's QUESTIONS; the boss bar still shows
only once Pink Bean is hurt.

## Tests

- `net`: the event's maps and terms and the guardians' waves
  (`events.rs`), the reactor's action (`reactors.rs`), the gatherer's level and
  ask (`npc_scripts.rs`), the keeper, and the two portals (`scripts.rs`).
- Harness `pink_bean_entry`: two gather and set out. The keeper asks first,
  and a dead member comes back in the Forgotten Twilight and goes back to
  the leader through `Pinkin`. The member leaves and may rejoin, and the
  last out ends the expedition. `PPinkOut` leads to the gate.
- `runtime`: a warded mob keeps its controller and takes no harm until
  the ward lifts, and is still granted its skills (`field_mobs.rs`); an
  immune mob takes 1 a line from the kind it is immune to, and Seduce
  takes at most its count (`field.rs`).
- `net`: Pink Bean's and its guardians' skills and Munin's banish, from
  the NX data (`mobs.rs`).
- Harness `pink_bean_waves`: the five waves wake in order out of their
  statues, the warded statue takes no hit, and after the fifth the item
  falls on the reactor, the statue dies and Pink Bean comes out. Its fall
  shows the clear, takes every mob, and tells the world the channel.
- Harness `pink_bean_entry` also counts the dead: deaths in the revival
  room count, the third and fourth warn, and the fifth sends everyone out.
- `net`: which deaths warn and which ends it (`events.rs`).
