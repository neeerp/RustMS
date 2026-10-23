//! Hired merchants (report 165): the permit check, the merchant on the
//! map (265-267) and its room, mini room type 5 of PLAYER_INTERACTION. The
//! room shares the personal shop's visit, chat, leave, buy result and sold
//! notice packets (`mini_room`).

use super::mini_room::room_packet;
use crate::codec::encode_avatar_look;
use crate::codec::item::{encode_item, SlotPrefix};
use crate::hired_merchant::{balloon_counts, skin, MAX_ITEMS, ROOM_TYPE, ROOM_USERS};
use crate::personal_shop::tax;
use crate::{error::NetworkError, packet::op::SendOpcode};
use game::hired_merchant::{HiredMerchant, Sale};
use game::mini_room::Listing;
use game::AvatarLook;
use packet::{io::write::PktWrite, Packet};
use std::time::Duration;

/// Server → client room modes of a merchant's own.
const ENTER_MODE: u8 = 0x05;
const ITEMS_MODE: u8 = 0x19;
const ORGANIZED_MODE: u8 = 0x28;
const CLOSE_RESULT_MODE: u8 = 0x2a;
const MESO_TAKEN_MODE: u8 = 0x2c;
const VISITORS_MODE: u8 = 0x2e;
const BLACKLIST_MODE: u8 = 0x2f;

/// ENTRUSTED_SHOP_CHECK_RESULT with a code of no body
/// (`hired_merchant::check_result`).
pub fn build_check_result(code: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::EntrustedShopCheckResult as i16)?;
    packet.write_byte(code)?;
    Ok(packet)
}

/// The player's merchant already stands on `map_id` in `channel` (0-based).
pub fn build_open_elsewhere(map_id: i32, channel: u8) -> Result<Packet, NetworkError> {
    let mut packet = build_check_result(crate::hired_merchant::check_result::OPEN_ELSEWHERE)?;
    packet.write_int(map_id)?;
    packet.write_byte(channel)?;
    Ok(packet)
}

/// The balloon over a merchant (0x50d897): it lists in room `sn`, open or
/// in maintenance, with `visitors` in it.
fn write_balloon(
    packet: &mut Packet,
    merchant: &HiredMerchant,
    sn: i32,
    open: bool,
    visitors: u8,
) -> Result<(), NetworkError> {
    let (cur, max) = balloon_counts(open, visitors);
    packet.write_byte(ROOM_TYPE)?;
    packet.write_int(sn)?;
    packet.write_str_with_length(&merchant.title)?;
    packet.write_byte(skin(merchant.item_id))?;
    packet.write_byte(cur)?;
    packet.write_byte(max)?;
    Ok(())
}

/// SPAWN_HIRED_MERCHANT: `merchant`, with room `sn`, stands on the map.
pub fn build_spawn_merchant(
    merchant: &HiredMerchant,
    sn: i32,
    open: bool,
    visitors: u8,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::SpawnHiredMerchant as i16)?;
    packet.write_int(merchant.owner_id)?;
    packet.write_int(merchant.item_id)?;
    packet.write_short(merchant.stand.x)?;
    packet.write_short(merchant.stand.y)?;
    packet.write_short(merchant.stand.foothold)?;
    packet.write_str_with_length(&merchant.owner_name)?;
    write_balloon(&mut packet, merchant, sn, open, visitors)?;
    Ok(packet)
}

/// DESTROY_HIRED_MERCHANT: `owner_id`'s merchant left the map.
pub fn build_destroy_merchant(owner_id: i32) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::DestroyHiredMerchant as i16)?;
    packet.write_int(owner_id)?;
    Ok(packet)
}

/// UPDATE_HIRED_MERCHANT: the balloon over `merchant` now.
pub fn build_update_merchant(
    merchant: &HiredMerchant,
    sn: i32,
    open: bool,
    visitors: u8,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::UpdateHiredMerchant as i16)?;
    packet.write_int(merchant.owner_id)?;
    write_balloon(&mut packet, merchant, sn, open, visitors)?;
    Ok(packet)
}

/// What the owner sees of their merchant on entering its room.
#[derive(Clone, Copy, Debug)]
pub struct OwnerView {
    /// How long it has been open, for the countdown.
    pub open_for: Duration,
    /// Being set up, not yet opened.
    pub setup: bool,
}

/// The receiving client enters `merchant`'s room in `my_slot` (0 for the
/// owner, who gets `owner`), with `visitors` as `(slot, look, name)` and
/// the room's `chat` as `(line, slot)`.
pub fn build_merchant_room(
    merchant: &HiredMerchant,
    my_slot: u8,
    owner: Option<OwnerView>,
    visitors: &[(u8, &AvatarLook, &str)],
    chat: &[(String, u8)],
) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(ENTER_MODE)?;
    packet.write_byte(ROOM_TYPE)?;
    packet.write_byte(ROOM_USERS)?;
    packet.write_byte(my_slot)?;
    // The merchant sits in slot 0, as its permit (0x518531).
    packet.write_byte(0)?;
    packet.write_int(merchant.item_id)?;
    packet.write_str_with_length(&merchant.owner_name)?;
    for &(slot, look, name) in visitors {
        packet.write_byte(slot)?;
        encode_avatar_look(&mut packet, look, false)?;
        packet.write_str_with_length(name)?;
    }
    packet.write_byte(0xFF)?;
    packet.write_short(chat.len() as i16)?;
    for (line, slot) in chat {
        packet.write_str_with_length(line)?;
        packet.write_byte(*slot)?;
    }
    packet.write_str_with_length(&merchant.owner_name)?;
    if let Some(owner) = owner {
        packet.write_int(i32::try_from(owner.open_for.as_millis()).unwrap_or(i32::MAX))?;
        packet.write_byte(owner.setup as u8)?;
        write_sales(&mut packet, &merchant.sales)?;
    }
    packet.write_str_with_length(&merchant.title)?;
    packet.write_byte(MAX_ITEMS)?;
    let listings: Vec<&Listing> = merchant.stock.iter().map(|stock| &stock.listing).collect();
    write_items(&mut packet, merchant.meso, &listings)?;
    Ok(packet)
}

/// The owner's sold-items list, the latest 255 sales, then what they made
/// in all after tax.
fn write_sales(packet: &mut Packet, sales: &[Sale]) -> Result<(), NetworkError> {
    let shown = &sales[sales.len().saturating_sub(usize::from(u8::MAX))..];
    packet.write_byte(shown.len() as u8)?;
    for sale in shown {
        packet.write_int(sale.item_id)?;
        packet.write_short(sale.quantity)?;
        packet.write_int(sale.mesos)?;
        packet.write_str_with_length(&sale.buyer)?;
    }
    let net: i64 = sales
        .iter()
        .map(|sale| i64::from(sale.mesos - tax(sale.mesos)))
        .sum();
    packet.write_int(i32::try_from(net).unwrap_or(i32::MAX))?;
    Ok(())
}

/// The merchant's mesos, then each row: bundles left, items per bundle,
/// the price of a bundle, one bundle as an item.
fn write_items(packet: &mut Packet, meso: i32, listings: &[&Listing]) -> Result<(), NetworkError> {
    packet.write_int(meso)?;
    packet.write_byte(listings.len() as u8)?;
    for listing in listings {
        packet.write_short(listing.bundles)?;
        packet.write_short(listing.item.quantity)?;
        packet.write_int(listing.price)?;
        encode_item(packet, SlotPrefix::None, &listing.item)?;
    }
    Ok(())
}

/// What the merchant holds now: its mesos and its rows.
pub fn build_merchant_items(meso: i32, listings: &[&Listing]) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(ITEMS_MODE)?;
    write_items(&mut packet, meso, listings)?;
    Ok(packet)
}

/// The sold-out rows are struck off; the merchant holds `meso`.
pub fn build_organized(meso: i32) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(ORGANIZED_MODE)?;
    packet.write_int(meso)?;
    Ok(packet)
}

/// The owner closed the merchant (`hired_merchant::close_result`); their
/// client leaves the room.
pub fn build_close_result(code: u8) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(CLOSE_RESULT_MODE)?;
    packet.write_byte(code)?;
    Ok(packet)
}

/// The owner took the merchant's mesos.
pub fn build_meso_taken() -> Result<Packet, NetworkError> {
    room_packet(MESO_TAKEN_MODE)
}

/// Who visited, and for how long each stayed.
pub fn build_visitor_list(visitors: &[(String, Duration)]) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(VISITORS_MODE)?;
    packet.write_short(visitors.len() as i16)?;
    for (name, stayed) in visitors {
        packet.write_str_with_length(name)?;
        packet.write_int(i32::try_from(stayed.as_millis()).unwrap_or(i32::MAX))?;
    }
    Ok(packet)
}

/// The owner's blacklist.
pub fn build_blacklist(names: &[String]) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(BLACKLIST_MODE)?;
    packet.write_short(names.len() as i16)?;
    for name in names {
        packet.write_str_with_length(name)?;
    }
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;
    use game::hired_merchant::{MerchantState, Stand, Stock};
    use game::Item;
    use std::collections::BTreeMap;

    const OP: i16 = SendOpcode::PlayerInteraction as i16;

    fn merchant() -> HiredMerchant {
        HiredMerchant {
            owner_id: 7,
            owner_name: "Alice".into(),
            stand: Stand {
                channel_id: 1,
                map_id: 910000001,
                x: 120,
                y: 34,
                foothold: 5,
            },
            item_id: 5030002,
            title: "Potions".into(),
            state: MerchantState::Open,
            opened_at: None,
            closed_at: None,
            meso: 900,
            stock: vec![Stock {
                id: 11,
                listing: Listing {
                    item: Item::stack(2000000, 5),
                    bundles: 3,
                    price: 100,
                },
            }],
            sales: vec![Sale {
                item_id: 2000000,
                quantity: 10,
                mesos: 200_000,
                buyer: "Bob".into(),
            }],
        }
    }

    fn encoded(item: &Item) -> Vec<u8> {
        let mut packet = Packet::new_empty();
        encode_item(&mut packet, SlotPrefix::None, item).unwrap();
        packet.bytes
    }

    fn look() -> AvatarLook {
        AvatarLook {
            gender: 0,
            skin: 0,
            face: 20000,
            hair: 30030,
            visible: BTreeMap::from([(5, 1040002)]),
            masked: BTreeMap::new(),
            cash_weapon: None,
            pets: [0; 3],
        }
    }

    #[test]
    fn check_results_and_the_other_channel() {
        let ok = build_check_result(7).unwrap();
        Expect::new(&ok.bytes).short(0x32).byte(7).end();
        let elsewhere = build_open_elsewhere(910000003, 2).unwrap();
        Expect::new(&elsewhere.bytes)
            .short(0x32)
            .byte(8)
            .int(910000003)
            .byte(2)
            .end();
    }

    #[test]
    fn the_merchant_spawns_with_its_balloon() {
        let packet = build_spawn_merchant(&merchant(), 42, true, 1).unwrap();
        Expect::new(&packet.bytes)
            .short(0x109)
            .int(7)
            .int(5030002)
            .short(120)
            .short(34)
            .short(5)
            .string("Alice")
            .byte(5)
            .int(42)
            .string("Potions")
            .byte(2)
            .byte(1)
            .byte(4)
            .end();
        let update = build_update_merchant(&merchant(), 42, false, 0).unwrap();
        Expect::new(&update.bytes)
            .short(0x10B)
            .int(7)
            .byte(5)
            .int(42)
            .string("Potions")
            .byte(2)
            .byte(0)
            .byte(1)
            .end();
        let destroy = build_destroy_merchant(7).unwrap();
        Expect::new(&destroy.bytes).short(0x10A).int(7).end();
    }

    #[test]
    fn a_visitor_enters_without_the_owners_block() {
        let merchant = merchant();
        let bob = look();
        let chat = vec![("Bob : hi".to_string(), 1)];
        let packet = build_merchant_room(&merchant, 1, None, &[(1, &bob, "Bob")], &chat).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(OP).byte(0x05).byte(5).byte(4).byte(1);
        expect.byte(0).int(5030002).string("Alice");
        expect.byte(1).look(&bob, false).string("Bob").byte(0xFF);
        expect.short(1).string("Bob : hi").byte(1);
        expect.string("Alice").string("Potions").byte(16).int(900);
        expect.byte(1).short(3).short(5).int(100);
        expect
            .bytes(&encoded(&merchant.stock[0].listing.item))
            .end();
    }

    #[test]
    fn the_owner_gets_the_clock_and_sales() {
        let merchant = merchant();
        let owner = OwnerView {
            open_for: Duration::from_secs(90),
            setup: false,
        };
        let packet = build_merchant_room(&merchant, 0, Some(owner), &[], &[]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(OP).byte(0x05).byte(5).byte(4).byte(0);
        expect.byte(0).int(5030002).string("Alice").byte(0xFF);
        expect.short(0).string("Alice");
        expect.int(90_000).byte(0);
        expect
            .byte(1)
            .int(2000000)
            .short(10)
            .int(200_000)
            .string("Bob");
        // 200 000 less the 0.4 % tax.
        expect.int(199_200);
        expect
            .string("Potions")
            .byte(16)
            .int(900)
            .byte(1)
            .short(3)
            .short(5)
            .int(100);
        expect
            .bytes(&encoded(&merchant.stock[0].listing.item))
            .end();
    }

    #[test]
    fn room_modes() {
        let items = build_merchant_items(5, &[]).unwrap();
        Expect::new(&items.bytes)
            .short(OP)
            .byte(0x19)
            .int(5)
            .byte(0)
            .end();
        let organized = build_organized(77).unwrap();
        Expect::new(&organized.bytes)
            .short(OP)
            .byte(0x28)
            .int(77)
            .end();
        let closed = build_close_result(3).unwrap();
        Expect::new(&closed.bytes)
            .short(OP)
            .byte(0x2a)
            .byte(3)
            .end();
        let taken = build_meso_taken().unwrap();
        Expect::new(&taken.bytes).short(OP).byte(0x2c).end();
        let visitors =
            build_visitor_list(&[("Bob".to_string(), Duration::from_millis(1500))]).unwrap();
        Expect::new(&visitors.bytes)
            .short(OP)
            .byte(0x2e)
            .short(1)
            .string("Bob")
            .int(1500)
            .end();
        let blacklist = build_blacklist(&["Eve".to_string()]).unwrap();
        Expect::new(&blacklist.bytes)
            .short(OP)
            .byte(0x2f)
            .short(1)
            .string("Eve")
            .end();
    }
}
