# The Guild Quest

The Guild Quest to Sharenian, after Cosmic's `GuildQuest.js`, Shuang's
script (9040000) and `EventManager`'s guild queue. Built so far: the
registration with Shuang, each channel's line of guilds, the waiting room
and its gathering, the quest's clock, the ends for too few members or a
missing registrant, the Returning Path and Nuris, and the client's
waitlist line, the outskirts' brief and earrings, the guide out on every
stage, the quest's items taken on leaving, and the statue keeper's test.
Not yet: the stages beyond the hall of statues, their rewards and the rest
of the guild points the quest earns. The plan is in v83ref's
`reports/b252-guild-quest.md`, with the event machinery's anchors in
`reports/b253-gq-event-infra.md` and the first stages in
`reports/b254-gq-stages-1-3.md`.

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
- `net/src/guild_quest.rs` (`statues_*`): the test's parts and judging.
- `net/src/events.rs` (`leaving_the_guild_quest_takes_the_earring_worn`)
  and `net/src/scripts.rs` (`shuang_briefs_whoever_reaches_sharenians_outskirts`).
- `HeavenClient/tests/guild.cpp`: the waitlist lines.
