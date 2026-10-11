use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// v83 text dialogue: message type 0, followed by separate Back/Next bytes.
pub fn build_npc_dialogue(page: &crate::dialogue::DialoguePage) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::NpcDialogue as i16)?;
    packet.write_byte(4)?; // NPC speaker category
    packet.write_int(page.npc_id)?;
    packet.write_byte(0)?; // text dialogue
    packet.write_byte(0)?; // NPC portrait
    packet.write_str_with_length(&page.text)?;
    packet.write_byte(u8::from(page.has_previous))?;
    packet.write_byte(u8::from(page.has_next))?;
    Ok(packet)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForeignNpc {
    pub object_id: i32,
    pub npc_id: i32,
    pub x: i16,
    pub y: i16,
    pub flip: bool,
    pub foothold: i16,
    pub rx0: i16,
    pub rx1: i16,
}

pub fn build_spawn_npc(npc: &ForeignNpc) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::SpawnNpc as i16)?;
    packet.write_int(npc.object_id)?;
    packet.write_int(npc.npc_id)?;
    packet.write_short(npc.x)?;
    packet.write_short(npc.y)?;
    packet.write_byte(if npc.flip { 1 } else { 0 })?;
    packet.write_short(npc.foothold)?;
    packet.write_short(npc.rx0)?;
    packet.write_short(npc.rx1)?;
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use packet::io::read::PktRead;
    use std::io::Cursor;

    #[test]
    fn build_spawn_npc_matches_expected_payload() {
        let packet = build_spawn_npc(&ForeignNpc {
            object_id: 1_000_000_000,
            npc_id: 2100,
            x: 1200,
            y: 175,
            flip: true,
            foothold: 32,
            rx0: 1150,
            rx1: 1250,
        })
        .expect("build spawn npc");

        let mut cursor = Cursor::new(&packet.bytes[..]);
        assert_eq!(
            cursor.read_short().expect("opcode"),
            SendOpcode::SpawnNpc as i16
        );
        assert_eq!(cursor.read_int().expect("object id"), 1_000_000_000);
        assert_eq!(cursor.read_int().expect("npc id"), 2100);
        assert_eq!(cursor.read_short().expect("x"), 1200);
        assert_eq!(cursor.read_short().expect("y"), 175);
        assert_eq!(cursor.read_byte().expect("flip"), 1);
        assert_eq!(cursor.read_short().expect("foothold"), 32);
        assert_eq!(cursor.read_short().expect("rx0"), 1150);
        assert_eq!(cursor.read_short().expect("rx1"), 1250);
    }
}
