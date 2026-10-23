# Horntail

The way to Horntail in Leafre's Cave of Life, after Cosmic. Built so far:
the quests, the doors, and the ways out. The expedition, its trials, the
fight and the Horntail party quest are still to come. The slice plan and its
open questions are in v83ref's `reports/a115-horntail.md`; the defaults
shipped are under "a115" in `lanes/QUESTIONS.md`.

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

## Tests

`net`'s `keroben_and_the_slate_let_the_dragon_squad_on_toward_horntail`
covers the three NPCs, and `game-data` checks that 7300 is supported. The
harness's `tests/horntail_way.rs` walks a level-100 character from
Keroben's refusal through the shop, the slate and both ways out.
