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
  invitations are refused with "taking care of another invitation".
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

The packets are built in `net/src/packet/build/world/party.rs`. The member
block always has six slots.

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
  messages and shows invitations as a `UIYesNo`.
- `UIUserList` is the v83 "Maple User List & Guild" window
  (`UIWindow.img/UserList`). Only the Party tab works so far.
  - Members are listed under online and offline. The leader has a star.
    Members off the player's map are greyed.
  - Clicking a row selects that member, for Party Leader, Whisper, and Expel.
  - Invite opens `UIEnterText` to ask for a name.
- `UIPartyHP` is the translucent HP list at the top right
  (`UserList/Party/PartyHP`). `UIQuestHelper` moves down below it.
- `MapChars::draw_party_hp` draws a red bar over the head of each member on
  the map. The HP MARK button turns these bars on and off.
- In `UIChatBar`, the party chat key (2 by default) or the Talk button sets
  the target to "To Party". The say key sets it back to "To All".
- `IO/PartyCommands` runs `/partyinfo`, `/makeparty`, `/leaveparty`,
  `/partyinvite name` and `/partyexpel name` from the chat bar. The party
  window's Create, Leave, Invite and Expel buttons go through it too, as in
  v83. It refuses what v83 refuses before contacting the server: creating or
  inviting as a Beginner, Noblesse or Legend below level 10, inviting
  yourself, inviting or expelling without leading the party, inviting into a
  full party, and expelling someone who isn't a member. v83's check for maps
  where leaving or expelling isn't allowed is not implemented.

## Unverified against period footage

- The party chat colour (#FF99CC).
- The exact wording of "has become the leader" and the invite prompt.
- The bottom button grid's exact pitch.
- v83 used a fading notice for invitations; this uses a plain yes/no box.
- Whisper fills in `/w name` in the chat bar. In footage, v83 asked for the
  name in a dialog first.
