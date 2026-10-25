# Mu Lung Dojo

The Mu Lung Dojo's 38 floors, taken alone or with a party. The survey is v83ref's
`reports/b158-dojo.md`; it plans six slices, and this page says which are
in. Slice 1 is: the way in and out, the floors and their bosses, the
door, and So Gong's solo run. Slice 2 is the section timers and the
entrance's taunt. Slice 3a is training points, the rooftop's EXP, and the
score So Gong records. Slice 3b is the belts, the points reset and the
medals. Slice 3c is So Gong's first-time trial. Slice 4 is the party
Dojo. Slice 5 is the energy bar and its secret skills, and slice 5b Power
Explosion's buff. Slice 6 is the rest of the client's `muruengRaid` HUD:
its clock and the two HP gauges.

## The way in

| Map | Id | What is there |
| --- | --- | --- |
| Mu Lung, the board's map | 250000100 | the notice board 2091006 |
| Entrance | 925020000 | `out00` (`dojang_exit`) back out, `in00` to the hall |
| Hall | 925020001 | So Gong 2091005, the event's lobby |
| Exit | 925020002 | where a challenger who gives up or runs out of time lands |
| Rooftop | 925020003 | after the 38th floor |
| Trial room | 925020010 | So Gong as monster 9300269, and `out000` (`dojang_tuto`) back to the hall |
| Floor n | 925020100 + (n - 1) * 100 | So Gong again, and on a fighting floor a boss |
| Party floor n | 925030100 + (n - 1) * 100 | the same as floor n, by its `info/link` |

- **The board** (`npc_scripts::dojo_board`, Cosmic `2091006`) has two
  choices.
  - Its challenge asks first. On yes it saves the map it was touched on as
    `DOJO_RETURN` (Cosmic's `MIRROR`), then warps to the entrance's
    portal 4.
  - Its notice is two pages.
- **The taunt.** Arriving on the entrance, the player sees one of three
  taunts (`dojo.taunt.*`) on weather 5120024's message bar, for them
  alone, taken down 30 s later (`taunt_challenger`, Cosmic `dojang_Msg`'s
  `startMapEffect`). Cosmic shows it only when the closest spawn point is
  portal 0, but the entrance's only one is, so it shows on every arrival.
- **The way out** is the entrance's `dojang_exit` (`scripts::portal`). It
  goes back to the saved map and clears it. With nothing saved it goes to
  the board's map, not Cosmic's Henesys.
- **So Gong in the hall** (`npc_scripts::so_gong`) says below level 25
  that the challenger is too weak. Otherwise he offers six choices, in
  Cosmic's order:
  - taking him on alone, which is `EventKind::Dojo`;
  - taking him on with a party (see "The party Dojo");
  - a belt, the points reset and a medal (see "Belts and medals");
  - what the Dojo is.

  With a score recorded (see "Points and the record"), taking him on
  alone first asks whether to go back to its round. Yes starts the run
  after the recorded floor; no erases the record and ends the talk.

  A challenger who has not beaten his trial (see "The first-time trial")
  is offered that instead of the Dojo, with a record or without.

## The event

`events::DOJO` is an `Event` with a few twists:

- `team` is `1..=1`, so `Event::solo` is true:
  - the directory's `event_entry` takes the asker alone, party or not,
    with no leader check;
  - the client starts it without waiting for a party (`event_started`);
  - a ticket is never handed over (`EventCommand::HandOver`).
- `runs` is 15: the directory keeps the ids of each kind's runs per channel
  (`WorldDirectory::events`), and an event is busy once it has `runs` of
  them. Every other event has 1.
- `stride` is 100, so `stage_on` gives each floor its number. All 38
  stages clear one by one (`member_clears`).
- `Cleared` keeps 64 bits, for the 38 floors.
- `minutes` is 5, the first section's limit.

## The clock

The run goes by sections: floors 1 to 5, then each rest floor's next five,
and the last 37 and 38. Each has its own time (Cosmic
`Channel.startDojoSchedule`), and the time running out ends the run, at
the exit.

| Section | Floors | Minutes |
| --- | --- | --- |
| 1 | 1–5 | 5 |
| 2 | 7–11 | 6 |
| 3 | 13–17 | 7 |
| 4 | 19–23 | 8 |
| 5 | 25–29 | 9 |
| 6 | 31–35 | 10 |
| 7 | 37–38 | 15 |

- Reaching a section's first floor after the first starts the clock over
  with its minutes (`events::clock_on` gives `ClockOn::Restart`). The
  client tells the event on the way there (`EventCommand::Reached`), as
  for the Pirate Ship, and the event shows the new clock to its member
  (`restart_clock`).
- A rest floor stops the clock (`ClockOn::Stop`, `stop_clock`). The event's
  `ends` is `None`, the ticket's clock too, and the client takes the
  countdown down (STOP_CLOCK) and shows none there. The challenger may
  stay as long as they like.
- The challenger is sent out 3 seconds after the clock shows 0
  (`DOJO_OVERTIME`, `EventActor::overtime`), as Cosmic's task runs 3000 ms
  after the time it shows.
- Cosmic frees the run's slot on a rest floor (`dismissDojoSchedule`) and
  takes one again for the next section. Here the run keeps its slot
  throughout, so a challenger on a rest floor always finds room to go on.

## A floor

Every sixth floor (6, 12, ..., 36) is a rest floor: no boss, no door, no
`dojang_up` (`events::rest_floor`). The other 32 are fighting floors.

- **The boss.** Floor n's round is n less its rest floors before it
  (`dojo_round`), and its boss is 9300183 plus the round
  (`events::dojo_boss`): Mano 9300184 on the first, 9300215 on the 38th.
  - The field of an event's copy brings it out on the first join
    (`FieldActor::dojo_boss`, `bring_out_dojo_boss`).
  - It comes at one of three spots chosen at random (`DOJO_BOSS_AT`), on
    the ground below, with summon effect 15.
  - Its data marks it a boss, but it comes out as an ordinary mob, as
    Cosmic's `setBoss(false)` has it: statuses and MP Eater take, and
    Power Guard is not halved.
- **The challenger's arrival** on a fighting floor shows the stage start
  (`show_dojo_floor`): the sound `Dojang/start` and the effects
  `dojang/start/stage` and `dojang/start/number/<round>`.
- **The clear.** The boss's death clears the floor (`cleared_by_boss`).
  The challenger hears `Dojang/clear` and sees `dojang/end/clear`, with no
  EXP line.
  - The boss's revive, the invincible marker 9300216, comes out as any
    revive does. It is what Cosmic's `dojang_up` looks for.
- **`dojang_up`** (portal 7, by the door, `ClientActor::dojo_portal`) is
  shut with a pink line until the floor is cleared. Then:
  - the server sends `DOJO_WARP_UP` (0xCF: byte 0, then portal 6, the
    upper tier's `out01`);
  - and opens the `door` reactor (state 1) for everyone there.
- **`dojang_next`** (portal 8, behind the door) is shut with a pink line
  until the floor is cleared. Then it leads to the next floor's portal 0,
  or from the 38th to the rooftop (`scripts::dojo_next`).
  - Cosmic wants the door open, not just the floor cleared, but the
    portal is on the upper tier `dojang_up` leads to, so that makes no
    difference in play. Its 3-second cooldown is left out.
- **So Gong on a floor** (`npc_scripts::so_gong_on_floor`):
  - On a fighting floor he asks whether the challenger gives up. Yes goes
    to the exit's portal 0 with a pink line.
  - On a rest floor he offers going on to the next floor, or leaving after
    a yes to the exit's `st00` (portal 1), or recording the score (see
    "Points and the record"). Going on erases a recorded score.

## Points and the record

`game::DojoRecord` is a player's `dojo`: training points, the recorded
rest floor (`stage`, 0 for none), the medal's stage and wins, and whether
the first-time trial is beaten (`tutorial_done`). It is saved with the player, in
`dojo_records`, one row per character once it differs from a new one's
(`save_dojo`). Cosmic keeps these as `dojoPoints`, `lastDojoStage`,
`finishedDojoTutorial`, `vanquisherStage` and `vanquisherKills`.

- **Points.** Leaving floor n (1 to 37) by `dojang_next` earns
  `2 + n / 6` (`DojoRecord::floor_points`, Cosmic `addDojoPointsByMap`
  with one for going alone): 2 on floors 1 to 5, up to 8 on 36 and 37.
  With a party it is `1 + n / 6`.
  None are earned once the points reach 17,000 (`MOST_POINTS`). A pink
  line says how many, and the total (`dojo.points`), even when that is 0.
  - Cosmic's formula reads the map id, whose last two digits are the
    copy; RustMS's copies are instances of one map id, so the floor
    number is used directly.
- **The rooftop.** From the 38th floor `dojang_next` goes to the rooftop
  and gives 2,000 EXP for each point held, with no points for the floor.
- **The record.** On a rest floor So Gong's third choice asks a yes or
  no (`Page::Decide`):
  - yes records the floor (`Effect::DojoStage`), or says it is already
    recorded;
  - no wishes the challenger luck.

  In the hall he then offers round `stage / 6 * 5` (5 for floor 6). On
  yes the event starts as usual, and `event_started` enters it on the
  floor after the recorded one and erases the record. That floor always
  starts a section, so its clock starts over there. If the Dojo is full,
  nothing is applied and the record stays, as Cosmic restores it.

## Belts and medals

So Gong's hall menu (`so_gong_belts`, `so_gong_medal`) follows Cosmic's
`2091005`, with the real belt prices, not its fast-upgrade option.

- **Belts.** With no free slot in the EQUIP inventory he says to make
  room. Otherwise he lists the five belts (`game::dojo::BELTS`), marking
  those held, worn or not. A belt needs:
  - a level over its own (25, 35, 45, 60, 75);
  - its price in points (200, 1,800, 4,000, 9,200, 17,000);
  - from the yellow on, the belt before it, unequipped in the EQUIP
    inventory. It is taken in.

  He hands it over and takes the price (`Effect::DojoPoints` with a
  negative amount, refused if the points are no longer there). Otherwise
  he says what it takes and what is missing, as Cosmic's
  `sendBeltRequirements`.
- **The points reset.** A yes or no; yes sets the points to 0.
- **Medals.** Medal n, from Mano's (1142033) to the Mu Lung Dojo
  Vanquisher's (1142064), is for round n's boss (`game::dojo::medal`).
  - With none tried for, he asks whether to try for Mano's. Yes sets
    the stage to 1 (`Effect::DojoMedal`) and ends the talk.
  - Going on by `dojang_next` from the floor whose round is the stage
    counts a win (`scripts::dojo_next`).
  - With fewer than 100 wins he says how many are left. With 100, and room
    in the EQUIP inventory, he hands the medal over and sets the next
    stage with no wins. Past the 32nd he
    says every medal is won.

  Cosmic's version is off by one: it counts wins on the round after the
  stage, names one medal and hands over the next. It also counts a win at
  `dojang_up`, which can be taken again and again, and checks the
  recorded floor for "every medal". RustMS counts the round's own boss,
  on the way on, as the medal text says, and checks the stage. A party
  run's wins don't count, as in Cosmic.

## The party Dojo

So Gong's second choice is `EventKind::DojoParty` (`events::DOJO_PARTY`,
Cosmic `2091005`'s party run). It is `DOJO` on the party floors, with
these differences:

- **Who goes.** `team` is `1..=6`, so it is not solo. The asker must lead
  a party (`dojo.so_gong.party_leader` otherwise), and the levels of the
  members going, those in the hall, must be within 30 of each other
  (`Event::spread`, Cosmic `isBetween(party, 30)`;
  `dojo.so_gong.party_spread` otherwise). There is no trial check.
- **Copies.** `runs` is 5, Cosmic's 925030100 to 925030104 copies.
- **The floors.** Each party floor links to its solo floor (`info/link`),
  which `game-data` resolves, so the portals, So Gong, the boss and the
  clock are the same. `events::dojo_floor` gives a solo or party map's
  floor; `dojo_boss`, `cleared_by_boss` and `clock_on` use it.
- **Points.** One less per floor than alone (`floor_points(n, false)`), and
  no medal wins (`scripts::dojo_next` with `party`).
- **Gathering.** Going on by `dojang_next` to a rest floor sends
  `EventCommand::DojoRest(rest)`, taken when the floor before it is
  cleared, each time anyone goes that way, so whoever comes later is taken
  too.
  The event moves the members on each of the section's five floors before
  it (`move_team` from each). Each member's client (`event_moved`) applies
  the points of every floor from their own up to the one before the rest
  floor, a pink line each (`scripts::dojo_points`), then warps. Cosmic's
  `dojang_next` does the same for the players on those maps.
- **The rest floor.** So Gong offers going on or leaving, and no record
  (Cosmic hides it for a party). Going on as the leader sends
  `EventCommand::DojoOn(rest)`, which moves everyone on the rest floor to
  the next. A member who isn't the leader goes on alone, as Cosmic's
  `warp`.
- Any other floor goes on by `dojang_next` with the portal sound, as
  alone, and the 38th to the rooftop.

## The first-time trial

Cosmic's `2091005` asks a challenger going alone for the first time
whether they would take So Gong on himself (`dojo.so_gong.trial_ask`).
No ends the talk (`dojo.so_gong.trial_no`); yes starts
`EventKind::DojoTrial` (`events::DOJO_TRIAL`), a solo event like the Dojo:

- its one map is the trial room, 925020010, with 5 copies (Cosmic's
  925020010 to 925020014), and its exit is the Dojo's;
- it has no clock: `minutes` is 0, and the event actor sets no end
  (Cosmic's `ingressDojo` gives it none either);
- arriving, the challenger sees `dojo.trial.taunt` on weather 5120024
  (`taunt_challenger`, Cosmic `dojang_Msg`);
- So Gong is the room's only monster, 9300269. His death is the stage's
  clear (`cleared_by_boss`), with the Dojo's clear effects;
- his `revive` is 9300216, the map's `info/timeMob`. When a monster that
  revives into a map's `timeMob` dies, the field sends the map's
  `timeMob` message ("So Gong ran away crying...") as a yellow notice
  (`build_item_line`, Cosmic's `killBy` serverNotice 6). The text is the
  client's own data, so it is not a placeholder.

The room's `out000` (`dojang_tuto`, at the top of the ladder) gives
`dojo.trial.stay` while he stands. Once the run is won it sets
`tutorial_done` (`Effect::DojoTrialDone`), plays the portal sound and
warps to the hall's portal 0. Cosmic's script checks for 9300216 on the
map; RustMS checks that the run is won. Cosmic also skips the trial for
anyone who has been on the entrance (`hasEntered("dojang_Msg")`); RustMS
asks everyone until they beat it.

## Energy and the secret skills

`Character::dojo_energy` (0 to `dojo::FULL_ENERGY`, 10000) is not saved.
On the Dojo's maps (`dojo::in_dojo`, 925020000 to 925033804), an attack
that hits a mob adds 100 once it passes validation, and every hit taken
(TAKE_DAMAGE, before the death check) adds 20; mists and map hazards add
nothing. Each change, and every arrival on a fighting floor, sends the
whole value as SESSION_VALUE (`0x5A`, `energy` and the number as text).
So Gong's new runs, resumed runs and party runs, and the entrance, empty
it, and tell the client so.

Two gaps are left open. A hit that does no damage (dodged, or under
Invincibility) still adds 20; whether v83's servers counted those is not
known. And character data, as on the way back from the Cash Shop, empties
the client's bar while the server keeps its count, until the next change
or floor sends it again.

The secret skills (`skills::pq_skill`) take a full bar:

| Skill | Keys | Server |
|---|---|---|
| Bamboo Rain (1009) | Up + Ctrl + Alt | an attack; dropped if the bar isn't full |
| Invincibility (1010) | Left + Ctrl + Alt | the `DivineBody` buff; refused (`Refusal::NoEnergy`) if it isn't |
| Power Explosion (1011) | Right + Ctrl + Alt | the `DojangBerserk` buff (bit 59) at its `x`, 200, for 30 s; refused if it isn't |

Each copy (10001009 and so on) works the same. Using one empties the bar,
sends `energy` 0, and the pink notice `dojo.energy_reset`. As in Cosmic,
the server never sends FIELD_SET_VARIABLE `Bamboo_Used`, so Bamboo Rain
may be used again on the same floor once the bar is full again.

Power Explosion is not Cosmic's (which gives Booster and leaves out Evan's
20011011). v83's client reads bit 59's value n and makes every weapon hit
n percent, after all else and before the 199999 clamp (PDamage, 0x79053b),
and every spell twice whatever n is (MDamage, 0x791f26); nothing there
checks the map (v83ref `reports/b158-power-explosion.md`). The server's
damage caps allow the same (`combat::explosion_percent`,
`explosion_magic`). Others see the stat with no data: their client plays
the skill's `repeat` loop on the player.

## The client

`DOJO_WARP_UP` (207) is handled by `DojoWarpUpHandler`. v83's handler
(0x959727) reads two bytes:

1. a flag, which when set frees the client's exclusive request;
2. a portal index. If that portal is on the map, the client moves the
   player there by the teleport an in-map portal uses (0x957b74).

HeavenClient does the same with `Stage::teleport_to_portal`: the teleport
effect and the portal sound, then the move on the next update.

The bosses' Mob.img entries hold only `info`: `info/link` names the mob
they copy (9300184 links to Mano, 2220000), and its img has the actions.
HeavenClient's `Mob::look_of` follows the link, so a boss keeps its own
stats and takes the linked mob's look, and its sounds when it has none.

The taunt's weather comes right after the entrance's SET_FIELD.
HeavenClient loads a map after the packets that follow it, and loading
resets the weather, so `Stage::blow_weather` and `set_weather_flag` hold
one that comes before the map has loaded (`pending_weather`), as
`pending_clock` does for clocks. v83 builds the field in SET_FIELD's
handler itself, so nothing waits there. Weather 5120024 has no particles
(`path` is `weather/none`), only its message bar (`floatType` 33).

The Dojo's HUD (`UIDojoHud`, v83 0x554005, updated by 0x554792; RE in
v83ref `reports/b159-dojo-hud.md`) is up on the fields where the secret
skills may be used (field types 14 and 18), rest floors and the trial
included. It comes down at every map change and up again once a map of
those types has loaded. From `UIWindow.img/muruengRaid`, at fixed screen
points:

- The `player`, `clock` and `monster` frames at (169, 50), (400, 26) and
  (631, 50), always shown. There are no names, levels or floor numbers.
- The player's gauge, `player/Gage` stretched to `hp * 305 / maxhp` and
  ending at x 318 (it loses HP from its left), at y 32. It is hidden with
  no maximum HP.
- The monster's gauge, from the same `player/Gage` (v83 never loads
  `monster/Gage`), starting at x 482: `percent * 305 / 100`, with the
  percent kept to 100. It follows the first boss-flagged mob to have
  entered the map, apart from 9300216 and 9999999
  (`MapMobs::dojo_boss_percent`, 0x679025), and its last SHOW_MOB_HP
  percent (`Mob::get_last_percent`, 100 before any, kept for every mob,
  0x66d64c). With no such mob the gauge is hidden and the frame stays.
- The clock, its digits drawn from `number/0`..`9` with `number/bar`
  between them: MM:SS, the minutes' last two digits only
  (`dojo_energy::clock_digits`). The Dojo's field handles only CLOCK kind
  2 (0x55522b); `ClockHandler` gives it to `dojo_energy::Countdown` and
  drops kinds 0, 1 and 3 while the HUD is up. It shows whole seconds
  left, truncated (600 first shows 09:59), stops at 00:00, and plays
  `dojang/timeOver` and its sound once there; the server sends no such
  effect. Until a clock comes only the frame shows, as on rest floors,
  since the clock goes with its map. A kind 2 that comes while the map
  loads starts both the field clock and the HUD's; the HUD then takes the
  field clock down. STOP_CLOCK leaves the HUD's clock be (v83 0x53184a
  only closes the field clock's window).

The server sends SHOW_MOB_HP only to the player who hit the mob, so in
the party Dojo a member's monster gauge moves only with their own hits.

The energy bar sits at the left: `muruengRaid/energy`, its frame at
(20, 130) with the dark `Gage` strip stretched over the unfilled part of
the 77-pixel strip (`dojo_energy::cover`), or, once full, the flashing
`full` animation with the keys' legend at (9, 80). `SessionValueHandler` (90) keeps `energy` and
`FieldSetVariableHandler` (92) keeps `Bamboo_Used` in
`dojo_energy::session()`. The character's data resets both, and every map
change resets `Bamboo_Used`.

With the bar full, on such a field, with no dialog or text field open,
`UI::secret_skill_key` runs v83's chord (FUN_0094c79c,
`dojo_energy::Chord`): a Ctrl or Alt going down is held back; the other
one next completes the chord, and anything else, or a key going up, lets
the held one through first. On a chord the arrow held picks the skill
(FUN_0094ce06), cast through `Combat::use_move`. Up after `Bamboo_Used`
shows "Bamboo Rain can be used only once per stage." in chat type 0xc
(v83's line is Korean). With no arrow held, both keys go on as usual.

Bit 59 is HeavenClient's `BERSERK_FURY`. `Player::prepare_attack` copies
its value into `Attack::explosion`, which `physical_damage::exploded` and
`magic_damage::exploded` apply. While it lasts the player, and others who
see them, loop `Skill/<job>.img/skill/<X>1011/repeat`
(`Char::show_power_explosion`, FUN_00943085), as Invincibility's loop is
drawn for bit 60. v83 also skips the loop while another stat it keeps at
+0xb50 is on; which stat that is isn't known, so HeavenClient doesn't.

## Text

The `dojo.*` keys are all unsourced placeholders (`server_content.toml`).
