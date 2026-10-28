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
(8830003) comes out at the same spot.

## The win

Mu Young in the tomb (1061018) asks before the win whether to leave, and
sends those who say yes to the temple's entrance; after it, it takes them
on to the clear map (105100301, `BALROG_CLEAR`). The clear map's way out
(`out99`, portal script `balog_end`) gives each a Piece of Balrog Leather
(4001261, `BALROG_LEATHER`) and leads back to Mu Young; one with no room in
Etc is told to make some.

## NPCs

| NPC | Where | Script |
| --- | --- | --- |
| Mu Young 1061014 | 105100100 | The squad |
| Mu Young 1061018 | 105100300 | Leave, or after the win on to 105100301 |

## Tests

`net` has unit tests for the event (`events.rs`), the shields (`mobs.rs`),
the squad and the tomb's Mu Young (`npc_scripts.rs`) and `balog_end`
(`scripts.rs`); `runtime`'s `field_mobs.rs` checks the body stays shielded
between the claw's death and the left arm's coming out. The harness's
`tests/balrog_tomb.rs` gathers two, has Mu Young in the tomb offer only the
way out, waits for the left arm, finds the body unhurt, beats both arms and
the body, sees the clear, the world line and the sealed Balrog, and takes
both through the clear map's way out for the leather.
