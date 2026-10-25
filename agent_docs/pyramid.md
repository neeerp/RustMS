# Nett's Pyramid

Nett's Pyramid, in the Ariant desert, is a run of five timed stages. Players
kill as many of its one-hit mobs as they can while a gauge runs down. The RE
is in v83ref `reports/b160-pyramid-server.md` (the rules, after Cosmic's
`Pyramid.java`; Cosmic's own Pyramid is a stub) and
`reports/b160-pyramid-client.md` (the v83 client's fields and packets).

Status: the map and mob data and the packet builders are in. The run itself
(Duarte, the stages, the gauge, the result room) is still to come.

## Maps

- The lobby, 926010000, is reached from Ariant by Nett (`nets_in`), and
  left by `nets_out` to the map kept in `PYRAMID_RETURN`.
- Stage maps: 926010000 + party·10000 + mode·1000 + stage·100 + copy, with
  party 0 (solo) or 1, mode 0 to 3 (easy, normal, hard, hell), stage 1 to 5
  and copy 0 to 4 solo or 0 to 9 party. Each copy only has `info` with a
  `link` to its stage's copy 0, which holds the body (see `fields.md`).
- Every stage map has field type 23 (the client's `CField_Massacre`),
  `fieldLimit` 271100, `forcedReturn` 926010000, `returnMap` 926010001,
  `info/fixedMobCapacity` 70 and `info/createMobInterval` 3000. So it holds
  70 mobs whoever is on it and refills every 3 s (`combat.md`, Spawning).
- The result maps 926010001 to 4 have field type 24
  (`CField_MassacreResult`) and send players on to 926020001 to 4.

## Stage rules (`FieldTemplate::massacre`)

Each stage's copy 0 has a `mobMassacre` node, read into
`game_data::Massacre`:

- `gauge`: `total`, and how much it loses each second (`decrease`), gets
  back for a kill (`hitAdd`) or a cool kill (`coolAdd`), and loses for a
  miss (`missSub`).

  | | total | decrease | hitAdd | coolAdd | missSub |
  |---|---|---|---|---|---|
  | solo easy / normal / hard / hell | 100 | 1 / 2 / 2 / 3 | 1 | 5 / 6 / 7 / 8 | 4 / 5 / 6 / 7 |
  | party easy / normal / hard / hell | 200 | 3 / 3 / 5 / 8 | 1 | 5 / 5 / 8 / 7 | 10 / 12 / 14 / 15 |

  All five stages of a mode share their values. The party rows differ from
  Cosmic's rules, which give every run 100 and 1, 2, 2 or 3 a second.
- `countEffect`: at kill counts 250, 500, 1000, 1500, 2000, 2500 and 3000.
  Those at 250, 500, 1000 and 2000 give Pharaoh's Blessing, items 2022585
  to 2022588. Every count from 500 gives a use of Rage of Pharaoh (1020;
  `skillUse`).
- `disableSkill` is set only on some Kerning City event rooms (910320xxx,
  910330xxx), which share the node.

The client reads only `disableSkill`, `gauge/total` and `gauge/decrease`; the
rest are the server's.

## Mobs

The Pyramid's mobs, 9700001 to 9700028, have 1 HP and three keys of their
own, in `game_data::MobInfo`:

- `cool_damage` (`coolDamage`) and `cool_damage_prob` (`coolDamageProb`, in
  percent): a hit of at least that much counts as a cool kill that often.
  9700001 has 200 and 9.
- `remove_on_miss` (`removeOnMiss`): a hit for 0 takes the mob off the map
  as a miss.

## Packets

- SESSION_VALUE (0x5A) carries the run's tallies under `massacre_hit`,
  `massacre_miss`, `massacre_cool` and `massacre_skill`. The client draws
  them on its Count panel and its result screen. It uses 1020 only while
  `massacre_skill` is above 0 (0x74d841).
- PYRAMID_GAUGE (0x9D, `build_pyramid_gauge`): an int. The client draws its
  gauge as the stage's elapsed seconds times `decrease`, less this value,
  kept within 0 and `total`, filling right to left. So the server sends what
  the run's kills and cool kills have put back, less its misses. The bar
  turns red at 65%.
- CLOCK (0x93) kind 2 gives the stage's length; on field type 23 the client
  ignores the other kinds.
- PYRAMID_SCORE (0x9E, `build_pyramid_score`), on the result map: a byte
  rank, 0 for S to 4 for D, then the EXP as an int. The tallies come from
  the session values sent before it.
