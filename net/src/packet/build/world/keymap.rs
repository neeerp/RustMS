use crate::{error::NetworkError, packet::op::SendOpcode};
use db::keybinding::KeybindSet;
use packet::{io::write::PktWrite, Packet};

pub fn build_keymap(binds: &mut KeybindSet) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();

    let op = SendOpcode::KeyMap as i16;
    packet.write_short(op)?;
    packet.write_byte(0)?;

    for i in 0..90 {
        let bind = binds.get(i);
        packet.write_byte(bind.bind_type.ord())?;
        packet.write_int(bind.action as i32)?;
    }

    Ok(packet)
}
