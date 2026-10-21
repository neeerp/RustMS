# Buddy lists

Each character has a list of up to 20 buddies. Buddies see each other's
channel and can talk in buddy chat, but only once both have added each other.

## Server

The `buddies` table (`db/src/buddy.rs`) holds one row per list entry. A row
`(owner, buddy)` puts `buddy` on `owner`'s list. A `pending` row is a request
from `buddy` that `owner` hasn't answered yet. A pair is *mutual* when both
characters list each other without `pending`.

- `BUDDYLIST_MODIFY` (`net/src/packet/handle/world/buddy.rs`) becomes a
  `BuddyRequest`: add by name (mode 1), accept (2), or remove/decline (3).
  The buddy mode (0) of `MULTICHAT` becomes `BuddyRequest::Chat`.
- The client actor (`runtime/src/actor/client/buddies.rs`) handles requests
  against the database. It keeps no copy of the list, so every change
  re-reads it.
- Adding someone who hasn't listed you gives them a pending row, and they
  are asked to accept. Adding someone who has already asked you accepts
  their request instead. Names match case-insensitively.
- A group is only a name on each row (`group_name`, 16 characters, empty
  meaning "Default Group"). v83 has no group packet: it moves a buddy by
  adding them again under the new group, and adds, renames and deletes
  groups by doing that for each buddy. So adding a buddy already listed
  under another group moves them, sends the list again and asks them
  nothing; adding them under the same group is refused as already
  registered.
- `db::buddy::add` locks both characters' rows, so two requests that cross
  can't both insert a pending row.
- The capacity is the character's `buddy_capacity` (20; not persisted). It
  limits your accepted rows, and all of the target's rows, pending
  included.
- The world directory fills in channels (`DirectoryMessage::BuddyList`)
  because it knows who is online. It shows a channel only for buddies who
  are both mutual and online; everyone else gets -1.
- `DirectoryMessage::Deliver` sends a packet to whichever of a set of
  characters is online. Requests, channel updates, and buddy chat use it.
- Every list carries the unanswered requests too, flagged (`GW_Friend` flag
  1), as v83 expects: the client hides them, and asks about them itself
  once a map has loaded. A request made while the player is online is also
  sent on its own (mode 9).
- On entering the game, the client actor:
  - sends the list (or nothing, if it is empty);
  - tells mutual buddies its channel.
- On logout, mutual buddies see channel -1. A channel change skips that,
  since the new channel announces itself.
- Buddy chat only reaches the listed recipients who are mutual buddies. The
  client prints its own line itself.

The packets are built in `net/src/packet/build/world/buddy.rs`. Each list
entry is the 39-byte `GW_Friend` record. Its channel is zero-based, or -1.

## Client

- `Character/BuddyList` holds the list as the server last described it,
  requests included. The player owns it. `SET_FIELD` sets its capacity.
  Each entry keeps v83's flag: 1 is a request, hidden and not counted; 2 is
  always offline; 0, 3 and 4 go by the channel; higher ones are always
  online. Buddy chat goes to flags 0, 3 and 4 on a channel.
- `Net/Handlers/BuddyHandlers` applies `BUDDYLIST` (63):
  - the list (modes 7, 10 and 0x12), or one entry (8);
  - channel changes (0x14). A buddy coming online shows a pink "<name> is
    logged in." card, unless they are in the player's guild; one moving to
    the player's channel shows "You're on the same ch. with <name>.";
  - capacity;
  - requests (9), added to the list and shown as a fading notice
    (`UIInviteCard`, see [client.md](client.md)); O accepts and X declines;
  - errors, shown as a `UIOk`, and modes 0x10, 0x11, 0x13 and 0x16, whose
    text the server supplies.
- `IO/BuddyCommands` makes v83's checks before asking the server, each
  refusal a `UIOk`: adding an invalid name, a buddy already in that group,
  or yourself; accepting someone who didn't ask; removing someone not on
  the list. After each map loads it shows the unanswered requests again,
  unless a request card is still up.
- `MultiChatHandler` prints buddy chat in orange, and drops lines from
  buddies the player has blocked.
- The Friend tab of `UIUserList` follows v83's groups (reports b015, b018
  and b019). `Character/FriendGroups` holds the logic the tests cover:
  - The list is grouped by each buddy's group name: groups by name, the
    Default Group last and always shown, requests left out. Each header
    reads `name(online/total)` in blue, behind an arrow. Clicking a header
    picks the group and folds or unfolds it; the folded groups are kept in
    the `FriendFolded` setting.
  - Members sit two to a row, each with v83's icon: `icon01` if blocked,
    otherwise `icon00` if on the player's channel. Names are black if
    online, grey if away (offline, on no channel, or blocked), red if
    blocked, and white when picked. An empty group shows "none". A
    header's online count leaves out those away.
  - The tab header counts buddies against the capacity.
  - SHOW ONLINE / SHOW ALL hides or shows those away (kept in
    `FriendShowOnline`). The two buttons share a spot, so `filter_clicked`
    makes sure one press toggles only once.
  - Clicking a name picks that buddy; the buttons act on the pick.
    Double-clicking a buddy whispers one online or writes a note to one
    away; one blocked is offered unblocking first, and then the same.
    Double-clicking a picked header edits the group.
  - ADD BUDDY opens v83's Add Buddy dialog (`UIGroupPicker` with a name
    field, report b021): a name, and a combo that starts on the Default
    Group and lists the other groups after it (FUN_00a3fc85). A group typed
    blank is sent blank, and the server reads it as the Default Group.
    EDIT on a buddy opens the same dialog with the name filled in, which
    moves them to the group chosen; as in v83, it doesn't start on their
    own group.
  - ADD GROUP, or EDIT on a group, opens the Edit Group dialog
    (`UIFriendGroup`, art `GroupMod`): the group's name over every buddy,
    each with a checkbox, those already in the group checked. OK refuses a
    blank name, nothing checked among the 7 rows in view, or a name taken,
    as v83 does. Otherwise it moves each buddy checked into the group and
    each unchecked one out to the Default Group, one add each.
  - DELETE removes a buddy after asking. On a group it asks, then moves
    every member to the Default Group. The Default Group can be neither
    edited nor deleted.
  - GROUP WHISPER sets the chat target to a group. CHAT opens a messenger
    and invites the buddy; WHISPER, MESSAGE (a note) and PARTY do what they
    say.
  - BLOCK / UNBLOCK toggles a buddy, or a whole group (blocking unless
    every member already is), and prints "Buddy chat with "<name>" has been
    disabled." or "enabled.". As in v83 the server is told nothing: blocks
    are kept per character in the client's `FriendBlocked` setting. A
    blocked buddy counts as away, gets no buddy or group chat from the
    player, and their buddy chat is not shown (report b020).

## Not implemented

- Raising the capacity (packet 0x15 is parsed but never sent), and the
  Cash Shop flags.
- The MAP button, and the blacklist.
- Deleting a group's buddies along with it (v83's "DELETE BUDDIES AS WELL"
  checkbox).
- The hover tip on a buddy ("Double-click to send a note." or to whisper).
- Keeping folded groups per character (they are kept per client), and
  scrolling back to a header after it is clicked shut.

## Unverified against period footage

The tab's group layout, the Edit Group dialog and the delete-group prompt
match 2010 GMS footage (report b019). Still inferred from the client's
code alone:

- the Add Buddy dialog, whose group combo covers the prompt's second line
  (the coordinates are v83's own);
- where names and icons sit in a row;
- the buddy chat colour (orange);
- the height of the "none" line in an empty group;
- whether v83's header counts include a request still waiting, which the
  tab leaves out.
