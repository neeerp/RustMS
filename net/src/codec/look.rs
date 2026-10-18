use crate::error::NetworkError;
use game::AvatarLook;
use packet::{io::write::PktWrite, Packet};

/// Write an avatar look (`addCharLook` in HeavenMS). `megaphone` selects the
/// variant used inside avatar megaphones.
pub fn encode_avatar_look(
    packet: &mut Packet,
    look: &AvatarLook,
    megaphone: bool,
) -> Result<(), NetworkError> {
    packet.write_byte(look.gender)?;
    packet.write_byte(look.skin)?;
    packet.write_int(look.face)?;
    packet.write_byte(if megaphone { 0 } else { 1 })?;
    packet.write_int(look.hair)?;

    for (&slot, &item_id) in &look.visible {
        packet.write_byte(slot)?;
        packet.write_int(item_id)?;
    }
    packet.write_byte(0xFF)?;
    for (&slot, &item_id) in &look.masked {
        packet.write_byte(slot)?;
        packet.write_int(item_id)?;
    }
    packet.write_byte(0xFF)?;

    packet.write_int(look.cash_weapon.unwrap_or(0))?;
    for pet in look.pets {
        packet.write_int(pet)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;
    use std::collections::BTreeMap;

    #[test]
    fn look_lists_visible_then_masked_equipment() {
        let look = AvatarLook {
            gender: 0,
            skin: 1,
            face: 20000,
            hair: 30030,
            visible: BTreeMap::from([(5, 1042003), (7, 1072001)]),
            masked: BTreeMap::from([(5, 1040002)]),
            cash_weapon: Some(1702000),
            pets: [0; 3],
        };
        let mut packet = Packet::new_empty();
        encode_avatar_look(&mut packet, &look, false).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.byte(0).byte(1).int(20000).byte(1).int(30030);
        expect.byte(5).int(1042003).byte(7).int(1072001).byte(0xFF);
        expect.byte(5).int(1040002).byte(0xFF);
        expect.int(1702000).int(0).int(0).int(0);
        expect.end();
    }
}
