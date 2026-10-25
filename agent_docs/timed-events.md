# Timed GM events

Some GM events are ordinary maps on a clock: from the GM's `/start` the
clock runs, notices go out, and when the time is up everyone still inside
goes out. v83 ran them from Nexon's FieldSets (`FieldSet.img/Event<n>`),
which the server kept. The RE is in v83ref `reports/b172-gm-events-rest.md`
(the events, their maps and FieldSets) and `reports/b173-portal-gate.md`
(how the client goes through the lobby's gate). The rules the client
doesn't fix are listed under b172 in v83ref `lanes/QUESTIONS.md`.

Status: the Physical Fitness Test runs. Ola Ola and Find the Jewel are
the same kind of event and not done yet; neither are the prizes.

## The Physical Fitness Test (`net::gm_event::FITNESS`)

- Its maps are 109040000, the lobby, and 109040001 to 109040004, its four
  stages, linked by ordinary portals (`in00`). Stage 4's `in00` leads to
  109050000, where the winners get their reward from Pietro. Mobs are on
  stages 1 and 4; each map's return map is 109050001.
- A GM's `/fitness1` to `/fitness4` open it (the v83 words; all four open
  the one test), and the Event Assistants take players to one of the
  lobby's spawn points (`sp`), on its lower floor (see
  `agent_docs/coconut.md`, "Opening an event to players").
- The gate, `join00`, is a type 4 portal on the lower floor to `join01` in
  the same map, up by the way on (`in00`). It is closed until the GM
  starts the test.
- The clock is 900 s (FieldSet `Event6`), and those still inside go out a
  second after it runs out. Its 13 notices go out at the FieldSet's times
  (`fitness.notice.0` to `fitness.notice.12`; only placeholders yet).
- Dying puts a player out: like every event map, the test's maps return
  the dead to 109050001.

## The gate

- v83 asks the server to go through a portal of type 4 or 5 even when it
  leads within the map (`FUN_0094df9b`, report b173), and does nothing
  until it answers. No packet opens or closes a portal: the server answers
  `BLOCKED_MAP` reason 1, "The portal is closed for now.", or moves the
  player. HeavenClient used to move the player itself for any portal
  within the map; it now asks for types 4 and 5 as well
  (`Portal::is_intramap`).
- The client actor checks a walk through a timed event's gate
  (`net::gm_event::is_gate`) before the handler runs
  (`ClientActor::gm_gate_closed`): it asks its field whether the event is
  on (`FieldMessage::TimedEventOn`).

## The game (`runtime/src/actor/field_race.rs`)

- A GM's `/start` or `/reset` on any of the test's maps goes to all five
  (`ClientActor::run_gm_event`, `FieldMessage::RunTimedEvent`). Each map
  keeps the event as a `Race`, all from the same start, and each sends its
  own players the clock and notices.
- `/start` starts the clock on each map (`CLOCK` kind 2) unless the event
  is on already, and opens the gate. A player arriving on one of the maps
  while it is on gets the clock at its time left.
- When the time is up, or on `/reset`, the clock is set to 0 (`CLOCK`
  kind 2), the gate closes, and every player still on the maps but GMs
  goes to 109050001. Those already on 109050000 stay there.
- A player who isn't a GM and comes to one of the stages while the event
  is off goes out as well. That catches anyone who crosses at the time up
  into a stage whose time came a tick earlier.
- The times are scaled by `RUSTMS_EVENT_SCALE` (`net::events::scaled`).
