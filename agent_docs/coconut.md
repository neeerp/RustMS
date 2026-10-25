# Coconut Season

Coconut Season is a GM event: two teams, Maple and Story, hit the coconuts
hanging over the field, and whichever team brings down more of them wins.
The RE is in v83ref `reports/b167-coconut.md` (the v83 client's field and
packets, and the rules after Cosmic's `MapleCoconut.java`).

Status: the server runs the game, started and reset by a GM. HeavenClient
doesn't read the packets yet, so it shows neither the nuts nor the
scoreboard.

## Maps

- The game is played on 109080000 to 109080003, field type 4 (the client's
  `CField_Coconut`). Their `coconut` node (`FieldTemplate::coconut`,
  `Coconut`) holds how many nuts may fall, burst or stick (their sum is the
  number of nuts), the game's length (`timeDefault`), the overtime a tie
  adds (`timeExpand`), the wait before the warp out (`timeFinish`), and the
  win and lose effects and sounds.
- When the game is over, the winning team goes to 109050000 and everyone
  else to 109050001 (`WINNERS_MAP`, `EXIT_MAP`).

## The game (`runtime/src/actor/field_coconut.rs`)

`CoconutGame` is the game's state, kept by the field's `FieldActor`
(`field.coconut`), which sends the packets.

- A player entering joins the smaller team, Maple on a tie, and keeps it
  while they stay in the field. The team is the avatar's `team`, written
  in its spawn packet. The player is sent the score and their team
  (`FIELD_SPECIFIC_DATA`). During a game they are also shown the nuts,
  with those that burst or fell already in that state, and the clock: the
  client keeps the nuts hidden until told to show them.
- A GM starts the game with `/start` and stops it with `/reset`, typed in
  the chat box (see below). Starting sets every nut back up and the score
  to 0, shows the nuts after 5 s, and starts the clock.
- A hit on a nut (`COCONUT`) shakes it. Once it has taken more than 2 hits,
  each hit brings it down 4 times in 10: it sticks for good while stuck
  nuts remain, 1 time in 100; it bursts while bursts remain, 5 times in
  100; else it falls while falls remain, else bursts, else sticks. A nut
  that falls scores a point for the hitter's team, and everyone is told
  who brought it down (`coconut.fell.maple`, `coconut.fell.story`). A nut
  that shook takes no hits for 750 ms.
- When the clock runs out on a tie, the game plays on for `timeExpand`
  once (`coconut.overtime`). Then the team ahead gets the win effect and
  sound and the other the lose ones; on a second tie everyone gets the
  lose ones. After `timeFinish` everyone is warped out.
- The times are scaled by `RUSTMS_EVENT_SCALE` (`net::events::scaled`).

## GM event commands

The v83 client sends a chat line starting with `/` that it doesn't know
itself as ordinary chat, but only for a GM: the login packet's grade byte
is 1 when the account's `gm_level` is above 0. The chat handler drops
every `/` line from chat. A GM's `/start` and `/reset` go to the field as
`FieldRequest::GmEvent` (`net::GmEvent`), in any case; on a field with
no event they do nothing.

## Packets

- `COCONUT_HIT` (0x11D, `build_coconut_hit`): a short nut index, -1 for
  all of them, a u16 delay in ms and a u8 state: 0 shows, 1 shakes, 2
  bursts, 3 falls.
- `COCONUT_SCORE` (0x11E, `build_coconut_score`): Maple's and Story's
  scores, as u16s.
- `FIELD_SPECIFIC_DATA` (0x85, `build_field_team`): the player's team as a
  u8, 0xFF for none. It changes only the player's own look.
- The clock is `CLOCK` type 2 (`conti::build_countdown`).
- `COCONUT` (recv 0xD5, `CoconutHandler`): a u16 nut index and a u16 delay,
  which the server ignores.
