# Zakum

The way to Zakum in El Nath's Dead Mine, after Cosmic. Built so far: the
chiefs' approval, the Dead Mine party quest (stage 1, in
[party-quests](./party-quests.md)), the Breath of Lava (stage 2), the
forging of the Eyes of Fire (stage 3), the gate, the expedition squad, the
boss on its altar, and its fall. The slice plan and its open questions are
in v83ref's `reports/a097-zakum.md`, `reports/a105-zakum-dead-mine.md`,
`reports/a106-zakum-boss.md` and `reports/a108-zakum-squad.md`.

All dialogue is placeholder text (`zakum.*` keys in
`game-data/placeholder_text.toml`): no licensed script has any.

## Progress

Upstream tracks the way in two quests that the client data lacks (100200,
approved, and 100201, trials done). Here they are script markers (see the
third job in [npc-dialogue](./npc-dialogue.md)): Started records the client
never sees, `npc_scripts::ZAKUM_APPROVED` (32400) and `ZAKUM_TRIALS_DONE`
(32401).

## Approval

The third job's chiefs in El Nath (Tylus 2020008, Robeira 2020009, Rene
2020010, Arec 2020011, Pedro 2020013) approve players for Zakum. A second job
of the chief's line from level 70 picks from a menu: the third job's test or
Zakum. Once they hold the Necklace of Wisdom for the Holy Stone's assignment,
the chief speaks only of the advance. Any other job of the chief's class from
level 50 (`job / 100 % 10`, so Aran and Cygnus too, as upstream) gets the
approval at once. Approval marks `ZAKUM_APPROVED` the first time; the chief
says the same words after.

## Adobis and the trials

Adobis (2030008) stands at the Door to Zakum (211042300). Unapproved
players get a warning, which also tells those of level 50 and up what to do.
Approved ones get a menu:

- **The Dead Mine (stage 1):** the party's leader takes it in
  (`talk_about_event`, see Zakum's Dead Mine in
  [party-quests](./party-quests.md)). The win's prize includes the Piece of
  Fire Ore the next two stages need.
- **Breath of Lava (stage 2):** with a Piece of Fire Ore (4031061, from the
  Dead Mine) and no Breath of Fire (4031062), yes goes to 280020000, portal 0.
  Holding the Breath of Fire, he says the stage is done; otherwise, he says to
  do the earlier trials.
- **Forging the Eyes of Fire (stage 3):** with the Fire Ore and the Breath of
  Fire and 30 Zombie's Lost Gold Teeth (4000082), he takes them all, gives
  five Eyes of Fire (4001017) and marks `ZAKUM_TRIALS_DONE`. He says what is
  missing otherwise, and that the Etc bag is full if it is.

At the far end of the Breath of Lava (280020001), Lira (2032003) gives the
Breath of Fire and 10,000 EXP and sends the player back to the Door to
Zakum. Amon (2030010) asks before taking players out: to the Door to Zakum
from the Breath of Lava, and to the Entrance to Zakum (211042400) from the
altar (280030000).

Where this differs from Cosmic:

- Amon's own script sends players to the Door to Zakum from every map,
  the altar included. Here the altar leads to the Entrance to Zakum, the
  exit of Cosmic's ZakumBattle event.
- The chief skips the menu once the player holds the Necklace of Wisdom.
  Choosing Zakum never takes the Necklace of Strength; in Cosmic, any choice
  takes it.
- Lira's 10,000 EXP is not multiplied by the server's EXP rate.
- Adobis doesn't warp holders of item 4001109 to 921100000, a Cosmic
  custom.

## The gate

The Door to Zakum's portal `ps00` (script `Zakum05`, in `scripts::portal`)
checks in turn for the approval, the finished trials and an Eye of Fire,
turning the player back with a pink line at the first missing one. With all
three it then needs the gate reactor (2118002, named `gate`) not shut
(`Effect::ShutByReactor`, `events::ZAKUM_GATE`): while an expedition
fights it is in state 1, and the portal turns the player back with
`zakum.gate.blocked`. Otherwise it plays the portal sound and leads to the
Entrance to Zakum's `west00`.

## The expedition

Players reach the altar in an expedition (Cosmic's `Expedition` and
`ZakumBattle`), gathered by 2030013 at the Entrance to Zakum (211042400)
whatever their parties. The rules are `net::squads::Squad`; the directory
keeps one squad of each kind per channel (`directory/squad.rs`), and the NPC
asks it what the player finds (`SquadView`) and tells it what they chose
(`SquadCommand`, through `Effect::Squad`). The same squads gather Horntail's
expedition ([horntail](./horntail.md)): `npc_scripts::recruiter` gives each
kind's NPC and the item one needs to lead one, and `squads::text` the lines
that name its foe; the rest (`squad.*` keys) is every kind's.

- **Opening:** a player of level 50 or more holding an Eye of Fire (only
  checked) leads a new squad. Those on their map read a notice (chat line,
  BROADCAST_MSG 6), and the leader's clock counts down the five minutes it
  has to set out (`squads::REGISTRATION`, not scaled). One that doesn't set
  out in time lapses quietly, found so the next time anyone asks.
- **Gathering:** others of level 50 or more join it, up to 30; the members
  read a notice, and the joiner's clock counts down what is left. The
  leader's menu turns a member out for good (they read a notice and their
  clock stops), sets out, or calls it off. Members can't leave it, and
  if the leader goes it just runs out of time.
- **Setting out:** with at least 6 members (`Event::least`,
  `RUSTMS_EXPEDITION_MIN` overrides it; the harness uses 2), those of them
  on the Entrance to Zakum and the channel go to the altar, however few;
  the rest of the roster may come in later (Cosmic counts the whole roster
  too). It is the event `events::ZAKUM` (`Event::squad`): the
  altar is a copy of 280030000 of its own, and it lasts 120 minutes, scaled
  by `RUSTMS_EVENT_SCALE` like the party quests, before everyone is sent
  to the Entrance to Zakum. Unlike the party quests it doesn't end when the
  leader leaves or too few are left, only when the last member has gone.
  Its win frees the channel at once, as a party quest's does: the next
  squad may gather and set out while the winners are still on their altar.
- **The gate:** setting out shuts the gate on the Door to Zakum (the event
  sends `FieldMessage::SetReactor` to the channel's 211042300). It opens
  again when Zakum falls or the run ends otherwise. The end of a won run
  leaves it alone, as a later expedition may have shut it.
- **Coming back:** while the run lasts, a member on its roster who stepped
  out, or wasn't there at the start, may go in through 2030013 (`EventMessage::Join`,
  `Team::rejoin`), until Zakum falls. Anyone else hears that an expedition
  is under way.
- **The win:** Zakum's fall on the expedition's altar clears its last stage
  (`ServerMessage::ClearEventStage`), so the run is won.
- **The shrine's time:** v83's ZAKUM_SHRINE (303, `FUN_0053347c`; byte
  summoned, int minutes) writes a chat line from the client's own strings:
  the shrine "will close if you do not summon Zakum in" so many minutes, or
  once it is out "will close in" them, or at 0 that it "has closed"
  (HORNTAIL_CAVE, 302, is the same for Horntail's Cave). The server sends
  it (`build_boss_site_closing`) to each member arriving on the altar,
  summoned if there are monsters there, to everyone there as the Eye
  brings Zakum out (`ServerMessage::BossSummoned`), and as "closed" when
  the time runs out, before they are sent back. The minutes are the run's
  time left, rounded up. Nothing closes the shrine sooner for a Zakum
  never summoned: no source says how soon that is.

Where this differs from Cosmic (the choices are in v83ref's
`lanes/QUESTIONS.md`):

- A squad lapses quietly instead of being dissolved by a timer, with no
  notice.
- The leader may come back in like any member.
- Leaving a won run doesn't complete quest 100200 (the approval marker
  `ZAKUM_APPROVED` stands in for it).
- There is no daily limit (Cosmic's is off by default too).

## The boss

An Eye of Fire dropped on the altar's reactor (2111001, at the Zakum altar
280030000) breaks it (`reactors::Action::Zakum`, Cosmic's `2111001`
script). The field (`bring_out_zakum`) then does the following, in order:

- It plays Zakum's music to the map (FIELD_EFFECT 6, `Bgm06/FinalFight`).
- It brings out the body (8800000) on the ground below the altar. The body
  trembles in (SPAWN_MONSTER appearance -4, `MobView::fake`), and the v83
  client shakes the screen for it.
- It brings out the eight arms (8800003-8800010) on the same spot, under the
  first player.
- It sends a pink line to the map (placeholder `zakum.altar.summoned`).

While any arm is on the map, the body is shielded
(`net::mobs::shielded_by`, `MobSet::shielded`, Cosmic's
`MapleMap.damageMonster`):

- It has no controller, however near a player comes (MOB_DISTANCE) or
  hits it.
- Attacks on it are dropped whole: no HP, no HP bar, no aggro and no
  statuses. Skills that only put statuses on it (mists, debuffs) put none,
  and poison already on it takes no HP. Its arms' own buffs still reach it,
  as in Cosmic.
- Dazzled mobs can't hurt it either.

When the last arm dies, `make_real` makes the body real. It sends the body
again to everyone, just there (-1), and gives control of it to the first
player. Cosmic's `makeMonsterReal` does the same. From there Mob.nx's
`revive` chain does the rest: the body dies into 8800001, and that into
8800002, which alone has drops and EXP.

Players who come in while an arm lives see the body plainly (-1). Mobs, and
Zakum's summons among them, summon nothing on a field that already holds 80
mobs.

When 8800002 dies on the altar map (anywhere else it is just a mob),
`zakum_falls` shows the map the party quests' clear
effect (`quest/party/clear`) and plays their jingle (`Party1/Clear`). It
also has the first player's client that takes the message tell the whole world (a chat line that
links no item, BROADCAST_MSG 6; placeholder `zakum.victory`), through
`ServerMessage::Announce`, since a field has no way to the directory. This
is Cosmic's `ZakumBattle.monsterKilled` and `broadcastZakumVictory`.

On an expedition's altar the fall also wins the expedition (see above). A
debug warp still reaches the channel's own altar map, outside any
expedition, where the boss works the same.

## Tests

- `npc_scripts::tests::a_chief_approves_zakum_and_adobis_holds_its_trials`
- `integration-harness/tests/zpq_dead_mine.rs`: the Dead Mine, in
  [party-quests](./party-quests.md).
- `field_mobs::tests::zakums_arms_shield_its_body_until_the_last_dies`
- `integration-harness/tests/zakum_boss.rs`: an Eye of Fire dropped on the
  altar brings out the music, the trembling body and its arms. Hits on the
  body do nothing, and no one moves it, until all eight arms are dead. Then
  it comes back plainly under control, and dies through 8800001 to 8800002.
  Its last death shows the map the clear effect and jingle, and a player on
  another map reads the victory line.
- `squads::tests` and `events::tests::a_squad_*`: gathering, banning,
  starting, the size limit, lapsing, and coming (back) in.
- `integration-harness/tests/zakum_squad.rs`: a leader opens a squad and
  two more join it from no party, one wandering off before the start. The
  other two go to the altar; the gate shuts and turns a fourth player back,
  as does the recruiter. The one who wandered off comes in late, and the
  member steps out and comes back, both to the leader's altar. Zakum
  falls, the gate opens, and the fourth player may gather a new squad.
  The leader reads the shrine's time on arrival and as Zakum comes out.
  The new squad never summons it: its time runs out, the shrine has
  closed, and its members are back at the Entrance.
- `conti::tests::a_boss_site_says_whether_the_boss_is_out_then_the_minutes_left`
- `integration-harness/tests/zakum_trials.rs`: a level-50 Fighter, unapproved
  and then without the trials, is turned back at the gate. They are approved
  by Tylus, cross the Breath of Lava, get the Breath of Fire from Lira, have
  the Eyes forged, and pass the gate.
