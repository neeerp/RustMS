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
- On entering the game, the client actor:
  - sends the list (or nothing, if it is empty);
  - tells mutual buddies its channel;
  - asks about the first unanswered request.
- Answering a request asks about the next one.
- On logout, mutual buddies see channel -1. A channel change skips that,
  since the new channel announces itself.
- Buddy chat only reaches the listed recipients who are mutual buddies. The
  client prints its own line itself.

The packets are built in `net/src/packet/build/world/buddy.rs`. Each list
entry is the 39-byte `GW_Friend` record. Its channel is zero-based, or -1.

## Client

- `Character/BuddyList` holds the list as the server last described it. The
  player owns it. `SET_FIELD` sets its capacity.
- `Net/Handlers/BuddyHandlers` applies `BUDDYLIST` (63):
  - the list;
  - channel changes;
  - capacity;
  - requests, shown as a `UIYesNo` (yes accepts, no declines);
  - errors, shown as a `UIOk`.
- `MultiChatHandler` prints buddy chat in orange.
- The Friend tab of `UIUserList` is built from `UserList/Friend` art:
  - **ONLINE** (`friend0`), over a `SAME CH.` / `OTHER CH.` legend
    (`friend1`). Each `friend2` row holds a same-channel buddy on the left
    and an other-channel buddy on the right. Buddies on other channels show
    their channel number on the right.
  - **OFFLINE** (`friend4`), two greyed names per `friend5` row. This also
    covers buddies who haven't added the player back.
  - Clicking a name selects that buddy.
  - The buttons are ADD BUDDY (asks for a name with `UIEnterText`), DELETE
    (asks for confirmation), WHISPER, TALK (sets the chat target to "To
    Buddy"), and INVITE TO PARTY. MAP is always disabled.
  - SHOW ONLINE / SHOW ALL hides or shows the offline section. The two
    buttons share a spot, so `filter_clicked` makes sure one press toggles
    only once.

## Not implemented

- Groups: everyone is in "Default Group", and there is no ADD GROUP or EDIT.
- Raising the capacity (packet 0x15 is parsed but never sent), and the
  Cash Shop flags.
- The MAP button, notes, and the blacklist.

## Unverified against period footage

No 2008–2010 footage of the Friend tab was found, so the layout is inferred
from the art:

- the two columns;
- where names and channels sit in a row;
- the button order;
- the request and delete wording;
- the buddy chat colour (orange).
