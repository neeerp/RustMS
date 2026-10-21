# Guilds

A guild is a named group of up to its capacity of characters, founded at
Heracle in the Orbis guild headquarters. Members see the guild's name and
emblem under each other's names, the stat window and character info show it,
and guild chat reaches the members online. The master and the Jr. Masters
invite players on their channel, move members a rank up or down, and post
the notice; the master names the ranks, at Heracle grows the guild or
disbands it, and at Lea makes or changes its emblem. Members leave, and are
expelled.

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
- Heracle (2010007, `NpcScript::heracle` in `net/src/npc_scripts.rs`)
  offers a menu: make a guild, disband it, or make it bigger. His script
  needs the player's guild, so `dialogue.rs` sends him to the client actor's
  `talk_about_guild`, which loads it, rather than through `for_npc`. His lines
  are `heracle.*` content keys; no licensed script has his text, so they
  stay placeholders (`unsourced` in `game-data/server_content.toml`).
  - Making one: he says why not if the player is already in a guild, under
    level 10, or short of the fee. Otherwise he asks, and yes sends the name
    prompt (`GUILD_OPERATION` mode 0x01) and sets the actor's
    `naming_guild` flag.
  - Disbanding and growing are for the master only; anyone else is told so.
    Disbanding asks first. Growing adds 5 members up to 100
    (`guild::MAX_CAPACITY`) and costs `guild::capacity_cost`: 500,000 mesos
    up to 15, then 1,000,000 more for each 5 beyond 15, and 5,000,000 past
    30 (HeavenMS's prices). He quotes the price and the new size; a player
    short of it hears so after saying yes.
- Lea (2010008, `NpcScript::lea`) offers a menu with one choice, the
  emblem; `dialogue.rs` sends her to `talk_about_guild` too. Only the master
  is asked: she quotes 5,000,000 mesos (`guild::EMBLEM_COST`, the price in
  2008 footage and HeavenMS), and yes ends the conversation with
  `Effect::OpenEmblemMaker`, which sends 0x11. Anyone else is told only the
  master may. She doesn't check the mesos, as in HeavenMS; the save does.
  Her lines are `lea.*` content keys, placeholders like Heracle's.
- `GUILD_OPERATION` (`net/src/packet/handle/world/guild.rs`) becomes a
  `GuildRequest`: reload (mode 0), create with a name (2), invite by name
  (5), accept an invitation (6), leave (7), expel (8), set the five titles
  (0x0D), move a member to a rank (0x0E, the id and a byte), set the emblem
  (0x0F: background as a short, its colour as a byte, mark as a short, its
  colour as a byte), or post the notice (0x10). Other modes are ignored. `DENY_GUILD_REQUEST` (127) turns an invitation down: 0x35 when
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
- The world directory tells the guild who comes and goes. A member entering
  the game sends every other member online 0x3D (the guild, the member, 1),
  and leaving it 0x3D with 0; the member themself gets neither, as the
  client would show its own card. A second login of a character already
  online says nothing. A channel change says nothing either: the directory
  waits 30 seconds (the messenger's grace, `CHANNEL_CHANGE_GRACE`) for the
  member to arrive, sweeping on its 10-second tick, and only then sends the
  0x3D with 0. Meanwhile the guild data shows them online.
- When a member's level or job changes (a `DirectoryMessage::Presence`
  update, handled in `update_presence`), every member online gets 0x3C (the
  guild, the member, level, job), the member included. The others also get `NOTIFY_LEVELUP` (105: type 2, the
  level, the name) for each level gained, and `NOTIFY_JOB_CHANGE` (107: type
  0, the job, the name) for a new job. The client's job line has no `>`
  after "<Guild", so the name goes as "> " and the name. Nothing is sent
  when the guild itself changes in the same update.
- Saying yes to Heracle ends the conversation with an effect
  (`Effect::GrowGuild { from }`, `Effect::DisbandGuild`) that runs after
  the conversation's changes are saved:
  - Growing checks the mesos again, then `db::guild::grow` grows the guild
    under its lock only if the player is still the master and the capacity
    is still the `from` Heracle quoted, so a stale yes can't pay twice.
    Every member online gets 0x3A (the guild, the new capacity), and the
    master pays and gets the meso update and loss line. Any refusal is
    0x3B.
  - `db::guild::disband` deletes the guild, if the player is still its
    master, and returns who was in it; the member rows go with it. The
    directory drops invitations to it (`DirectoryMessage::GuildDisbanded`),
    every member online gets 0x32 (the guild), and each member's actor reads
    their guild again, so the map sees the name and emblem go. A refusal is
    0x34.
  - A guild that is gone is no error. Loading it gives nothing, and an actor
    that finds its player's guild gone sets it to none.
- Setting the emblem (`set_emblem` in `runtime/src/actor/client/guilds.rs`)
  checks, in this order, that the player is in the headquarters
  (200000301, `guild::HEADQUARTERS`), that the emblem is one the maker can
  make (`Emblem::is_valid`: backgrounds 1000 to 1030, marks 2000 to 2020,
  3000 to 3006, 4000 to 4021, 5000 to 5035 or 9000 to 9026, colours 1 to
  16), and that the player is the master. Any of these failing is logged
  and ignored, as no client sends it. Then a master short of the price gets
  a popup (`lea.no_mesos`); otherwise `db::guild::set_emblem` saves it under
  the guild's lock if they are still the master, every member online gets
  0x42 (the guild, the emblem), each member's actor reads their guild
  again so their map sees the new emblem (203), and the master pays.
- Guild chat is the guild mode (2) of `MULTICHAT`. The server ignores the
  client's recipient list and sends the line to every other member online
  (`DirectoryMessage::Deliver`). The speaker's client prints its own line.

### Bulletin board

The guild board (`GUILD_BBS`, recv 0x9B, send 0x3B; the RE notes are
`v83ref/scratch/251/bbs.md`, outside this repository) lives in `guild_bbs_threads` and `guild_bbs_replies`
(`db/src/guild_bbs.rs`). Each guild numbers its threads from 1 (the largest
number plus one, so the newest thread's number comes back after it is
deleted). The notice is thread 0, so a guild has at most one. Replies are
numbered across every guild. Deleting a guild, a thread or a poster deletes
their threads and replies; a member who leaves or is expelled keeps theirs,
which the client shows with no writer name.

- The client sends a mode byte: 0 write or edit, 1 delete a thread, 2 list
  a page, 3 show a thread, 4 reply, 5 delete a reply. After every change it
  asks for the list again, so the server answers only the asker, and only
  this: mode 7 (the thread with its replies, oldest first) after a post,
  edit, reply or deleted reply; mode 6 (the notice, the number of other
  threads, then up to ten of them, newest first) for a list; mode 8 ("This
  message had been deleted by another user.") when the thread to show or
  reply to isn't there. A deleted thread and a refused request get nothing. A
  successful write needs mode 7, or the client stays in the write panel.
- `runtime/src/actor/client/guild_bbs.rs` ignores a player with no guild,
  an empty title, body or reply, text with a NUL character, and icons other than 0 to 2, or 100 to 106
  without the emoticon item 5290000 + (icon - 100). It cuts titles to 25
  characters, bodies to 600 and replies to 25, as the reference server does.
- The DB functions read the member's guild and rank in their transaction.
  Only the master and jr. masters (rank 2 or better) post the notice, and
  only when there is none; the poster or those ranks edit or delete a
  thread or reply (so a demoted member still edits the notice they posted).
  An edit dates the thread now. A page past the end is
  the first page. A reply is found only on the asker's own guild's board
  (Cosmic doesn't check this).
- Dates are UTC FILETIMEs. The client prints them unconverted, so players
  see UTC.
- The harness test `guild_bbs` covers posting, pages, icons, the notice,
  replies, cuts and every permission; `found_guild` makes a guild in the
  database and `Trace::bbs` collects the board's packets.

The client's board is `UIGuildBBS` (`GuildBBS.img/GuildBBS`, 734 x 526,
centred). The rules it tests are in `Character/GuildBbs.h` and the packet
parser in `Net/Handlers/Helpers/GuildBbsParser.cpp` (both covered by
`tests/guild_bbs.cpp`); the handler drops recv 59 unless the board is open.

- Opening asks for page 0. The left lists the notice (no number) and the
  page's threads: number and title, the reply count in blue, the icon, the
  writer cut to 70 px, and the date (the time if it is today, else
  month/day). As in v83, the time is shown as sent (UTC) but "today" is the
  player's local date, so near midnight a post away from UTC can show the
  wrong form. Clicking a row asks for that thread
  (the notice is 0) and the page again; clicking a page number asks for
  that page and keeps the thread shown. A page past the last (after its
  last thread is deleted) gets the first page from the server, and the
  board then shows page 1 as current.
- The header shows the guild's emblem when it has a mark, else the empty
  frame, as the Guild tab does.
- Mode 7 rebuilds the right panel: the title, writer and full date, the
  body wrapped at 240 px (its own line breaks kept) in lines of 12 px
  with a scroll bar, and the replies four at a time, starting at the last
  four. The wheel scrolls the body above the buttons and the replies
  anywhere else. DELETE (the poster, the master or a Jr. Master) and the reply rows'
  delete buttons ask "Are you sure you want to delete the post?" first.
  Deleting a thread drops it from the view and asks for the page; deleting
  a reply asks for the page and waits for the server's thread.
- Mode 8 says "This message had been deleted by another user. Please check
  again." and drops the thread. QUIT closes the board; Esc does not.
- DELETE follows rank changes when the next thread arrives; the reply
  rows' delete buttons also on every reply scroll, as in v83.
- WRITE, NOTICE (masters and Jr. Masters) and EDIT (the thread's poster or
  a moderator) swap the thread view for the write panel (`backgrnd3`; RE
  notes `v83ref/reports/b009-bbs-write-re.md`). NOTICE while a notice
  exists says "Please delete the current notice and then try again."
  instead. The panel has a 25-byte title, a 600-byte body that shows 15
  lines and scrolls past them (its scroll bar and the wheel move it, and it
  follows the caret), the icon palette, SUBMIT and CANCEL. The list, the
  page selector, the reply scroll bar and QUIT stay. Nothing takes the
  focus until a click, Tab does nothing, and Enter in the body breaks the
  line with a lone "\r" (a pasted "\r\n" also gives one).
- The palette offers icons 0 to 2, then the cash icons 100 to 106 whose
  items (5290000 to 5290006) the player has. A click picks one; the arrows
  move only the highlight, so the icon sent stays the last one clicked, as
  in v83. An edit starts on the thread's icon if the palette has it, else
  on icon 0.
- SUBMIT refuses an empty title ("Please enter the title."), an empty body
  ("Please enter the text.") and text the curse filter won't take in either
  ("Cursing is not allowed."), and stars out curse words in both; otherwise it sends the thread, asks for page 0, and keeps the
  panel until the server's thread (mode 7) replaces it. CANCEL asks for the
  last thread clicked or shown and the page again.
- The reply box (25 bytes) sends with the pencil button only; Enter does
  nothing. It refuses empty text and censors as SUBMIT does, sends the
  reply, asks for the page, and empties only when the view is rebuilt.

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
  - 0x32 clears the guild; Heracle tells the master "The guild has been
    disbanded..", and the others get the chat line "The guild has been
    disbanded.". 0x3A sets the capacity, and Heracle tells the master the
    new size. Both only count for the player's guild. 0x34 and 0x3B have
    Heracle say the disbanding or growing went wrong.
  - 0x11 opens the emblem maker, for the master only; v83 ignores it
    otherwise. 0x42 stores the emblem, for the player's guild only, and
    redraws the player's tag; the master gets "It is completed." v83's box
    is modal, so there the tag changes only once it is dismissed.
  - 0x3D marks a member online or offline. Coming online shows the buddy
    list's card, "<name> is" over "logged in." with the invite sound; v83
    checks neither the old flag nor who it is, and going offline shows
    nothing. 0x3C sets a member's level and job without a word. Both only
    count for the player's guild and a member it knows.
- `NOTIFY_LEVELUP` (105) prints "<Guild> [name] has reached Lv. [level]."
  for type 2 and "<Family> ..." for any other; v83's type 0 also adds a
  family line about Reps, which needs families. `NOTIFY_JOB_CHANGE` (107)
  prints "<Guild[name] has advanced to a(an) [job]." for type 0 and
  "<Family..." for 1, and nothing for any other type. Both are
  `LineType::SKILL` lines. The job is named by `Job::get_name`, v83's
  table (`FUN_004a77ef`), spelling included (100 "Swordman", 320 "Crossbow
  man"), but for 800's Korean name; a job it doesn't name has an empty
  name. Every window that names a job from its id uses it; the skill
  window's caption is the book's name from `Skill.img`.
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
- `IO/UITypes/UIGuildMarkMaker` is Lea's emblem maker (`MakeMark` art,
  250x343 at (275, 100) on an 800x600 screen), with its rules in
  `Character/GuildEmblemRules.h`:
  - It starts from the guild's emblem, or from background 1000, mark 2000
    and colour 1 for both if the guild has none.
  - The scroll unrolls under the emblem preview (a white 17x17 square with
    the emblem on it). A second after the animation ends the title, the four
    labelled boxes, the arrows, the category combo, OK and CANCEL appear;
    until then nothing responds, Esc and Enter included.
  - The arrows step the background, the mark within its category, and the
    two colours (1 to 16), wrapping at either end. The last id of each
    category is the largest child name in `GuildMark.img`.
  - The combo lists Animal, Plant, Pattern, Character and ETC (marks 2000,
    3000, 4000, 5000 and 9000 on); picking one starts at its first mark.
    v83 indents the names, so they carry six spaces.
  - OK or Enter asks "Would you like to save it as a guild emblem?"; yes
    sends mode 0x0F and closes the maker, no leaves it as it was. CANCEL or
    Esc closes it and sends nothing. v83's box has OK and CANCEL; this one
    is the client's usual YES and NO.
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
  - WHERE stays disabled: there is no member search.
  - BBS, for any member, opens the board window (below) unless it is open.
  - The client places the guild buttons by their top left, ignoring the
    origin that BtGuildBBS and BtInfo have (inferred from the button grid),
    so the tab adds the origin back.

## Not implemented

- The card's option to refuse all invitations, and the blacklist, which
  v83 also checks before showing one.
- The rank panel's slide in and out. v83 keeps the panel's edits if it is
  opened again while sliding back; this one always starts over. A title
  too short closes the prompt, says so and asks again, where v83's prompt
  stays open.
- Guild points, alliances, and the guild quest.
- The board's page selector art: the labels are drawn as text, 32 px apart,
  with "<" and ">" for other groups of ten. The original's pitch and arrows
  are unknown.
- The board shows 14 body lines where v83's edit box shows 15: our font's
  fifteenth line would cross the panel's edge.
- The board looks writers' names up when a list or thread arrives, where
  v83 looks them up on every draw, so a member who leaves keeps their name
  until the next request.
- Nothing closes the board on a map change or when the player leaves the
  guild; the server then ignores its requests.
- Sorting by column.

## Inferred or unverified

- The scroll range: v83 sets "height/33+2", read as a count of positions,
  so the last one is height/33+1 steps (height being what overflows the
  192 px view).
- How text is clipped at the list's and the notice strip's edges.
- The name tag's exact placement, which no footage has confirmed.
- `UI::modal_open` stands in for v83's FUN_0049ed00; which windows count
  as modal there hasn't been checked.
- The board's notice is thread 0 on the wire, inferred from the client
  sending 0 when it is clicked.
- The board shows bodies split on "\r\n", "\n" and "\r" (v83 writes a
  lone "\r"; how its view splits the others is untraced), so a closing
  break shows a blank last line.
- The board's 12 px body line and 32 px page-number pitch are guesses.
- Whether Esc closes v83's board; the spec names only QUIT.
