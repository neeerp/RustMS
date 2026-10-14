use crate::codec::encode_character_data;
use crate::codec::stats::write_padded_name;
use crate::codec::time::expiration;
use crate::{error::NetworkError, packet::op::SendOpcode};
use db::cash_shop::CashItem;
use game::Player;
use packet::{io::write::PktWrite, Packet};

/// Cash Shop tabs with a best-sellers row, and how many items each shows.
const BEST_SELLER_TABS: i32 = 8;
const BEST_SELLERS_PER_TAB: usize = 5;
/// Slots in the Cash Shop wish list.
pub const WISH_LIST_SLOTS: usize = 10;
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

/// Why a Cash Shop request failed; the client shows the matching notice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CashShopFailure {
    /// "Due to an unknown error, failed."
    Unknown = 0x00,
    /// "You don't have enough cash."
    NotEnoughCash = 0xA5,
    /// The item is for the other gender.
    Gender = 0xAA,
    /// The item isn't on sale.
    SoldOut = 0xC0,
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

/// CASHSHOP_OPERATION: no gifts waiting.
pub fn build_cash_gifts() -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CashShopOperation as i16)?;
    packet.write_byte(GIFTS)?;
    packet.write_short(0)?;
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

        let packet = build_cash_gifts().unwrap();
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
            },
            CashItem {
                id: 8,
                account_id: 3,
                item_id: 5220000,
                sn: 50200003,
                quantity: 35,
                gift_from: "bob".to_string(),
                expires_at: Some(std::time::UNIX_EPOCH),
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
}
