# Party quests and instanced events

A party event is played in a copy of its maps of its own: an instance.
Kerning City's party quest (KPQ) is the first. So far it has its entry,
exit and timer; its stages, gates and rewards come next (see
`v83ref/reports/a005-instancing-kpq.md`, section 3).

## Rules

`net/src/events.rs` holds what doesn't move:

- An `Event` gives the lobby where the party gathers, the maps it plays
  in its copy, the entry and exit maps, the levels, the team size and the
  duration.
- `KPQ` has lobby 103000000, maps 103000800-805, exit 103000890, levels
  21-30, 3-4 players and 30 minutes (Cosmic `KerningPQ`).
- `Event::team` picks who goes: the party's members online on the
  leader's channel, standing in the lobby, at the right levels. The leader
  must be one of them, and there must be enough of them and not too many.
- `Team::leave` ends the event when the leader leaves or too few remain.
- `RUSTMS_EVENT_SCALE` (a whole number, default 1) shortens every event.
  The harness world sets it to 60, so KPQ lasts 30 seconds.

## Who owns what

| Owner | Holds |
| --- | --- |
| Directory | Which event each channel is playing, as `(channel, kind)` → id. There is one at a time per kind, like a GMS lobby. |
| Event actor (`runtime/src/actor/event.rs`) | The team, the deadline, and the members' handles. |
| Field registry | The open instance ids. The fields of a copy are keyed by `FieldKey{channel, map, instance_id}`. |
| Client actor (`client/events.rs`) | The member's `EventTicket`: the id, the kind, the end time, and the event's handle. |

## Flow

1. **Asking.** Lakelis (9020000) asks the directory
   (`DirectoryMessage::EventEntry`) whether the party may go:
   - `NoParty`, `NotLeader` and `Busy` each get a Say page.
   - `NotEligible` gets a Say page that states the terms.
   - `Ready` gets a yes/no question (`NpcScript::lakelis`).
2. **Reserving.** Saying yes runs `Effect::StartEvent`, which must be the
   script's last effect. The leader's client asks the directory to reserve
   the event (`StartEvent`). The directory checks again and returns an
   `EventStart` with the team's handles, or `None`; then the script's
   `refused` text, the terms, is shown.
3. **Starting.** The leader's client spawns the event actor
   (`event::spawn`). That opens the instance in the registry and sends each
   member `ServerMessage::EventStarted(ticket)`.
4. **Entering.** A member who gets a ticket warps to the entry map. While a
   member holds a ticket, `change_field` uses `field_key`, so the event's
   maps resolve to its copy. On arrival the client is sent the countdown
   (`show_event_clock`).
5. **Leaving.** Each of these ways out hands the ticket back
   (`EventMessage::Left`) and stops the clock:
   - going to a map outside the event, whether by Nella, by dying, or by a
     debug warp;
   - leaving or losing the party (`PartyChanged{None}`), which also warps
     the member to the exit;
   - logging out, changing channel, or entering the Cash Shop and leaving
     it again, all of which come through `cleanup`.
6. **Ending.** The event ends when its time is up, when its leader leaves,
   or when too few remain. It sends `EventEnded{id}`, and members still on
   its maps go to the exit. Two seconds later the actor exits. Its
   `on_exit` closes the instance (`FieldRegistry::close_instance` sends each
   field `FieldMessage::Close`) and tells the directory (`EventClosed`),
   which frees the lobby.

A player saved on an event map logs back in at its `forcedReturn`
(103000890), outside the copy.

## Closed copies

A closed copy never comes back:

- `FieldRegistry::field` starts a field of a closed instance for its caller
  alone, without registering it. It stops when the caller leaves, so a
  stale ticket can't make a copy outlive its event.
- `send_to_field` doesn't rejoin a gone instanced field. The event has
  already sent its members out.

## NPCs

| NPC | Where | Script |
| --- | --- | --- |
| Lakelis 9020000 | 103000000 | Runtime (`talk_about_event`), like Heracle |
| Nella 9020002 | 103000800-805 | Out to 103000890 |
| Nella 9020002 | 103000890 | Home to 103000000 |

Their lines are unsourced placeholder keys `kpq.lakelis.*` and
`kpq.nella.*`.

## Not yet

These are not done yet:

- Cloto's stages, coupons and passes.
- The `next00` gates, which stay open until the stage logic lands.
- Rewards and the bonus stage.
- Removing the quest's items on exit.
