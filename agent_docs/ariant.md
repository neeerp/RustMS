# The Ariant Coliseum

The Battle Arena of the Ariant Coliseum (Cosmic's AriantPQ) is a ten-minute
fight between players of levels 20 to 30 for Spirit Jewels (`4031868`).
The rules are in `net::ariant`, the rosters in
`runtime/src/actor/directory/ariant.rs`, and the battle is an event,
`EventKind::Ariant(arena)`, in `runtime/src/actor/event.rs`.

## Maps

| Map | What it is |
| --- | --- |
| `980010000` | The lobby. Cesar (`2101014`) opens and joins arenas. Its `out00` leads back the way the Dimensional Mirror came, or to Ariant (`260000000`) |
| `980010n00` (n = 1 to 3) | Arena n's entrance, where its players wait. Cesar (`2101017`) shows the roster, bans, and starts the battle |
| `980010n01` | Arena n's battle map. Cesar gives each player 50 Element Rocks and 5 Bombs, once a run |
| `980010010` | The King's Room. Areda (`2101016`) and Abdullah VIII (`2101015`) |

## Rosters and the start

- A host types the most players the arena takes (2 to 5 at Cesar, 7 at most
  when they join). The roster is a squad (`net::squads`), one an arena a
  channel.
- Only the host starts. The checks run in Cosmic's order, and the battle's
  event takes everyone at the entrance into the battle map.
- A battle ends once fewer than two are left in it, and the rest go back to
  the lobby.

## Scores

- After each step, the client actor of a player in a battle counts their
  Spirit Jewels. It tells the event (`EventTicket::ariant_jewels`) when the
  count changes, or when a bomb knocked some out
  (`FieldRequest::Shed` of jewels).
- The event keeps the scores in `net::ariant::Scores` in the order players
  first told, and the jewels lost to bombs.
- On a change, the team sees the board (`ARIANT_ARENA_USER_SCORE`, 0x129:
  each name and their jewels) within `SCORES_EVERY` (500 ms).

## Results

- The results come `RESULTS_BEFORE` (10 s) before the ten minutes are up
  (Cosmic's 9:50 `pqTimerBoard`). The clock on the map still shows the full
  ten minutes (`EventActor::clock`).
- At the results:
  - The team sees the banner (`ARIANT_RESULT`, 0x9B, no body).
  - The battle's mobs go (`FieldMessage::ClearMobs`).
  - Each player is told their points (`ServerMessage::AriantResult`), which
    their client actor keeps as `Player::ariant_tier`.
- Points are `net::ariant::points`:
  - Each player earns one point for every ten jewels.
  - The winner is the first with the most jewels.
  - The winner gets only one point if far ahead, when both of these hold:
    - The others' jewels and those lost, over the winner's, are under
      0.8177.
    - Either that sum is under 7, or the second's jewels and those lost,
      over the winner's, are under 0.5929.
  - "Lost" is the jewels bombs knocked out plus the jewels on the battle
    map's floor at the results.
- After the results, one player is enough to keep the battle on.
- At the end, the event takes everyone left to the King's Room, portal 0.

## Jewels

- Leaving a battle before its results takes the player's jewels (the
  event's `items`).
- After the results the jewels stay for Areda.
- Leaving for anywhere but the King's Room while holding a tier takes the
  jewels and the tier. So does going offline.

## The King's Room

- Areda gives what a battle earned.
  - Without a tier: "didn't see you".
  - With no jewels: "no jewels".
  - Otherwise she takes all the jewels and gives EXP of 92.7 for each point
    times the player's EXP rate (`prize_exp`). She adds the points to the
    player's Battle Arena points (`Player::ariant_points`, saved in
    `ariant_records`) and sets the tier to 0.
- Abdullah VIII also needs a tier. His menu has two choices:
  - Points: with 100 or more, he trades 100 for the Palm Tree Beach Chair
    (`3010018`), if there is room. If the points with the tier would reach
    100, he sends the player to Areda first. Otherwise he says how many
    they have and need.
  - About: what the points are.

All the lines are placeholders (`ariant_arena.*`).

## The client

HeavenClient draws the battle as v83's arena field (fieldType 13) does
(`Character/Ariant`, `AriantHandlers`, `UIAriant`):

- **The board.** ARIANT_ARENA_USER_SCORE (0x129) is a count, then each
  name and its total of jewels. The client keeps a row per name:
  - a negative score takes that name's rows off;
  - an empty name is skipped, and a GM (a job of group 8 or 9) keeps only
    their own row;
  - scores are capped at 9999;
  - a new name goes at the end, with the icon `characterIcon/<n>`, where n
    is the number of rows already there.

  The rows are sorted by score, highest first, and ties keep their order.
  They are drawn at the top left of the screen from (0, 30), 17 px apart:
  - the icon at x 5;
  - the name at x 21;
  - the score as `%4d` at x 106.

  The text is bold, 12 point and blue. There is no frame. The board shows
  nothing until the first 0x129 and goes on a change of map.
- **The results.** ARIANT_RESULT (0x9B) has no body:
  - the board goes;
  - `UIWindow.img/AriantMatch/Result` unrolls once at (400, 100) of the
    screen (14 frames of 100 ms), with `Sound/UI.img/ScrollOpen`.

  Once it is open, every row is written on it and fades in over a second.
  The text is 12 point and black, the rows 26 px apart from y 104:
  - the place from x 63, as "1st" to "6th" or a number past those;
  - the name centred on x 140;
  - the score centred on x 235.

  The x values are measured from the scroll's left edge, 160 px left of
  its anchor. Players with the same score share the lowest of their places:
  10, 5, 5 and 1 are 1st, 3rd, 3rd and 4th. The scroll stays until the
  player leaves the field. The art and icons are in `UIClassic.nx`.

## Deviations from Cosmic

- Cosmic's `runnerups.remove(firstTop)` removes by index. Here the winner
  is left out by who they are.
- Cosmic keeps the jewels when a battle is disposed of early. Here they
  go.
- Areda has no GM bypass.
- Abdullah checks for room for the chair.
