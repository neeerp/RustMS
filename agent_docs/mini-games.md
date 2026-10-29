# Mini-games

Omok and Match Cards are two-player games. A player opens one with a game
set from their Etc bag, and the game appears as a balloon over their head;
one other player can sit down at it. Both run as mini rooms of
PLAYER_INTERACTION, sharing the visit, chat and leave modes with the
trading room and the shops (see `agent_docs/inventory.md`). The RE is in
v83ref `reports/164-shops-games.md` §3–5 (packets and rules),
`reports/164-balloon.md` (the balloon) and `reports/b100-minigame-layout.md`
(the windows).

Status: both games are played to the end, with records kept, and
HeavenClient shows them (see "The client").

## Opening a game

- **Game sets** (`net::mini_game::game_set`). Omok uses one of
  4080000 to 4080011, each with its own stones. The room's `piece` is the
  set's index, so the set it names must be in the Etc bag. Match Cards uses
  4080100; there `piece` is the board size: 0 for 12 cards (4 × 3),
  1 for 20 (5 × 4) and 2 for 30 (6 × 5). The set stays in the bag.
- **Create** (mode 0x00, room type 1 Omok or 2 Match Cards): the title, an
  optional password, then `piece`. The server ignores it in these cases:
  - the player is already in a trade, a shop, a hired merchant or a game;
  - the title is empty;
  - the set isn't in the bag;
  - the map's fieldLimit has 0x80 (`FieldTemplate::may_play_games`). The
    client refuses this first, with "You can't start the game here."
- The client also won't open the create dialog while the player is
  morphed (`agent_docs/combat.md`).
- The room's id comes from the same counter as trades
  (`trade::NEXT_ROOM`), and the room seats the owner in slot 0.

## The room (`runtime/src/actor/field_games.rs`)

`Games` holds the field's rooms and is pure state, like `field_shops`: it
works out who gets which packet, and `FieldActor` delivers them. Each
player's `ClientActor` holds the game they're at
(`runtime/src/actor/client/mini_game.rs`) and their records.

- **The balloon.** `MiniRoomBox` shows the game's type, id, title, `piece`
  (the stones or the board), the seats taken out of 2, whether it has a
  password, and whether a game is in progress. A change goes out as
  `build_char_box` to everyone who can see the owner, and the box is also
  part of the owner's spawn packet.
- **Visiting** (mode 0x04: the room id, then an optional password). The
  visit is refused (enter error) in these cases:
  - "closed" (1): the visitor is busy in another room or already at a game,
    or the room is gone;
  - "full" (2): the room already has a visitor;
  - "wrong password" (0x16): the password doesn't match.

  Otherwise the visitor gets the room (mode 0x05, both members' looks and
  records), and the owner gets the visitor (mode 0x04).
- **Chat** (mode 0x06) goes to both as "name : text".
- **Records.** A record is four ints after the game type: wins, ties,
  losses, and a score. The score is always 2000 because nothing keeps one
  (inferred from common v83 servers). The records live in the
  `minigame_records` table (`db::mini_game`), one row per character and
  game. They are read once a session and counted up after each game.

## Playing

- **Ready and start.** Only the visitor says they are ready, or not
  (0x3a/0x3b, echoed to both). The owner starts (0x3d) only once the
  visitor is ready, and the server sends 0x3d with the slot that moves
  second. For Match Cards it also sends the shuffled deck (each face
  twice). The player who moves second is the last game's winner, so the
  loser opens (inferred); the owner moves second in the first game and
  after a new visitor sits down. Whoever opens plays black (color 1) at
  Omok.
- **Omok** (`net::mini_game::Omok`, 15 × 15, inferred from the board art).
  - A stone (0x40: x, y, color) must be the mover's color, on an empty
    cell, on their turn. If it isn't allowed, only the mover gets 0x41:
    `'C'` for a double three ("You have double-3's.") and 0 for anything
    else.
  - Five or more in a row wins, and a winning stone may make double threes
    (the usual rule, inferred). A full board is a tie.
  - Taking a stone back (0x36 asked, 0x37 answered): each player may do it
    once a game, if they have a stone down, with one request at a time.
    The other player has to agree. The asker's last stone and any played
    after it are removed, and the asker moves again.
- **Match Cards.** The first card of a turn (0x44, first = 1) is shown to
  the other player only. The second is shown to both, with whether the
  two matched. A pair goes to the finder, who keeps the turn and gets a
  fresh clock, and both see game notice 9 ("[%s] has matched cards.
  Please continue."). A miss passes the turn. When every card is matched,
  whoever has more pairs wins; equal pairs is a tie.
- **The turn's clock.** An Omok turn is 30 s on the client and a Match
  Cards turn 10 s or more (report b100 §2.7). When time runs out, both
  clients send 0x3f. Only the player whose turn it is counts, and only
  after 25 s (Omok) or 8 s (Match Cards). The turn then passes, and the
  server sends 0x3f with the new mover's slot.
- **Ties** (0x32 asked, 0x33 answered): once a turn, whoever asks. If the
  other player agrees, the game ends in a tie; if not, the asker gets
  0x33.
- **Giving up** (0x34) loses the game: both see game notice 2 ("[%s] have
  forfeited."), and the game ends as a forfeit.
- **Leave after this game** (0x38/0x39), only during a game: both see game
  notice 5 or 6, and once the game ends that player leaves (reason 0).
- **The end** (0x3e): the result (0 won, 1 tie, 2 forfeit; the client
  tells only a tie apart), the winner's slot, and both records after the
  game. Each player's actor adds a win, tie or loss to their own record
  in the database.
- **Expelling** (0x3c): the owner can expel the visitor, but only between
  games. The visitor gets leave reason 5, and the owner gets the seat
  emptied.

## Leaving

- A visitor leaving (mode 0x0a, or by leaving the field, which goes
  through `leave_mini_game` first) empties the seat. During a game it
  loses the game: the leaver counts the loss themselves, and the other
  player gets the forfeit result.
- An owner leaving closes the room. The visitor gets leave reason 3 and
  the balloon goes.
- If the field itself goes, its players' games end with no result
  counted (`abandon_mini_game`, reason 3).

## The client

- `UIMiniGameCreate`: the create dialog, opened by double-clicking a game
  set (0xa1d4d8). It takes a title (at most 30 characters), a password (at
  most 12) and, for Match Cards, the number of cards.
- `UIMiniGame`: what both rooms share (CMiniGameDlg): the two players,
  their records, the chat, and the Ready/Start/Tie/Give up/Leave/Expel
  controls. Exit during a game toggles "leave after this game".
- `UIOmok` draws the board, with each cell a 14 px square every 24 px from
  (25, 32). `UIMatchCards` draws the cards, the matched pair's glow, and
  the flip and ticking sounds.
- `Net/Handlers/MiniGame.h` holds the logic with no UI: game notices, leave
  reasons, the board's geometry and the turn state. It is tested by
  `mini_game_tests`, and the packet parsing by `mini_room_tests`.
- `Character/MiniRoomBalloon.cpp` draws the balloon: the title wrapped
  once, the game's icon, lock and seats, or the Progress sign while a game
  is on.
- Hovering someone's balloon shows the game cursor over a game and the
  house cursor over a shop. If the title was cut short with "..", a tooltip
  also shows the whole title 20 px below and right of the cursor
  (FUN_0094cfc3, balloon+0x58).

## Tests

- `runtime/src/actor/field_games.rs` unit tests cover the rules: ready and
  start, five in a row, refused stones, ties, taking a stone back, turn
  skips, giving up, leaving, leave-after, expelling, chat, and Match Cards
  pairs and scoring.
- `integration-harness/tests/mini_games.rs` (`five_in_a_row_wins_and_leaving_loses`)
  plays Omok between two characters through five in a row, then leaves a
  game under way, and checks that both records last for whoever sits down
  next.
