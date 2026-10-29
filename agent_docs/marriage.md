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
follows `MARRIAGE_RESULT` 11. Its two item ids are the couple's wedding
ring (`MarriageRecord::ring_id`), the box's ring among 1112803, 1112806,
1112807 and 1112809. v83 draws the ring look with the first of them.

## Getting a ring box

Moony (9201000) in Amoria makes the ring boxes (`net::npc_scripts`, after
Cosmic's scripts; report b064). The steps are script markers, which are saved
with the quests but never sent to the client:

1. Moony sends the player to Mom and Dad (9201003) in "Meet the Parents"
   (680000004). They ask for four different Proofs of Love. The way there
   is a touch portal in 104040002, beyond Henesys Hunting Ground II.
2. The Nanas give the proofs. Each Nana gives one, in her own town only,
   for 50 of the town's drop:

   | Town | Drop | Proof |
   | --- | --- | --- |
   | Henesys | 4000001 | 4031367 |
   | Kerning City | 4000037 | 4031368 |
   | Perion | 4000215 | 4031369 |
   | Ellinia | 4000026 | 4031370 |
   | Orbis | 4000070 | 4031371 |
   | Ludibrium | 4000128 | 4031372 |

   The first proof from each Nana also gives EXP, from 2,000 to 30,000. A
   lost proof can be had again for the drops.
3. With four different proofs, Mom and Dad take every proof the player
   holds, give 20,000 EXP, and bless the player.
4. Once blessed, a man without a box can buy one from Moony. A woman can
   earn the blessing, but Moony makes rings only for men. Each box takes
   a Diamond (4021007), one more stone, and a fee:

   | Box | Stone | Fee |
   | --- | --- | --- |
   | 2240000 | 4011007 | 30,000 |
   | 2240001 | 4021009 | 20,000 |
   | 2240002 | 4011006 | 10,000 |
   | 2240003 | 4011004 | 5,000 |

Moony also throws away the boxes the player holds. He doesn't check
whether the buyer is already engaged; the proposal does.

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

A married couple holds no token after the wedding, so only one kept
elsewhere (storage, say) breaks them up. Their wedding rings go too, worn
or in the bags (`marriage::take_wedding_rings`), since v83 refuses to drop
one. Cosmic leaves the rings and only unlinks them, so taking them is
RustMS's pick (QUESTIONS b313). A partner who was away loses theirs on
the next login: whoever logs in with no record at all has every wedding
ring taken. An engaged player keeps one. A ring still in a parcel stays
until the login after it is taken out.

## Booking the wedding

Nicole (9201005, the Cathedral) and Bonnie (9201008, the Chapel) in Amoria
book an engaged couple's wedding (report b066; `net::npc_scripts`,
`wedding_assistant`, and `runtime::actor::client::wedding`). Their menu
explains the steps or books. The booker needs the venue's ticket
(Cathedral 5251000 or premium 5251003, Chapel 5251001 or premium 5251002)
in the Cash inventory, no wedding ring held or worn, and Etc room for the
invitations. In Cosmic, the scripts are `9201005.js` and `9201008.js`. The
Cash Shop sells the four tickets under Etc > Wedding (SN 50400000-3, at
25,000, 20,000, 24,000 and 29,000 NX for 90 days); the Event-tab
copies of them are off sale. Victoria (9201013), beside Nicole, explains
the booking in one page (`wedding.victoria.info`).

The partner's side lives in the partner's actor, so a booking is a round
trip through the directory:

1. The script ends with `Effect::BookWedding`, and the booker's actor sends
   `ReserveWedding`. The directory refuses a partner who is offline, or not
   on the booker's map and channel.
2. The partner's actor checks for a wedding ring and Etc room, and answers
   `WeddingNews::Checked` with any refusal.
3. The booker's actor checks its own side again and books
   (`db::wedding::book`). Then it takes the ticket and gives itself 15
   invitations (Cathedral 4031395, Chapel 4031377), which expire as the
   wedding starts. If that commit fails, it cancels the booking.
4. The partner's actor gets `WeddingNews::Booked` and gives itself 15
   invitations too.

Both partners get `MARRIAGE_RESULT` 16 ("Congratulations! Your
reservation was successfully made!"), a pink line saying when, and one
reminding them of the wedding clothes the priest asks for
(`wedding.reserve.dress_chat`), as Cosmic's "Get dressed and don't be
late!". The assistant's own line names the Wedding Tuxedo and Dress. A
partner who misses step 4 (offline by then, out of Etc room, or in the
Cash Shop, where it arrives on leaving) still has the booking. Only a
partner offline or full goes without invitations; the booker's 15 serve
both.

The booker's own refusals come first, in the script: not engaged, booked
already, a wedding ring, no Etc room, no ticket. Then the partner's:
offline, elsewhere (another map or channel, or in the Cash Shop), a
wedding ring, no Etc room. Invitations expire, so they never join a stack
and always need a free slot.

`wedding_reservations` holds one row per couple: the venue, premium or
not, the channel, and `starts_at`. A booking takes the first top of the
hour at least two hours away (`RUSTMS_WEDDING_LEAD_MINUTES` on the world
server changes the lead) and after the venue's latest booking on that
channel. A unique key on (channel, venue, starts_at) keeps two couples off
the same hour. A booked couple asking again hears the minutes left. A
couple who has not begun their wedding 10 minutes after its start loses
it: the row goes the next time it is read, and at world start.
`wedding_guests` lists the guests (below). The couple's `marriages.status`
stays engaged, as the client sends invitations only for status 1.

While a booking stands, dropping a token is refused with `MARRIAGE_RESULT`
32 ("You can't break the engagement after making reservations."). The
check and the delete are one transaction, and booking and breaking off
both lock the couple's row first, so one of the two always sees the
other.

## Inviting guests

A partner double-clicks an invitation in the Etc tab and types a guest's
name. The client sends `RING_ACTION` 5: the name, the couple's record id,
and the invitation's Etc slot as an `i32`. The server needs the slot to
hold the couple's invitation (Cathedral 4031395, Chapel 4031377) and the
record id to be the sender's own engaged couple; otherwise it does
nothing more. Every request ends with the client's actions re-enabled,
since the client waits for that after sending one (report b069). Then
it puts the guest on the list
(`db::wedding::invite`, which locks the couple's row), and refuses in pink
chat, as Cosmic does:

- no character of that name, or one of the couple;
- a guest already invited;
- no wedding booked at the invitation's venue (a Chapel invitation for a
  Cathedral wedding counts as none);
- a wedding already under way (sending is only before `starts_at`);
- a full list (`game::wedding::MAX_GUESTS`, 60).

A guest on the list costs the sender one invitation (Cosmic takes it on
"already invited" too) and gets a received invitation (Cathedral 4031407,
Chapel 4031406) that expires a slot after the wedding starts. The
directory hands it to the guest's actor if they are online
(`WeddingNews::Invited`). Without room there, or offline, it goes to
Duey as a parcel from the sender, with a note saying whose wedding it is
(`client::wedding::leave_at_duey`). For a guest offline the directory
does this itself, in the background, so the sender may leave meanwhile.
A guest online is told in pink chat either way. With the guest's Duey
inbox full too, the guest goes off the list again and the invitation
goes back to its sender (`WeddingNews::Returned`, through
`DirectoryMessage::ReturnInvitation` from a guest online), who is told
in pink chat and may send another; a sender gone by then loses it. An
invitation handed to a guest's actor that is just leaving the channel
is lost the same way, the guest staying on the list.

A guest double-clicks the received invitation and the client sends
`RING_ACTION` 6 with its Etc slot and item id. With the item there, the
server answers `MARRIAGE_RESULT` 15 for the soonest wedding at its venue
the guest is invited to: the groom's and bride's names and the window
(2 the Cathedral's, 1 the Chapel's). Without one it answers 34 ("This
invitation is not valid."), where Cosmic answers nothing.

The assistants' menu has two more choices. "I am a guest" answers from the
list: not invited (no list entry at this venue, or no received
invitation), the minutes left, the channel the wedding is on, or, once it
is due there, whether to go in (see "Guests" below). With a Wedding
Invitation Ticket (5251100) in the Cash tab, a booked couple may trade it
for 3 more invitations to their wedding at that venue, before it starts
and with a free Etc slot (`Effect::ExtraInvitations`).

## The wedding

Slices 7a to 7f of report b071 marry a couple at the Chapel or the
Cathedral before their guests, with the priest's lines over the altar, the
Cathedral guests' blessings and their reward, the news to the couple's
guild and family, and a premium wedding's party room and hunting grounds.
The Giant Cake comes out in the hunting ground, and the onyx chests are
left for later.

Wayne (9201012) in Amoria starts a booked Chapel wedding, and High Priest
John (9201002) in Amoria a booked Cathedral one
(`NpcScript::wedding_starter`). The High Priest also marries couples at
the Cathedral's altar, so he starts weddings only in Amoria. The starter
answers a player who isn't engaged, has no booking at his venue, is on
another channel than the booking's, or comes before its `starts_at`. A
partner without wedding clothes worn or in the Equip tab
(`game::wedding::has_clothes`) is told to bring them. The couple has
`NO_SHOW` (10 minutes) after `starts_at` to start; then the booking goes
when next read. Saying yes (`Effect::StartWedding`) re-reads the booking
and asks the directory (`DirectoryMessage::StartWedding`): the partner
must stand beside the player, and one wedding at a time may use a venue on
a channel (`weddings` by channel and venue; ids come from the same counter
as party events', so copies never meet).

`runtime::actor::wedding` runs one wedding. Like a party event
(`party-quests.md`) it opens its own copy of the venue's hall (Chapel
680000100, Cathedral 680000200) and altar (680000110, 680000210) and gives
each partner a `WeddingTicket` (the client actor's `wedding`);
`field_key`, `leaving_for` and the clock follow either ticket. The stages,
scaled by `RUSTMS_EVENT_SCALE`:

1. Gathering (`GATHERING`, 10 minutes) in the hall, with its countdown.
2. Ceremony (`CEREMONY`, 20 minutes) at the altar. The priest (Chapel
   9201011, Cathedral 9201002) asks each partner for their vows once they
   wear wedding clothes, hold their token (the box or the ring of the
   couple's set) and have room for the wedding ring. The checks are made
   again on yes. The first vows wait; the second marry the couple in one
   transaction (`db::marriage::marry`: status 3, booking deleted). Each
   partner then loses their tokens, gets the wedding ring of the set
   (`ring_id`) and `MARRIAGE_RESULT` 12 with the record (`build_married`),
   and their look updates if the ring is worn. A partner whose equipment
   filled up while the other vowed gets the ring as a Duey parcel from the
   other instead. The second vows end with the priest's pronouncement
   (below), not a dialog of his.
3. Married (`AFTER_VOWS`, 45 minutes). Jackie (9201009) at the Chapel or
   Nancy (9201007) at the Cathedral asks whether to end it; yes sends the
   couple to Ames's room (680000500), as does the time running out. A
   premium booking (the reservation's `premium`, report b080) goes on
   instead: yes moves the couple and the guests to the party room
   (`PARTY_ROOM`, 680000300) under the same clock. There Travis (9201010)
   offers the way back to Amoria and Robin (9201021) the way out to
   680000500; each only moves the one who asks, and the time running out
   sends whoever is left to 680000500. See "The hunting grounds" below.

A partner who leaves the wedding's maps, logs out or changes channel
before the vows ends it: the booking is deleted, nothing is refunded
(b071 Q9), and whoever is still in it goes to 680000500 with
`MARRIAGE_RESULT` 31. A ceremony that runs out without vows ends the
same way. The venue's maps return to 680000500, so a player logging in
there after a restart lands outside; a booking still inside its window
can be started again (b071 Q10).

### Guests

A guest on the list holding the received invitation asks the assistant in
Amoria to go in (`Effect::EnterWedding`). The client actor reads the list
again and asks the directory (`DirectoryMessage::JoinWedding`), which
keeps each wedding's queue beside its id and hands the guest's handle to
the wedding of that couple under way at the venue on the guest's channel.
The wedding lets guests in during the gathering only, up to `MAX_GUESTS`,
and answers a `WeddingTicket` with `guest` set; the guest goes to the
hall. Otherwise the assistant says the couple haven't begun yet, the
doors are closed, or the hall is full (`GuestRefusal`). Guests without
wedding clothes are reminded of them but let in (b071 Q1).

Guests go on to the altar with the couple, and their clock follows the
couple's after the vows. The priest treats them as anyone else. The
assistant in the hall and Jackie or Nancy at the altar offer them the way
out (`Effect::LeaveWedding`, to 680000500); a guest who leaves, logs out
or changes channel only leaves the wedding. When it ends, guests go out
with the couple, without the couple's `MARRIAGE_RESULT` 31. Cosmic leaves
the guests in the hall to walk to the altar themselves; RustMS moves them.

### The priest's lines

v83 shows the priest's lines over the altar itself (report b075).
`WEDDING_PROGRESS` (0x140: `u8` step, the groom's and the bride's ids)
opens a page with the priest's `String.nx` line `Npc.img/<priest>/wedding<step>`
for everyone at the altar; `WEDDING_CEREMONY_END` (0x141, empty) ends the
open page and shows the hearts over the couple. The couple's client
answers each page with `WEDDING_TALK` (0x8B: `u8`, the latest step it
heard); a guest's client sends nothing.

The wedding actor sends `ServerMessage::WeddingPriest` with a
`PriestLine` to the couple and the guests, and each client actor passes it
on while the player stands on the venue's altar:

- Step 0 (`PRIEST_WELCOME`) when the gathering ends and they reach the
  altar. It also starts the venue's music.
- At the Cathedral, before the vows, `Venue::before_vows`: step 1 (his
  call for blessings) after the welcome, then step 2
  (`Venue::blessing`), which asks each guest "Do you want to bless this
  couple?" and shows the couple nothing.
- `Venue::pronouncing` (Chapel 1, Cathedral 3) when the second vows marry
  them, whatever step came before.
- The hearts and then `Venue::farewell` (Chapel 2, Cathedral 4) after the
  pronouncement.

The priest goes on from a line that leads on (the Cathedral's 0 and 1,
and the pronouncement) when either partner answers it
(`WeddingMessage::Talk` with the step he last said), or `LINE_WAIT`
(10 seconds, not scaled) after it if neither does. Each partner's ticket
passes on only the first answer to each step, never going back
(`WeddingTicket::talk`); the rest, and a guest's, stop at the ticket, so
a client repeating 0x8B can't fill the actor's queue. A player away from
the altar when a line goes out (in the Cash Shop, for one) misses it, and
nothing sends it again.

A Cathedral guest's Yes sends `WEDDING_BLESS` (0x8C, empty). From a guest
on the altar during the ceremony, the guest's ticket asks the actor
(`WeddingTicket::bless`, `WeddingMessage::Bless`), which counts it
(`blessers`) once per guest while the priest's last line is the blessing
and the couple aren't married yet; the guest then reads `wedding.blessed`
in pink. A blessing at any other time or place, or from the couple, does
nothing. A guest who leaves drops out of the count, and the couple may
vow before the blessing, which then never comes.

### After the vows

Each partner gets `BLESSING_EXP` (30000) for every guest who blessed and
is still in the wedding when the second vows marry them, with no EXP rate
applied (report b080). Then each partner's client actor tells the
directory (`DirectoryMessage::Married`), which sends `NOTIFY_MARRIAGE`
(0x6A: `u8` 0 guild or 1 family, then "> name") to everyone of the
partner's guild online and to the partner's family. The client shows
"<Guild> name is now married. Please congratulate them." as a chat line
(report b079). The directory only knows a player's senior and juniors, so
the family notice goes to them and the partner, not to the whole tree as
in Cosmic.

### The hunting grounds

Robin's menu in the party room also leads to the hunting ground
(`HUNTING_GROUND`, 680000400), where her menu instead offers the box room
(`BOX_ROOM`, 680000401) for `BOX_KEYS` (7) Amoria Heart Keys
(`HEART_KEY`, 4031409), and the way out in both (report b082). In the box
room she offers the way back to the hunting ground; coming back costs
seven more keys. Both maps are the wedding's own copies under its clock,
and each player goes alone.

`String.nx` says the Heart Key opens the hunting ground's treasure chest,
and the content pack has the Cake and Candle Monsters drop it. Cosmic uses
the Golden Key 4031217 instead; RustMS follows the client and the pack and
takes Cosmic's count of seven. The keys are the wedding's own: leaving it
any way takes them (`game::wedding::take_keys`, from
`drop_wedding_ticket` and `forget_event`), and so does logging in on one
of its maps, in case a crash saved them there. If the keys are gone by the time the player picks the
box room, Robin's `refused` text says how to get them. Robin asks for no
Gold Maple Leaf (b071 Q4). The box room's boxes (6802000, 6802001) drop
their loot with piles of 100-399 mesos, at least 15 drops in all
(`net::reactors`, Cosmic `sprayItems(true, 1, 100, 400, 15)`). Most of
them come back a second after breaking (`reactorTime 1`), so one entry
lets a player break boxes until the clock runs out (review b083).

The wedding's copy of the hunting ground opens with one Giant Cake
(`GIANT_CAKE`, 9400606) on the ground below `GIANT_CAKE_AT` (777,-177),
as Cosmic's `spawnCakeBoss` does (`FieldActor::new`). The copy lasts the
wedding, so there is one cake per wedding, and the public map has none.
Killing it leaves the clock running: Cosmic's kill clears the event and
stops the clock, and an instance with no time limit would never close
(QUESTIONS Q3). It drops nothing yet: the content pack has no row for
it, and Cosmic's mesos and equips would need one.

### The wishlist and gifts

In the hall or at the altar, Travis (Chapel) or Debbie (Cathedral) asks
each of the couple once whether to write a wishlist (reports b084, b085).
Yes sends `MARRIAGE_REQUEST` mode 9 alone, which opens the client's
wishlist window, and its OK sends `RING_ACTION` 9: a `u16` count and that
many strings (at most 10 of 16 characters are kept). Only the first list
counts. The lists live in the wedding's `game::wedding::Registry`, in the
wedding actor's memory, so they go with the wedding. A guest gets the
greeter's `guest` text.

Pila (9201014) stands in both halls and in Amoria. In a wedding she asks
whose wishlist to open; a guest's or the other partner's choice sends
`WEDDING_GIFT_RESULT` (0x4A) mode 9 with that list, which opens the give
window. Her `own` and `given` texts answer someone choosing themselves or
a partner they have given to. The give window sends `WEDDING_ACTION`
(0x8A) mode 6, `u16 slot, u32 itemId, u16 count`. What a parcel could
carry may be given (`net::wedding_gift::giftable`), equips and stars
whole. Each giver gives each partner one gift per wedding (`Registry`'s
`give`, undone if the save fails). The gift goes into `wedding_gifts`
(with `wedding_gift_equipment_stats`) in the same transaction that saves
the giver. The giver gets the inventory change and mode 11 with the
wishlist and the gift, then the client closes the window and sends mode 8.
Mode 14 refuses a gift that isn't there or may not go; mode 12 answers a
second gift that got past Pila's check. The client keeps mode 12's window
waiting, so it can only be closed. The partner is not told.

Outside a wedding Pila asks someone with gifts waiting whether to see
them; yes sends mode 10, the gifts by bag (`i64` mask `1 << (type + 1)`,
then per set bit a count and the items without slots). The receive
window's take sends mode 7, `u8 type, u8 index` within that bag as last
listed. The gift goes into the inventory and out of the table in one
transaction, and mode 15 lists that bag again. A take that doesn't fit, a
one-of-a-kind already held, or a stale index gets mode 16. Gifts wait
without end. The window's state is the client actor's `gift_window`, which
a map change drops.

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
Engaged couples are told as well as married ones. The client doesn't
tell the two apart, and whether v83's server told engaged couples is
inferred (report b057 §6).

## Spouse chat

The chat bar's To Spouse target sends `SPOUSE_CHAT` (0x79): the partner's
name, then the line (FUN_008defff). The client takes the name from its
marriage record, so an engaged couple can talk too. Without a record it
prints "You are not married or your spouse is currently offline." (0x9f)
and sends nothing.

The server ignores the name the client sent and uses its own record
(`SpouseChatHandler`). The line travels like a whisper (`WorldAction::Whisper`),
so it reaches the partner on any channel. `SPOUSE_CHAT` (0x88) mode 5 gives
the partner the sender's name and the line. Mode 4 tells the sender
whether the line went: if it did, the client prints the line with the
player's own name, and if not, it prints the 0x9f notice (FUN_00532087).
Lines print in chat type 6's pink (#FF71BF).

## The ring look

A couple wearing their wedding rings show it to the map (report b059).
`SPAWN_PLAYER` and `UPDATE_CHAR_LOOK` both end with three ring blocks:
crush, friendship and marriage (`write_ring_looks`, from
`Player::ring_looks`). The crush and friendship blocks are described
below. The marriage block is a flag, then the wearer's id, the
partner's id, and the ring's item id. It is set while the character has a
couple and wears a wedding ring in a ring slot
(`Player::marriage_ring`), whatever the couple's status.
Every viewer gets the real ids. v83 pairs a user with any other whose
partner id is its own id, so zeros would pair strangers.

A look update replaces the character's block, so wearing or taking off
the ring updates the look. An engagement does too when the player already
wears a ring.

Each frame the client pairs two users when one's partner id is the
other's id. The player's own block comes from its record and its worn
rings, not from a packet. Within 700 px across and 500 px up or down, each
of the two shows `Effect/ItemEff.img/<ring>/0`, a small mark. Within
250 px across and 100 px up or down, the lower id also shows
`<ring>/1`, the big effect, centred between the two. v83's placement is
COM layer code that wasn't decoded. The mark is drawn at each
character's feet, and the big effect at the point midway between their
feet, with the sprites' own origins. Footage of a 2012 couple's ring
agrees with this (inferred).

## Crush and friendship rings

Crush rings (1112001 to 1112099) and friendship rings (1112800 to 1112802,
1112810 to 1112812, and so on) are cash rings sold in pairs (report b061,
`game::ring::kind`). `CASHSHOP_OPERATION` 0x1D buys a crush ring and 0x23
a friendship ring. Both send the birthday code, the balance to pay with,
the commodity's serial number, the partner's name and a message. The
server checks the commodity is a ring of the request's kind, then the
birthday code and the message, and then, as for a gift, that the partner
exists, is on another account and may have the item. One transaction
pays, puts one ring in the buyer's Cash Inventory, sends the other to
the partner's as a gift with the message, and records the pair
(`db::cash_shop::buy_ring`). The answer is mode 0x87, the buyer's
ring's 0x37-byte item, the partner's name, the item id and the
quantity, followed by the balance.

The `rings` table has a row for each ring of a pair, keyed by the ring's
cash serial, with its partner ring's serial, the partner's character
id and the item. `Player::rings` holds the pairs of the rings the
character carries, worn or in the Equip bag (`db::ring::carried`).
Taking a ring out of the Cash Inventory adds its pair, and putting it
back drops it.

The character data lists them after the minigame records
(`write_couple_rings`). A crush record is 0x21 bytes: the partner's id,
name (13 bytes), the ring's serial and the partner ring's serial. A
friendship record adds the item id. While a ring of a pair is worn in a
ring slot, the look's crush or friendship block is a flag, the ring's
serial, the partner ring's serial (both i64) and the item. Each client
pairs two users when one's partner serial is the other's serial.

A character wears one ring of each kind (crush, friendship, wedding) at a
time. `Inventory::equip` refuses a second, though a ring may replace one
of its own kind. v83's client refuses first, with its own notices.

Plain Buy and Gift requests for a pair ring fail, as does paying for one
with Maple Points, which the client's ring dialog doesn't offer. A
dropped pair ring vanishes. The partner named in a record is the
character typed in; the ring goes to that account's Cash Inventory, so
another of its characters may take it out. Deleting a character leaves
the other half of a pair as a row no one pairs with. As with gifts, the
inventory-full checks (0xAB, 0xBB) and the Cash Inventory cap (0xAC) are
not made.

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
- `marriage::read_ring_looks` reads the three ring blocks for
  `SpawnCharHandler` and `UpdateCharLookHandler`. `OtherChar` keeps the
  marriage block. `Stage::show_rings` pairs the wearers with
  `marriage::ring_effects` each frame, and `Char::show_ring` draws the
  result.
- `ItemTooltip` puts "YOU ARE ENGAGED WITH : <name>" under a token's
  orange lines, in the same orange. A token keeps 16 px for the line even
  without a couple, as v83 does. `EquipTooltip` gives a wedding ring
  "YOU ARE MARRIED WITH : <name>" in place of its description. Both
  appear whenever there is a record, whatever its status. The name is the
  partner's, chosen by the player's own gender (report b056).
- Double-clicking a wedding invitation in the Etc tab calls
  `MarriageActions::invite` (report b069). It does nothing unless the
  record says engaged. It asks for the guest's name (4 to 12
  characters; a shorter one gives v83's notice and asks again), refuses
  the groom's or the bride's exact name with v83's "yourself" notice, and
  sends `RING_ACTION` 5 as a `USE_WAIT` request, which the server's stat
  update ends. A received invitation sends `RING_ACTION` 6, or, on a map
  whose field limit has 0x2000, says "You can't use it here in this map."
  in the chat.
- `MARRIAGE_RESULT` 15 opens `UIInvitation`: the Cathedral art
  (`UIClassic.nx UIWindow.img/Wedding/Invitation`) for type 2, else the
  Vegas art, with the groom's and the bride's names. OK, Enter and Esc
  close it, and nothing is sent. Modes 16, 31 and 34 say v83's notices.
- `MARRIAGE_RESULT` 12 sets the married couple's record, as 11 does
  the engaged one's, and says "You are now married!" (StringPool
  0x108C).
- `Gameplay/MapleMap/MapWedding` is the altar's side of
  `WEDDING_PROGRESS` (320) and `WEDDING_CEREMONY_END` (321), on 680000110
  and 680000210 only, with each altar's priest known to the client
  (`WeddingRules.h`). A step shows the priest's line in `UINpcTalk` with
  NEXT and End Chat, answered locally; step 0 also clears the hearts and
  loops `BgmGL.img/chapel` or `cathedral`. A Cathedral guest gets "Do you
  want to bless this couple?" on step 2 instead, and Yes sends
  `WEDDING_BLESS` (0x8C, empty); the couple see nothing then. The groom or
  the bride sends `WEDDING_TALK` with the latest step when a page returns.
  v83 runs each page as a modal inside the one it replaced, so a page
  that a newer one ended returns only with it (`wedding::Pages`).
  `WEDDING_CEREMONY_END` ends the open page and loops
  `Effect/BasicEff.img/Wedding` 20 px above the couple's midpoint, on the
  mists' layer, while both are on the map, until the next step 0 or map.
  A line that arrives during a map change's fade waits for the new map
  (`Stage::pending_wedding`, like `pending_clock`), so the welcome at the
  altar shows. The priest's page is tagged
  (`UINpcTalk::set_local_responder`'s `tag`): if another NPC's dialog
  took its place, the next line neither closes that dialog nor answers
  for the lost page.

Any result ends the client's wait, as does a new field (a channel change
or the Cash Shop, where the server drops the proposal). The result's OK
dialog replaces the waiting one, so no withdrawal is sent. v83's waiting
dialog has a Cancel button; HeavenClient's has OK.

HeavenClient shows one notice at a time, and a new one replaces the open
one without calling its handler. A partner whose Yes/No is replaced never
answers, and the proposer waits until they withdraw.

For the wishlist and the gifts (report b084 §6):

- `MARRIAGE_REQUEST` 9 opens `UIWishlistInput` unless a dialog is open:
  the groom's or the bride's art, a 16-character field whose Enter or
  ENTER button adds a line (at most 10, then v83's "exceeded" notice),
  DEL ITEM for the selected line, and OK, which asks once more and then
  sends `WishlistPacket`. There is no other way out, as in v83.
- `WEDDING_GIFT_RESULT` 9 and 10 open `UIWeddingRegistry`, the give
  window (always the groom's art) or the receive window (the player's
  gender's). `wedding_gifts::read_wishlist` and `read_gifts` read the
  bodies; a bag whose bit is clear keeps the gifts listed before. Both
  windows show the player's giftable items by tab (`item_drop::giftable`:
  storable and not account-bound, v83 0x9d877b). That lists some items
  the server refuses with 14: sealed ones, ones with an expiry, party
  quest items and ones that go at logout. A double click on a row, or
  the window's button, gives or takes: a stack asks how many,
  then a Yes/No, then `WeddingActionPacket` 6 or 7, and the window waits
  for the answer before the next. Mode 11 shows "You have sent a gift."
  and closes; 12 shows the one-gift notice and keeps waiting; 13 and 14
  say the gift failed; 15 lists the bag again and turns to its tab; 16
  ends the wait. Exit, Esc and Enter close the window and send mode 8.
  A mode 11 that comes after the window closed still says the gift was
  sent. A map change closes either window (the server forgets it too).

For crush and friendship rings:

- `CharacterParser` reads the character data's crush and friendship
  lists into `Player::get_couple_rings`. `read_ring_looks` reads the
  crush and friendship blocks, which `OtherChar` keeps by kind.
- `Player::ring_wearer` makes the player's own block of a kind from
  the worn cash rings and the records (`marriage::own_pair_ring`).
  `Stage::show_rings` pairs each kind's wearers on its own, and each
  kind has its own effect.
- `EquipTooltip` names the partner of a ring whose serial a record has,
  in place of the description: "A COUPLE-RING WITH : <name>" for a crush
  ring, "This is your Friendship Ring with '<name>'." for a friendship
  ring. The records come with the character data, so a ring bought or
  taken out during a Cash Shop visit gets its line on the next map.
- `UIItemInventory::can_wear_equip` refuses a second ring of a kind
  with v83's notice (`marriage::equip_gate`). `Inventory::find_equipslot`
  looks for a free ring slot on the cash layer for a cash ring, so a
  double-clicked second crush ring meets the notice instead of
  replacing the worn one.
- In the Cash Shop, BUY or GIFT on a pair ring opens `UICashGift` in
  ring mode: the birthday code, a check that credit or prepaid covers
  the price, the partner's name and message, a Yes/No naming the price
  and the partner, then `UICashBuy`'s payment choice without Maple
  Points. It sends `BuyCoupleRingPacket` (0x1D or 0x23). Mode 0x87 adds
  the ring to the Cash Inventory and says what was given. HeavenClient
  clears its wait on any Cash Shop reply, so a second ring can be bought
  in the same visit.

## Not done yet

- The rest of the wedding: the onyx chests 4031423/4 that Pila 9201014
  opens (report b082), and the Giant Cake's drops (the content pack
  has none). The content pack gives the keys and the
  chests a `period`, which RustMS ignores: the chests it grants don't
  expire (review b083).
- Sending `MARRIAGE_RESULT` 31 ("The reservation has been canceled...")
  when a no-show's booking goes unstarted. A wedding that fails sends it.

Report b052 §7 lists them.
