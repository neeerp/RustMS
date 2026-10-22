# Engagement and marriage

Two players of different genders become engaged with an engagement ring
box, may break it off, and may later marry. Only the engagement and
breaking it off are done so far. The protocol comes from reports b052 and
b054 in the v83 reference notes.

## Storage

The `marriages` table (`db/src/marriage.rs`) has one row per couple:

- `groom_id` and `bride_id` are both unique, so a character is in at most
  one couple. The rows go when either character is deleted.
- `status` is 1 when the couple is engaged and 3 when they are married.
- `item_id` is the ring box the couple were engaged with.

`Player.marriage` holds the couple's `game::marriage::MarriageRecord`. It is
loaded with the character, and the character data sends it as the third
ring list, a 0x30-byte record (`write_marriage_record`). The same record
follows `MARRIAGE_RESULT` 11.

## Proposing

The ring boxes are 2240000 to 2240003. Using one from the Use tab makes
the client check these, in this order:

1. The player has a free Etc slot.
2. The player is male. v83 ignores a woman's box without a word.
3. The player isn't already engaged or married.
4. No dialog is open, which includes the wait for an earlier proposal.

If all of them pass, the client asks for the partner's name and sends
`RING_ACTION` 0 with the name and the box. While the proposal waits, it
shows "Waiting for her response...". Closing that dialog sends
`RING_ACTION` 1, which withdraws the proposal.

The proposer's client actor (`runtime/src/actor/client/marriage.rs`)
refuses with a `MARRIAGE_RESULT` mode (`game::marriage::Refusal`) in these
cases:

- the proposer is engaged or married;
- the proposer's Etc tab is full;
- the name is their own, longer than 12 characters, or not a character's;
- the partner is engaged or married;
- the two are the same gender.

If the proposer is below level 50, they get a `MARRIAGE_RESULT` 36 notice
instead (`marriage.level` in the content pack).

The directory (`runtime/src/actor/directory/marriage.rs`) then holds the
proposal, keyed by the partner. It refuses these cases:

- the proposer already has a proposal out;
- the partner is offline;
- the partner is on another map;
- the partner is already being asked;
- the partner is below level 50 (a notice).

If the directory accepts the proposal, the partner's actor checks the
partner's own record and Etc room, then sends `MARRIAGE_REQUEST` 0 with the
proposer's name and id. The client shows a Yes/No dialog. The answer
(`RING_ACTION` 2) goes back to the directory:

- A no gives the proposer mode 30.
- A withdrawn proposal gives the partner mode 29.
- A player who logs off, changes channel, enters the Cash Shop, or logs in
  again from elsewhere ends the proposals they are part of.

Proposals never expire: a partner who leaves the dialog open keeps others
from asking them until the proposer withdraws or leaves.

On a yes, the partner's actor stores the couple (`db::marriage::engage`; the
groom is the male one). Its player is online because it is answering, so
it takes the ring (the empty box's id + 1) and `MARRIAGE_RESULT` 11 itself.
If its Etc tab filled while the dialog was open, it gets mode 20 and the
proposer mode 21. It then sends the record through the directory to the
proposer, whose actor replaces the box with its empty box (4031357 + 2 × the
box's offset) and sends `MARRIAGE_RESULT` 11. A proposer who left meanwhile
is engaged and keeps the box. Every refusal at this point reaches the
proposer, so their wait always ends.

## Breaking off

The empty boxes and rings, 4031357 to 4031364, are the couple's tokens.
Dropping one onto the field asks one Yes/No: "Dropping this item will break
your engagement...", or for a record that says married, "You will not be
able to receive the wedding ring...". A yes sends `RING_ACTION` 3 with the
token's id and nothing else; the client keeps the token until the server
takes it. v83 refuses to drop a wedding ring at all, so only a token breaks
a couple off.

The client actor (`break_off`) deletes the couple's row, clears the
record, takes every token the player holds, and sends `MARRIAGE_RESULT` 13
("Your engagement has been broken."), or 14 for a married couple. The
directory passes the record to the partner (`CoupleEnded`), whose actor
does the same, without the OK box if they are in the Cash Shop. A partner
offline, or between channels, finds no record on their next login and
keeps the token until they drop it, which then just takes it. Tokens in
storage, a trade or a parcel are not taken. Both players are saved at once,
as on engaging.

## The partner's map

While both of a couple are online, each client is told where the other is
with `NOTIFY_MARRIED_PARTNER_MAP_TRANSFER` (0x4B): the partner's map and
id (report b057). The client only keeps the two numbers. Its own map
changes don't clear them; leaving the game does.

The directory sends them through the client actors (`PartnerMap`). Each
`Presence` names the partner from the record. Both are told when the
second of the two logs in or changes channel, and when an engagement makes
each name the other. A partner who changes map tells the other. Someone
who logs off, changes channel, or ends the couple sends the other `(0, 0)`,
and an ended couple both get it once.

## Client

- `Character/Marriage.h` has the record, the tokens, and the result
  texts.
- `IO/MarriageActions` has the proposing flow.
- `UIItemInventory::drop_on_stage` asks before a token is dropped and sends
  `RING_ACTION` 3.
- `Net/Handlers/MarriageHandlers` handles `MARRIAGE_REQUEST` (72),
  `MARRIAGE_RESULT` (73) and `NOTIFY_MARRIED_PARTNER_MAP_TRANSFER` (75),
  which sets the player's `PartnerSpot`.
- The minimap draws the partner, when they're on the map, with
  `MapHelper.img/minimap/match` instead of their usual dot. The markers
  follow v83's order: others, the partner, NPCs, portals, then the player.
  The world map shows `worldMap/lovePos`, the heart, on the partner's spot
  unless it is the player's own.
- `ItemTooltip` puts "YOU ARE ENGAGED WITH : <name>" under a token's
  orange lines, in the same orange. A token keeps 16 px for the line even
  without a couple, as v83 does. `EquipTooltip` gives a wedding ring
  "YOU ARE MARRIED WITH : <name>" in place of its description. Both
  appear whenever there is a record, whatever its status. The name is the
  partner's, chosen by the player's own gender (report b056).

Any result ends the client's wait, as does a new field (a channel change
or the Cash Shop, where the server drops the proposal). The result's OK
dialog replaces the waiting one, so no withdrawal is sent. v83's waiting
dialog has a Cancel button; HeavenClient's has OK.

HeavenClient shows one notice at a time, and a new one replaces the open
one without calling its handler. A partner whose Yes/No is replaced never
answers, and the proposer waits until they withdraw.

## Not done yet

- Mode 32, refusing to break off after a reservation, and taking the
  wedding rings when a married couple breaks up.
- The ring's look.
- The wedding NPCs, reservations, the wedding field, and the wishlist.

Report b052 §7 lists them.
