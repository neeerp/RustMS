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
  to the senior. Nothing earns rep yet.
- `precepts` matters only on the leader's row.

`game::family` holds the rules and the types:

- `MAX_JUNIORS` is 2.
- A junior must be level 10 or above, below the senior's level, and at
  most 20 levels below the senior (`check_levels`).
- A family may span at most 1000 generations.
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
  received INFO. The privilege list is empty for now.
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
  - The chart lists the leader, the two seniors above the character, the
    character, their sibling, their juniors, and their juniors' juniors,
    each parent before its children. These are the 11 slots of the window.
  - A senior's senior whose own senior isn't shown hangs from the leader.
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

### Login notices

Relatives (the senior and the juniors) get `FAMILY_NOTIFY_LOGIN_OR_LOGOUT`
when a member enters or leaves the game. A channel change is held for
`CHANNEL_CHANGE_GRACE` (`family_away`), as with guilds, so it says nothing.

### Not yet

These are not implemented yet:

- separating from a senior or a junior;
- entitlements (`USE_FAMILY`) and the summon;
- changing the precepts;
- earning rep.

## Client

HeavenClient has no family windows yet.
