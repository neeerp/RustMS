# Mu Lung Dojo

The Mu Lung Dojo's 38 floors, taken alone. The survey is v83ref's
`reports/b158-dojo.md`; it plans six slices, and this page says which are
in. Slice 1 is: the way in and out, the floors and their bosses, the
door, and So Gong's solo run. Slice 2 is the section timers and the
entrance's taunt. Slice 3a is training points, the rooftop's EXP, and the
score So Gong records. Belts, the points reset, the medal, the first-time
tutorial, the party Dojo, energy and the `muruengRaid` HUD come later.

## The way in

| Map | Id | What is there |
| --- | --- | --- |
| Mu Lung, the board's map | 250000100 | the notice board 2091006 |
| Entrance | 925020000 | `out00` (`dojang_exit`) back out, `in00` to the hall |
| Hall | 925020001 | So Gong 2091005, the event's lobby |
| Exit | 925020002 | where a challenger who gives up or runs out of time lands |
| Rooftop | 925020003 | after the 38th floor |
| Floor n | 925020100 + (n - 1) * 100 | So Gong again, and on a fighting floor a boss |

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
  that the challenger is too weak. Otherwise he offers two choices:
  - taking him on alone, which is `EventKind::Dojo`;
  - what the Dojo is.

  With a score recorded (see "Points and the record"), taking him on
  alone first asks whether to go back to its round. Yes starts the run
  after the recorded floor; no erases the record and ends the talk.

  The party run, belts, the points reset and the medal are not in his menu
  yet, and the first-time tutorial is skipped.

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
rest floor (`stage`, 0 for none), and, for later slices, the first-time
trial and the medal's stage and kills. It is saved with the player, in
`dojo_records`, one row per character once it differs from a new one's
(`save_dojo`). Cosmic keeps these as `dojoPoints`, `lastDojoStage`,
`finishedDojoTutorial`, `vanquisherStage` and `vanquisherKills`.

- **Points.** Leaving floor n (1 to 37) by `dojang_next` earns
  `2 + n / 6` (`DojoRecord::floor_points`, Cosmic `addDojoPointsByMap`
  with one for going alone): 2 on floors 1 to 5, up to 8 on 36 and 37.
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

## Text

The `dojo.*` keys are all unsourced placeholders (`server_content.toml`).
