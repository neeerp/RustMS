use crate::{
    error::NetworkError,
    packet::op::SendOpcode,
    shop::{is_rechargeable, Shop},
};
use packet::{io::write::PktWrite, Packet};

/// OPEN_NPC_SHOP: the shop's list. Stars and bullets carry their recharge
/// rate where everything else carries a bundle size of 1.
pub fn build_open_shop(shop: &Shop) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::OpenNpcShop as i16)?;
    packet.write_int(shop.npc_id)?;
    packet.write_short(shop.items.len() as i16)?;
    for item in &shop.items {
        packet.write_int(item.item_id)?;
        packet.write_int(item.price)?;
        packet.write_int(0)?; // price in tokens
        packet.write_int(0)?; // sale period
        packet.write_int(0)?;
        if is_rechargeable(item.item_id) {
            packet.write_long(item.unit_price.to_bits() as i64)?;
        } else {
            packet.write_short(1)?; // bundle size
        }
        packet.write_short(item.slot_max)?;
    }
    Ok(packet)
}

/// CONFIRM_SHOP_TRANSACTION: how a buy, sell, or recharge went (see
/// `shop::result`).
pub fn build_shop_result(code: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::ConfirmShopTransaction as i16)?;
    packet.write_byte(code)?;
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;
    use crate::shop::ShopItem;

    #[test]
    fn open_shop_lists_bundles_and_recharge_rates() {
        let shop = Shop {
            npc_id: 1011100,
            items: vec![
                ShopItem {
                    item_id: 2000000,
                    price: 50,
                    slot_max: 100,
                    unit_price: 0.0,
                },
                ShopItem {
                    item_id: 2070000,
                    price: 500,
                    slot_max: 530,
                    unit_price: 0.3,
                },
            ],
        };
        let packet = build_open_shop(&shop).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::OpenNpcShop as i16);
        expect.int(1011100).short(2);
        expect.int(2000000).int(50).int(0).int(0).int(0);
        expect.short(1).short(100);
        expect.int(2070000).int(500).int(0).int(0).int(0);
        expect.bytes(&0.3f64.to_le_bytes()).short(530);
        expect.end();
    }

    #[test]
    fn shop_result_is_one_code() {
        let packet = build_shop_result(crate::shop::result::INVENTORY_FULL).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::ConfirmShopTransaction as i16)
            .byte(3)
            .end();
    }
}
