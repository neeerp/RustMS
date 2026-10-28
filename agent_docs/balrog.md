# The Balrog's Tomb

The battle with the Balrog in its tomb under Sleepywood, after Cosmic's
`BalrogBattle`. The analysis and its open questions are in v83ref's
`reports/b271-balrog-tomb.md`; the defaults shipped are under "b275" in
`lanes/QUESTIONS.md`.

All dialogue is placeholder text (`balrog.*` keys in
`game-data/placeholder_text.toml`): no licensed script has any.

## The expedition

The battle is a squad event, `EventKind::Balrog` (`net/src/events.rs`,
`BALROG`), gathered like Zakum's (see [zakum](./zakum.md) and
`net/src/squads.rs`): Mu Young (1061014, `npc_scripts::MU_YOUNG`) at the
temple's entrance (105100100, `MU_YOUNG_MAP`) opens it for a leader of level
50 or more, with no item, and others of level 50 join; the leader sets out
with 6 to 30 (two in the harness, `RUSTMS_EXPEDITION_MIN`). One expedition
a channel fights at a time. A member who left may go back in while it
lasts. Mu Young's Tristan's Tomb script (quest 2245) is the quest's, so a
plain talk is the squad's.

It goes into its own copy of the tomb (105100300, `BALROG_TOMB`) for an
hour, and lasts until its last member leaves; when its time is up everyone
left goes back to the temple's entrance. The tomb takes 50 HP every 15 s
(the map's `decHP`, see [combat](./combat.md)).

## The boss

The copy opens with the Balrog's right arm (8830002), its left claw
(8830006) and its body (8830000), in that order, on the ground below
(412, 258) (`events::opening_mobs`). A minute after the copy opens the
claw dies (`FieldMessage::Slay`, which the field sends itself through its
`later` list), and Mob.nx's `revive` lets out the left arm (8830001). Each
arm dies into a corpse (8830004, 8830005) that does nothing.

The body is shielded as Zakum's (`net::mobs::shields`,
`MobSet::shields` in `runtime/src/actor/field_mobs.rs`): it takes no harm
and has no controller while an arm or the claw is out, or due to come out
of a death. The second arm's death makes it real (`make_real`).

The body's death in the copy is `boss_falls` (the clear effect and jingle,
and the event's one stage cleared, which wins it), with a world item line
(`balrog.victory`), as Zakum's. Ten seconds later the sealed Balrog
(8830003) comes out at the same spot. Everyone in the tomb then counts a
kill of 9101003 (`mobs::BALROG_SEALED_CREDIT`, a `MobKilled` with no EXP
and no loot), the mob Mu Young's Balrog and the Seal (2239) asks for
(Quest.nx `Check.img/2239/1/mob`); no mob of that id ever comes out.

The content pack has no drops for the Balrog, so its body's come from
`game-data/reward_overrides.toml` (`[mobs.8830000]`): Cosmic's table, the
Balrog's weapons and shoes, potions, scrolls and its leather (60%), less its
Monster Book card, with one pile of about 7,000 mesos.

## The win

Mu Young in the tomb (1061018) asks before the win whether to leave, and
sends those who say yes to the temple's entrance; after it, it takes them
on to the clear map (105100301, `BALROG_CLEAR`). The clear map's way out
(`out99`, portal script `balog_end`) gives each a Piece of Balrog Leather
(4001261, `BALROG_LEATHER`) and leads back to Mu Young; one with no room in
Etc is told to make some.

The clear map's box (reactor 1052001, `reactors::BALROG_BOX`) breaks after
its hits and sprays its loot with piles of 500 to 999 mesos, at least 15
drops (Cosmic's `sprayItems`). Its loot is Cosmic's too
(`[reactors.1052001]`): each of 44 Balrog weapons, shoes and scrolls one
time in three.

The Suspicious Man (1061016, `npc_scripts::BALROG_SCROLLS`) in the
Balrog's hall (105100000) trades one leather for one of the Balrog's twelve
scrolls (2040728 to 2040739, 30% stat scrolls and the 5% Twilight Scroll),
the player's pick. One with no leather is told so; one with no room in Use
is told to make some.

## NPCs

| NPC | Where | Script |
| --- | --- | --- |
| Mu Young 1061014 | 105100100 | The squad |
| Mu Young 1061018 | 105100300 | Leave, or after the win on to 105100301 |
| Suspicious Man 1061016 | 105100000 | A scroll for a leather |

## Tests

`net` has unit tests for the event (`events.rs`), the shields (`mobs.rs`),
the squad, the tomb's Mu Young and the Suspicious Man (`npc_scripts.rs`),
the box (`reactors.rs`) and `balog_end` (`scripts.rs`); `game-data` checks
the drops load without the pack; `runtime`'s `field_mobs.rs` checks the body stays shielded
between the claw's death and the left arm's coming out. The harness's
`tests/balrog_tomb.rs` gathers two, has Mu Young in the tomb offer only the
way out, waits for the left arm, finds the body unhurt, beats both arms and
the body, sees the clear, the world line, the sealed Balrog and both
counting 2239's kill, breaks the clear map's box for its spray, takes both
through the clear map's way out for the leather, and trades one for the
Twilight Scroll.
