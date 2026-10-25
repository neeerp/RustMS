# Nett's Pyramid

Nett's Pyramid, in the Ariant desert, is a run of five timed stages. Players
kill as many of its one-hit mobs as they can while a gauge runs down. The RE
is in v83ref `reports/b160-pyramid-server.md` (the rules, after Cosmic's
`Pyramid.java`; Cosmic's own Pyramid is a stub) and
`reports/b160-pyramid-client.md` (the v83 client's fields and packets).

Status: the map and mob data, the packet builders, the run's entry, stages
and clocks, its tally and gauge, and its blessings and uses of Rage of
Pharaoh are in. The score and the Tomb are still to come.

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

## The run (`net::events`, `EventKind::Pyramid`)

Each mode has two event kinds, `EventKind::Pyramid(mode)` alone and
`EventKind::PyramidParty(mode)` with a party (`PYRAMID`, `PYRAMID_PARTY`,
built by `pyramid()`). Their maps are the stage maps' copy 0, played in the
event's own instance (see `party-quests.md`), so the copy digit in the ids
is unused. Five runs alone and ten with a party may go at once in each
mode on a channel, as many as the maps have copies.

- Duarte (2103013) in the lobby (`NpcScript::duarte`, after Cosmic's
  commented-out `2103013.js`): About (three pages), Enter, and the
  treasures. Enter asks alone or with a party, then the mode by its icon
  (3994115 to 3994118). Under level 40 he turns the player away; above 60
  only Hell is open. A party run needs its leader and one or more others,
  all at the mode's levels, in the lobby; the others stay behind. He asks
  the directory about all eight runs before he speaks
  (`ClientActor::talk_to_duarte`). Cosmic's Find a Party, the Tomb and the
  medal are not offered yet.
- The first stage lasts 2 minutes (the event's `minutes`), the others 3
  (`PYRAMID_STAGE_TIME`; Cosmic `Pyramid.timer`). When a stage's clock
  runs out the event actor moves everyone on to the next one, with the new
  clock (`EventActor::next_pyramid_stage`). The fifth's ends the run, which
  sends everyone to its exit, the mode's result map. CLOCK kind 2 shows
  each stage's time.
- The run lasts until its last player leaves, whoever leads.
- Duarte on a stage lets a player give up (`duarte_forfeit`): they go back
  to the lobby and leave the run. On a result map he sends them on to the
  mode's room in the Shades, and from there back to the lobby
  (`duarte_after`).
- The lobby's `piramid_in00` portal (a script portal) isn't modelled;
  Duarte is the only way in.

## The tally and the gauge (`net::pyramid`)

- On a copy of a stage (`FieldActor::massacre`), each attack's kills, cool
  kills and misses go to the attacker's client as
  `ServerMessage::Massacre`, and from there to the run as
  `EventCommand::Massacre`, whoever made them.
  - A kill is cool when the hit was as hard as the mob's `coolDamage`,
    `coolDamageProb` percent of the time (`pyramid::cool`).
  - A hit for 0 on a `removeOnMiss` mob is a miss: the mob goes, giving
    nothing, and its spawn point fills again (`MobSet::missed`). This
    holds on any map, though only the Pyramid's mobs have the flag.
- The run keeps a `Tally` (Cosmic `Pyramid.kill`, `cool`, `miss`): the
  counts, and `count`, what the stage's kills and cool kills have put back
  on the gauge less what its misses took off.
  - A kill puts back `hitAdd` and a cool kill `coolAdd`, each at most what
    the gauge has lost; a miss always takes `missSub`.
  - The gauge has lost the stage's whole seconds times `decrease`, less
    `count`. Once that reaches `total` the run ends, which sends everyone to
    its result map. The event actor times this (`gauge_out`), and times it
    again after each change.
  - The seconds aren't scaled as the event clocks are. The client's gauge
    goes down with the seconds its clock shows, so at the harness's 60x
    speed a stage's gauge loses at most 2 or 3 seconds' worth.
- After each attack the team's clients get the counts it changed and the
  gauge: SESSION_VALUE `massacre_hit`, `massacre_cool` or `massacre_miss`,
  then PYRAMID_GAUGE with `count`. At each stage's start `count` goes back to
  0, and they get all four counts (with `massacre_skill`) and the gauge, sent
  after the move so that the gauge reaches the new stage. They go as
  `ServerMessage::EventPackets`, which a client sends on only while its
  player holds the run's ticket and is on one of its maps.
- Cosmic also sends `massacre_party` and `massacre_laststage`. The v83
  client's Pyramid fields read only the four counts, so these aren't sent.

## Blessings and Rage of Pharaoh

- The run's kills and cool kills together, over all its stages, reach the
  stage's `countEffect`s in turn, each once (`Tally::reached`; Cosmic
  `checkBuffs`). One attack may pass more than one.
  - One with a `buff` gives every living member on the run's maps that
    item's effect, Pharaoh's Blessing 2022585 to 2022588
    (`ServerMessage::PyramidBlessing`).
  - One with `skillUse` gives the run a use of Rage of Pharaoh, and the
    members hear `massacre_skill` again.
  - Cosmic writes the counts in code, and never moves past 1500 (it adds a
    use there on every kill). The data's seven counts are used instead.
- The run's uses are shared by its members. Each client actor keeps the
  last count it was told in `Player::pyramid_rage`
  (`ServerMessage::PyramidRage`), cleared when it leaves the run.
- Rage of Pharaoh (1020, and its Cygnus, Aran and Evan copies) is refused
  without a use (`AttackHandler`), as the client does. With one, the
  attack takes it and tells the run (`EventCommand::Rage`). The run spends
  one if any are left and tells everyone how many remain. Two members who
  use the last one at once both get their attack.

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
