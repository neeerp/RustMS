# The Guild Quest

The Guild Quest to Sharenian, after Cosmic's `GuildQuest.js`, Shuang's
script (9040000) and `EventManager`'s guild queue. Built so far: the
registration with Shuang, each channel's line of guilds, the waiting room
and its gathering, the quest's clock, the ends for too few members or a
missing registrant, the Returning Path and Nuris, and the client's
waitlist line, the outskirts' brief and earrings, the guide out on every
stage, the quest's items taken on leaving, the statue keeper's test, the
knights' hall's spears and gates, the vassals' offering at the
fountain, the waterway's doors and gargoyles, and Sharen III's clothes
and soul at the grave. Not yet: the stages beyond the grave, their rewards
and the rest of the guild points the quest earns. The plan is in v83ref's `reports/b252-guild-quest.md`, with the
event machinery's anchors in `reports/b253-gq-event-infra.md`, the first
stages in `reports/b254-gq-stages-1-3.md`, the knights' hall in
`reports/b255-gq-stage-400.md`, the fountain in
`reports/b256-gq-stage-500.md` and the waterway in
`reports/b257-gq-stage-600.md`.

All dialogue is placeholder text (`guild_quest.*` keys in
`game-data/placeholder_text.toml`): no licensed script has any.

## The line

`net/src/guild_quest.rs` has the rules and the map and NPC ids:

- Shuang (9040000) stands at the Excavation Site's camp (101030104,
  `CAMP`). Nuris (9040001) stands on the Returning Path (990001100,
  `RETURNING_PATH`).
- A guild's master or Jr. Master (ranks 1 and 2, `REGISTRANT_RANKS`)
  registers it. That member is its registrant.
- `Line` is one channel's line: the guild playing, if any, and up to ten
  waiting (`QUEUE_MAX`, Cosmic `EVENT_MAX_GUILD_QUEUE`). A guild in line
  or playing on any channel can't register again.
- `Line::next` gives the turn to the first guild in line whose registrant
  is online on the channel. A guild whose registrant is away when its turn
  comes is passed over and leaves the line, as in Cosmic's
  `attemptStartGuildInstance`.

The world directory keeps each channel's line
(`runtime/src/actor/directory/guild_quest.rs`, `WorldDirectory::guild_quests`).
A guild's turn comes when it registers with no one else playing, or when
the run before it ends (`guild_quest_closed`, from `EventClosed`). Either
way the directory starts the event itself (`next_guild`), with the field
registry and its own sender that `WorldDirectory::with_events` gives it in
`runtime/src/world.rs`: a turn may come with no client asking.

## Shuang

`NpcScript::shuang` (`net/src/npc_scripts.rs`) offers three choices. The
client actor first loads the player's guild rank and asks the directory
for the channel's line (`talk_to_shuang` in
`runtime/src/actor/client/guilds.rs`, `DirectoryMessage::GuildQuestTalk`,
`guild_quest::View`).

- **Register.** Without a guild, or below Jr. Master, or with the guild
  already listed, or with the line full, Shuang says so. Otherwise Shuang
  asks yes or no, telling how many guilds wait, and on yes
  `Effect::GuildQuest(Command::Register)` puts the guild in line.
- **Join.** While the player's guild gathers on the channel,
  `Effect::GuildQuest(Command::Join)` takes the player into the waiting
  room (990000000). The event answers with a ticket, as a squad's does for
  a member coming back. Otherwise Shuang says the guild isn't called.
- **Details.** A page about the quest.

If the directory refuses after all (the line changed since Shuang looked),
the script's refused line is said instead.

## The run

`EventKind::GuildQuest` (`events::GUILD_QUEST`) is an event like an
expedition's: no party, and it lasts until its last member leaves
(`until_empty`).

- Its maps are 990000000 to 990001101, played in a copy of their own, and
  one guild plays a channel's at a time (`runs: 1`).
- The registrant, if at the camp when the turn comes, goes in at once, as
  Cosmic's `startInstance` takes the leader. Cosmic takes them from
  anywhere on the channel; RustMS leaves a registrant elsewhere where they
  are, to come in through Shuang.
- **Gathering.** For 3 minutes (`GATHERING`) the guild's members may come
  in through Shuang, and the event stays open even with no one in yet (it
  holds its own handle, `EventActor::gathering`). The waiting room's way on,
  `join00`, is shut (`gated` and `events::gate_stage`).
- **Under way.** When the gathering is over (`guild_quest_gathered`), no
  one else comes in. With fewer than 6 in (`Event::least`) or the
  registrant not among them, the quest ends. Otherwise stage 1 counts as
  cleared, which opens `join00`, and the 90-minute clock starts on every
  member's screen.
- After that, if the registrant leaves or fewer than 6 stay, the quest
  ends too (Cosmic `isEventTeamLackingNow`).
- **Ending.** An end for too few or for the registrant sends its notice,
  GUILD_OPERATION 0x4A or 0x4B, and comes 5 seconds later (`CLOSING`, as
  the client's lines say). The time running out ends it at once.
  Everyone left goes to the Returning Path, where Nuris offers the way back
  to the camp (`ask_warp`).

## The outskirts

The waiting room's way on leads to Sharenian's outskirts (990000100,
`guild_quest::OUTSKIRTS`), stage 2 by RustMS's map-based numbers.

- **The brief.** Whoever arrives there hears Shuang's brief
  (`scripts::unscripted_map_enter` opens Shuang, and away from the camp
  Shuang's script there is the brief), as Cosmic's `afterChangedMap` has
  Shuang say it.
- **The rocks.** The glowing rocks (reactors 9202000 and 9202009) break
  after their hits and drop the protector earring 1032033
  (`guild_quest::EARRING`), from the content pack's reactor drops. Its
  120-minute `period` there is ignored: the earring is taken on leaving
  instead. The outskirts and 990000200 have no mobs, and no gate: their
  `next00` portals are plain.
- **The guide.** The guide (9040005, `guild_quest::EXIT_GUIDE`) on each
  stage asks whether to give up and, on yes, takes the player back to the
  camp.

## The hall of statues

In the hall of statues (990000300, `guild_quest::STATUE_HALL`), stage 4,
the statue keeper (9040009, `STATUE_KEEPER`) tests the registrant alone
(Cosmic `9040009` and the statues' reactor scripts). `guild_quest::Statues`
keeps the test; the event holds it (`EventMessage::Statues`,
`EventTicket::statues`).

- **Showing.** Told the test (`NpcScript::statue_keeper`), the registrant
  goes on and `EventCommand::ShowStatues` picks part `n`'s `n + 3`
  different statues of the 20 (reactors 9208000-9208002, named "1" to
  "20"). The hall sees each move in turn (`FieldMessage::PlayReactor`, a
  hit at its own state), the first after 5 seconds and the rest 3.5 apart
  (`shown_after`), with pink lines before and after.
- **Striking.** A statue struck (`reactors::Action::Statue`) tells the
  striker's event its number (`ServerMessage::StatueStruck`) before the
  hall sees it hit. Strikes count once the statues are all shown, up to the
  combo's length, whoever in the guild strikes.
- **Judging.** The registrant talks to the keeper again. While the statues
  still move the keeper says to watch. Otherwise the guess is judged:
  - Wrong: the hall sees and hears the wrong answer, and the test starts
    over from part 1.
  - Right on part 1 or 2: a pink line, and the next part shows one statue
    more.
  - Right on part 3: the stage is cleared, the gate (reactor "statuegate")
    goes to state 1, and the guild gains 15 points (`STATUE_POINTS`).
- **The gate.** `next00` is the `statuegate_open` script: shut until the
  gate is set, then on to 990000301.

## The knights' hall

The knights' hall (990000400, `guild_quest::KNIGHT_HALL`) is stage 5
(`KNIGHT_STAGE`). Its portals lead to rooms off it (410, 420, 440, and
430 past a door, with 431 past that), all in the event's copy and all
hurting a player without the earring.

- **The spears.** Thrown on one of the four spears in the room of the
  spears (440; reactor 9208007, `reactors::Action::Spear`), a spear
  (4001025, from the reactors in 410) sets it, and it stays. The field
  tells the event of whoever threw it, or of someone still there, how
  many are set (`ServerMessage::SpearsSet`, `EventCommand::SpearsSet`).
  The event moves the hall's gate (reactor "speargate", `SPEAR_GATE`) to
  that state. The fourth clears the stage and gains the guild 20 points
  (`SPEAR_POINTS`, `ServerMessage::GainGuildPoints` to that member).
- **The clear.** It shows on the hall and every room off it
  (`KNIGHT_MAPS`), but not on 401 past the gate (the client actor's
  `stage_cleared`).
- **The gates.** `next00` is `speargate_open`: shut until the stage is
  cleared, then on to 990000401. `in02` is `stonegate_open`: a stone
  (4001026, from 420's boss) set in the door (9208006, which holds it,
  `Action::Hold`) opens the way to the metal room (430). There `in00`,
  `metalgate_open`, needs a piece of metal (4001037, from 430's big
  reactor) set in its door (9208005), and leads on to 431.

## The fountain

The Fountain of the Wiseman (990000500, `guild_quest::FOUNTAIN`) is stage
6. The dining hall and the cellar off it (501, 502) are rooms of the
event, with no stage of their own.

- **The wish.** The first time the registrant talks to the vassals'
  keeper (9040006, `VASSAL_KEEPER`), each of the four vassals wishes for
  one of the four kinds of offering (4001027 to 4001030, `OFFERINGS`),
  repeats allowed (`guild_quest::Vassals`, kept by the event). Anyone else
  hears that only the registrant is heard.
- **The offering.** On each later talk the client actor lists the items
  on the floor (`FieldMessage::FloorItems`, where each came to rest) and
  makes them an offering (`guild_quest::offering`): exactly four of the
  kinds, one in each of the map's first four areas, in NX order. Anything
  else is no offering and uses no try.
- **Right.** The water gate (reactor "watergate", `WATER_GATE`) goes to
  state 1, the stage clears, the four items go
  (`FieldMessage::TakeItems`) and the guild gains 25 points
  (`VASSAL_POINTS`, in the keeper's script).
- **Wrong.** The keeper counts the vassals pleased, those given a kind
  another of them wished for, and those given a kind no one left wished
  for, with the try's number. The map sees the wrong-answer effect, and a
  knight of each kind (9300036, 9300037, `KNIGHTS`) comes out. The seventh
  wrong try (`OFFERING_TRIES`) brings out six of each along the floor
  instead, and the vassals wish anew on the next talk.
- **The gate.** `next00` is `watergate_open`: shut until the water gate
  is open, then on to 990000600's portal 1.

The harness's `RUSTMS_GQ_VASSALS` (four digits 0-3) pins the wish.

## The waterway

The Underground Waterway (990000600, `guild_quest::WATERWAY`) is stage
7, a maze of portals in front of four rooms (610, 620, 630, 640). Past
the doors of three of them are end rooms (611, 631, 641). They are rooms
of the event, with no stage of their own.

- **The doors.** The doors of 610, 630 and 640 (reactors "secretgate1"
  to "secretgate3", 9208010 to 9208012, `SECRET_GATES`) each take a stack
  of exactly 9, 8 and 20 Marks of the guild (4001035) thrown down on them,
  and stay open (`reactors::Action::Hold`). Each room's `in00`
  (`secretgate1_open` to `3`) is blocked until then, and leads on to its
  end room's portal 1. The Marks are the drops of the room's monsters.
- **The gargoyles.** In 630 the Gargoyle on the ledge drops onto the
  altar (reactor 9201000, `GARGOYLE_ALTAR`), which takes it and brings out
  eight Jr. Gargoyles (9300033, `JR_GARGOYLES`) on the ground below
  (-100, 50) (`reactors::Action::BringOutAt`). Their Marks open 630's door.
- **The last room.** 600's `in03` (`under30gate`) leads to 640's portal
  1 only for those of level 30 or lower (`UNDER_30_LEVEL`); others are
  told so in pink.
- **The clothes.** A box in 611, 620, 631 and 641 (9202005 to 9202008)
  gives up a piece of Sharen III's clothes on its fifth hit and comes back
  a second later (`Action::Drops`, the loot in the content pack).
- **The grid.** The way to the grave in 611, 620, 641 and 631 (`guild1F01`
  to `4`, `GRID_WAYS`) leads to the grave's `st00` and records the room on
  the member's ticket (`EventTicket::grid`). The grave's `out00`
  (`guild1F00`) leads back to the waterway's portal beside that room
  (`BACK_PORTALS`), or to its `st00` for one who never came through a
  room (Cosmic fails then). `guild_quest::grid_way` has the rule and
  `ClientActor::guild_quest_grid` keeps the room.

## The grave

Sharen III's Grave (990000700, `GRAVE`) is stage 8.

- **The clothes.** Its altar (9201001, `CLOTHES_ALTAR`) takes the shoes,
  pants, top and crown (4001032, 4001031, 4001033, 4001034) in that turn;
  a piece out of turn stays on the floor. The crown brings out Sharen
  III's soul (9040003, `SHAREN_SOUL`) with a pink notice to the map
  (`Action::Appear`).
- **The soul.** Only the registrant is heard. Their talk clears the stage,
  opens the gate (reactor "ghostgate", `GHOST_GATE`, forced to state 1)
  and earns the guild 30 points (`SOUL_POINTS`, in the soul's script).
  Afterwards anyone hears thanks (`EventActor::soul`, `SoulTalk`).
- **The gate.** `next00` (`ghostgate_open`) is shut until the gate is
  open, then leads on to 990000800's portal 0.

## The quest's items

`events::GUILD_QUEST.items` (`guild_quest::ITEMS`) are Cosmic's exclusive
items: the earring, 4001024 to 4001035 and 4001037. Leaving the quest, by
any way, takes them all (`Event::take_items`), a worn earring too, which
also refreshes the player's look for the map.

The harness's `RUSTMS_EXPEDITION_MIN` lowers the 6, and
`RUSTMS_EVENT_SCALE` shortens the gathering and the clock (not the 5
seconds).

## The waitlist line

GUILD_OPERATION 0x4C (`build_quest_waitlist`) carries the channel (from 0)
and a number, sent to the guild's members online
(`WorldDirectory::tell_guild_quest`):

- 1: the guild's turn has come (`guild_quest::waitlist_number(None)`).
- 2: it is next.
- More: it is number n-1 in line.
- 0: it left the line, or its run is over.

A guild hears its place when it registers and whenever the line moves
ahead of it.

The 2025 client prints the line for 1, 2 and more (strings 0xddb to 0xddd,
with the channel's name), and keeps it in a status slot until 0 clears it.
HeavenClient prints the same lines in chat (`Guild::quest_waitlist_line`)
and has no slot. 0x4A and 0x4B print 0xdd9 and 0xdda.

## Differences from Cosmic

- Cosmic's chat callouts of the guild's place and its strategy phase are
  server text. RustMS sends the client's own 0x4C instead.
- Cosmic gives a random blessing (2023000 to 2023003) to a team with every
  job; not yet.
- Cosmic turns away a non-member in the waiting room. Here only members of
  the guild come in through Shuang, and the copy is the guild's own.
- The earring doesn't expire after 120 minutes; it can't leave the quest.
- Cosmic's `speargate_open` checks the gate's state; here it checks the
  stage, which clears as the gate reaches its last state.

## Tests

- `net/src/guild_quest.rs`: the line's rules and the waitlist numbers.
- `net/src/packet/build/world/guild.rs` (`guild_quest_notices`): 0x4A,
  0x4B and 0x4C.
- `integration-harness/tests/guild_quest.rs`: registration, the turn and
  the waiting room, the shut way on, a second guild's place, the start,
  too few, the Returning Path and Nuris, the next guild's turn, and a
  gathering too small; then the brief, a rock's earring and the guide out,
  the earrings taken; then the statue keeper's refusal of a member, the
  shut gate, a wrong guess, three parts passed, the guild's points and the
  gate.
- `integration-harness/tests/guild_quest_knights.rs`: the knights' hall's
  shut gate and door, four spears moving the gate, the clear in the hall
  and the room of the spears, the guild's points, and the stone and the
  metal opening their doors. Its helpers for the way in are in
  `integration-harness/src/guild_quest.rs`.
- `net/src/reactors.rs` (`the_guild_quest_s_gates_and_spears_stay_set`),
  `net/src/scripts.rs`
  (`the_knights_hall_s_gates_open_on_the_spears_the_stone_and_the_metal`)
  and `net/src/guild_quest.rs`
  (`the_knights_hall_plays_its_stage_and_the_maps_off_it_are_rooms`).
- `net/src/guild_quest.rs` (`statues_*`): the test's parts and judging.
- `integration-harness/tests/guild_quest_vassals.rs`: the keeper's
  refusal of a member, the shut gate, the intro, no offering, a wrong one
  with its hint and knights, the right one with the clear, the points, the
  offering taken and the gate. `net/src/guild_quest.rs` (`the_vassals_*`,
  `an_offering_*`) has the hints, the tries and the offering's rules, and
  `net/src/scripts.rs` (`the_fountain_s_water_gate_opens_on_the_vassals_offering`)
  the gate.
- `integration-harness/tests/guild_quest_waterway.rs`: the first door
  shut, a stack one short left, the stack of nine opening it and the way
  through; the altar's Jr. Gargoyles and their shut door; a new
  character's way to the last room; the grave's shut gate. Then the shoes'
  box, the grid's ways there and back, the clothes set out of turn and in
  turn, the soul's notice, its refusal of a member, the clear, the points,
  its thanks and the open gate. `net/src/guild_quest.rs`
  (`the_grid_takes_each_end_room_to_the_grave_and_back_beside_it`) has
  the grid.
  `net/src/reactors.rs` (`the_guild_quest_s_gates_and_spears_stay_set`)
  has the doors' and the altars' actions, and `net/src/scripts.rs`
  (`the_waterway_s_doors_open_on_their_marks_and_its_last_room_on_low_levels`)
  the portals.
- `net/src/events.rs` (`leaving_the_guild_quest_takes_the_earring_worn`)
  and `net/src/scripts.rs` (`shuang_briefs_whoever_reaches_sharenians_outskirts`).
- `HeavenClient/tests/guild.cpp`: the waitlist lines.
