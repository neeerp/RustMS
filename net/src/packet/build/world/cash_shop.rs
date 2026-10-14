use crate::codec::encode_character_data;
use crate::{error::NetworkError, packet::op::SendOpcode};
use game::Player;
use packet::{io::write::PktWrite, Packet};

/// Cash Shop tabs with a best-sellers row, and how many items each shows.
const BEST_SELLER_TABS: i32 = 8;
const BEST_SELLERS_PER_TAB: usize = 5;
/// Slots in the Cash Shop wish list.
const WISH_LIST_SLOTS: usize = 10;
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

/// CASHSHOP_OPERATION: the empty cash inventory, with the account's storage
/// and character slots.
pub fn build_cash_inventory(character_slots: i16) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CashShopOperation as i16)?;
    packet.write_byte(0x4B)?;
    packet.write_short(0)?; // items
    packet.write_short(STORAGE_SLOTS)?;
    packet.write_short(character_slots)?;
    Ok(packet)
}

/// CASHSHOP_OPERATION: no gifts waiting.
pub fn build_cash_gifts() -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CashShopOperation as i16)?;
    packet.write_byte(0x4D)?;
    packet.write_short(0)?;
    Ok(packet)
}

/// CASHSHOP_OPERATION: an empty wish list.
pub fn build_wish_list() -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CashShopOperation as i16)?;
    packet.write_byte(0x4F)?;
    for _ in 0..WISH_LIST_SLOTS {
        packet.write_int(0)?;
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

        let packet = build_cash_inventory(6).unwrap();
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

        let packet = build_wish_list().unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::CashShopOperation as i16)
            .byte(0x4F);
        for _ in 0..10 {
            expect.int(0);
        }
        expect.end();
    }
}
