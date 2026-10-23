# Zakum

The way to Zakum in El Nath's Dead Mine, after Cosmic. Built so far: the
chiefs' approval, the Breath of Lava (stage 2), the forging of the Eyes of
Fire (stage 3), and the gate. Not yet: the Dead Mine party quest (stage 1),
the fight itself, the altar, and the expedition squad. The slice plan and its
open questions are in v83ref's `reports/a097-zakum.md`.

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

- **Breath of Lava (stage 2):** with a Piece of Fire Ore (4031061, from the
  Dead Mine) and no Breath of Fire (4031062), yes goes to 280020000, portal 0.
  Holding the Breath of Fire, he says the stage is done; otherwise, he says to
  do the earlier trials.
- **Forging the Eyes of Fire (stage 3):** with the Fire Ore and the Breath of
  Fire and 30 Zombie's Lost Gold Teeth (4000082), he takes them all, gives
  five Eyes of Fire (4001017) and marks `ZAKUM_TRIALS_DONE`. He says what is
  missing otherwise, and that the Etc bag is full if it is.

Upstream's first choice, the Dead Mine party quest, comes with its slice.
Choices are by position, so the menu indexes shift when it does.

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

## Tests

- `npc_scripts::tests::a_chief_approves_zakum_and_adobis_holds_its_trials`
- `integration-harness/tests/zakum_trials.rs`: a level-50 Fighter, unapproved
  and then without the trials, is turned back at the gate. They are approved
  by Tylus, cross the Breath of Lava, get the Breath of Fire from Lira, have
  the Eyes forged, and pass the gate.
