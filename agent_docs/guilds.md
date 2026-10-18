# Guilds

A guild is a named group of up to its capacity of characters, founded at
Heracle in the Orbis guild headquarters. Members see the guild's name and
emblem under each other's names, the stat window and character info show it,
and guild chat reaches the members online. So far a guild can only be
founded; nobody can join, leave, or change it in game.

## Server

The `guilds` and `guild_members` tables (`db/src/guild.rs`) hold the guilds
and who is in them. A character is in at most one guild (the member row's
key), and rank 1 is the master. Names are unique within a world, ignoring
case (a unique index on `lower(name)`).

- `game::guild` has the model (`Guild`, `GuildMember`, `GuildTag`,
  `Emblem`) and the server-only rules: founding needs level 10 and costs
  1,500,000 mesos (HeavenMS defaults; the client checks neither), and names
  follow the character name rules (4 to 12 ASCII letters or digits). A new
  guild has capacity 10 and the titles Master, Jr. Master and Member.
- `Player::guild` is the guild's tag (id, name, emblem), loaded with the
  player. `FieldAvatar` carries it, so `SPAWN_PLAYER` shows it to others;
  `CHAR_INFO` shows the name.
- Heracle (2010007, `net/src/npc_scripts.rs`) says why not if the player is
  already in a guild, under level 10, or short of the fee. Otherwise he asks,
  and yes sends the name prompt (`GUILD_OPERATION` mode 0x01) and sets the
  actor's `naming_guild` flag. His lines are `heracle.*` content keys; no
  licensed script has his text, so they stay placeholders (`unsourced` in
  `game-data/server_content.toml`).
- `GUILD_OPERATION` (`net/src/packet/handle/world/guild.rs`) becomes a
  `GuildRequest`: reload (mode 0) or create with a name (mode 2). Other
  modes are ignored.
- The client actor (`runtime/src/actor/client/guilds.rs`) founds the guild
  only while `naming_guild` is set, so a forged create does nothing. It
  checks everything again: a GM gets 0x38, a member 0x21, a low level 0x23,
  and a bad or taken name 0x1C, which asks for another. If the mesos went
  since Heracle asked, nothing is sent. On success it takes the fee, saves,
  and sends the meso update and 0x20 with the guild. The map sees the new
  name and emblem (`GUILD_NAME_CHANGED`, `GUILD_MARK_CHANGED`) through
  `FieldMessage::UpdateAvatar`, whose `announce` is a list.
- On entering the game, a channel change included, a member's actor loads the
  guild and the world directory sends it (0x1A), marking members online on
  any channel (`DirectoryMessage::GuildInfo`). The client starts with no
  guild, so nothing is sent for none.
- Guild chat is the guild mode (2) of `MULTICHAT`. The server ignores the
  client's recipient list and sends the line to every other member online
  (`DirectoryMessage::Deliver`). The speaker's client prints its own line.

The packets are built in `net/src/packet/build/world/guild.rs`. The guild
data lists the member ids, then a 37-byte record per member: the name padded
to 13 bytes, then job, level, rank, online, contribution and alliance rank as
ints. The client takes the first member for the master.

## Client

- `Character/Guild` holds the guild as the server last described it; the
  player owns it. `Character/GuildMark` draws an emblem (the background,
  with the mark centred on it).
- `Net/Handlers/GuildHandlers` applies `GUILD_OPERATION` (65):
  - 0x01 asks for the name with `UIEnterText` (v83 uses a UtilDlgEx input).
    Too short or not allowed says so and asks again; cancel sends nothing.
    The client's forbidden word list isn't in HeavenClient.
  - 0x1A replaces the guild; 0x20 does too, and Heracle congratulates the
    master while other members get a chat line.
  - 0x1C has Heracle say the name is taken, then asks again. 0x21, 0x23,
    0x38 and unknown modes are chat lines.
  - `GUILD_NAME_CHANGED` (202) and `GUILD_MARK_CHANGED` (203) change what
    another character shows.
- `Char` draws the guild name under the character's name, with the emblem
  to its left when it has one. The stat window shows the name, or "-".
- Guild chat: the GUILDCHAT key and `/guild` set the chat target to "To the
  Guild". A line goes to the members the client knows are online; with
  nobody online it says so and sends nothing. Guild lines are mauve.
- The Guild tab of `UIUserList` (`UserList/Guild` art on `backgrnd2`):
  - The header shows the emblem, or the empty frame, before the guild name,
    centred; or "You are not in the guild." The notice scrolls through a
    270 px strip one pixel every 30 ms, or reads "* Guild Notice" if empty.
  - The member list has an online and an offline section, each with a count,
    20 members a page with arrows, and a button to fold it. Columns are name,
    job (cut to 67 px with ".."), level and title. The list scrolls 33 px a
    step by the scroll bar or the wheel. A section's buttons show only while
    its header is wholly in view.
  - Clicking a row picks the member. WHISPER and TALK work for a picked
    member online who isn't the player; PARTY INVITE also needs the player
    to lead a party. The other nine buttons are always disabled for now.
  - The client places the guild buttons by their top left, ignoring the
    origin that BtGuildBBS and BtInfo have (inferred from the button grid),
    so the tab adds the origin back.

## Not implemented

- Invitations, joining, leaving, expelling, rank changes, and disbanding.
- Editing the notice, titles, or emblem; raising the capacity.
- Telling members when someone logs in or out, levels up, or changes job:
  the window shows who was online when the player entered the game.
- Guild points, the guild BBS, alliances, and the guild quest.
- Sorting by column.

## Inferred or unverified

- The scroll range: v83 sets "height/33+2", read as a count of positions,
  so the last one is height/33+1 steps (height being what overflows the
  192 px view).
- How text is clipped at the list's and the notice strip's edges.
- The name tag's exact placement, which no footage has confirmed.
