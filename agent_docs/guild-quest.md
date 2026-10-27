# The Guild Quest

The Guild Quest to Sharenian, after Cosmic's `GuildQuest.js`, Shuang's
script (9040000) and `EventManager`'s guild queue. Built so far: the
registration with Shuang, each channel's line of guilds, the waiting room
and its gathering, the quest's clock, the ends for too few members or a
missing registrant, the Returning Path and Nuris, and the client's
waitlist line. Not yet: the stages beyond the waiting room, their rewards
and the guild points the quest earns. The plan is in v83ref's
`reports/b252-guild-quest.md`, with the event machinery's anchors in
`reports/b253-gq-event-infra.md`.

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

## Tests

- `net/src/guild_quest.rs`: the line's rules and the waitlist numbers.
- `net/src/packet/build/world/guild.rs` (`guild_quest_notices`): 0x4A,
  0x4B and 0x4C.
- `integration-harness/tests/guild_quest.rs`: registration, the turn and
  the waiting room, the shut way on, a second guild's place, the start,
  too few, the Returning Path and Nuris, the next guild's turn, and a
  gathering too small.
- `HeavenClient/tests/guild.cpp`: the waitlist lines.
