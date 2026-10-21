# Guilds

A guild is a named group of up to its capacity of characters, founded at
Heracle in the Orbis guild headquarters. Members see the guild's name and
emblem under each other's names, the stat window and character info show it,
and guild chat reaches the members online. The master and the Jr. Masters
invite players on their channel, move members a rank up or down, and post
the notice; the master names the ranks. Members leave, and are expelled.

## Server

The `guilds` and `guild_members` tables (`db/src/guild.rs`) hold the guilds
and who is in them. A character is in at most one guild (the member row's
key), and rank 1 is the master. Names are unique within a world, ignoring
case (a unique index on `lower(name)`).

- `game::guild` has the model (`Guild`, `GuildMember`, `GuildTag`,
  `Emblem`) and the server-only rules: founding needs level 10 and costs
  1,500,000 mesos (HeavenMS defaults; the client checks neither), and names
  follow the character name rules (4 to 12 ASCII letters or digits, with
  fewer than four of 'I' and 'l', and none of the forbidden words in
  `Etc.nx`, `net::names::forbidden_names`). A new guild has capacity 10 and the
  titles Master, Jr. Master, Member, Member and Member.
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
  `GuildRequest`: reload (mode 0), create with a name (2), invite by name
  (5), accept an invitation (6), leave (7), expel (8), set the five titles
  (0x0D), move a member to a rank (0x0E, the id and a byte), or post the
  notice (0x10). Other modes are ignored. `DENY_GUILD_REQUEST` (127) turns an invitation down: 0x35 when
  the client was busy, 0x36 when it already shows one, 0x37 when declined.
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
- Invitations: the actor checks that the inviter is the master or a Jr.
  Master and that the guild has room (`Guild::may_invite`), then the world
  directory finds the invitee by name, matching case, on the inviter's
  channel. Nobody there gets 0x2A; someone in a guild gets 0x28. The
  directory remembers the invitation for 3 minutes, one guild at a time
  (`runtime/src/actor/guild_invites.rs`): another guild's invitation is
  refused with 0x36 while one is waiting. A refusal reaches the inviter
  once, as 0x35, 0x36 or 0x37 with the invitee's name.
- Accepting an invitation that is still waiting tells the invitee's actor
  to join. `db::guild::add_member` adds them at the lowest rank with a title
  (`Guild::lowest_rank`, the client's FUN_00a3930b; rank 5 with the default
  titles), locking the guild so it can't pass its capacity (0x29 when full). Every member online gets
  0x27 with the newcomer's record, the newcomer included; their client asks
  for the whole guild again (mode 0).
- Leaving (anyone but the master) sends 0x2C to the members and the leaver.
  Expelling follows the client's rule (`Guild::may_expel`): the master and
  Jr. Masters expel, but nobody the master, and only the master a Jr.
  Master. 0x2F goes to every member, the expelled one included, and their
  actor, if online, reads their guild again (`DirectoryMessage::GuildChanged`).
  Either way the map sees the name and emblem go.
- Ranks, titles and the notice change under the guild's lock
  (`db::guild::set_rank`, `set_titles`, `set_notice`) if the client's own
  rules allow it, which it checks before sending:
  - `Guild::may_change_rank`: the master moves members ranked 2 and lower,
    a Jr. Master those ranked 3 and lower, one step at a time, and nobody
    goes down past the lowest rank with a title.
  - `Guild::may_set_titles`: only the master; ranks 1 to 3 keep a title,
    titles are guild names of at most 10 letters (forbidden words
    included), title 5 needs title 4, and no member is left below the
    lowest titled rank. Titles sent back unchanged aren't checked again;
    the default "Master" holds the forbidden word "master".
  - `Guild::may_set_notice`: the master or a Jr. Master, 1 to 100 bytes,
    with no control characters but CR and LF, as the client's prompt
    allows. The client censors bad words with `*`; the server doesn't.
  - Every member online then gets 0x40 (id, rank), 0x3E (the titles) or
    0x44 (the notice). A change the client wouldn't send is logged and
    dropped.
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
    Too short or not allowed (`Util/CharacterName.h` and the forbidden
    words, `CurseData::in_name`) says so and asks again; cancel sends
    nothing.
  - 0x1A replaces the guild; 0x20 does too, and Heracle congratulates the
    master while other members get a chat line.
  - 0x1C has Heracle say the name is taken, then asks again. 0x21, 0x23,
    0x38 and unknown modes are chat lines.
  - 0x05, an invitation, shows a card above the status bar
    (`UIInviteCard::GUILD`): "from '<name>'" over "Guild invite". O sends
    mode 6 and X sends 0x37; letting it fade sends nothing. As in v83, the
    client refuses by itself with 0x36 while a guild card is up, or 0x35
    while a window that holds the cursor is open (`UI::modal_open`).
  - 0x27 says "You have entered the guild." and asks for the guild (mode
    0), or adds someone else's record. 0x2C and 0x2F take a member out, or
    clear the guild if it is the player. These three only count for the
    player's guild. 0x28, 0x29, 0x2A, 0x2D, 0x30 and the refusals 0x35 to
    0x37 are chat lines.
  - For the player's guild only: 0x3E stores the titles, and the master
    gets "It is saved."; 0x40 moves a member and says "[name]'s position
    has been changed to [title]." with the new rank's title; 0x44 stores
    the notice, an empty one clearing it, and prints "* Guild Notice :
    <notice>" unless it is empty. v83 prints every guild line in its type
    0xc colour, which is `LineType::SKILL`.
- `IO/GuildCommands` has v83's checks before asking the server:
  - `/guildinfo` lists every member, online or not, and says if the player
    is the master.
  - `/leaveguild` asks "Would you like to quit the guild?"; the master can't.
  - `/guildinvite <name>` needs the master or a Jr. Master and room.
  - `/guildexpel <name>` needs the name of a member (matching case) who
    isn't the master, and only the master names a Jr. Master. It asks
    first, and leaves the player's own rank to the server.
  - Commands match without regard to case, as the party commands do; the
    context menu's GUILD INVITE uses `invite` too.
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
    to lead a party.
  - INVITE (the master and Jr. Masters) asks for a name, LEAVE (anyone but
    the master) and EXPEL run the commands. EXPEL needs a picked member
    the player may expel (`Guild::may_kick`), online or not.
  - UP and DOWN move the picked member a rank without asking
    (`Guild::may_raise`, `may_lower`): the master raises members ranked 3
    and lower and lowers anyone else, a Jr. Master raises those ranked 4
    and lower and lowers those ranked 3 and lower, and nobody is lowered
    from the lowest rank with a title.
  - NOTICE (the master and Jr. Masters) asks for up to 100 letters; OK
    trims them, censors bad words with `*` (`CurseData::filter`) and sends
    them unless nothing is left.
  - INFO, with a guild, opens or closes the rank panel (`GuildGrade` art)
    beside the window, at (311, 86): the guild's GP, then a row per rank
    with its number, its title and its fixed name (Master, Jr.Master,
    Member), the name grey while the rank has no title. Anyone may pick a
    row. The master's MODIFY asks for the picked rank's title: ranks 1 to
    3 need 4 letters, 4 and 5 may be emptied unless a member holds the
    rank (or, for 4, rank 5 has a title); a title follows the guild name
    rules, with 10 letters at most. Giving rank 5 a title names an empty
    rank 4 "Member". SAVE asks first, then sends the titles if any differ,
    ignoring case, or says "It is saved.". The panel edits a copy of the
    titles taken when it opens; it closes with the window, on Esc, or on
    another tab. Leaving the guild leaves it open with only its
    background.
  - Clicking a tab, even the one shown, drops the guild pick.
  - WHERE and BBS stay disabled: there is no member search or guild board.
  - The client places the guild buttons by their top left, ignoring the
    origin that BtGuildBBS and BtInfo have (inferred from the button grid),
    so the tab adds the origin back.

## Not implemented

- Disbanding.
- The card's option to refuse all invitations, and the blacklist, which
  v83 also checks before showing one.
- Making the emblem; raising the capacity.
- The rank panel's slide in and out. v83 keeps the panel's edits if it is
  opened again while sliding back; this one always starts over. A title
  too short closes the prompt, says so and asks again, where v83's prompt
  stays open.
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
- `UI::modal_open` stands in for v83's FUN_0049ed00; which windows count
  as modal there hasn't been checked.
