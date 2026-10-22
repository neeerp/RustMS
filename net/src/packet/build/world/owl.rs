//! The Owl of Minerva's answers (report c024): OWL_RESULT (0x46) by mode
//! and OWL_GO_RESULT (0x47).

use crate::codec::item::{encode_item, SlotPrefix};
use crate::owl::ShopHit;
use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

const RESULTS: u8 = 6;
const POPULAR: u8 = 7;

fn owl_result(mode: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::OwlResult as i16)?;
    packet.write_byte(mode)?;
    Ok(packet)
}

/// The most searched items, for the search window's list; dropped if the
/// window has closed.
pub fn build_owl_popular(items: &[i32]) -> Result<Packet, NetworkError> {
    let mut packet = owl_result(POPULAR)?;
    packet.write_byte(items.len() as u8)?;
    for &item_id in items {
        packet.write_int(item_id)?;
    }
    Ok(packet)
}

/// A search's rows for `item_id`. None says "Unable to find the item you
/// have entered."; either way the client may make requests again. The
/// client takes the row's channel as 0-based and lets GO on its own only,
/// and reads a whole item after an equip's row.
pub fn build_owl_results(item_id: i32, hits: &[ShopHit]) -> Result<Packet, NetworkError> {
    let mut packet = owl_result(RESULTS)?;
    // A count the client gives a header row of its own when above 0
    // (0xa28cec); Cosmic sends 0.
    packet.write_int(0)?;
    packet.write_int(item_id)?;
    packet.write_int(hits.len() as i32)?;
    for hit in hits {
        packet.write_str_with_length(&hit.owner)?;
        packet.write_int(hit.map_id)?;
        packet.write_str_with_length(&hit.title)?;
        packet.write_int(i32::from(hit.bundles))?;
        packet.write_int(i32::from(hit.item.quantity))?;
        packet.write_int(hit.price)?;
        // What GO sends back to name the shop.
        packet.write_int(hit.room)?;
        packet.write_byte(hit.channel_id)?;
        let kind = (hit.item.item_id / 1_000_000) as u8;
        packet.write_byte(kind)?;
        if kind == 1 {
            encode_item(&mut packet, SlotPrefix::None, &hit.item)?;
        }
    }
    Ok(packet)
}

/// Whether GO may go (`owl::go_result`).
pub fn build_owl_go_result(code: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::OwlGoResult as i16)?;
    packet.write_byte(code)?;
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::Item;

    #[test]
    fn the_popular_list_counts_its_items() {
        let packet = build_owl_popular(&[2_000_000]).unwrap();
        assert_eq!(packet.bytes, [0x46, 0, 7, 1, 0x80, 0x84, 0x1e, 0]);
    }

    #[test]
    fn a_row_names_the_shop_and_its_price() {
        let hit = ShopHit {
            owner: "Ann".into(),
            title: "S".into(),
            map_id: 910_000_001,
            room: 9,
            channel_id: 1,
            item: Item::stack(4_000_000, 5),
            bundles: 2,
            price: 100,
        };
        let packet = build_owl_results(4_000_000, &[hit]).unwrap();
        let mut expected = vec![0x46, 0, 6, 0, 0, 0, 0, 0, 0x09, 0x3d, 0, 1, 0, 0, 0];
        expected.extend([3, 0, b'A', b'n', b'n']);
        expected.extend(910_000_001i32.to_le_bytes());
        expected.extend([1, 0, b'S']);
        expected.extend([2, 0, 0, 0, 5, 0, 0, 0, 100, 0, 0, 0, 9, 0, 0, 0, 1, 4]);
        assert_eq!(packet.bytes, expected);
    }
}
