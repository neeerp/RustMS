# Parties

Up to six characters form a party. They share kill EXP, see each other's HP,
and have their own chat channel.

## Server

The world directory (`runtime/src/actor/directory.rs`) owns parties. It keeps
them in `Parties` (`runtime/src/actor/parties.rs`), plain state with no I/O.
Parties live in memory: they survive logouts and channel changes, but not a
server restart.

- Every `PARTY_OPERATION`, `DENY_PARTY_REQUEST`, and party `MULTICHAT` packet
  becomes a `PartyRequest` (`net/src/packet/handle/world/party.rs`), which
  the client actor forwards to the directory.
- Inviting someone without a party creates one first. An invitation keeps
  its receiver busy for `INVITE_TIMEOUT` (60 s); meanwhile, other parties'
  invitations are refused with "taking care of another invitation". A
  receiver whose client is still showing an invitation answers a second one
  with `DENY_PARTY_REQUEST` reason 0x16 instead of 0x17; that refuses it the
  same way and leaves the first one pending.
- When the leader leaves, the party disbands. When anyone else leaves or is
  expelled, the party goes on.
- The directory tracks each online character's `Presence`: name, job,
  level, map, and HP. Whenever that changes, or when someone registers (on
  login or a channel change), it sends the party a silent update and
  exchanges HP with members on the same map.
- Members on another channel are sent with map 0; offline members with
  channel -2. The client greys out both.
- Party chat goes to the other online members only. The client prints its
  own line itself.
- The directory tells the client actor of joins and departures
  (`ServerMessage::PartyChanged`). The client actor passes them on to its
  field (`FieldMessage::SetParty`), because the field decides who shares a
  kill.

- Members on the same channel see each other's Mystic Doors and may use
  them ([Mystic Door](./combat.md#mystic-door)).

The packets are built in `net/src/packet/build/world/party.rs`. The member
block always has six slots.

### Party search

v83's Party Search window lets a leader ask for members. It sends
`PARTY_SEARCH_START` (0xDE) with four ints: the minimum and maximum level,
the party size wanted, and a mask of the window's 25 job boxes.
`PARTY_SEARCH_STOP` (0xDF) carries nothing. The client sends it to pause or
to stop a search, and after any map change within a channel. There is no
reply. The server answers only with invitations.

- The directory keeps searches in `PartySearches`
  (`runtime/src/actor/party_search.rs`). `job_bit` maps a job to its box.
  Each class has a head box and branch boxes, and a branch box covers every
  advancement of that branch.
- A start is ignored unless its sender leads a party and passes v83's own
  checks:
  - the maximum level is at most 200;
  - the range spans at most 30 levels and the sender's level is in it;
  - the size is 2 to 6 and larger than the party;
  - at least one box is set.
- A search asks one character right away, then one more on each 10 s purge
  tick. The character must:
  - be on the leader's channel;
  - have no party and no pending invitation;
  - fit the levels and boxes;
  - not have been asked already by this search.

  The highest level goes first, and ties go to the lowest id. These rules
  are RustMS's own: Cosmic searches the whole world, and v83's server rules
  are unknown.
- The invitation is the ordinary one with its last byte set to 1. v83 then
  joins without asking if the player allows party searches and the leader
  isn't on the Blacklist. Otherwise it drops the invitation silently. A search's invitation never makes anyone
  busy, and an ordinary invitation replaces it.
- That Game Option is the only opt-out, and the server never sees it: a
  character who turned it off is still asked, and that tick's ask is spent
  on them. Cosmic adds a server-side opt-out, an "enable/disable Party
  Search" line on its party quest NPCs; it is not v83's and is left out.
- A search ends on 0xDF, when the leader goes offline or changes channel,
  and at a tick where the party is gone, has a new leader, or has reached the
  size wanted. The client starts it again after a channel change.

### EXP

`split_exp` (`runtime/src/actor/field_mobs.rs`) follows Cosmic's
reconstruction of v83:

- Each solo attacker, and each party taken as a whole, earns EXP for its
  share of the damage.
- A party's EXP goes to members on the field who are within 5 levels of the
  mob or of a member who hit it.
- Of that EXP, 80% is split by level. The member who did the most damage
  gets the other 20%.
- With more than one member sharing, each member also gets a bonus of 5% per
  sharing member. The client shows this bonus separately.

## Client

- `Character/Party` holds the party as the server last described it. The
  player owns it. Its HP values are only meaningful for members on the
  player's map.
- `Net/Handlers/PartyHandlers` applies `PARTY_OPERATION`,
  `UPDATE_PARTYMEMBER_HP`, and party `MULTICHAT`. It prints the v83 chat-log
  messages and shows invitations as a fading notice (`UIInviteCard`, see
  [client.md](client.md)). As in v83, an invitation arriving while one is
  shown is answered busy, and one with a nonzero party-search flag is
  accepted without asking.
- `UIUserList` is the v83 "Maple User List & Guild" window
  (`UIWindow.img/UserList`). Only the Party tab works so far.
  - Members are listed under online and offline. The leader has a star.
    Members off the player's map are greyed.
  - A job name longer than 10 characters is cut to its first 9 and "..",
    as v83 cuts it (`FUN_0090bddb`): "Dark Knight" shows as "Dark Knig..".
  - Clicking a row selects that member, for Party Leader, Whisper, and Expel.
    Party Leader asks "Do you wish to pass the leader position to NAME?"
    first, as v83 does (`FUN_0090e41a`). It is greyed out on a map whose
    `fieldLimit` has bit 0x800 (`FUN_0090bddb`).
  - Invite opens `UIEnterText` to ask for a name.
- `UIPartyHP` is the translucent HP list at the top right
  (`UserList/Party/PartyHP`). `UIQuestHelper` moves down below it.
- `MapChars::draw_party_hp` draws a red bar over the head of each member on
  the map. The HP MARK button turns these bars on and off.
- In `UIChatBar`, the party chat key (2 by default) or the Talk button sets
  the target to "To the party". The say key sets it back to "To All".
- Party notices and errors take their text from the v83 client's
  StringPool, and are logged in its notice colour (#FFAFAF, chat type 0xc),
  as in its party result handler (FUN_00a3e31c). Codes the client doesn't
  know log "Your request for a party didn't work due to an unexpected
  error."
- `IO/PartyCommands` runs `/partyinfo`, `/makeparty`, `/leaveparty`,
  `/partyinvite name` and `/partyexpel name` from the chat bar. The party
  window's Create, Leave, Invite and Expel buttons go through it too, as in
  v83. It refuses what v83 refuses before contacting the server: creating or
  inviting as a Beginner, Noblesse or Legend below level 10, inviting
  yourself, inviting or expelling without leading the party, inviting into a
  full party, and expelling someone who isn't a member. v83's check for maps
  where leaving or expelling isn't allowed is not implemented.

### Party search

- `Character/PartySearch` holds the leader's search: idle, searching or
  paused, and what was asked. It outlives the window, as v83 keeps it in its
  context. Each change says which packet to send, and
  `IO/PartySearchActions` sends it and updates the window and the indicator.
  - Every field entry sends `PARTY_SEARCH_STOP` and ends the search. After a
    channel change or a return from the Cash Shop, a running search sends
    `PARTY_SEARCH_START` again instead, and a paused one stays paused.
  - A join that brings the party to the size wanted ends the search, and so
    does logging out.
- `UIPartySearch` is v83's Party Search window (`UIWindow.img/PartySearch`).
  The O key (`KeyAction::PARTYSEARCH`, 26), `/partysearch`, and the Party
  tab's Search button open it. With the user list open, it docks to its
  right.
  - Its edits take digits only. Its 25 job boxes show the job names for the
    player's tier (below 70, to 119, from 120). Below level 30 only All,
    Beginner, Aran and the class heads show. All and each head set their
    whole group, and are set again when all of it is.
  - Start refuses what v83 refuses, in its order and words
    (`PartySearch::refusal`), and only the party leader may start.
  - While searching, the boxes and edits are disabled and Start is greyed.
    Paused, they work again.
- `UIPartySearchIcon` is the indicator at the top right, magnifier art
  animated while searching and still while paused. Clicking it pauses or
  resumes the search, and right-clicking stops it. It fades in and out.

## Unverified against period footage

- The party chat colour (#FF99CC).
- Whether the leading tab in v83's "has become the leader" line shows.
- The bottom button grid's exact pitch.
- Whisper puts the chat bar into whisper mode for the member. In footage,
  v83 asked for the name in a dialog first.
