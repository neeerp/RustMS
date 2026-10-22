# Families

A family is a tree of characters. Each character can have one senior and up
to two juniors, who can have juniors of their own. The senior at the top
leads the family, which goes by the leader's name and has the leader's
message (the precepts). The protocol comes from report b039 in the v83
reference notes.

## Server

The `family_members` table (`db/src/family.rs`) has one row per character
who has ever been in a family:

- `senior_id` points at the character's senior; the juniors are the rows
  that point at them.
- It also holds the rep counters: current, total, today, and rep recorded
  to the senior. `rep_day` is the day the two daily counters belong to;
  on another day they read as 0 and the next gain starts them again.
- `precepts` matters only on the leader's row. A junior who leaves keeps an
  empty one until they gain juniors of their own.

`game::family` holds the rules and the types:

- `MAX_JUNIORS` is 2.
- A junior must be level 10 or above, below the senior's level, and at
  most 20 levels below the senior (`check_levels`).
- A family may span at most 1000 generations.
- The precepts are at most 200 characters (`MAX_PRECEPTS`).
- Separating costs Rep and mesos (`separation_rep`, `separation_fee`; see
  below).
- Juniors earn their seniors Rep (`REP_PER_KILL`, `REP_PER_BOSS`,
  `REP_PER_LEVEL`, `rep_gain`; see below).
- `Refusal` maps each refusal to the `FAMILY_RESULT` code that picks the
  client's line.

`Player.family` (`Relatives`) holds the senior and juniors. It is loaded
with the character, read again once the character is in the directory (a
change made while they were entering the game reached no one), and copied
into the directory's `Presence`.

### Requests

The handlers are in `net/src/packet/handle/world/family.rs`. They turn
packets into `FamilyRequest`s, which the client actor
(`runtime/src/actor/client/family.rs`) carries out:

- **Entering the game, and `OPEN_FAMILY`:** the server sends
  `FAMILY_INFO_RESULT` and `FAMILY_PRIVILEGE_LIST`. The client sends
  nothing except `OPEN_FAMILY` and `OPEN_FAMILY_PEDIGREE` until it has
  received INFO. The privilege list holds the eleven entitlements (see
  below).
- **`ADD_FAMILY` (name):**
  1. The actor checks the database rules (`db::family::may_add`): not
     yourself, the junior has no senior and isn't one of yours, you have a
     free junior slot, and the generation limit holds.
  2. The world directory (`runtime/src/actor/directory/family.rs`) checks
     that the junior is online, on the same channel and map, and of the
     right level.
  3. `GuildInvites`, keyed by the senior's id, lets one senior ask at a
     time, for 180 s. A second senior gets "busy". The same senior asking
     again while the card is up is ignored, because the junior's client
     would decline the second card. A senior's requests end when they
     leave the channel.
  4. The junior gets `FAMILY_JOIN_REQUEST`.
- **`ACCEPT_FAMILY` (inviter id, name, yes/no):**
  - A no reaches the senior as `FAMILY_JOIN_REQUEST_RESULT` 0.
  - A yes is checked again for the map and levels, since either may have
    moved or levelled meanwhile. It then goes to the senior's actor
    (`ServerMessage::AddJunior`), which
    runs `db::family::add_junior`. That function checks the rules again
    inside a transaction that locks both rows. On success:
    - the senior gets `FAMILY_JOIN_REQUEST_RESULT` 1;
    - the junior gets `FAMILY_JOIN_ACCEPTED`;
    - everyone whose family changed (the senior, and the junior's whole
      branch) reloads `Relatives` and gets INFO again
      (`DirectoryMessage::FamilyChanged`).
- **`OPEN_FAMILY_PEDIGREE` (name):** the server sends
  `FAMILY_CHART_RESULT` for the named character (`db::family::chart`).
  - The chart lists the leader, then the two seniors above the character,
    the character, their sibling, their juniors, and their juniors'
    juniors, each parent before its children. These are the 11 slots of
    the window.
  - The client takes the first entry only as the top of the tree and never
    links anyone to it (report b044), so the leader comes again in the
    chain when it is one of the seniors or the character.
  - The directory marks who is online (a channel change counts) and fills
    in their current level, job and map.
  - Head counts:
    - key -1: the whole family;
    - key 0: the character's seniors;
    - each junior's id: that junior's branch.
  - Out of a family the chart is empty, and the client draws the player
    alone.
  - When INFO arrives while the player's own tree is open, the client asks
    for the chart again.

- **`CHANGE_FAMILY_MESSAGE` (text):** only the leader (a character with
  juniors and no senior) may set the precepts (`db::family::set_precepts`).
  The whole family then gets INFO again.
- **Separating:** `SEPARATE_FAMILY_BY_SENIOR` (0x94, the junior's id) is
  sent when the player cuts off one of their juniors, and
  `SEPARATE_FAMILY_BY_JUNIOR` (0x95, empty) when they leave their senior
  (report b048 corrects b039 on which is which).
  1. The actor names the pair from `Player.family`. A junior who isn't the
     player's, or 0x95 without a senior, is ignored.
  2. The directory adds the levels of whichever of the two is online
     (`DirectoryMessage::FamilySeparate`), and the player's actor runs
     `db::family::separate` (`ServerMessage::Separate`). It reads an offline
     character's level from the database.
  3. In one transaction that locks both rows and the senior's senior, it
     checks that they are still senior and junior and works out the fee:
     `(d + 2500) * d` mesos, where `d` is how many levels the senior is
     above the junior. The player pays, whichever side they are on. If the
     two are no longer related, the player just gets INFO again. If they can't, `FAMILY_RESULT` 0x50
     (leaving a senior) or 0x51 (cutting a junior) tells them how much they
     need.
  4. With `X = (L / 20 + 10) * L` for the junior's level `L`, the senior
     loses `2X` Rep and the senior's own senior `X`, never below 0. The
     junior's link and the Rep recorded to the senior are cleared.
  5. The player gets their mesos as a stat update, then `FAMILY_RESULT` 1,
     which the client shows with the name it remembered. Everyone in the old
     family gets INFO again.

### Earning Rep

v83's own amounts aren't known, so these are Cosmic's (report b049): 4 Rep
for each kill a junior gets EXP from (a share counts; a kill worth no EXP
doesn't), 20 for a boss, and 200 for each level they gain. The player's actor (`family_earn`) asks the directory
(`DirectoryMessage::FamilyEarn`) for the senior's level if they are online,
and `ServerMessage::EarnRep` then runs `db::family::earn`. The senior and
the senior's own senior each get the gain, halved when the senior's level
is below the junior's. It is added to their current, total and today's
counters and to the Rep the junior has recorded to the senior. Each gets
`FAMILY_REP_GAIN` (the gain and the junior's name) and then INFO again.

### Level and job notices

When a member goes up a level, the senior gets `NOTIFY_LEVELUP` for each
level: type 1, or type 0 when the new level is above the senior's, which
the client adds the halving line to. When a member changes job, the senior
and the juniors get `NOTIFY_JOB_CHANGE` type 1 with the job and "> " and the
name (`tell_family_progress`, next to the guild's).

### Login notices

Relatives (the senior and the juniors) get `FAMILY_NOTIFY_LOGIN_OR_LOGOUT`
when a member enters or leaves the game. A channel change is held for
`CHANNEL_CHANGE_GRACE` (`family_away`), as with guilds, so it says nothing.

### Entitlements

The Family window sells entitlements for Rep (report b050). v83's own list
isn't known, so `game::family::ENTITLEMENTS` has Cosmic's costs, limits and
benefits (report b049): 0 goes to a family member, 1 summons one, and 2-10
raise EXP or drop rates. Their names and descriptions are content-pack
text. `FAMILY_PRIVILEGE_LIST` sends them in that order, and INFO lists how
often each was used today, which the client needs to enable the button.

`USE_FAMILY` (0x97) carries the index and, for 0 and 1, a name. Spending
(`db::family::spend`) takes the cost and counts the use in
`family_entitlement_uses`, one row per character and entitlement for the
day. A player short of Rep or past the day's limit gets no answer, since
the client's button already stops both.
After the spend, and after the trip or the summons is under way, the
player gets `FAMILY_REP_GAIN` with minus the cost and an empty name, then
INFO.

- **Going to someone (0, 300 Rep):** the name must be a character
  (`FAMILY_RESULT` 0x41) in the same family (0x43). The directory finds
  them on the player's channel, and the player's actor checks the Teleport
  Rock rules for their map (`may_travel`, which refuses the player's own
  map). Either failing is 0x4b, "The summons has failed", a chat line. The
  player then pays and goes to the spawn point nearest the member.
- **Summoning (1, 500 Rep):** after the same checks, and the Teleport Rock
  rules for the summoner's own map (`may_summon_to`), the summoner pays at
  once. The directory keeps one summons per member (`family::Summons`); a
  second gets 0x4a, a member not on the channel 0x41, and both are
  refunded. The member gets `FAMILY_SUMMON_REQUEST` (104) with the
  summoner's name and the map's name from String.nx, and answers with
  `FAMILY_SUMMON_RESPONSE` (0x99). On no, or after 120 s without an
  answer, the summoner gets the Rep back and a notice. On yes the member
  goes to the summoner, unless the summoner has moved, or the member's map
  forbids it: then the summoner gets it back and 0x4b. The answer names
  the summoner, so a yes from an older summons' dialog, which the client
  never closes, does nothing. A member who leaves, before or just after
  saying yes, refunds the summoner. A summoner who leaves, even for
  another channel, drops their summonses and gets the Rep back without a
  word (`refund_summons`).

### Not yet

These are not implemented yet:

- the rate entitlements (2-10) and `FAMILY_SET_PRIVILEGE` (103), the icon
  that shows one is running.

## Client

HeavenClient follows reports b042 (Family window), b044 (Family Tree) and
b045 (how they open).

- **State:** `Character/Family.h` keeps the player's INFO and the privilege
  list on `Player`. `FamilyChart::arrange` puts a chart's members into the
  tree's 11 boxes as the v83 decoder does. `tests/family.cpp` covers the
  parser and the arrangement.
- **Packets:** `Net/Handlers/FamilyHandlers.cpp` handles receive opcodes
  94-102 and 104. `Net/Packets/FamilyPackets.h` builds the sends.
- **Family window (`UIFamily`):** the Family key (F by default) and BtFamily
  on the player's own info open it, and each opening sends `OPEN_FAMILY`.
  It shows the family's name, the scrolling precepts, juniors, Rep and the
  entitlements one at a time, with their tooltips. BtJuniorEntry asks for
  a name, BtTree opens the player's own tree, the precepts button (leader
  only) sends `CHANGE_FAMILY_MESSAGE` and BtSpecial sends `USE_FAMILY`.
  OK closes it, and so does Esc when it is the topmost window.
- **Family Tree (`UIFamilyTree`):** BtTree, and BtFamily on someone else's
  info, open it and send `OPEN_FAMILY_PEDIGREE`. It always opens at the same
  place and fills when the chart arrives. Only the player's own tree
  answers the mouse: the senior's and juniors' boxes light up, an empty
  junior box asks for a name, and each member has a tooltip. Esc closes it.
  Texts are centred in their boxes as v83 does it, halving the room left as
  unsigned, so one wider than its box is never seen: the empty boxes'
  "(You do not have family members.)" shows nothing.
- **Separating (report b048):** BtBye is enabled on the player's own tree.
  With the senior or a junior selected it goes straight to the Yes/No;
  otherwise it asks for a name and matches it exactly against the senior,
  then the juniors, saying nothing if none match. Double-clicking a filled
  senior or junior box does the same as BtBye. There is a single Yes/No
  (`FamilyParting::text`). It gives the Rep lost and, when there is one, the
  fee, from the levels on the chart. Either answer clears the selection.
  Yes sends 0x94 or 0x95 and remembers the name for `FAMILY_RESULT` 1's "You
  have severed ties with %s." The tree stays open and refreshes on INFO.
- **Adding a junior:** the name prompt, or the avatar menu's Family row
  (shown once INFO says there is room), sends `ADD_FAMILY`.
- **Being asked:** `FAMILY_JOIN_REQUEST` shows a Family card, unless one is
  up or the Game Option blocks it, when the client declines by itself. The
  card's X declines, and fading sends nothing. O asks once more with a
  Yes/No before answering.
- **Login notices:** a relative logging in shows a card, and the open tree
  updates who is online.
- **Being summoned:** `FAMILY_SUMMON_REQUEST` asks "From %s, %s has
  requested a summons. Would you like to transport?" with a Yes/No, and
  either answer sends `FAMILY_SUMMON_RESPONSE` with the summoner's name.
- **Level notices:** `NOTIFY_LEVELUP` type 0 is a family member who has
  passed the player's level. After the usual line the client adds "- The
  Reps you have received from %s will be reduced in half."
  (`GuildHandlers.cpp`).
