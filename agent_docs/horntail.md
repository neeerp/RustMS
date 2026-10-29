# Horntail

The way to Horntail in Leafre's Cave of Life, after Cosmic. Built so far:
the quests, the doors, the ways out, the expedition into the cave, its
two trials, the fight and its win, and the Nine Spirit's Nest, where the
egg Horntail drops hatches. The Horntail party quest, on the
Trial Grounds before the slate, is in
[party-quests](./party-quests.md) ("Horntail's Trial Grounds"). The slice
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

As at Zakum's shrine (see [zakum](./zakum.md)), a chat line tells those
in the expedition's cave when it closes (HORNTAIL_CAVE, 302,
`BossSite::HorntailCave`): on arrival, that it closes unless Horntail is
summoned within the time left; when his clock starts over, that it closes
in 60 minutes; and, if the time runs out before the win, that it has closed.
Cosmic never sends it.

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

## The win

When the eighth dead part (`mobs::DEAD_HORNTAIL_PARTS`) comes out, so that
all eight are on the map, the body dies, then each dead part from 8810017
down to 8810010 (`horntail_defeated` in the field, `MobSet::slay`; Cosmic's
`revive` with `isHorntailDefeated`). Each death is a kill like any other:
the body's EXP (none) and drops go by who hurt it, its parts' damage
included, and the dead parts, which no one hurt, give nothing.

His death (`horntail_falls`):

- in the expedition's copy, clears its last stage, a member clear
  (`member_clears` 3), which wins it: those in the cave see the party
  quests' clear effect, no one gets EXP, and every member's clock stops;
- gives each living player on the map his `buff`, Horntail Squad :
  Victory (2022108; see [combat](./combat.md));
- tells the world (`horntail.victory`, BROADCAST_MSG type 6, a placeholder),
  as Cosmic's `broadcastHorntailVictory` does.

Cosmic also counts the kill toward quests for every member in the cave
(`dispatchRaiseQuestMobCount`); here it counts, as any kill does, for
those who hurt him.

Not in yet:

- HeavenClient shows the -3 children at once, rather than once their
  parent leaves as the original client does.

## The Nine Spirit's Nest

Horntail drops the Nine Spirit's Egg (4001094) every time (the content
pack's drops). Dragon Squad's Mission (3706, from Raul 2081007) ends at
Nine Spirit's Baby Dragon (2081008), which is on no map: it hatches from
that egg in the Nine Spirit's Nest (240040611), after Cosmic's NineSpirit
event and its `dragonNest` and reactor `2406000` scripts.

- **The way in.** Below the Dangerous Nest (240040610), `dragonNest`
  leads, once 3706 is completed, to the hatched dragon's nest (240040612
  `out00`, where 3714 is; see [quests](./quests.md)). Before that, one
  holding an egg, or who has placed one (marker `NINE_SPIRIT_EGG_PLACED`,
  32700, for Cosmic's unknown quest 100203), goes in alone
  (`EventKind::NineSpirit`) onto the nest's `out00` (portal 1, as
  Cosmic's `changeMap(.., 1)`), with the portal sound; anyone else gets
  the pink `portal.nine_spirit_nest.blocked`.
- **The run.** Solo, any level, 5 minutes (Cosmic's and Server.NX
  FieldSet DragonBaby's `timeOut`), then back below the nest; its exit is
  the same. Eight may go at once on a channel, each in their own copy;
  with all eight taken, the pink `nine_spirit.nest.busy`.
- **The egg.** The nest's reactor `dragonBaby` (2406000) has no touch in
  our Reactor.nx (Cosmic's WZ adds `activateByTouch`), but the nest's
  touch portal `act01` (type 9, `summondragon`, no source script) sits on
  it. The first egg brought there is taken, the reactor goes to state 1
  (`Effect::SetReactorState`, the egg on the nest) and the marker is
  set. A hit then takes it to state 2: the Baby Dragon comes out on the
  ground below it, with a pink `nine_spirit.nest.hatched` to the map
  (`reactors::Action::Appear`), and 3706 can be completed at it from the
  quest window.
- **Coming back.** One with the marker who enters the nest finds the Baby
  Dragon waiting where the egg would be (`scripts::unscripted_map_enter`,
  `Effect::BringOutNpc`, which brings out an NPC unless it is there
  already). Cosmic does so for anyone without an egg and sets 100203 only
  at the hatch; here the marker is set at placing, so one who placed an
  egg and left before the hit still finds the dragon (v83ref QUESTIONS
  b322).

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
breaks the stone, checks the cave's closing lines, the summoning, the body,
the parts and the clock,
and beats the tail, which lowers the body's bar but leaves it alive; then
beats all eight parts and checks that the body and dead parts die, the
clear, the stopped clock, the leader's Victory buff, and the world's line.
`tests/nine_spirit_nest.rs` places an egg, hatches it, ends 3706 at the
Baby Dragon, sees the time run out, and takes `dragonNest` on to 240040612;
finds the dragon waiting on coming back without an egg; and is kept out
with neither. `net`'s
`the_nine_spirits_nest_takes_one_alone_to_place_the_egg_for_its_baby_dragon`
and `the_nine_spirits_nest_is_five_minutes_alone_back_below_it` cover the
scripts, the data and the event.
`field_mobs`'s `horntails_parts_share_its_hp_but_never_kill_it` and
`net`'s `horntails_stone_breaks_on_its_sixth_hit` and
`a_mob_out_of_another_names_it` cover the rest.
