# The Monster Carnival

The Monster Carnival is a party-versus-party battle off Kerning City's
Carnival Field. Two parties, Maple Red and Maple Blue, hunt the monsters
the other side summons, and earn Carnival Points (CP) to spend on more. The
RE is in v83ref `reports/b163-carnival-client.md` (the v83 client's fields
and packets) and `reports/b165-carnival-server.md` (the rules, after
Cosmic's `MonsterCarnival.java` and `MonsterCarnivalParty.java`); Carnival
Field 2's is `reports/b323-carnival2.md`.

Status: the field data, the Carnival Field and Spiegelmann, the waiting
rooms and their challenges, two parties on a room's battle field with its
clock, the sides' look, CP from kills, spending CP on mobs, skills and
guardians, the CP and skill items, CP lost on death and the revive room,
the results with their effects, overtime and draws, the result rooms'
grades and EXP, and the exit room are in. HeavenClient reads the carnival's
packets, shows the sides' tags, spares a side its own mobs, and has the
CP window for spending CP, with its side panel counting the mobs summoned
(see `client.md`; the window's RE is `reports/b166-carnival-window.md`).

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
  players back to the Carnival Field. 2042002 also stands in each room's
  winners' and losers' rooms (`win_map`, `lose_map`, `result_room`), whose
  `returnMap` is the exit room.
- A battle field is field type 10 (the client's `CField_MonsterCarnival`).
  Its `monsterCarnival` node (`FieldTemplate::carnival`, `Carnival`) holds
  the battle's length (`timeDefault`, 610 s), the mobs each side may summon
  with their CP prices, the skills, the guardians and the spots they take.
- The battle field's own mobs carry a `team` (0 Maple Red, 1 Maple Blue),
  written in their spawn packets. The client lets a side neither hit nor
  be hurt by its own side's mobs, and draws them half see-through; the
  server doesn't check.
- A mob's `info/getCP` (`MobInfo::cp`) is what killing it earns.

## Carnival Field 2

The second Monster Carnival, for levels 51 to 70, is played the same way
off Carnival Field 2, 980030000 (`CARNIVAL2_LOBBY`), as rooms 7 to 9 of
`EventKind::Carnival` (`CARNIVAL2_ROOMS`; `second`, `lobby`, `exit` and
`levels` tell a room's field). Spiegelmann gives their numbers as 1 to 3
(`number`); `rooms_off` lists a field's rooms.

- Spiegelmann there (2042005, `CARNIVAL2_HOST`) goes straight to its rooms,
  with no About. Spiegelmann in town takes one of 51 to 70 there instead
  of to the first field, and the Dimensional Mirror's entry 3 lands on its
  `st00` (portal 3); its `mc_out` leads back by `CARNIVAL_RETURN` too.
- Room r (7 to 9) is 98003k000 for its k (1 to 3), whose parts go up by
  100: the battle field 98003k100 (field type 15), the revive room k200
  (type 11, whose `pt00` runs `MC2revive`, back to the battle field of the
  room it is in), the winners' k300 and the losers' k400. Rooms 7 and 8
  take two to four a side and room 9 three to six, as quest 1302's text
  says.
- In a waiting room (field type 16) 2042007 (`CARNIVAL2_GUIDE`) tells the
  party to wait and 2042008 (`CARNIVAL2_ROOM_EXIT`) takes a member back.
  The challenge is still put by 2042001's face. 2042007 also stands in the
  exit room, 980030010 (`CARNIVAL2_EXIT`), and grades the result rooms.
- The battle fields' rules come from their own `monsterCarnival` nodes:
  mobs 9300315 to 9300324, ten of a side's summons out at once and eight
  guardians. The fields' own mobs have no team, so either side may hunt
  them (Cosmic gives them one by their node's parity).
- The EXP is Cosmic's: a win 875000, 700000, 555000 or 100000, a loss
  35000, 25000, 12500 or 3500. Its record is 1302, whose necklace is
  1122058; Cosmic's Shiny Maple Coin exchange isn't in.

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
  `timeDefault` scaled by `RUSTMS_EVENT_SCALE` (`battle_length`). From
  then on the leader leaving doesn't end the event: if a side has nobody
  left before the result is known, the other is told (RESULT 11) and
  everyone goes to the exit room. While both sides still play, those left
  hear who quit (LEAVE, below), or, if they led their side, the next of
  that side, who leads it now (`Score::quit`). A side's leader is its
  party's at the challenge, or whoever the party passes to; each member's
  client tells the event their name with the ticket
  (`EventMessage::Named`). The end is under "Results".
- The event keeps the score (`net::carnival::Score`): each member's side and
  CP, and each side's. A kill on the battle field
  (`FieldActor::handle_attack`) earns the mob's CP for the killer and their
  side.
- Spending CP (MONSTER_CARNIVAL, 0xDA, `MonsterCarnivalHandler`) names a
  tab and a place in it. Tab 0 is the battle field's mob list
  (`Carnival::mobs`, `net::carnival::summon`). The event
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
- Tab 1 is the battle field's skill list (`Carnival::skills`), ids into
  Skill.nx `MCSkill.img` (`SkillData::carnival_skills`,
  `net::carnival::skill`): a price, a mob skill and its level, and whether
  it reaches one of the other side (`target` 1) or all of it (2). The
  event (`carnival_skill`) takes the CP and casts it on the other side's
  members (`cast_on_rivals`): a disease (`ServerMessage::Diseased`), or
  Cancel Buff's dispel, which works on those on the battle field. A
  one-target skill picks among all of the other side. Darkness, Weakness,
  Poison and Slow take hold 80% of the time, one roll a cast
  (`takes_hold`). The two Random skills, 8 and 9, cast one of the others
  that reach as many, at the Random's price (`net::carnival::cast`). The
  battle field then tells everyone who cast it
  (`FieldMessage::CarnivalAnnounce`), as only it knows the names.
- Tab 2 is the battle field's guardian list (`Carnival::guardians`), ids
  into Skill.nx `MCGuardian.img` (`SkillData::carnival_guardians`,
  `net::carnival::guardian`): a price and a mob skill (150-156 raise a
  stat, 140/141 make the mobs immune). The event takes the CP and asks the
  battle field (`FieldMessage::CarnivalGuard`). The field places reactor
  9980000 (Red) or 9980001 (Blue) in state 1 at a free `guardianGenPos`
  spot of the side's (`net::carnival::guardian_spot`,
  `FieldReactors::place`) and gives the skill's statuses to every mob of
  the side, and to those that come out later, while it stands. It refuses
  with code 2 when the side has `guardianGenMax` (16 by default) out, 3
  when no spot is free, and 4 when that guardian already stands. Only the
  other side can hit it; four hits break it for good (`Action::Guardian`),
  which frees its spot and takes its statuses off the side's mobs.
- Mobs drop items used on pickup (`ConsumeSpec::consume_on_pickup`): CP
  (`cp`, 2022157-59) for the picker and their side, and skills
  (`nuffSkill`, 2022164-66), which always take hold and nobody hears of
  (`EventCommand::CarnivalCast`). Out of a battle they go and do nothing.
- A member's side is on their ticket (`EventTicket::side`), and from there
  in their look (`FieldAvatar::team`, SPAWN_PLAYER's last byte).
- A death on the battle field (`ClientActor::died`) costs no EXP and
  keeps the buffs (Cosmic `playerDead`). In the battle the event takes up
  to the field's `deathCP` (10) of the player's own CP, from them and their
  side (`Score::lose`; the totals stay), and the battle field tells
  everyone there through DIED (`FieldMessage::CarnivalDied`). Reviving
  takes the player to the field's `returnMap`, the room's revive room
  (980000n02, `net::carnival::revive_map`), whose screen takes their side
  from START. Its `pt00` portal (`MCrevive<n>`) leads back to the battle
  field's portal 0, where START gives the score again.

## Results

- Ten seconds before the clock runs out (`result_lead`, scaled) the battle
  closes (`Stage::Closing`): the side with more CP in all
  (`Score::leader`) wins, and from then on no CP is earned, spent or lost
  to deaths. A side leaving now lets the battle play out. With a
  winner, the field's mobs are cleared and each side is shown its effect
  and sound (`result_effects`: `quest/carnival/win` or `lose`, and
  `MobCarnival/Win` or `Lose`) and RESULT 8 (won) or 9 (lost).
- A tie plays on once: the field's `timeExpand` (120 s, scaled,
  `overtime_length`) on a new clock, told so (`carnival.overtime`, since
  the client doesn't print its own line). A second tie is a draw
  (`Outcome::Draw`): RESULT 10, and both sides get the lose effect.
- At the end (`Stage::Over`) the room is freed and each member is told
  their result (`ServerMessage::CarnivalResult`, which keeps it in
  `Player::carnival_result` and cures their diseases) and goes to the
  winners' room (980000n03) or, on a loss or draw, the losers' (n04).
- 2042002 there (`npc_scripts::carnival_grade`) grades the side's CP in all
  (`grade`: A from 300, B from 100, C from 50, else D), pays its EXP
  (`result_exp`: a win 50000, 25500, 21000 or 19505, a loss 17500, 12000,
  5000 or 2500) and sends the player to the Carnival Field. Without a
  result kept it just sends them there; leaving a result room forgets it.

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
- MONSTER_CARNIVAL_DIED (0x126): the side, the name of who died, and the
  CP their side lost, a byte, to the whole battle field, which the
  client's chat log shows.
- MONSTER_CARNIVAL_RESULT (0x128): a byte, 8 won, 9 lost, 10 a draw, 11
  the other side left, which the client prints only on a battle field or
  revive room.
- MONSTER_CARNIVAL_LEAVE (0x127): a byte, 6 if the side's leader quit and
  the member named leads it now, else 0, a member quit (`LEAVE_QUIT`, ours:
  the client checks for 6 alone); then the side and the name. Sent to the
  whole battle while it goes on; the client prints it only on a battle
  field, and v83 passes the side first to both lines, so a quit reads
  "[Maple Red] of Team [name]" (b163).
