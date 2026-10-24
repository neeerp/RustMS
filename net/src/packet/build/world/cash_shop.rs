use crate::codec::encode_character_data;
use crate::codec::item::{encode_item, SlotPrefix};
use crate::codec::stats::write_padded_name;
use crate::codec::time::expiration;
use crate::{error::NetworkError, packet::op::SendOpcode};
use db::cash_shop::CashItem;
use game::{InventoryType, Item, Player};
use packet::{io::write::PktWrite, Packet};

/// Cash Shop tabs with a best-sellers row, and how many items each shows.
const BEST_SELLER_TABS: i32 = 8;
const BEST_SELLERS_PER_TAB: usize = 5;
/// Slots in the Cash Shop wish list.
pub const WISH_LIST_SLOTS: usize = 10;
/// The longest gift message, in bytes.
pub const GIFT_MESSAGE_LEN: usize = 73;

/// SET_CASH_SHOP: take the player out of the map and into the Cash Shop.
/// `best` holds the Best Items for men and for women, up to five each; every
/// tab gets the same. The shop has no special-priced items.
pub fn build_open_cash_shop(
    player: &Player,
    account_name: &str,
    best: &[Vec<i32>; 2],
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::SetCashShop as i16)?;
    encode_character_data(&mut packet, player)?;
    packet.write_byte(1)?; // the Cash Shop, not the MTS
    packet.write_str_with_length(account_name)?;
    packet.write_int(0)?;
    packet.write_short(0)?; // special-priced items
    packet.write_bytes(&[0; 121])?;
    for tab in 1..=BEST_SELLER_TABS {
        for (gender, best) in (0..).zip(best) {
            for index in 0..BEST_SELLERS_PER_TAB {
                packet.write_int(tab)?;
                packet.write_int(gender)?;
                packet.write_int(best.get(index).copied().unwrap_or(0))?;
            }
        }
    }
    packet.write_int(0)?;
    packet.write_short(0)?;
    packet.write_byte(0)?;
    packet.write_int(75)?;
    Ok(packet)
}

/// QUERY_CASH_RESULT: the account's NX Credit, Maple Points and NX Prepaid.
pub fn build_cash_balance(credit: i32, points: i32, prepaid: i32) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::QueryCashResult as i16)?;
    packet.write_int(credit)?;
    packet.write_int(points)?;
    packet.write_int(prepaid)?;
    Ok(packet)
}

/// CASHSHOP_OPERATION modes.
const CASH_INVENTORY: u8 = 0x4B;
const GIFTS: u8 = 0x4D;
const WISH_LIST: u8 = 0x4F;
const WISH_LIST_UPDATED: u8 = 0x55;
const BOUGHT: u8 = 0x57;
const BUY_FAILED: u8 = 0x58;
const COUPON_REDEEMED: u8 = 0x59;
const FAILED: u8 = 0x5C;
const GIFT_SENT: u8 = 0x5E;
const INVENTORY_SLOTS: u8 = 0x60;
const STORAGE_SLOTS: u8 = 0x62;
const CHARACTER_SLOTS: u8 = 0x64;
const TAKEN_OUT: u8 = 0x68;
const PUT_BACK: u8 = 0x6A;
const BOUGHT_RING: u8 = 0x87;
const BOUGHT_PACKAGE: u8 = 0x89;
const BOUGHT_WITH_MESOS: u8 = 0x8D;
const BOUGHT_NAME_CHANGE: u8 = 0x9E;

/// Why a Cash Shop request failed; the client shows the matching notice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CashShopFailure {
    /// "Due to an unknown error, the request for Cash Shop has failed."
    Unknown = 0x00,
    /// "You don't have enough cash."
    NotEnoughCash = 0xA5,
    /// "You cannot send a gift to your own account. Please purchase it
    /// after logging in with the related character."
    OwnAccount = 0xA8,
    /// "Please confirm whether the character's name is correct."
    NoSuchCharacter = 0xA9,
    /// "This item has a gender restriction. Please confirm the gender of
    /// the recipient."
    Gender = 0xAA,
    /// "Please check and see if the coupon number is right."
    CouponNotFound = 0xB0,
    /// The third wrong code on this connection: v83 says so and leaves the
    /// shop itself (report b072).
    CouponStrikes = 0xB1,
    /// "This coupon has expired."
    CouponExpired = 0xB2,
    /// "This coupon was already used."
    CouponUsed = 0xB3,
    /// "Please check if your inventory is full or not."
    InventoryFull = 0xBB,
    /// "The item is out of stock, and therefore not available for sale."
    SoldOut = 0xC0,
    /// "You do not have enough mesos."
    NotEnoughMesos = 0xC2,
    /// "Check your birthday code and please try again."
    BirthdayCode = 0xC4,
}

/// CASHSHOP_OPERATION: the account's Cash Inventory, with its storage and
/// character slots.
pub fn build_cash_inventory(
    items: &[CashItem],
    storage_slots: i16,
    character_slots: i16,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CashShopOperation as i16)?;
    packet.write_byte(CASH_INVENTORY)?;
    packet.write_short(items.len() as i16)?;
    for item in items {
        encode_cash_item(&mut packet, item)?;
    }
    packet.write_short(storage_slots)?;
    packet.write_short(character_slots)?;
    Ok(packet)
}

/// CASHSHOP_OPERATION: a purchase is in the Cash Inventory.
pub fn build_bought_cash_item(item: &CashItem) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CashShopOperation as i16)?;
    packet.write_byte(BOUGHT)?;
    encode_cash_item(&mut packet, item)?;
    Ok(packet)
}

/// CASHSHOP_OPERATION: a coupon was redeemed. `cash_items` are new in the
/// Cash Inventory; the Maple Points, the bag items (item id, quantity) and
/// the mesos are already the player's and only shown in the notice. The
/// balances follow in QUERY_CASH_RESULT, which unlocks the shop.
pub fn build_coupon_redeemed(
    cash_items: &[CashItem],
    maple_points: i32,
    items: &[(i32, i16)],
    mesos: i32,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CashShopOperation as i16)?;
    packet.write_byte(COUPON_REDEEMED)?;
    packet.write_byte(cash_items.len() as u8)?;
    for item in cash_items {
        encode_cash_item(&mut packet, item)?;
    }
    packet.write_int(maple_points)?;
    packet.write_int(items.len() as i32)?;
    for &(item_id, quantity) in items {
        packet.write_short(quantity)?;
        packet.write_short(0)?;
        packet.write_int(item_id)?;
    }
    packet.write_int(mesos)?;
    Ok(packet)
}

/// CASHSHOP_OPERATION: a crush or friendship ring was bought. The buyer's
/// ring, `item`, is in its Cash Inventory, and its partner's went to `to`
/// as a gift (report b061).
pub fn build_bought_ring(item: &CashItem, to: &str) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CashShopOperation as i16)?;
    packet.write_byte(BOUGHT_RING)?;
    encode_cash_item(&mut packet, item)?;
    packet.write_str_with_length(to)?;
    packet.write_int(item.item_id)?;
    packet.write_short(item.quantity)?;
    Ok(packet)
}

/// CASHSHOP_OPERATION: a package's items are in the Cash Inventory.
pub fn build_bought_package(items: &[CashItem]) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CashShopOperation as i16)?;
    packet.write_byte(BOUGHT_PACKAGE)?;
    packet.write_byte(items.len() as u8)?;
    for item in items {
        encode_cash_item(&mut packet, item)?;
    }
    packet.write_short(0)?;
    Ok(packet)
}

/// CASHSHOP_OPERATION: the bag of `kind` now has `slots` slots.
pub fn build_inventory_slots(kind: InventoryType, slots: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CashShopOperation as i16)?;
    packet.write_byte(INVENTORY_SLOTS)?;
    packet.write_byte(kind as u8)?;
    packet.write_short(i16::from(slots))?;
    Ok(packet)
}

/// CASHSHOP_OPERATION: the account's storage now has `slots` slots.
pub fn build_storage_slots(slots: i16) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CashShopOperation as i16)?;
    packet.write_byte(STORAGE_SLOTS)?;
    packet.write_short(slots)?;
    Ok(packet)
}

/// CASHSHOP_OPERATION: the account now has `slots` character slots. v83
/// takes only a count above the one it knows, up to 15, and leaves the shop
/// locked until the balance follows.
pub fn build_character_slots(slots: i16) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CashShopOperation as i16)?;
    packet.write_byte(CHARACTER_SLOTS)?;
    packet.write_short(slots)?;
    Ok(packet)
}

/// CASHSHOP_OPERATION: an item left the Cash Inventory for `slot` of its bag.
pub fn build_taken_out(slot: i16, item: &Item) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CashShopOperation as i16)?;
    packet.write_byte(TAKEN_OUT)?;
    packet.write_short(slot)?;
    encode_item(&mut packet, SlotPrefix::None, item)?;
    Ok(packet)
}

/// CASHSHOP_OPERATION: a character's item is back in the Cash Inventory.
pub fn build_put_back(item: &CashItem) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CashShopOperation as i16)?;
    packet.write_byte(PUT_BACK)?;
    encode_cash_item(&mut packet, item)?;
    Ok(packet)
}

/// CASHSHOP_OPERATION: `quantity` of `item_id`, bought with mesos, is in
/// `slot` of its bag, which the shop's view of the bags turns to and
/// selects; the item and meso changes come first. v83 says the purchase is
/// made (0x47b603).
pub fn build_bought_with_mesos(
    item_id: i32,
    quantity: i16,
    slot: i16,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CashShopOperation as i16)?;
    packet.write_byte(BOUGHT_WITH_MESOS)?;
    packet.write_int(1)?;
    packet.write_short(quantity)?;
    packet.write_short(slot)?;
    packet.write_int(item_id)?;
    Ok(packet)
}

/// CASHSHOP_OPERATION: a Name Change request is made; its receipt is in the
/// Cash Inventory (0x47bccb). The balances follow.
pub fn build_bought_name_change(item: &CashItem) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CashShopOperation as i16)?;
    packet.write_byte(BOUGHT_NAME_CHANGE)?;
    encode_cash_item(&mut packet, item)?;
    Ok(packet)
}

/// CASHSHOP_OPERATION: a request failed.
pub fn build_cash_shop_failure(failure: CashShopFailure) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CashShopOperation as i16)?;
    packet.write_byte(FAILED)?;
    packet.write_byte(failure as u8)?;
    Ok(packet)
}

/// CASHSHOP_OPERATION: a buy of `sn` that failed. v83's Buy Avatar queue
/// goes on to its next SN after this one (0x479985), where the general
/// failure would stall it, and a lone buy shows the same notice. A sold-out
/// answer names the SN, which the catalog then shows as sold out.
pub fn build_buy_failure(failure: CashShopFailure, sn: i32) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CashShopOperation as i16)?;
    packet.write_byte(BUY_FAILED)?;
    packet.write_byte(failure as u8)?;
    if failure == CashShopFailure::SoldOut {
        packet.write_int(sn)?;
    }
    Ok(packet)
}

/// One Cash Inventory item: 55 bytes.
fn encode_cash_item(packet: &mut Packet, item: &CashItem) -> Result<(), NetworkError> {
    packet.write_long(item.id)?;
    packet.write_int(item.account_id)?;
    packet.write_int(0)?; // the character it was bought for
    packet.write_int(item.item_id)?;
    packet.write_int(item.sn)?;
    packet.write_short(item.quantity)?;
    write_padded_name(packet, &item.gift_from)?;
    packet.write_long(expiration(item.expires_at))?;
    packet.write_long(0)?;
    Ok(())
}

/// CASHSHOP_OPERATION: the gifts received since the account last entered
/// the shop, with who sent them and their messages. They are already in the
/// Cash Inventory's list; the client only announces them, one at a time.
pub fn build_cash_gifts(gifts: &[CashItem]) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CashShopOperation as i16)?;
    packet.write_byte(GIFTS)?;
    packet.write_short(gifts.len() as i16)?;
    for gift in gifts {
        packet.write_long(gift.id)?;
        packet.write_int(gift.item_id)?;
        write_padded_name(&mut packet, &gift.gift_from)?;
        let message = gift.gift_message.as_deref().unwrap_or_default().as_bytes();
        let message = &message[..message.len().min(GIFT_MESSAGE_LEN)];
        packet.write_bytes(message)?;
        packet.write_bytes(&[0; GIFT_MESSAGE_LEN][message.len()..])?;
    }
    Ok(packet)
}

/// CASHSHOP_OPERATION: `count` of `item_id` went to the character named `to`
/// for `price`.
pub fn build_gift_sent(
    to: &str,
    item_id: i32,
    count: i16,
    price: i32,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CashShopOperation as i16)?;
    packet.write_byte(GIFT_SENT)?;
    packet.write_str_with_length(to)?;
    packet.write_int(item_id)?;
    packet.write_short(count)?;
    packet.write_int(price)?;
    Ok(packet)
}

/// CASHSHOP_OPERATION: the character's wish list, on entering the shop or
/// (`updated`) after changing it.
pub fn build_wish_list(sns: &[i32], updated: bool) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CashShopOperation as i16)?;
    packet.write_byte(if updated {
        WISH_LIST_UPDATED
    } else {
        WISH_LIST
    })?;
    for slot in 0..WISH_LIST_SLOTS {
        packet.write_int(sns.get(slot).copied().unwrap_or(0))?;
    }
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::stats::sample_stats;
    use crate::codec::testing::Expect;
    use game::{Inventory, Keymap, QuestLog, SkillBook};

    fn player() -> Player {
        Player::new(
            9,
            0,
            sample_stats(),
            0,
            Inventory::default(),
            SkillBook::default(),
            Keymap::defaults(),
            QuestLog::default(),
        )
    }

    #[test]
    fn open_cash_shop_follows_the_character_data() {
        let player = player();
        let best = [vec![50200003, 20000053], vec![50200003]];
        let packet = build_open_cash_shop(&player, "alice", &best).unwrap();

        let mut character = Packet::new_empty();
        encode_character_data(&mut character, &player).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::SetCashShop as i16);
        expect.bytes(&character.bytes);
        expect.byte(1).string("alice").int(0).short(0);
        expect.bytes(&[0; 121]);
        for tab in 1..=8 {
            for (gender, sns) in [
                (0, [50200003, 20000053, 0, 0, 0]),
                (1, [50200003, 0, 0, 0, 0]),
            ] {
                for sn in sns {
                    expect.int(tab).int(gender).int(sn);
                }
            }
        }
        expect.int(0).short(0).byte(0).int(75);
        expect.end();
    }

    #[test]
    fn cash_shop_lists_match_reference() {
        let packet = build_cash_balance(1, 2, 3).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::QueryCashResult as i16);
        expect.int(1).int(2).int(3);
        expect.end();

        let packet = build_cash_inventory(&[], 4, 6).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::CashShopOperation as i16)
            .byte(0x4B);
        expect.short(0).short(4).short(6);
        expect.end();

        let packet = build_cash_gifts(&[]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::CashShopOperation as i16)
            .byte(0x4D);
        expect.short(0);
        expect.end();

        let packet = build_wish_list(&[], false).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::CashShopOperation as i16)
            .byte(0x4F);
        for _ in 0..10 {
            expect.int(0);
        }
        expect.end();

        let packet = build_wish_list(&[50200005, 20000053], true).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::CashShopOperation as i16)
            .byte(0x55);
        expect.int(50200005).int(20000053);
        for _ in 2..10 {
            expect.int(0);
        }
        expect.end();

        let packet = build_bought_with_mesos(4031191, 1, 3).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::CashShopOperation as i16)
            .byte(0x8D);
        expect.int(1).short(1).short(3).int(4031191);
        expect.end();

        let packet = build_cash_shop_failure(CashShopFailure::NotEnoughCash).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::CashShopOperation as i16)
            .byte(0x5C)
            .byte(0xA5);
        expect.end();

        let packet = build_buy_failure(CashShopFailure::NotEnoughCash, 10000000).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::CashShopOperation as i16)
            .byte(0x58)
            .byte(0xA5);
        expect.end();

        let packet = build_buy_failure(CashShopFailure::SoldOut, 10000000).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::CashShopOperation as i16)
            .byte(0x58)
            .byte(0xC0)
            .int(10000000);
        expect.end();
    }

    #[test]
    fn cash_items_are_55_bytes() {
        let items = [
            CashItem {
                id: 7,
                account_id: 3,
                item_id: 5220000,
                sn: 50200005,
                quantity: 1,
                gift_from: String::new(),
                expires_at: None,
                gift_message: None,
            },
            CashItem {
                id: 8,
                account_id: 3,
                item_id: 5220000,
                sn: 50200003,
                quantity: 35,
                gift_from: "bob".to_string(),
                expires_at: Some(std::time::UNIX_EPOCH),
                gift_message: None,
            },
        ];
        let expect_item = |expect: &mut Expect, item: &CashItem, time: i64| {
            expect.long(item.id).int(3).int(0).int(5220000);
            expect.int(item.sn).short(item.quantity);
            let mut from = item.gift_from.clone().into_bytes();
            from.resize(13, 0);
            expect.bytes(&from).long(time).long(0);
        };

        let packet = build_bought_cash_item(&items[0]).unwrap();
        assert_eq!(packet.bytes.len(), 2 + 1 + 55);
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::CashShopOperation as i16)
            .byte(0x57);
        expect_item(&mut expect, &items[0], crate::codec::time::DEFAULT_TIME);
        expect.end();

        let packet = build_put_back(&items[1]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::CashShopOperation as i16)
            .byte(0x6A);
        expect_item(&mut expect, &items[1], 116_444_736_000_000_000);
        expect.end();

        let packet = build_cash_inventory(&items, 12, 6).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::CashShopOperation as i16)
            .byte(0x4B)
            .short(2);
        expect_item(&mut expect, &items[0], crate::codec::time::DEFAULT_TIME);
        expect_item(&mut expect, &items[1], 116_444_736_000_000_000);
        expect.short(12).short(6);
        expect.end();

        let packet = build_bought_package(&items).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::CashShopOperation as i16)
            .byte(0x89)
            .byte(2);
        expect_item(&mut expect, &items[0], crate::codec::time::DEFAULT_TIME);
        expect_item(&mut expect, &items[1], 116_444_736_000_000_000);
        expect.short(0);
        expect.end();
    }

    #[test]
    fn expansions_report_the_new_slot_count() {
        let packet = build_inventory_slots(InventoryType::Setup, 28).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::CashShopOperation as i16)
            .byte(0x60)
            .byte(3)
            .short(28);
        expect.end();

        let packet = build_storage_slots(12).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::CashShopOperation as i16)
            .byte(0x62)
            .short(12);
        expect.end();

        let packet = build_character_slots(4).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::CashShopOperation as i16)
            .byte(0x64)
            .short(4);
        expect.end();
    }

    #[test]
    fn gifts_carry_who_sent_them_and_why() {
        let gift = CashItem {
            id: 9,
            account_id: 3,
            item_id: 1002357,
            sn: 20000000,
            quantity: 1,
            gift_from: "bob".to_string(),
            expires_at: None,
            gift_message: Some("happy birthday".to_string()),
        };
        let packet = build_cash_gifts(&[gift]).unwrap();
        assert_eq!(packet.bytes.len(), 2 + 1 + 2 + 8 + 4 + 13 + 73);
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::CashShopOperation as i16)
            .byte(0x4D)
            .short(1)
            .long(9)
            .int(1002357);
        let mut from = b"bob".to_vec();
        from.resize(13, 0);
        let mut message = b"happy birthday".to_vec();
        message.resize(73, 0);
        expect.bytes(&from).bytes(&message);
        expect.end();

        let packet = build_gift_sent("Bob", 1002357, 1, 3400).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::CashShopOperation as i16)
            .byte(0x5E)
            .short(3)
            .bytes(b"Bob")
            .int(1002357)
            .short(1)
            .int(3400);
        expect.end();
    }

    #[test]
    fn a_redeemed_coupon_lists_what_it_gave() {
        let item = CashItem {
            id: 9,
            account_id: 3,
            item_id: 5150000,
            sn: 50100000,
            quantity: 1,
            gift_from: String::new(),
            expires_at: None,
            gift_message: None,
        };
        let packet = build_coupon_redeemed(&[item], 500, &[(2000000, 20)], 1000).unwrap();
        assert_eq!(packet.bytes.len(), 2 + 1 + 1 + 55 + 4 + 4 + 8 + 4);
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::CashShopOperation as i16)
            .byte(0x59)
            .byte(1)
            .long(9)
            .int(3)
            .int(0)
            .int(5150000)
            .int(50100000)
            .short(1)
            .skip(13 + 8 + 8)
            .int(500)
            .int(1)
            .short(20)
            .short(0)
            .int(2000000)
            .int(1000);
        expect.end();

        let packet = build_cash_shop_failure(CashShopFailure::CouponStrikes).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::CashShopOperation as i16)
            .byte(0x5C)
            .byte(0xB1);
        expect.end();
    }
}
