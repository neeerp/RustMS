//! Mini rooms (v83 `PLAYER_INTERACTION`): the trading room, personal
//! shops, Omok and match cards. Every packet starts with a mode byte. Also the balloon over a
//! player who keeps a room open (UPDATE_CHAR_BOX).

use crate::codec::encode_avatar_look;
use crate::codec::item::{encode_item, SlotPrefix};
use crate::personal_shop::Listing;
use crate::{error::NetworkError, packet::op::SendOpcode};
use game::{AvatarLook, GameRecord, Item, MiniRoomBox};
use packet::{io::write::PktWrite, Packet};

/// Server → client modes.
const INVITE_MODE: u8 = 0x02;
const INVITE_RESULT_MODE: u8 = 0x03;
const VISIT_MODE: u8 = 0x04;
const ENTER_MODE: u8 = 0x05;
const CHAT_MODE: u8 = 0x06;
const LEAVE_MODE: u8 = 0x0a;
const PUT_ITEM_MODE: u8 = 0x0f;
const PUT_MESO_MODE: u8 = 0x10;
const CONFIRMED_MODE: u8 = 0x11;
const MESO_LIMIT_MODE: u8 = 0x15;
const BUY_RESULT_MODE: u8 = 0x18;
const SHOP_ITEMS_MODE: u8 = 0x19;
const SOLD_MODE: u8 = 0x1a;
const TIE_ASKED_MODE: u8 = 0x32;
const TIE_DENIED_MODE: u8 = 0x33;
const UNDO_ASKED_MODE: u8 = 0x36;
const UNDO_RESULT_MODE: u8 = 0x37;
const READY_MODE: u8 = 0x3a;
const UNREADY_MODE: u8 = 0x3b;
const GAME_START_MODE: u8 = 0x3d;
const GAME_RESULT_MODE: u8 = 0x3e;
const TURN_MODE: u8 = 0x3f;
const STONE_MODE: u8 = 0x40;
const STONE_REJECTED_MODE: u8 = 0x41;
const CARD_MODE: u8 = 0x44;

/// The trading room's type and size.
pub const TRADE_ROOM: u8 = 3;
const TRADE_USERS: u8 = 2;

/// The personal shop's type and size: the owner and three visitors.
pub const PERSONAL_SHOP: u8 = 4;
pub const SHOP_USERS: u8 = 4;
/// How many items a personal shop lists at most.
pub const SHOP_ITEMS: u8 = 16;

/// A chat line from a member, and a game's notice.
const CHAT_LINE: u8 = 8;
const GAME_MESSAGE: u8 = 7;

/// A game's notices in the room's chat, each with a name (mode 6 sub 7).
pub mod game_message {
    /// "[%s] have forfeited."
    pub const FORFEITED: u8 = 2;
    /// "[%s] have called to leave after this game."
    pub const LEAVE_AFTER: u8 = 5;
    /// "[%s] have cancelled the request to leave after this game."
    pub const STAY_AFTER: u8 = 6;
    /// "[%s] has matched cards. Please continue."
    pub const MATCHED: u8 = 9;
}

/// How a game ended (0x3e); the client tells only a tie apart.
pub mod game_result {
    pub const WON: u8 = 0;
    pub const TIE: u8 = 1;
    /// The loser gave up or left (inferred).
    pub const FORFEIT: u8 = 2;
}

/// Why an invitation went nowhere (INVITE_RESULT).
pub mod invite_result {
    /// "Unable to find the character."
    pub const NOT_FOUND: u8 = 1;
    /// "'%s' is doing something else right now."
    pub const BUSY: u8 = 2;
    /// "'%s' have denied invitation."
    pub const DECLINED: u8 = 3;
    /// "'%s' is currently not accepting any invitation."
    pub const NOT_ACCEPTING: u8 = 4;
}

/// Why the client couldn't enter a room (ENTER with type 0).
pub mod enter_error {
    /// "The room is already closed."
    pub const CLOSED: u8 = 1;
    /// "You can't enter the room due to full capacity."
    pub const FULL: u8 = 2;
    /// "The personal store can only be built at the free market available
    /// at main towns."
    pub const NOT_FREE_MARKET: u8 = 0x0f;
    /// "You may not enter this store."
    pub const BANNED: u8 = 0x11;
    /// "The owner of the store is currently undergoing store maintenance.
    /// ..." (a hired merchant).
    pub const MAINTENANCE: u8 = 0x12;
    /// "This item has expired." (a Store Remote Controller's period).
    pub const EXPIRED: u8 = 0x18;
    /// "The password is incorrect."
    pub const WRONG_PASSWORD: u8 = 0x16;
}

/// How a trade ended, shown to the member who stays (LEAVE).
pub mod leave_reason {
    /// "Trade cancelled by the other character."
    pub const CANCELLED: u8 = 2;
    /// "Trade successful. Please check the results."
    pub const DONE: u8 = 7;
    /// "Trade unsuccessful."
    pub const FAILED: u8 = 8;
    /// One of the items is one-of-a-kind and the receiver has one.
    pub const ONE_OF_A_KIND: u8 = 9;
    /// "The shop is closed."
    pub const SHOP_CLOSED: u8 = 3;
    /// "The user has been banned."
    pub const BANNED: u8 = 5;
    /// "The items are out of stock."
    pub const OUT_OF_STOCK: u8 = 0x0e;
    /// Banned for staying an hour; may not come back.
    pub const OVERSTAYED: u8 = 0x0f;
    /// A game: "You have been expelled."
    pub const EXPELLED: u8 = 5;
    /// A game: the player left it.
    pub const LEFT: u8 = 0;
}

/// Why a purchase in a personal shop failed (BUY_RESULT).
pub mod buy_result {
    /// "You don't have enough in stock."
    pub const NO_STOCK: u8 = 1;
    /// "You do not have enough mesos."
    pub const NO_MESOS: u8 = 2;
    /// "The price of the item is too high for the trade."
    pub const TOO_PRICEY: u8 = 3;
    /// "Please check if your inventory is full or not."
    pub const INVENTORY_FULL: u8 = 5;
    /// "You cannot carry more than one for this item."
    pub const ONLY_ONE: u8 = 6;
    /// "Level 15 and below may only trade 1 million mesos per day..."
    pub const LOW_LEVEL_LIMIT: u8 = 0x0e;
}

pub(super) fn room_packet(mode: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::PlayerInteraction as i16)?;
    packet.write_byte(mode)?;
    Ok(packet)
}

/// `from` invites the receiving client into trading room `sn`.
pub fn build_trade_invite(from: &str, sn: i32) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(INVITE_MODE)?;
    packet.write_byte(TRADE_ROOM)?;
    packet.write_str_with_length(from)?;
    packet.write_int(sn)?;
    Ok(packet)
}

/// The invited character couldn't be found.
pub fn build_invite_not_found() -> Result<Packet, NetworkError> {
    let mut packet = room_packet(INVITE_RESULT_MODE)?;
    packet.write_byte(invite_result::NOT_FOUND)?;
    Ok(packet)
}

/// `target` is busy or turned the invitation down (`invite_result`).
pub fn build_invite_result(result: u8, target: &str) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(INVITE_RESULT_MODE)?;
    packet.write_byte(result)?;
    packet.write_str_with_length(target)?;
    Ok(packet)
}

/// The receiving client enters a room of `kind` and size `users` in
/// `my_slot`, with the members in it as `(slot, look, name)`.
fn enter_room(
    kind: u8,
    users: u8,
    my_slot: u8,
    members: &[(u8, &AvatarLook, &str)],
) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(ENTER_MODE)?;
    packet.write_byte(kind)?;
    packet.write_byte(users)?;
    packet.write_byte(my_slot)?;
    for &(slot, look, name) in members {
        packet.write_byte(slot)?;
        encode_avatar_look(&mut packet, look, false)?;
        packet.write_str_with_length(name)?;
    }
    packet.write_byte(0xFF)?;
    Ok(packet)
}

/// The receiving client enters a trading room in `my_slot`, with the
/// members already in it as `(slot, look, name)`.
pub fn build_trade_room(
    my_slot: u8,
    members: &[(u8, &AvatarLook, &str)],
) -> Result<Packet, NetworkError> {
    enter_room(TRADE_ROOM, TRADE_USERS, my_slot, members)
}

/// The receiving client enters personal shop `title` in `my_slot` (0 for
/// the owner), with its members and what it lists.
pub fn build_shop_room(
    my_slot: u8,
    members: &[(u8, &AvatarLook, &str)],
    title: &str,
    listings: &[Listing],
) -> Result<Packet, NetworkError> {
    let mut packet = enter_room(PERSONAL_SHOP, SHOP_USERS, my_slot, members)?;
    packet.write_str_with_length(title)?;
    packet.write_byte(SHOP_ITEMS)?;
    write_listings(&mut packet, listings)?;
    Ok(packet)
}

/// Each listing: bundles left (0 when sold out), items per bundle, the
/// price of a bundle, then one bundle as an item.
fn write_listings(packet: &mut Packet, listings: &[Listing]) -> Result<(), NetworkError> {
    packet.write_byte(listings.len() as u8)?;
    for listing in listings {
        packet.write_short(listing.bundles)?;
        packet.write_short(listing.item.quantity)?;
        packet.write_int(listing.price)?;
        encode_item(packet, SlotPrefix::None, &listing.item)?;
    }
    Ok(())
}

/// What a personal shop lists now.
pub fn build_shop_items(listings: &[Listing]) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(SHOP_ITEMS_MODE)?;
    write_listings(&mut packet, listings)?;
    Ok(packet)
}

/// Tells the owner `buyer` bought `bundles` of listing `index`; the client
/// keeps a record for the sold-items window.
pub fn build_shop_sold(index: u8, bundles: i16, buyer: &str) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(SOLD_MODE)?;
    packet.write_byte(index)?;
    packet.write_short(bundles)?;
    packet.write_str_with_length(buyer)?;
    Ok(packet)
}

/// Why the receiving client's purchase failed (`buy_result`).
pub fn build_buy_result(result: u8) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(BUY_RESULT_MODE)?;
    packet.write_byte(result)?;
    Ok(packet)
}

/// The balloon block of UPDATE_CHAR_BOX and the player spawn: the room
/// type, 0 for none, then the room.
pub fn write_mini_room_box(
    packet: &mut Packet,
    room: Option<&MiniRoomBox>,
) -> Result<(), NetworkError> {
    let Some(room) = room else {
        packet.write_byte(0)?;
        return Ok(());
    };
    packet.write_byte(room.kind)?;
    packet.write_int(room.sn)?;
    packet.write_str_with_length(&room.title)?;
    packet.write_byte(room.private as u8)?;
    packet.write_byte(room.skin)?;
    packet.write_byte(room.users)?;
    packet.write_byte(room.max_users)?;
    packet.write_byte(room.in_progress as u8)?;
    Ok(())
}

/// UPDATE_CHAR_BOX: the balloon over `char_id` changed, or went (`None`).
pub fn build_char_box(char_id: i32, room: Option<&MiniRoomBox>) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::UpdateCharBox as i16)?;
    packet.write_int(char_id)?;
    write_mini_room_box(&mut packet, room)?;
    Ok(packet)
}

/// The receiving client couldn't enter the room (`enter_error`).
pub fn build_enter_error(error: u8) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(ENTER_MODE)?;
    packet.write_byte(0)?;
    packet.write_byte(error)?;
    Ok(packet)
}

/// Someone joined the receiving client's room in `slot`.
pub fn build_room_visit(slot: u8, look: &AvatarLook, name: &str) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(VISIT_MODE)?;
    packet.write_byte(slot)?;
    encode_avatar_look(&mut packet, look, false)?;
    packet.write_str_with_length(name)?;
    Ok(packet)
}

/// The member in `slot` said `line`, already formatted as `name : text`.
pub fn build_room_chat(slot: u8, line: &str) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(CHAT_MODE)?;
    packet.write_byte(CHAT_LINE)?;
    packet.write_byte(slot)?;
    packet.write_str_with_length(line)?;
    Ok(packet)
}

/// The room closes for the receiving client, sitting in `slot`, for
/// `reason` (`leave_reason`).
pub fn build_room_leave(slot: u8, reason: u8) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(LEAVE_MODE)?;
    packet.write_byte(slot)?;
    packet.write_byte(reason)?;
    Ok(packet)
}

/// The member in `slot` left; for the others in the room.
pub fn build_seat_left(slot: u8) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(LEAVE_MODE)?;
    packet.write_byte(slot)?;
    Ok(packet)
}

/// An item went into trade slot `slot`, on the receiver's own side
/// (`partner` false) or the partner's.
pub fn build_trade_item(partner: bool, slot: u8, item: &Item) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(PUT_ITEM_MODE)?;
    packet.write_byte(partner as u8)?;
    packet.write_byte(slot)?;
    encode_item(&mut packet, SlotPrefix::None, item)?;
    Ok(packet)
}

/// A side's offered mesos are now `meso` in all.
pub fn build_trade_meso(partner: bool, meso: i32) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(PUT_MESO_MODE)?;
    packet.write_byte(partner as u8)?;
    packet.write_int(meso)?;
    Ok(packet)
}

/// The partner pressed Trade.
pub fn build_trade_confirmed() -> Result<Packet, NetworkError> {
    room_packet(CONFIRMED_MODE)
}

/// A character of level 15 or below offered more than the meso limit; the
/// client takes back its Trade press.
pub fn build_trade_meso_limit() -> Result<Packet, NetworkError> {
    room_packet(MESO_LIMIT_MODE)
}

/// A player's record at a game: five ints, of which the client shows the
/// last four (inferred, as common v83 servers send it: the game, wins,
/// ties, losses, and a score that starts at 2000).
fn write_record(packet: &mut Packet, kind: u8, record: &GameRecord) -> Result<(), NetworkError> {
    packet.write_int(i32::from(kind))?;
    packet.write_int(record.wins)?;
    packet.write_int(record.ties)?;
    packet.write_int(record.losses)?;
    packet.write_int(GAME_SCORE)?;
    Ok(())
}

/// The score every record shows; nothing keeps one (inferred).
const GAME_SCORE: i32 = 2000;

/// The receiving client enters game `title` of `kind` in `my_slot` (0 for
/// the owner), with the members in it and their records. `piece` is the
/// Omok stones or the cards' board size.
pub fn build_game_room(
    kind: u8,
    my_slot: u8,
    members: &[(u8, &AvatarLook, &str, &GameRecord)],
    title: &str,
    piece: u8,
) -> Result<Packet, NetworkError> {
    let seats: Vec<_> = members
        .iter()
        .map(|&(slot, look, name, _)| (slot, look, name))
        .collect();
    let mut packet = enter_room(kind, crate::mini_game::GAME_USERS, my_slot, &seats)?;
    for &(slot, _, _, record) in members {
        packet.write_byte(slot)?;
        write_record(&mut packet, kind, record)?;
    }
    packet.write_byte(0xFF)?;
    packet.write_str_with_length(title)?;
    packet.write_byte(piece)?;
    packet.write_byte(0)?; // not a tournament
    Ok(packet)
}

/// Someone joined the receiving client's game in `slot`, with their record.
pub fn build_game_visit(
    kind: u8,
    slot: u8,
    look: &AvatarLook,
    name: &str,
    record: &GameRecord,
) -> Result<Packet, NetworkError> {
    let mut packet = build_room_visit(slot, look, name)?;
    write_record(&mut packet, kind, record)?;
    Ok(packet)
}

/// A game's notice about `name` in the room's chat (`game_message`).
pub fn build_game_message(message: u8, name: &str) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(CHAT_MODE)?;
    packet.write_byte(GAME_MESSAGE)?;
    packet.write_byte(message)?;
    packet.write_str_with_length(name)?;
    Ok(packet)
}

/// The visitor is ready to play, or no longer.
pub fn build_game_ready(ready: bool) -> Result<Packet, NetworkError> {
    room_packet(if ready { READY_MODE } else { UNREADY_MODE })
}

/// The opponent asks for a tie.
pub fn build_tie_asked() -> Result<Packet, NetworkError> {
    room_packet(TIE_ASKED_MODE)
}

/// The opponent turned the tie down.
pub fn build_tie_denied() -> Result<Packet, NetworkError> {
    room_packet(TIE_DENIED_MODE)
}

/// Omok: the opponent asks to take back their last stone.
pub fn build_undo_asked() -> Result<Packet, NetworkError> {
    room_packet(UNDO_ASKED_MODE)
}

/// Omok: the answer to taking back a stone: refused, or the last `count`
/// stones go and the player in `slot` moves.
pub fn build_undo_result(undone: Option<(u8, u8)>) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(UNDO_RESULT_MODE)?;
    packet.write_byte(undone.is_some() as u8)?;
    if let Some((count, slot)) = undone {
        packet.write_byte(count)?;
        packet.write_byte(slot)?;
    }
    Ok(packet)
}

/// Omok starts; the player in `waits` moves second.
pub fn build_omok_start(waits: u8) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(GAME_START_MODE)?;
    packet.write_byte(waits)?;
    Ok(packet)
}

/// Match cards start with the cards dealt face down; the player in `waits`
/// moves second.
pub fn build_cards_start(waits: u8, deck: &[i32]) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(GAME_START_MODE)?;
    packet.write_byte(waits)?;
    packet.write_byte(deck.len() as u8)?;
    for &card in deck {
        packet.write_int(card)?;
    }
    Ok(packet)
}

/// A game ended (`game_result`), won by the player in `winner` unless a
/// tie, with both records after it.
pub fn build_game_result(
    kind: u8,
    result: u8,
    winner: u8,
    records: [&GameRecord; 2],
) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(GAME_RESULT_MODE)?;
    packet.write_byte(result)?;
    if result != game_result::TIE {
        packet.write_byte(winner)?;
    }
    for record in records {
        write_record(&mut packet, kind, record)?;
    }
    Ok(packet)
}

/// The player in `slot` moves now: the last one's time ran out.
pub fn build_game_turn(slot: u8) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(TURN_MODE)?;
    packet.write_byte(slot)?;
    Ok(packet)
}

/// Omok: a stone of `color` went down at (x, y).
pub fn build_omok_stone(x: i32, y: i32, color: u8) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(STONE_MODE)?;
    packet.write_int(x)?;
    packet.write_int(y)?;
    packet.write_byte(color)?;
    Ok(packet)
}

/// Omok: the receiving client's stone may not go there (`'C'`: double
/// threes).
pub fn build_omok_rejected(code: u8) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(STONE_REJECTED_MODE)?;
    packet.write_byte(code)?;
    Ok(packet)
}

/// Match cards: the turn's first card, `index`, went face up.
pub fn build_card_first(index: u8) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(CARD_MODE)?;
    packet.write_byte(1)?;
    packet.write_byte(index)?;
    Ok(packet)
}

/// Match cards: the player in `slot` turned up `index` after `first`, and
/// matched them or not.
pub fn build_card_second(
    index: u8,
    first: u8,
    slot: u8,
    matched: bool,
) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(CARD_MODE)?;
    packet.write_byte(0)?;
    packet.write_byte(index)?;
    packet.write_byte(first)?;
    // A match counts from the room's size up.
    let code = if matched {
        slot + crate::mini_game::GAME_USERS
    } else {
        slot
    };
    packet.write_byte(code)?;
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;
    use std::collections::BTreeMap;

    const OP: i16 = SendOpcode::PlayerInteraction as i16;

    fn look(hair: i32) -> AvatarLook {
        AvatarLook {
            gender: 0,
            skin: 0,
            face: 20000,
            hair,
            visible: BTreeMap::from([(5, 1040002), (6, 1060002)]),
            masked: BTreeMap::new(),
            cash_weapon: None,
            pets: [0; 3],
        }
    }

    #[test]
    fn invite_names_the_room_type_inviter_and_room() {
        let packet = build_trade_invite("Alice", 42).unwrap();
        Expect::new(&packet.bytes)
            .short(OP)
            .byte(0x02)
            .byte(3)
            .string("Alice")
            .int(42)
            .end();
    }

    #[test]
    fn invite_results_carry_the_target_except_not_found() {
        let missing = build_invite_not_found().unwrap();
        Expect::new(&missing.bytes)
            .short(OP)
            .byte(0x03)
            .byte(1)
            .end();
        let busy = build_invite_result(invite_result::BUSY, "Bob").unwrap();
        Expect::new(&busy.bytes)
            .short(OP)
            .byte(0x03)
            .byte(2)
            .string("Bob")
            .end();
    }

    #[test]
    fn enter_lists_each_member_and_ends_with_ff() {
        let (alice, bob) = (look(30000), look(30030));
        let packet = build_trade_room(1, &[(0, &alice, "Alice"), (1, &bob, "Bob")]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(OP).byte(0x05).byte(3).byte(2).byte(1);
        expect.byte(0).look(&alice, false).string("Alice");
        expect.byte(1).look(&bob, false).string("Bob");
        expect.byte(0xFF).end();
    }

    #[test]
    fn enter_error_has_room_type_zero() {
        let packet = build_enter_error(enter_error::CLOSED).unwrap();
        Expect::new(&packet.bytes)
            .short(OP)
            .byte(0x05)
            .byte(0)
            .byte(1)
            .end();
    }

    #[test]
    fn visit_carries_the_slot_look_and_name() {
        let bob = look(30030);
        let packet = build_room_visit(1, &bob, "Bob").unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(OP).byte(0x04).byte(1).look(&bob, false);
        expect.string("Bob").end();
    }

    #[test]
    fn chat_and_leave() {
        let chat = build_room_chat(1, "Bob : hi").unwrap();
        Expect::new(&chat.bytes)
            .short(OP)
            .byte(0x06)
            .byte(8)
            .byte(1)
            .string("Bob : hi")
            .end();
        let leave = build_room_leave(0, leave_reason::DONE).unwrap();
        Expect::new(&leave.bytes)
            .short(OP)
            .byte(0x0a)
            .byte(0)
            .byte(7)
            .end();
    }

    #[test]
    fn offers_say_whose_side_they_are_on() {
        let item = Item::stack(2000000, 12);
        let mut encoded = Packet::new_empty();
        encode_item(&mut encoded, SlotPrefix::None, &item).unwrap();
        let packet = build_trade_item(true, 3, &item).unwrap();
        Expect::new(&packet.bytes)
            .short(OP)
            .byte(0x0f)
            .byte(1)
            .byte(3)
            .bytes(&encoded.bytes)
            .end();
        let meso = build_trade_meso(false, 5000).unwrap();
        Expect::new(&meso.bytes)
            .short(OP)
            .byte(0x10)
            .byte(0)
            .int(5000)
            .end();
    }

    #[test]
    fn a_shop_enter_adds_the_title_and_listings() {
        let alice = look(30000);
        let potions = Listing {
            item: Item::stack(2000000, 10),
            bundles: 3,
            price: 500,
        };
        let mut encoded = Packet::new_empty();
        encode_item(&mut encoded, SlotPrefix::None, &potions.item).unwrap();
        let packet = build_shop_room(0, &[(0, &alice, "Alice")], "Cheap!", &[potions]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(OP).byte(0x05).byte(4).byte(4).byte(0);
        expect
            .byte(0)
            .look(&alice, false)
            .string("Alice")
            .byte(0xFF);
        expect.string("Cheap!").byte(16).byte(1);
        expect
            .short(3)
            .short(10)
            .int(500)
            .bytes(&encoded.bytes)
            .end();

        let items = build_shop_items(&[]).unwrap();
        Expect::new(&items.bytes).short(OP).byte(0x19).byte(0).end();
    }

    #[test]
    fn sales_and_failed_purchases() {
        let sold = build_shop_sold(2, 5, "Bob").unwrap();
        Expect::new(&sold.bytes)
            .short(OP)
            .byte(0x1a)
            .byte(2)
            .short(5)
            .string("Bob")
            .end();
        let failed = build_buy_result(buy_result::NO_MESOS).unwrap();
        Expect::new(&failed.bytes)
            .short(OP)
            .byte(0x18)
            .byte(2)
            .end();
    }

    #[test]
    fn the_balloon_names_the_room_and_its_seats() {
        let shop = MiniRoomBox {
            kind: PERSONAL_SHOP,
            sn: 77,
            title: "Cheap!".into(),
            skin: 2,
            users: 1,
            max_users: 4,
            private: false,
            in_progress: false,
        };
        let packet = build_char_box(9, Some(&shop)).unwrap();
        Expect::new(&packet.bytes)
            .short(SendOpcode::UpdateCharBox as i16)
            .int(9)
            .byte(4)
            .int(77)
            .string("Cheap!")
            .byte(0)
            .byte(2)
            .byte(1)
            .byte(4)
            .byte(0)
            .end();
        let gone = build_char_box(9, None).unwrap();
        Expect::new(&gone.bytes)
            .short(SendOpcode::UpdateCharBox as i16)
            .int(9)
            .byte(0)
            .end();
    }

    #[test]
    fn confirmed_and_meso_limit_are_bare_modes() {
        let confirmed = build_trade_confirmed().unwrap();
        Expect::new(&confirmed.bytes).short(OP).byte(0x11).end();
        let limit = build_trade_meso_limit().unwrap();
        Expect::new(&limit.bytes).short(OP).byte(0x15).end();
    }

    fn record(wins: i32, ties: i32, losses: i32) -> GameRecord {
        GameRecord { wins, ties, losses }
    }

    fn expect_record(expect: &mut Expect, kind: u8, record: &GameRecord) {
        expect
            .int(i32::from(kind))
            .int(record.wins)
            .int(record.ties)
            .int(record.losses)
            .int(2000);
    }

    #[test]
    fn a_game_enter_adds_records_title_and_pieces() {
        let (alice, bob) = (look(30000), look(30030));
        let (a, b) = (record(3, 1, 2), record(0, 0, 4));
        let members = [(0, &alice, "Alice", &a), (1, &bob, "Bob", &b)];
        let packet = build_game_room(1, 1, &members, "omok!", 5).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(OP).byte(0x05).byte(1).byte(2).byte(1);
        expect.byte(0).look(&alice, false).string("Alice");
        expect.byte(1).look(&bob, false).string("Bob").byte(0xFF);
        expect.byte(0);
        expect_record(&mut expect, 1, &a);
        expect.byte(1);
        expect_record(&mut expect, 1, &b);
        expect.byte(0xFF).string("omok!").byte(5).byte(0).end();
    }

    #[test]
    fn a_game_visit_adds_the_record() {
        let bob = look(30030);
        let b = record(1, 2, 3);
        let packet = build_game_visit(2, 1, &bob, "Bob", &b).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(OP)
            .byte(0x04)
            .byte(1)
            .look(&bob, false)
            .string("Bob");
        expect_record(&mut expect, 2, &b);
        expect.end();
    }

    #[test]
    fn game_notices_and_bare_modes() {
        let notice = build_game_message(game_message::MATCHED, "Bob").unwrap();
        Expect::new(&notice.bytes)
            .short(OP)
            .byte(0x06)
            .byte(7)
            .byte(9)
            .string("Bob")
            .end();
        for (packet, mode) in [
            (build_game_ready(true), 0x3a),
            (build_game_ready(false), 0x3b),
            (build_tie_asked(), 0x32),
            (build_tie_denied(), 0x33),
            (build_undo_asked(), 0x36),
        ] {
            Expect::new(&packet.unwrap().bytes)
                .short(OP)
                .byte(mode)
                .end();
        }
    }

    #[test]
    fn omok_moves() {
        let start = build_omok_start(0).unwrap();
        Expect::new(&start.bytes).short(OP).byte(0x3d).byte(0).end();
        let stone = build_omok_stone(7, 8, 2).unwrap();
        Expect::new(&stone.bytes)
            .short(OP)
            .byte(0x40)
            .int(7)
            .int(8)
            .byte(2)
            .end();
        let rejected = build_omok_rejected(b'C').unwrap();
        Expect::new(&rejected.bytes)
            .short(OP)
            .byte(0x41)
            .byte(0x43)
            .end();
        let turn = build_game_turn(1).unwrap();
        Expect::new(&turn.bytes).short(OP).byte(0x3f).byte(1).end();
        let refused = build_undo_result(None).unwrap();
        Expect::new(&refused.bytes)
            .short(OP)
            .byte(0x37)
            .byte(0)
            .end();
        let undone = build_undo_result(Some((2, 1))).unwrap();
        Expect::new(&undone.bytes)
            .short(OP)
            .byte(0x37)
            .byte(1)
            .byte(2)
            .byte(1)
            .end();
    }

    #[test]
    fn results_name_the_winner_unless_a_tie() {
        let (a, b) = (record(1, 0, 0), record(0, 0, 1));
        let won = build_game_result(1, game_result::WON, 0, [&a, &b]).unwrap();
        let mut expect = Expect::new(&won.bytes);
        expect.short(OP).byte(0x3e).byte(0).byte(0);
        expect_record(&mut expect, 1, &a);
        expect_record(&mut expect, 1, &b);
        expect.end();
        let tie = build_game_result(2, game_result::TIE, 0, [&a, &b]).unwrap();
        let mut expect = Expect::new(&tie.bytes);
        expect.short(OP).byte(0x3e).byte(1);
        expect_record(&mut expect, 2, &a);
        expect_record(&mut expect, 2, &b);
        expect.end();
    }

    #[test]
    fn match_cards_deal_and_flip() {
        let start = build_cards_start(1, &[0, 1, 1, 0]).unwrap();
        Expect::new(&start.bytes)
            .short(OP)
            .byte(0x3d)
            .byte(1)
            .byte(4)
            .int(0)
            .int(1)
            .int(1)
            .int(0)
            .end();
        let first = build_card_first(3).unwrap();
        Expect::new(&first.bytes)
            .short(OP)
            .byte(0x44)
            .byte(1)
            .byte(3)
            .end();
        let missed = build_card_second(5, 3, 1, false).unwrap();
        Expect::new(&missed.bytes)
            .short(OP)
            .byte(0x44)
            .byte(0)
            .byte(5)
            .byte(3)
            .byte(1)
            .end();
        let matched = build_card_second(5, 3, 1, true).unwrap();
        Expect::new(&matched.bytes)
            .short(OP)
            .byte(0x44)
            .byte(0)
            .byte(5)
            .byte(3)
            .byte(3)
            .end();
    }

    #[test]
    fn a_game_balloon_says_private_and_in_progress() {
        let game = MiniRoomBox {
            kind: 1,
            sn: 5,
            title: "go".into(),
            skin: 3,
            users: 2,
            max_users: 2,
            private: true,
            in_progress: true,
        };
        let packet = build_char_box(9, Some(&game)).unwrap();
        Expect::new(&packet.bytes)
            .short(SendOpcode::UpdateCharBox as i16)
            .int(9)
            .byte(1)
            .int(5)
            .string("go")
            .byte(1)
            .byte(3)
            .byte(2)
            .byte(2)
            .byte(1)
            .end();
    }
}
