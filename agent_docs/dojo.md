# Mu Lung Dojo

The Mu Lung Dojo's 38 floors, taken alone. The survey is v83ref's
`reports/b158-dojo.md`; it plans six slices, and this page says which are
in. Slice 1 is: the way in and out, the floors and their bosses, the
door, and So Gong's solo run. Section timers, points, the rooftop's EXP,
belts, the party Dojo, energy and the `muruengRaid` HUD come later.

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
- **The way out** is the entrance's `dojang_exit` (`scripts::portal`). It
  goes back to the saved map and clears it. With nothing saved it goes to
  the board's map, not Cosmic's Henesys.
- **So Gong in the hall** (`npc_scripts::so_gong`) says below level 25
  that the challenger is too weak. Otherwise he offers two choices:
  - taking him on alone, which is `EventKind::Dojo`;
  - what the Dojo is.

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
- `minutes` is 5, the first section's limit. The whole run shares it until
  slice 2 brings the per-section timers (`(floor > 36 ? 15 : floor / 6 + 5)`
  minutes, plus 3 seconds), so after 5 minutes the challenger is sent to
  the exit.

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
    a yes to the exit's `st00` (portal 1). Recording the score is not in
    yet.

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

## Text

The `dojo.*` keys are all unsourced placeholders (`server_content.toml`).
