# OX Quiz

The OX Quiz is a GM event: a run of true-or-false questions, each answered
by standing in the O room or the X room when its time is up. Those who
answer wrong are sent out, and those who answer all of them win. The RE is
in v83ref `reports/b171-ox-quiz.md` (the v83 client's `CField_OXQuiz`, its
packets and `OXQuiz.img`). The rules the client doesn't fix are Cosmic's
or our own, and are listed under b171 in v83ref `lanes/QUESTIONS.md`.

Status: the server runs the game, started and reset by a GM; HeavenClient
shows the questions and answers (PR 885).

## The field

- The game is played on 109020001, field type 5 (the client's
  `CField_OXQuiz`). Its first area, `area/o`, is the O room on the left,
  and its second, `area/x`, the X room on the right. Players come in on the
  top platform between them, which is neither.
- A field of type 5 with two areas gets an `OxGame` (`field.ox`).
- Those who answer wrong go to 109050001, the field's forcedReturn, and
  the winners to 109050000, as after Coconut Season.
- The field's `join00`, a type 4 portal at its `start00`, also leads to
  109050000. Since the quiz sends its winners there itself, it never
  opens: the server answers it as a closed timed-event gate
  (`net::gm_event::is_gate`, `BLOCKED_MAP` reason 1). Cosmic opens it
  when the quiz ends (report b171).
- A GM's `/ox` opens it to the GM's channel, and the Event Assistants take
  players there, to one of its `sp` portals (see `agent_docs/coconut.md`,
  "Opening an event to players").

## The questions (`game-data/src/ox_quiz.rs`)

`Etc.nx`'s `OXQuiz.img` has nine sets, numbered from 1 and again from 100.
Each question's `a` is 1 for O and 0 for X. The client reads the text from
its own copy, so the server keeps only the set, the number and the answer
(`OxQuiz`, loaded once by `net::gm_event::ox_quiz`). Many questions are in
Korean, which the client draws as `?`; only those with no Korean are asked.
Sets 8 and 9 have none.

## The game (`runtime/src/actor/field_ox.rs`)

- A GM's `/start` on the field draws ten different questions from one set
  with at least ten English ones (`field_ox::draw`) and asks the first.
  `/reset` stops the game, clears the question and sends every player who
  isn't a GM to the exit, since the field has no other way out. `/start`
  does nothing while a game is on, and `/reset` while none is.
- Each question is up for 30 s. When the time is up the answer shows, and
  each player who isn't a GM is judged where they stand: in the O room, in
  the X room, or neither (`field_ox::answer`). A player on a ladder or rope,
  whose foothold is below 0, is in neither. Jumping in a room still counts.
- 5 s later those who were wrong or in neither room have the question
  cleared, are told so in pink (`ox.wrong`) and are warped out. 3 s after
  that the next question goes up.
- After the tenth, everyone left has it cleared. Those who answered it
  right are told so (`ox.won`) and go to the winners' map; anyone who came
  in while its answer showed goes to the exit. GMs stay. A player who came
  in while any earlier answer showed simply answers the next question.
- If after a question no player who isn't a GM is left, the game stops.
- While the game is on, the chat of players who aren't GMs is dropped.
- A player arriving is sent the map's help (`GMEVENT_INSTRUCTIONS` 0). If
  a question is up, they get the event clock at its time left and then the
  question; if its answer shows, the answer.
- The times are scaled by `RUSTMS_EVENT_SCALE` (`net::events::scaled`).
  The client's clock for a question is its own 30 s, so under scaling it
  runs slow. Nothing takes the client's clock down on `/reset`, so a GM
  who stays may see it run on.
- The winners' prize is Pietro's, on 109050000 (`agent_docs/coconut.md`,
  "Opening an event to players").

## Packets

- `OX_QUIZ` (0x91, `build_ox_quiz`): a u8, 1 for the question and 0 for
  its answer, the u8 set and the u16 number. Number 0 clears it.
- `GMEVENT_INSTRUCTIONS` (0x92, `build_gm_event_instructions`): a u8 index;
  0 shows the map's `help0` in a dialog.
- A question puts up the event clock, `CLOCK` kind 0, for 30 s if none is
  up; the clock goes when it reaches 0. `build_event_clock` with seconds
  below 0 puts up a running clock for those coming in during a question.
