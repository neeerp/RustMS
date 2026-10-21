use crate::{error::NetworkError, packet::op::SendOpcode};
use game::QuickSlots;
use packet::{io::write::PktWrite, Packet};

/// QUICKSLOT_INIT (0x9F, v83 FUN_0072b82a): 0 for the client's default
/// layout, or 1 and the eight keys. The client keeps its layout across
/// characters, so it goes on every entry to the game. Only a field's stage
/// reads it, so it follows SET_FIELD. The client answers with QUICK_SLOT.
pub fn build_quick_slot_init(slots: Option<QuickSlots>) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::QuickSlotInit as i16)?;
    match slots {
        None => {
            packet.write_byte(0)?;
        }
        Some(QuickSlots(keys)) => {
            packet.write_byte(1)?;
            for key in keys {
                packet.write_int(i32::from(key))?;
            }
        }
    }
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;

    #[test]
    fn none_is_the_default_layout() {
        let packet = build_quick_slot_init(None).unwrap();
        Expect::new(&packet.bytes)
            .short(SendOpcode::QuickSlotInit as i16)
            .byte(0)
            .end();
    }

    #[test]
    fn a_layout_is_its_eight_keys() {
        let keys = [2, 3, 4, 5, 16, 17, 18, 19];
        let packet = build_quick_slot_init(Some(QuickSlots(keys))).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::QuickSlotInit as i16).byte(1);
        for key in keys {
            expect.int(i32::from(key));
        }
        expect.end();
    }
}
