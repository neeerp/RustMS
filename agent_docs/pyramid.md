# Nett's Pyramid

Nett's Pyramid, in the Ariant desert, is a run of five timed stages. Players
kill as many of its one-hit mobs as they can while a gauge runs down. The RE
is in v83ref `reports/b160-pyramid-server.md` (the rules, after Cosmic's
`Pyramid.java`; Cosmic's own Pyramid is a stub) and
`reports/b160-pyramid-client.md` (the v83 client's fields and packets).

Status: the map and mob data, the packet builders, the run's entry, stages
and clocks, its tally and gauge, its blessings and uses of Rage of Pharaoh,
its score, Duarte's medal, the Shades' gems and Pharaoh Yeti's Tomb are
in.

The Abandoned Subway's Dusty Platform, under Kerning City, is a run of the
same kind, with its own section below (`EventKind::Subway`). The two are
the Massacre runs (`EventKind::massacre`), and share the tally, the gauge,
the blessings, the score and the packets.

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
  commented-out `2103013.js`): About (three pages), Enter, the Tomb (below),
  the treasures and the medal. Enter asks alone or with a party, then the mode by its icon
  (3994115 to 3994118). Under level 40 he turns the player away; above 60
  only Hell is open. A party run needs its leader and one or more others,
  all at the mode's levels, in the lobby; the others stay behind. He asks
  the directory about all eight runs before he speaks
  (`ClientActor::talk_to_duarte`). Cosmic's Find a Party is not offered.
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
    (`ServerMessage::PyramidBlessing`): Booster and its aura (`combat.md`,
    "Item auras").
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

## The score (`pyramid::score`)

- A run that ends by its last stage's clock or its gauge has a rank, 0
  for S to 4 for D, and EXP, from its kills and cool kills together
  (Cosmic `sendScore`). A run whose players all left has none.
  - Outlasting the fifth stage's clock: S at 3000, A at 2000, B at 1500,
    C at 500, else D.
  - With the gauge run out: C at 2000, else D.
  - EXP by rank, with the mode's index m (0 to 3): S 60500 + 5500m,
    A 55000 + 5000m, B 46750 + 4250m, C 22000 + 2000m, D 0. Then 2 more for
    each kill and 10 for each cool kill.
- The event actor works it out once, when the run ends, and sends it to
  each member after `EventEnded` (`ServerMessage::PyramidScore`). A
  member that the end took to the result map gets PYRAMID_SCORE and the
  EXP, without the server's rates, as Cosmic's `gainExp` does.

## Protector of Pharaoh (quest 29932)

- Dalair (9000066) starts 29932 from level 40 and ends it when record 7760
  is 50000 or more (`RECORD_QUESTS`). The quest's own Act gives nothing.
- Every kill and cool kill a player makes on a Pyramid stage adds to their own 7760
  (`ClientActor::count_massacre_kills`, before the run hears of it), saved
  with the player and sent as a quest record. Misses don't count, and the
  count goes on after 50000.
- Duarte's fifth choice gives the medal, 1142142
  (`npc_scripts::duarte_medal`): not to a player who holds or wears it, nor
  under 50000 (he says how many are left), nor with a full Equip bag.
  Giving it ends 29932 too for a player on it. The words are placeholders
  (`pyramid.duarte.medal*`).

## The Shades' gems and Pharaoh Yeti's Tomb (`EventKind::PyramidTomb`)

The defaults chosen here, where Cosmic has nothing, are in lanes/QUESTIONS.md
b161.

- A run whose fifth stage's clock ran out, rather than its gauge, marks
  each player it took to the result map (`PyramidScore.cleared`, kept in
  `Player::pyramid_gem` as the mode, not saved). The mark lasts from the
  result map to the mode's room in the Shades; any other move clears it
  (`ClientActor::leaving_for`).
- Duarte in the Shades, to a marked player (`npc_scripts::duarte_shades`):
  the mode's Tomb now, or its gem, 4001322 + mode, and back to the lobby.
  The gem needs a free Etc slot. To anyone else he only offers the lobby.
- Duarte's Tomb choice in the lobby (`duarte_tomb`) lists the four gems.
  From level 40 he takes the one picked and sends the player in.
- The Tomb's rooms are 926010010 + 20·mode, one map each, played alone in
  the event's own instance. Up to 20 may go at once in each mode on a
  channel. Its event kind has no clock and lasts until the player leaves;
  it starts on the lobby or the mode's room in the Shades (`Event.entrances`,
  `Event::starts_on`).
- Each room opens with one Pharaoh Jr. Yeti at (0, 0) (`opening_mobs`): 9700019,
  or 9700029 on Hell. Both have 1 HP. 9700019 always drops Pharaoh's
  Treasure Chest 2022613 and 9700029 drops 2022618
  (`reward_overrides.toml`).
- The room's `out00` portal goes back to the lobby, which ends the event.
- The words are placeholders (`pyramid.duarte.tomb*`, `shades_*`).

## The Dusty Platform (`EventKind::Subway`)

The RE is in v83ref `reports/b318-dusty-platform.md`. Cosmic has only Mr.
Lim's script (`1052115.js`) and the Mirror's option; the run's rules are
the Pyramid's, with guesses noted in v83ref `lanes/QUESTIONS.md` (b319).

- The way in: the Dimensional Mirror's sixth place (Construction Site, 25
  to 30) lands on the Abandoned Subway Station (910320000) by its `out00`,
  saving the town as `DOJO_RETURN`. The Construction Site's own tunnel
  (`102040000` `subway00`, `met_in`) leads there too. The station's `out00`
  (`met_out`) goes back to the saved town, or else to the Construction
  Site's portal 12. Its `metro00` opens Mr. Lim, and `chat00` shows the
  `Direction2.img/metro/Im` effect.
- Mr. Lim (1052115) on the station: a menu of entering, Train 999 and the
  medal, in Cosmic's order. The party's leader takes in the members of
  level 25 to 30 on the map, one to six of them, in up to 5 runs at once
  on a channel. Without a party, or as a member, one hears the terms. On a
  stage he lets one leave for the station; on the result map (910320001)
  he offers Train 999 (below).
- The run: three stages, 910320100, 910320200 and 910320300, of 3 minutes
  each, each in the event's own copy (the NX's copies 1 to 4 are not
  used). After the third, or once the gauge runs out, the run ends on
  910320001, with PYRAMID_SCORE.
- The score: outlasting the third stage's clock gives S at 1930 kills, A
  at 1290, B at 964, C at 321, else D; with the gauge run out, C at 1290,
  else D. EXP by rank: S 13500, A 9500, B 5400, C 2700, D 0, then 2 for a
  kill and 10 for a cool kill (`pyramid::SUBWAY_RANKS`).
- Honorary Subway Worker (quest 29931): Dalair (9000066) starts it from
  level 25 and ends it when record 7662 is 10000 or more
  (`RECORD_QUESTS`). Every kill and cool kill on a stage adds to the
  player's 7662, and to 7762, which the quest's window shows
  (`pyramid::kill_records`); Train 999's don't. Mr. Lim's third choice
  gives the Honorary
  Employee Medal, 1142141, from 10000, as Duarte gives his: not to one who
  holds it, nor with a full Equip bag. It ends 29931 too, starting it first
  for one who never took it (`Effect::FinishQuest`).
- The stages' own `mobMassacre`: a gauge of 100 losing 1 a second, 1 back
  for a kill, 10 for a cool kill, 5 off for a miss; Power Buff Lv.1 and
  Lv.2 (2022616, 2022617) at 100 and 300 kills; no Rage of Pharaoh, which
  stays the Pyramid's; and `disableSkill`.
- Train 999 (`EventKind::SubwayTrain`): the bonus room "Platform - Train
  999" (910320010), one player alone at any level, for a minute in their
  own copy, up to 20 at once on a channel (its 20 rooms). The room has no
  life of its own: each copy opens with 30 Metro Bubblings (9700020; its
  `fixedMobCapacity`), 10 on each platform and 10 on the floor, which don't
  come back (`events::opening_mobs`). Each drops a Subway Lost and Found
  box (2022615) every time (`reward_overrides.toml`). The time running out,
  or its `out00`, takes one back to the station.
  - After a run that outlasted the third stage's clock, Mr. Lim on the
    result map offers it at once, or a Train 999 Boarding Pass (4001321)
    for later, which needs a free Etc slot, and back to the station
    (Cosmic's Last Platform, 910330001, gives the pass so). Such a run's
    `PyramidScore.cleared` sets `Player::subway_ride`, which leaving the
    result map clears. Anyone else there, as one revived there (the stages'
    `returnMap`), he sends straight back to the station.
  - On the station his second choice takes a pass for it (Cosmic
    `1052115`, which checks nothing else); without one, or with every room
    taken, he says so.
  - The minute, the drop rate and the pass on the result map are guesses
    (QUESTIONS b320).
- Not in: the 910330xxx set and its Last Platform.

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

## HeavenClient

`massacre::Session` (`Gameplay/Combat/Massacre.h`) keeps the four tallies
across fields, and the gauge's count, the stage's clock and its length for
the field. `UIPyramidHud` is up on field type 23 and draws, from
`UIWindow.img/MonsterKilling`, what v83 draws where it draws it
(`v83ref/reports/b328-pyramid-hud-positions.md`): the Count panel at
(1, 21), with the uses and the key only when the map's
`mobMassacre/disableSkill` is 0; the gauge at (251, 64), its bar ending at x
540; and the clock, `Map/Obj/etc.img/killing`, at (304, 5). On that field a
CLOCK of kind 2 sets the HUD's clock and puts up no other. `UIPyramidResult`
shows PYRAMID_SCORE on field type 24 at (207, 158) for 5 seconds, with no
button; a score that comes while the room is still loading waits for it
(`massacre::score`). The client tests are in `tests/massacre.cpp`.
