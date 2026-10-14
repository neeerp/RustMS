use crate::codec::encode_character_data;
use crate::codec::item::{encode_item, SlotPrefix};
use crate::codec::stats::write_padded_name;
use crate::codec::time::expiration;
use crate::{error::NetworkError, packet::op::SendOpcode};
use db::cash_shop::CashItem;
use game::{Item, Player};
use packet::{io::write::PktWrite, Packet};

/// Cash Shop tabs with a best-sellers row, and how many items each shows.
const BEST_SELLER_TABS: i32 = 8;
const BEST_SELLERS_PER_TAB: usize = 5;
/// Slots in the Cash Shop wish list.
pub const WISH_LIST_SLOTS: usize = 10;
/// The longest gift message, in bytes.
pub const GIFT_MESSAGE_LEN: usize = 73;
/// Storage slots of an account that never expanded its storage.
const STORAGE_SLOTS: i16 = 4;

/// SET_CASH_SHOP: take the player out of the map and into the Cash Shop.
/// The shop has no special-priced items and no best sellers yet.
pub fn build_open_cash_shop(player: &Player, account_name: &str) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::SetCashShop as i16)?;
    encode_character_data(&mut packet, player)?;
    packet.write_byte(1)?; // the Cash Shop, not the MTS
    packet.write_str_with_length(account_name)?;
    packet.write_int(0)?;
    packet.write_short(0)?; // special-priced items
    packet.write_bytes(&[0; 121])?;
    for tab in 1..=BEST_SELLER_TABS {
        for gender in 0..2 {
            for _ in 0..BEST_SELLERS_PER_TAB {
                packet.write_int(tab)?;
                packet.write_int(gender)?;
                packet.write_int(0)?; // no item
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
const FAILED: u8 = 0x5C;
const GIFT_SENT: u8 = 0x5E;
const TAKEN_OUT: u8 = 0x68;
const PUT_BACK: u8 = 0x6A;

/// Why a Cash Shop request failed; the client shows the matching notice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CashShopFailure {
    /// "Due to an unknown error, failed."
    Unknown = 0x00,
    /// "You don't have enough cash."
    NotEnoughCash = 0xA5,
    /// "You cannot send a gift to your own account."
    OwnAccount = 0xA8,
    /// "Please confirm whether the character's name is correct."
    NoSuchCharacter = 0xA9,
    /// The item is for the other gender.
    Gender = 0xAA,
    /// "Please check if your inventory is full or not."
    InventoryFull = 0xBB,
    /// The item isn't on sale.
    SoldOut = 0xC0,
    /// The birthday code isn't a date.
    BirthdayCode = 0xC4,
}

/// CASHSHOP_OPERATION: the account's Cash Inventory, with its storage and
/// character slots.
pub fn build_cash_inventory(
    items: &[CashItem],
    character_slots: i16,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CashShopOperation as i16)?;
    packet.write_byte(CASH_INVENTORY)?;
    packet.write_short(items.len() as i16)?;
    for item in items {
        encode_cash_item(&mut packet, item)?;
    }
    packet.write_short(STORAGE_SLOTS)?;
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

/// CASHSHOP_OPERATION: a request failed.
pub fn build_cash_shop_failure(failure: CashShopFailure) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CashShopOperation as i16)?;
    packet.write_byte(FAILED)?;
    packet.write_byte(failure as u8)?;
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
/// the shop, with who sent them and their messages. The client adds them to
/// the Cash Inventory, so they are left out of its list.
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
        let packet = build_open_cash_shop(&player, "alice").unwrap();

        let mut character = Packet::new_empty();
        encode_character_data(&mut character, &player).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::SetCashShop as i16);
        expect.bytes(&character.bytes);
        expect.byte(1).string("alice").int(0).short(0);
        expect.bytes(&[0; 121]);
        for tab in 1..=8 {
            for gender in 0..2 {
                for _ in 0..5 {
                    expect.int(tab).int(gender).int(0);
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

        let packet = build_cash_inventory(&[], 6).unwrap();
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

        let packet = build_cash_shop_failure(CashShopFailure::NotEnoughCash).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::CashShopOperation as i16)
            .byte(0x5C)
            .byte(0xA5);
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

        let packet = build_cash_inventory(&items, 6).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::CashShopOperation as i16)
            .byte(0x4B)
            .short(2);
        expect_item(&mut expect, &items[0], crate::codec::time::DEFAULT_TIME);
        expect_item(&mut expect, &items[1], 116_444_736_000_000_000);
        expect.short(4).short(6);
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
}
