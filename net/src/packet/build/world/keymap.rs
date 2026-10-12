use crate::{error::NetworkError, packet::op::SendOpcode};
use game::keymap::KEY_COUNT;
use game::Keymap;
use packet::{io::write::PktWrite, Packet};

pub fn build_keymap(keymap: &Keymap) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::KeyMap as i16)?;
    packet.write_byte(0)?;

    for key in 0..KEY_COUNT {
        let binding = keymap.get(key);
        packet.write_byte(binding.map_or(0, |b| b.kind))?;
        packet.write_int(binding.map_or(0, |b| b.action))?;
    }

    Ok(packet)
}
