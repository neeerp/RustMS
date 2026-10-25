# The Monster Carnival

The Monster Carnival is a party-versus-party battle off Kerning City's
Carnival Field. Two parties, Maple Red and Maple Blue, hunt the monsters
the other side summons, and earn Carnival Points (CP) to spend on more. The
RE is in v83ref `reports/b163-carnival-client.md` (the v83 client's fields
and packets) and `reports/b165-carnival-server.md` (the rules, after
Cosmic's `MonsterCarnival.java` and `MonsterCarnivalParty.java`).

Status: the field data, the Carnival Field and Spiegelmann, one party alone
on a room's battle field with its clock, CP from kills, and the exit room
are in. Not yet: a second party and the waiting room's challenge, spending
CP (0xDA), summons, skills and guardians, CP lost on death and the revive
room, results and their effects, and the client's Carnival UI.

## Maps

- The Carnival Field, 980000000 (`CARNIVAL_LOBBY`), has Spiegelmann
  (2042000). Its `mc_out` portal leads back to the map kept in
  `CARNIVAL_RETURN` (5), or Kerning City.
- Room n (1 to 6) is 980000n00, its waiting room. Its battle field is
  980000n01 (`battle_map`), then the revive room (n02), the winners' (n03)
  and the losers' (n04). The revive rooms' `MCrevive<n>` portals go back to
  the battle field.
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
`carnival()`), whose maps are the room's battle field to its losers' room,
played on the event's own instance (see `party-quests.md`). Rooms 1 to 4
take two to four a side and rooms 5 and 6 three to six. The harness world
sets `RUSTMS_CARNIVAL_SOLO=1`, which lets one play alone.

- Spiegelmann on the Carnival Field (`NpcScript::spiegelmann`): Join, for
  levels 30 to 50, lists the rooms with their sizes, and a party's leader
  takes the party into a free one. About explains the battle. Elsewhere he
  offers the way to the Carnival Field, saving the way back.
- The battle starts with its clock, the field's `timeDefault` scaled by
  `RUSTMS_EVENT_SCALE` (`battle_length`). When it runs out, everyone goes to
  the exit room.
- The event keeps the score (`net::carnival::Score`): each member's side and
  CP, and each side's. A party alone plays Maple Red. A kill on the battle
  field (`FieldActor::handle_attack`) earns the mob's CP for the killer and
  their side.

## Packets (`net::packet::build::world::carnival`)

CP is sent as two shorts, the CP left to spend and the total ever earned.

- MONSTER_CARNIVAL_START (0x121), sent to a member when they arrive on the
  battle field: their side, their CP, their side's and the other side's,
  then a byte for each mob the field lists, how many are summoned.
- MONSTER_CARNIVAL_OBTAINED_CP (0x122): the killer's own CP.
- MONSTER_CARNIVAL_PARTY_CP (0x123): a side and its CP, to every member.
