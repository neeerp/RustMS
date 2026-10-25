# Snowball

Snowball is a GM event: two teams, Story and Maple, each push a giant
snowball along their own lane, and whichever team's ball gets further
wins. Each team can also bring down the snowman in its lane, which stops
the other team's ball for a while. The RE is in v83ref
`reports/b168-snowball.md` (the v83 client's `CField_SnowBall` and its
packets). The rules the client doesn't fix are Cosmic's or our own, and
are listed under b168 in v83ref `lanes/QUESTIONS.md`.

Status: the server runs the game, started and reset by a GM. HeavenClient
doesn't show it yet.

## Maps

- The game is played on 109060000, 109060002 and 109060004, field type 1
  (the client's `CField_SnowBall`). Their entrances, 109060001, 109060003
  and 109060005, are field type 7 and have no game.
- The field's `snowBall` node (`FieldTemplate::snowball`, `Snowball`)
  holds:
  - the lanes: `0` and `1` (each lane's portal, ball, snowman and height),
    `dx` (a step's width in pixels), `xMin`, `xMax` (the last step), and
    `section1` to `section3` (the steps whose passing the client
    announces);
  - the damage each hit shows: `damageSnowBall` on a ball,
    `damageSnowMan0` on a snowman, `damageSnowMan1` for a strong hit on
    one;
  - the snowmen: `snowManHP`, `snowManWait` (ms a snowman stays down),
    `recoveryAmount`;
  - `speed`, which the server doesn't use.
- When the game is over, the winning team goes to 109050000 and everyone
  else to 109050001, as after Coconut Season.

## The game (`runtime/src/actor/field_snowball.rs`)

`SnowballGame` is the game's state, kept by the field's `FieldActor`
(`field.snowball`), which sends the packets.

- **Teams.** A player entering joins the smaller team, Story on a tie, and
  keeps it while they stay in the field. Team 0, Story, has the lower lane
  and team 1, Maple, the upper one. The team is the avatar's `team`,
  written in its spawn packet.
- **Entering.** The player is sent the field's state with the three
  damages (the client reads them from the first state only), and the
  clock if a game is on.
- **Starting and stopping.** A GM starts the game with `/start` and stops
  it with `/reset` (see `agent_docs/coconut.md`, "GM event commands").
  Both put the balls back at step 0 and the snowmen at full HP. The game
  runs for 600 s. `/reset` works only while a game runs, not in the 10 s
  after it ends.
- **Hits** (`SNOWBALL`). A player may hit only their own team's ball and
  the snowman in their lane, and only once per 500 ms. The hit's delay,
  which the others' clients wait before showing it, is capped at 2000 ms.
  When a hit changes the field, the state goes out before the hit, since
  the client redraws the snowmen's HP bars only on a hit.
  - Each hit on a ball shows `damageSnowBall`, and every third one rolls
    it a step, up to `xMax`. Passing `section1` to `section3` tells
    everyone so.
  - A hit on a snowman misses 2 times in 10, is strong 1 time in 10, and
    otherwise does `damageSnowMan0`.
  - A snowman brought to 0 HP stops the other team's ball for
    `snowManWait`: hits on that ball show 0 and don't count. Then the
    snowman is back at full HP and the ball moves again; everyone is told
    both.
  - A snowman that is up regains `recoveryAmount` HP every 10 s.
- **Pace.** The client rolls a ball on its own at the stage it is sent,
  always forward, and catches up with the step it was last sent; at stage
  8 it only catches up, a step per 500 ms, and that may be one step back.
  The server measures the ball's time per step over its last 3 steps
  (within 3 s), and sends the fastest stage that isn't faster than that.
  When the next step is later than the stage's time, or the ball is
  stopped, it sends stage 8 at once, so the client is never more than a
  step ahead.
- **The end.** The game ends when the time runs out, or as soon as a ball
  reaches `xMax`. The team whose ball is further on wins: its ball stays
  and the other fades out; on a tie nobody wins. Everyone is told the game
  is over (`snowball.ended`), and 10 s later they are warped out.
- **Walking into a ball** (`LEFT_KNOCK_BACK`). A player on a team is
  thrown back, at most twice a second.
- The times are scaled by `RUSTMS_EVENT_SCALE` (`net::events::scaled`).

## Packets

- `SNOWBALL_STATE` (0x119, `build_snowball_state`):
  - a u8 state: 1 the game runs (the client sends hits only then), 2 ball
    0 fades out, 3 ball 1 fades out, 0 otherwise;
  - each snowman's HP, in percent, as an i32;
  - each ball's step as an i16 and its stage as a u8;
  - in the first a client is sent on the field, the three damages as
    u16s.
- `HIT_SNOWBALL` (0x11A, `build_hit_snowball`): a u8 target (ball 0, ball
  1, snowman 0, snowman 1), the u16 damage it shows (0 a miss) and a u16
  delay in ms, the hitter's own.
- `SNOWBALL_MESSAGE` (0x11B, `build_snowball_message`): a u8 team and a u8
  line about that team's ball: 1 to 3 it passed that section, 4 the other
  team is attacking the snowman and stopping it, 5 it moves again.
- `LEFT_KNOCK_BACK` (0x11C, `build_left_knock_back`): no body; it throws
  the player back.
- The clock is `CLOCK` type 2 (`conti::build_countdown`).
- `SNOWBALL` (recv 0xD3, `SnowballHandler`): a u8 target, a u16 damage
  (ignored) and a u16 delay.
- `LEFT_KNOCKBACK` (recv 0xD4, `LeftKnockbackHandler`): no body.
