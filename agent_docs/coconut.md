# Coconut Season

Coconut Season is a GM event: two teams, Maple and Story, hit the coconuts
hanging over the field, and whichever team brings down more of them wins.
The RE is in v83ref `reports/b167-coconut.md` (the v83 client's field and
packets, and the rules after Cosmic's `MapleCoconut.java`).

Status: the server runs the game, started and reset by a GM, and
HeavenClient plays it (see "The client" below).

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
has bit 0 when the account's `gm_level` is 3 or more
(`net::policy::CLIENT_GM_LEVEL`; see "The client's grade" in
`server-architecture.md`). Lower GMs' clients send no such line, though
the server takes one from any GM. The chat handler drops
every `/` line from chat. A GM's `/start` and `/reset` go to the field as
`FieldRequest::GmEvent` (`net::GmEvent`), in any case; on a field with
no event they do nothing.

## Opening an event to players

The v83 client has the words `/coconut1` to `/coconut3`, `/snowball`,
`/snowball1` to `/snowball3`, `/ox`, `/fitness1` to `/fitness4` and
`/ola10` to `/ola52` too
(reports b170, b171, b172), and RustMS
gives them these meanings; `/coconut`, `/jewel` and `/endevent` are our own. A GM
types them on any map, and
they go to the client actor as `WorldAction::GmEntry` (`net::GmEntry`):

- `/coconut`, `/coconut1`, `/coconut2` and `/coconut3` open Coconut Season
  on 109080000, 109080000, 109080001 and 109080002 to the GM's channel;
  `/snowball`, `/snowball1`, `/snowball2` and `/snowball3` open Snowball on
  109060000, 109060000, 109060002 and 109060004; `/ox` opens the OX
  Quiz on 109020001 (`agent_docs/ox-quiz.md`); `/fitness1` to `/fitness4`
  all open the Physical Fitness Test, whose lobby is 109040000
  (`agent_docs/timed-events.md`); `/ola10` to `/ola12` open Ola Ola's
  variant 1 on 109030001, `/ola20` to `/ola22` variant 2 on 109030101, and
  so on to `/ola50` to `/ola52` for variant 5 on 109030401; `/jewel` opens
  Find the Jewel on 109010000. A
  number after the word
  is the most players it takes, 50 if none is given. The directory keeps
  one open event per channel (`DirectoryMessage::OpenGmEvent`), in place
  of any open there, and the channel is told (`gm_event.opened`).
- `/endevent` closes entry (`CloseGmEvent`). Players already there stay.

The Event Assistants in town (Paul 9000000, Jean 9000001, Martin 9000011,
Tony 9000013, `NpcScript::event_assistant`) have a menu: what the event
is, and "go", which says nothing is open while no event is open with
places left (`gm_event.not_open`). Going (`Effect::JoinGmEvent`, in
`runtime/src/actor/client/gm_event.rs`):

1. takes a place from the directory (`JoinGmEvent`), or says the event
   closed or filled up meanwhile;
2. saves the map left as `EVENT_RETURN`;
3. on Snowball, warps the player to the field's entrance
   (`net::gm_event::lobby`: 109060001, 3 or 5) to wait for the GM;
4. on Coconut Season, asks the game's field for a team
   (`FieldMessage::ReserveTeam`) and warps the player to the team's spot
   (`net::gm_event::spot`). The game gives the team it would give the
   player on arriving, and keeps it for them for 30 s (`RESERVE_TIME`),
   so two players on their way don't get the same place. Its teams have
   no side, so the spot is any of its four `sp` portals.

A GM's `/start` in a Snowball entrance (`ClientActor::run_gm_event`)
starts the game on its field, if it isn't on yet, and the entrance's field
actor sends everyone there but the GM on (`FieldMessage::SendToGmEvent`,
then `ServerMessage::GoToGmEvent`). Each reserves a team the same way and
lands on their ball's portal, `st00` for Story and `st01` for Maple. The GM
stays, so a second `/start` sends whoever came since. Every other `/start`
and `/reset` goes to the GM's own field.

A player warped in any other way still joins the smaller team, at the
field's first `sp`.

Three NPCs take players back to `EVENT_RETURN`, or to Henesys if nothing
was saved, and forget it (`gm_event_exit` in `net/src/npc_scripts.rs`):

- Pietro (9000002) in 109050000, where the winners go, and Pietra
  (9000010) in 109050001, where the others go: a line, then the warp.
  Pietro also gives each winner a Scroll of Secrets (4031019) with the
  warp. If it doesn't fit, he says so (`gm_event.pietro_full`) and keeps
  them there. Every event whose winners go to 109050000 gives this one
  prize: Coconut Season, Snowball, the OX Quiz, the Fitness Test and Ola
  Ola (report b172).
- Harry (9000012) on the Coconut Season fields, during the game: a yes or
  no to leaving. Yes also gives the player's place back
  (`Effect::LeaveGmEvent`, then `DirectoryMessage::LeaveGmEvent`), if
  that event is still the one open on the channel, up to its limit.

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

## The client

HeavenClient follows v83's `CField_Coconut` (`Character/Coconut`,
`Net/Handlers/CoconutHandlers`, `IO/UITypes/UICoconut`).

- The coconut packets come before the new field loads, so
  `enter_coconut_field` keeps the new map's field type, its count of nuts
  and its uniforms (`coconut/avatar/<team>/<gender>`) from the moment the
  server moves the player, and forgets the last game.
- `COCONUT_HIT` queues a state for one nut, or for each of them on -1,
  due after its delay. `Stage::update` shows those due on the map objs
  named `%03d` of the index, in the order they came. The objs are named
  001 to 501, so nut 0 has no obj and obj 501 is never shown, as in v83.
- `FIELD_SPECIFIC_DATA` puts the team's cap and overall on the player's own
  look, for their gender; leaving the field puts their own back.
- `CLOCK` of any kind sets the scoreboard's time instead of a clock window.
- The scoreboard sits at (535, 8) on the screen: the scores as three digits
  (Maple's left, Story's right) and, once a `CLOCK` has come, the time left
  as mm:ss, which stays at 00:00 after the end. Each glyph is followed by
  a gap, 4 pixels in the scores and 1 in the time, as v83 draws them. v83
  draws the time at (60, 83), over the end of the backdrop's own "Time
  Remaining"; that is kept.
- On the field a basic attack (not a skill) is the field's own: the player
  swings whether or not anything is in reach, the server is sent the swing
  with no mobs, and `COCONUT` names the lowest-numbered nut whose box the
  swing's range meets, with the attack's delay. The nuts hang well above
  the platforms, so a standing swing rarely reaches one; jump and swing.
- A GM's chat line starting with `/` that the client doesn't know (and
  isn't a face item's name) is sent as chat, so `/start` and `/reset`
  reach the server; a player's isn't.
- Not done: v83 keeps the uniform over equips changed on the field; here
  changing the cap or overall there shows the real one.
