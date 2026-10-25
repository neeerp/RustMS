# The Monster Carnival

The Monster Carnival is a party-versus-party battle off Kerning City's
Carnival Field. Two parties, Maple Red and Maple Blue, hunt the monsters
the other side summons, and earn Carnival Points (CP) to spend on more. The
RE is in v83ref `reports/b163-carnival-client.md` (the v83 client's fields
and packets) and `reports/b165-carnival-server.md` (the rules, after
Cosmic's `MonsterCarnival.java` and `MonsterCarnivalParty.java`).

Status: the field data, the Carnival Field and Spiegelmann, the waiting
rooms and their challenges, two parties on a room's battle field with its
clock, the sides' look, CP from kills, spending CP on mobs, and the exit
room are in. Not yet: skills and guardians, CP lost on death and the
revive room, results and their effects, and the client's Carnival UI.

## Maps

- The Carnival Field, 980000000 (`CARNIVAL_LOBBY`), has Spiegelmann
  (2042000). Its `mc_out` portal leads back to the map kept in
  `CARNIVAL_RETURN` (5), or Kerning City.
- Room n (1 to 6) is 980000n00 (`room_map`), its waiting room, with
  2042001, who tells the party to wait, and 2042003, who takes a member
  back to the Carnival Field. Its battle field is 980000n01 (`battle_map`),
  then the revive room (n02), the winners' (n03) and the losers' (n04). The
  revive rooms' `MCrevive<n>` portals go back to the battle field.
- The exit room, 980000010 (`CARNIVAL_EXIT`), has 2042002, who sends
  players back to the Carnival Field.
- A battle field is field type 10 (the client's `CField_MonsterCarnival`).
  Its `monsterCarnival` node (`FieldTemplate::carnival`, `Carnival`) holds
  the battle's length (`timeDefault`, 610 s), the mobs each side may summon
  with their CP prices, the skills, the guardians and the spots they take.
- The battle field's own mobs carry a `team` (0 Maple Red, 1 Maple Blue),
  written in their spawn packets. The client lets a side neither hit nor
  be hurt by its own side's mobs; the server doesn't check.
- A mob's `info/getCP` (`MobInfo::cp`) is what killing it earns.

## The battle (`net::events`, `EventKind::Carnival`)

Each room is an event, `EventKind::Carnival(room)` (`CARNIVAL`, built by
`carnival()`), whose maps are the room's waiting room to its losers' room,
played on the event's own instance (see `party-quests.md`). Rooms 1 to 4
take two to four a side and rooms 5 and 6 three to six. The harness world
sets `RUSTMS_CARNIVAL_SOLO=1`, which lets one play alone.

- Spiegelmann on the Carnival Field (`NpcScript::spiegelmann`): Join, for
  levels 30 to 50, lists the rooms: free with its sizes, a party of so many
  of such a level waiting, or taken. About explains the battle. Elsewhere
  he offers the way to the Carnival Field, saving the way back.
- The directory keeps each channel's rooms (`directory/carnival.rs`,
  `DirectoryMessage::CarnivalRooms`, `CarnivalPick` and `CarnivalAnswer`).
  A party's leader picking a free room takes it: the room's event starts,
  and the party waits in the waiting room for 3 minutes (`WAITING`), told
  so (`carnival.waiting`). Picking a room where a party waits challenges
  them, if the party is as big, and the waiting leader is asked
  (`ServerMessage::CarnivalChallenge`, `NpcScript::carnival_challenge`,
  with the challengers' names and levels). A challenge waits 30 s for its
  answer (`CHALLENGE`), and one at a time; a room in battle is taken.
  A leader has one challenge out at a time (a new pick takes the last
  back), an answer counts only for the challenge asked about, and a room's
  size is its waiting party's members there now.
  Spiegelmann says how a pick went (`NpcScript::spiegelmann_picked`).
- No tells the challenger's leader (a pink line). Yes brings the
  challengers into the event (`EventMessage::Challengers`) as Maple Blue:
  they join the waiting room, and both sides see a 10 s clock (`READY`).
- The stages are `net::carnival::Stage`. Waiting's time running out, or the
  waiting leader leaving, sends the party back to the Carnival Field. After
  the 10 s, everyone goes to the battle field, each side told theirs
  (`carnival.team.red`/`blue`), and the battle's clock starts, the field's
  `timeDefault` scaled by `RUSTMS_EVENT_SCALE` (`battle_length`). When it
  runs out, or a side has nobody left, everyone goes to the exit room.
- The event keeps the score (`net::carnival::Score`): each member's side and
  CP, and each side's. A kill on the battle field
  (`FieldActor::handle_attack`) earns the mob's CP for the killer and their
  side.
- Spending CP (MONSTER_CARNIVAL, 0xDA, `MonsterCarnivalHandler`) names a
  tab and a place in it. Only tab 0, the battle field's mob list
  (`Carnival::mobs`, `net::carnival::summon`), is sold yet. The event
  (`carnival_spend`) checks the stage and the price and sets the CP aside
  (`Score::spend`, which leaves the totals), then asks the battle field
  (`FieldMessage::CarnivalSummon`). The field brings the mob out for the
  spender's side (`MobSet::buy`, its team byte the side's) at one of the
  side's `mobGenPos` spots, or where the spender stands, unless the side
  has `mobGenMax` (20 by default) of its bought mobs out. A bought mob
  hunts the other side, and the other side earns its CP. The event then
  tells the spender and their side their CP, or gives it back
  (`Score::refund`). Each mob summoned counts in MONSTER_CARNIVAL_START for
  those arriving later.
- A member's side is on their ticket (`EventTicket::side`), and from there
  in their look (`FieldAvatar::team`, SPAWN_PLAYER's last byte).

## Packets (`net::packet::build::world::carnival`)

CP is sent as two shorts, the CP left to spend and the total ever earned.

- MONSTER_CARNIVAL_START (0x121), sent to a member when they arrive on the
  battle field: their side, their CP, their side's and the other side's,
  then a byte for each mob the field lists, how many are summoned.
- MONSTER_CARNIVAL_OBTAINED_CP (0x122): the killer's own CP.
- MONSTER_CARNIVAL_PARTY_CP (0x123): a side and its CP, to every member.
- MONSTER_CARNIVAL_SUMMON (0x124): the tab, the place in it, and who spent
  the CP, to the whole field, which the client's chat log shows.
- MONSTER_CARNIVAL_MESSAGE (0x125): why a spend failed, a byte
  (`SpendRefusal`): 1 not enough CP, 2 the side's limit, 3 no spot for a
  guardian, 4 that guardian is out, 5 anything else. The client's window
  waits for one or the other after each request.
