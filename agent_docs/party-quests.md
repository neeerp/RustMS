# Party quests and instanced events

A party event is played in a copy of its maps of its own: an instance.
Kerning City's party quest (KPQ) is the first. So far it has its entry,
exit and timer, stage 1 (Cloto's quiz), and stages 2 to 4 (her puzzles
of areas to stand on), with their clears and the gates on. Stage 5 and
the rewards come next (see `v83ref/reports/a005-instancing-kpq.md`,
section 3, `a011-kpq-stage1.md` and `a016-kpq-stages-2-5.md`).

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
| Event actor (`runtime/src/actor/event.rs`) | The team, the deadline, the members' handles, and the stages' progress (`Stage1`, and `Puzzles` for stages 2-4). It alone writes the stages cleared (`Cleared`, one bit per stage), shared with the tickets. |
| Field registry | The open instance ids. The fields of a copy are keyed by `FieldKey{channel, map, instance_id}`. |
| Client actor (`client/events.rs`) | The member's `EventTicket`: the id, the kind, the end time, the event's handle, and the stages cleared. |

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
   `refused` text, the terms, is shown whatever the reason. A reservation
   whose reply the leader can't take is dropped at once.
3. **Starting.** The leader's client spawns the event actor
   (`event::spawn`). That opens the instance in the registry and sends each
   member `ServerMessage::EventStarted(ticket)`.
4. **Entering.** A member who gets a ticket warps to the entry map if they
   are still in the party and the lobby, and not in the Cash Shop;
   otherwise they drop it. While a
   member holds a ticket, `change_field` uses `field_key`, so the event's
   maps resolve to its copy. On arrival the client is sent the countdown
   (`show_event_clock`).
5. **Leaving.** A ticket hands itself back when dropped
   (`EventMessage::Left`), including one that never reached its member.
   Each of these ways out drops it and stops the clock:
   - going to a map outside the event, whether by Nella, by dying, or by a
     debug warp;
   - leaving or losing the party (`PartyChanged{None}`), which also warps
     the member to the exit;
   - logging out or changing channel, which come through `cleanup`. A
     member in the Cash Shop keeps the ticket until they come back out,
     which is a channel change.

   Whichever way they leave, the event's own items (`Event::items`, KPQ's
   coupons and passes, Cosmic's `setExclusiveItems`) are taken from them:
   `Event::take_items`, shown to the client unless they are logging out.
6. **Ending.** The event ends when its time is up, when its leader leaves,
   or when too few remain. It sends `EventEnded{id}`, and members still on
   its maps go to the exit. Two seconds later the actor exits. Its
   `on_exit`, which runs even after a panic, sends every member
   `EventEnded` again, closes the instance (`FieldRegistry::close_instance`
   sends each field `FieldMessage::Close`) and tells the directory
   (`EventClosed`), which frees the lobby, about two seconds after the
   event ended.

A player saved on an event map logs back in at its `forcedReturn`
(103000890), outside the copy. A player without a ticket who warps onto an
event map by a debug warp lands on the channel's own copy of it, which the
harness uses to check that copies are apart.

## Closed copies

A closed copy never comes back:

- `FieldRegistry::field` starts a field of a closed instance for its caller
  alone, without registering it. It stops when the caller leaves, so a
  stale ticket can't make a copy outlive its event.
- `send_to_field` doesn't rejoin a gone field of a closed instance
  (`FieldRegistry::is_open`). The event has already sent its members out.
- `field` and `close_instance` both lock the fields before the open
  instances, so no field of a copy is registered after it closes.

## Stage 1

On 103000800 Cloto (9020001) sets a quiz (`net::events::Stage1`, Cosmic
`9020001`):

- Each member but the leader is asked, on their first talk, for as many
  coupons (4001007) as one of seven job-advancement requirements
  (`ANSWERS`). The question is drawn then and stays theirs.
  `RUSTMS_KPQ_QUESTION` (1 to 7) pins it; the harness world pins 7, whose
  answer is 8.
- Holding exactly that many earns a pass (4001008) for the coupons. A
  wrong number costs nothing, and she asks again. A member who has a pass
  is told to give it to the leader.
- The leader clears the stage by handing in a pass from each of the other
  members (`members - 1`). Passes reach the leader by trade or by being
  dropped and picked up.
- After the clear, Cloto tells everyone to hurry on.

The client actor asks its ticket where the player stands
(`EventTicket::cloto`, a reply from the event actor), and builds
`NpcScript::cloto` from that and the player's bag. The items change hands
as her page is shown (`Page::Act`, as in Cosmic, where the page and the
items go in one call). The script's last effect, `Effect::EventCommand`,
tells the event (`PassGiven`, `ClearStage(1)`) once the items have changed
hands. On stage 5, and outside an event, Cloto has nothing to say yet.

## Stages 2 to 4

On 103000801-803 the team must take three of the map's areas, one member
to each (Cosmic `9020001`'s rope, platform and barrel stages):

- The areas are the map's own (`Map.nx` `area`, `FieldTemplate::areas`,
  in node order; `Area::contains` is half-open): 4 ropes, 5 platforms and
  6 barrels.
- A puzzle is three of them (`combos(n)`: the masks below `2^n` with three
  bits, in order). It is drawn at the leader's first talk on the stage and
  kept (`net::events::Puzzles`). `RUSTMS_KPQ_COMBO` pins the index into
  `combos(n)`, from 0, on every stage; an index past a stage's last is
  ignored there. The harness world pins 1, areas 0, 1 and 3.
- A member who isn't the leader is told to follow the leader
  (`Cloto::Follow`). The leader's first talk sets out the puzzle
  (`Cloto::Intro`). Each later talk by the leader is an answer.
- At the leader's later talks (`Cloto::Puzzle`) the client actor asks the
  field where everyone in it stands (`FieldMessage::Positions`) and checks
  them (`on_areas`): one occupant in each of the puzzle's areas and nobody
  in another, each counted in the first area holding them. Only the
  event's members can be in its copy.
- Wrong, everyone on the map is shown FIELD_EFFECT 3
  `quest/party/wrong_kor` and hears 4 `Party1/Failed`, and Cloto says so.
  Right, her page (`NpcScript::cloto_answer`) runs
  `EventCommand::ClearStage(stage)`, which the event takes only from the
  leader, for a stage whose puzzle has been set.
- Once the stage is cleared, Cloto tells everyone so.

Where a player stands is only what their client last sent in MOVE_PLAYER.
A client may hold back moves in the air or on a rope until it lands, so a
member hanging still on a rope may still count where they last stood
(`v83ref/reports/a018-kpq-stage-facts.md`).

## Clearing a stage

Stage n is played on `entry + n - 1`. A clear, taken once and only from the
leader, goes like this (`EventActor::clear`):

1. The event sets the stage's bit in `Cleared` before telling anyone, so a
   member who sees the clear can go on.
2. Every member gets `ServerMessage::StageCleared{id, stage}`. Those on
   the stage's map are shown FIELD_EFFECT 3 `quest/party/clear`, 4
   `Party1/Clear` (the sound), and 2 `gate` (every stage map has one).
   Everyone in the event, wherever they are in it, gets the stage's EXP
   as quest EXP (`Event::exp_per_stage`: 100, 200, 400, 800, 1500,
   Cosmic's, though not scaled by an EXP rate as Cosmic's is). A member in
   the Cash Shop gets none; leaving it takes them out of the event anyway.
3. A member who comes onto a cleared stage's map later is shown its gate
   again (`show_event_gate`, from `join_current_field`).

The client doesn't stop anyone at a closed gate: it sends CHANGE_MAP with
the portal's name either way. So `handle_packet` checks CHANGE_MAP before
the handler moves the player (`gate_closed`). For `next00` on 103000800-803
before that stage's clear, it answers BLOCKED_MAP 1, the client's own "The
portal is closed for now." line, which also unlocks it
(`v83ref/reports/a012-kpq-gate.md`). Map-id warps aren't checked.

## NPCs

| NPC | Where | Script |
| --- | --- | --- |
| Lakelis 9020000 | 103000000 | Runtime (`talk_about_event`), like Heracle |
| Cloto 9020001 | 103000800-803 | Runtime (`talk_to_cloto`): stage 1's quiz, stages 2-4's puzzles |
| Nella 9020002 | 103000800-805 | Out to 103000890 |
| Nella 9020002 | 103000890 | Home to 103000000 |

Their lines are unsourced placeholder keys `kpq.lakelis.*`,
`kpq.cloto.*` and `kpq.nella.*`.

## Not yet

These are not done yet:

- Stage 5, Cloto's last stage.
- Rewards and the bonus stage.
- A new leader chosen inside the copy: the event still counts the first
  leader, so the leader leaving ends it.
- Members in the Cash Shop still pass the entry check, which doesn't know
  where they are; they drop their ticket, which can end the event at once.
