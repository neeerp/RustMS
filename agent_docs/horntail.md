# Horntail

The way to Horntail in Leafre's Cave of Life, after Cosmic. Built so far:
the quests, the doors, the ways out, the expedition into the cave, its
two trials, and the start of the fight. Its win and the Horntail party
quest are still to come. The slice
plan and its open questions are in v83ref's `reports/a115-horntail.md`; the
defaults shipped are under "a115" in `lanes/QUESTIONS.md`.

All dialogue is placeholder text (`horntail.*` keys in
`game-data/placeholder_text.toml`): no licensed script has any.

## The quests

The Dragon Squad's quests run client-side ([quests](./quests.md)):

- 7300: Moira (2081006) asks the player about the dragons. Its offer asks
  questions (`ask`, `stop`).
- 7301: hand in items for 300000 EXP and the Dragon Elixir (2210003), which
  turns the player into a dragon (a Morph buff, `-2210003`).
- 7302: the proofs of Zakum (4001083), Papulatus (4001084) and Pianus
  (4001085), with three more, for the Certificate of the Dragon Squad
  (4001086).

The content pack's mobs drop all of these.

## The doors

| NPC | Where | Script |
| --- | --- | --- |
| Keroben 2081005 | Cave of Life - Entrance, 240040700 | Turns away all but dragons and certificate holders. For the rest, a menu: 10 Power Elixirs (2000005) for 100,000 mesos, or, at level 100 and above, on to the Cave Entrance (240050000) |
| Encrypted Slate of the Squad 2083000 | Cave Entrance, 240050000 | With the certificate, asks, then goes on to the Entrance to Horntail's Cave (240050400) |
| Crystal of Roots 2083002 | 240050400 and the maps past it | Asks, then from a map past 240050400 goes to Cave : The Road in Between (240050600), else to 240040700's `out00` |

The road's `next00` portal (`hontale_out1`) leads back to 240050400, and
240040700's `out00` (`hontale_morph2`) to the Peak of the Big Nest; both are
plain warps in `net/src/scripts.rs`. Cosmic also lets in, with solo
expeditions turned on, those holding the three proofs; that is not in.

## The expedition

The Mark of the Squad (2083004) on the Entrance to Horntail's Cave gathers
it like Adobis gathers Zakum's ([zakum](./zakum.md), "The expedition"):
anyone of level 100 or more leads one, with no item, others of those levels
join, up to 30, and with at least 6 (`RUSTMS_EXPEDITION_MIN`; 2 in the
harness) its leader takes it in. It is the event `events::HORNTAIL`
(`EventKind::Horntail`): a copy of the cave's maps from the first trial
(240060000) to Horntail's Cave (240060200), entered on the first trial, for
120 minutes, after which everyone goes to the Road in Between (240050600).
As in Cosmic (`isExpeditionTeamLackingNow`), neither the leader's leaving
nor falling below six ends it, only the last member's leaving; until then
those on its roster may go back in at the Mark, and others are turned away.
Leaving the cave's maps, by the Crystal of Roots or otherwise, leaves it.

Not yet: Cosmic's `[Expedition]` entered and left lines (Zakum has none
either).

## The trials

The event's stages are the Cave of Trial I (240060000), the Cave of Trial
II (240060100) and Horntail's Cave, the last (`stage_maps`). The expedition's
copy of each trial, made when its first member comes onto it, holds its head, on the ground below where Cosmic's
`setup` puts it (`events::HORNTAIL_HEADS`): the Left Head (8810000) below
(960,120) on the first, the Right Head (8810001) below (-420,120) on the
second. Whoever kills one clears its trial's stage (the field sends
`ClearEventStage`; `member_clears`). Those on the trial see the party
quests' clear effect; the clear gives no EXP beyond the head's own.

Each trial's door (`next00`, script `hontale_BR`) takes the player on to
the next map, with the portal sound, once its stage is cleared
(`Effect::RequireStage`). Until then it says a sealed line (BROADCAST_MSG
type 6, Cosmic's `dropMessage(6)`) and lets no one through. Cosmic counts
dead heads (`defeatedHead` 1 for the first door, 2 for the second). With
one head per trial that is the same thing.

Not in:

- The trials' tremble reactors (2408003). They are touch reactors, which
  the server doesn't handle yet. Cosmic's says a line, puts out a fake
  Zakum body (8800000) and a second head.
- The `hontale_boss1/2` touch portals, which have no source script.

## The fight

Horntail's Cave holds a stone (reactor 2401000, `reactors::HORNTAIL_STONE`)
at (598,-91), which never comes back once broken. Its sixth hit
(`Action::Horntail`, `bring_out_horntail` in the field) does what Cosmic's
`2401000.js` does:

- plays `Bgm14/HonTale` (`HORNTAIL_MUSIC`);
- unless his summoning is already out, puts on the ground below (71,260)
  (`mobs::HORNTAIL_AT`) the summoning (8810026, `HORNTAIL_INTRO`), which
  leaves after a second, then the body (8810018, `HORNTAIL`) and its eight
  parts (8810002-09, `HORNTAIL_PARTS`), each spawned as out of the
  summoning (appearance -3 with its object id, `MobView::parent`), and
  shows the body's HP bar. The summoning's own `revive` lists the nine
  again; it is placed without it, so that killing it (it has 40,000 HP)
  brings out no second Horntail;
- in the expedition's copy, restarts its clock at 60 minutes
  (`EventCommand::RestartClock`, `HORNTAIL_FIGHT_MINUTES`; Cosmic's
  `restartEventTimer`), shown to each member as a new countdown;
- says a line to the map (`horntail.summoned`, BROADCAST_MSG type 6).

The parts share the body's HP (`Link` in `field_mobs`, after Cosmic's
`spawnHorntailOnGroundBelow`):

- What a part takes, from hits or from poison and other statuses, it also
  takes off the body, credited to the attacker, but the body keeps at
  least 1 HP. What a part heals or recovers, the body heals too.
- The body itself loses no HP to hits or statuses and gains none from
  heals.
- Hitting a part shows the body's HP bar (`MobSet::bar_for`), not the
  part's.
- A part's death lets out its dead part (8810010-17, its `revive`).

Not in yet:

- The win: Cosmic kills the body once all eight dead parts are on the map.
- HeavenClient shows the -3 children at once, rather than once their
  parent leaves as the original client does.

## Tests

`net`'s `keroben_and_the_slate_let_the_dragon_squad_on_toward_horntail`
covers the three NPCs, `the_mark_of_the_squad_gathers_horntails_expeditions`
the Mark, `events::tests` its end rule and stages,
`horntails_trial_doors_open_on_to_the_next_once_their_stage_is_cleared` the
doors, and `game-data` checks that 7300 is supported. The harness's
`tests/horntail_way.rs` walks a level-100 character from Keroben's refusal
through the shop, the slate and both ways out;
`tests/horntail_squad.rs` gathers an expedition, takes it in, turns another
away, lets its leader step out and back, and frees the channel once the last
has left; `tests/horntail_trials.rs` beats both heads and walks through
the sealed and then open doors into Horntail's Cave; `tests/horntail_boss.rs`
breaks the stone, checks the summoning, the body, the parts and the clock,
and beats the tail, which lowers the body's bar but leaves it alive.
`field_mobs`'s `horntails_parts_share_its_hp_but_never_kill_it` and
`net`'s `horntails_stone_breaks_on_its_sixth_hit` and
`a_mob_out_of_another_names_it` cover the rest.
