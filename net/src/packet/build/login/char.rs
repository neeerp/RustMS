use crate::codec::{encode_avatar_look, encode_character_stats};
use crate::{error::NetworkError, packet::op::SendOpcode};
use game::{AvatarLook, CharacterStats};
use packet::{io::write::PktWrite, Packet};

/// PIC mode byte in the character list: 2 means PIC is disabled.
const PIC_DISABLED: u8 = 2;

pub fn build_char_list(
    entries: &[(CharacterStats, AvatarLook)],
    character_slots: i32,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CharList as i16)?;
    packet.write_byte(0)?; // status
    packet.write_byte(entries.len() as u8)?;
    for (stats, look) in entries {
        write_char_entry(&mut packet, stats, look)?;
    }
    packet.write_byte(PIC_DISABLED)?;
    packet.write_int(character_slots)?;
    Ok(packet)
}

pub fn build_char_name_response(name: &str, valid: bool) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CharNameResponse as i16)?;
    packet.write_str_with_length(name)?;
    packet.write_byte(!valid as u8)?;
    Ok(packet)
}

pub fn build_char_delete(character_id: i32, status: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::DeleteCharacter as i16)?;
    packet.write_int(character_id)?;
    packet.write_byte(status)?;
    Ok(packet)
}

/// ADD_NEW_CHAR_ENTRY after a successful creation.
pub fn build_char_packet(
    stats: &CharacterStats,
    look: &AvatarLook,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::NewCharacter as i16)?;
    packet.write_byte(0)?;
    write_char_entry(&mut packet, stats, look)?;
    Ok(packet)
}

/// One character in the list (`addCharEntry` in HeavenMS), with ranking off.
fn write_char_entry(
    packet: &mut Packet,
    stats: &CharacterStats,
    look: &AvatarLook,
) -> Result<(), NetworkError> {
    encode_character_stats(packet, stats, [0; 3])?;
    encode_avatar_look(packet, look, false)?;
    packet.write_byte(0)?; // not the "view all" list
    packet.write_byte(0)?; // world ranking disabled
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::stats::sample_stats;
    use crate::codec::testing::Expect;
    use std::collections::BTreeMap;

    fn sample_look() -> AvatarLook {
        AvatarLook {
            gender: 1,
            skin: 2,
            face: 20000,
            hair: 30030,
            visible: BTreeMap::from([(5, 1042003), (7, 1072001)]),
            masked: BTreeMap::from([(5, 1040002)]),
            cash_weapon: Some(1702000),
            pets: [0; 3],
        }
    }

    #[test]
    fn char_list_wraps_entries_with_pic_flag_and_slot_count() {
        let stats = sample_stats();
        let look = sample_look();
        let packet = build_char_list(&[(stats.clone(), look.clone())], 6).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::CharList as i16).byte(0).byte(1);
        expect.stats(&stats).look(&look, false);
        expect.byte(0).byte(0); // not "view all"; ranking off
        expect.byte(2).int(6); // PIC disabled; character slots
        expect.end();
    }

    #[test]
    fn empty_char_list() {
        let packet = build_char_list(&[], 3).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::CharList as i16).byte(0).byte(0);
        expect.byte(2).int(3);
        expect.end();
    }

    #[test]
    fn name_response_flags_taken_names() {
        let free = build_char_name_response("Fresh", true).unwrap();
        Expect::new(&free.bytes)
            .short(SendOpcode::CharNameResponse as i16)
            .string("Fresh")
            .byte(0)
            .end();

        let taken = build_char_name_response("Taken", false).unwrap();
        Expect::new(&taken.bytes)
            .short(SendOpcode::CharNameResponse as i16)
            .string("Taken")
            .byte(1)
            .end();
    }

    #[test]
    fn delete_response_carries_id_and_state() {
        let packet = build_char_delete(0x0102_0304, 0x14).unwrap();
        Expect::new(&packet.bytes)
            .short(SendOpcode::DeleteCharacter as i16)
            .int(0x0102_0304)
            .byte(0x14)
            .end();
    }

    #[test]
    fn new_character_entry_matches_reference() {
        let stats = sample_stats();
        let look = sample_look();
        let packet = build_char_packet(&stats, &look).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::NewCharacter as i16).byte(0);
        expect.stats(&stats).look(&look, false);
        expect.byte(0).byte(0);
        expect.end();
    }
}
