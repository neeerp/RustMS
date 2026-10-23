# Zakum

The way to Zakum in El Nath's Dead Mine, after Cosmic. Built so far: the
chiefs' approval, the Dead Mine party quest (stage 1, in
[party-quests](./party-quests.md)), the Breath of Lava (stage 2), the
forging of the Eyes of Fire (stage 3), the gate, the boss on its altar, and
its fall. Not yet: the expedition squad. The slice plan and its open
questions are in v83ref's `reports/a097-zakum.md`,
`reports/a105-zakum-dead-mine.md` and `reports/a106-zakum-boss.md`.

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
three it plays the portal sound and leads to the Entrance to Zakum's
`west00`. Upstream also keeps it shut while the gate reactor (2118002) is
closed after a win; nothing closes it yet.

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

Not yet built: the expedition that brings players to the altar and closes
it after the summoning. Upstream's gate reactor (2118002) shuts the Door
to Zakum while an expedition is in and opens again on its win or end, and a
member of a won expedition completes quest 100200 on leaving. Both belong
to the expedition: with none, nothing shuts the gate, and 100200 is the
client-invisible marker `ZAKUM_APPROVED` here.

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
- `integration-harness/tests/zakum_trials.rs`: a level-50 Fighter, unapproved
  and then without the trials, is turned back at the gate. They are approved
  by Tylus, cross the Breath of Lava, get the Breath of Fire from Lira, have
  the Eyes forged, and pass the gate.
