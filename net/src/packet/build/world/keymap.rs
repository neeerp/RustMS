use crate::{error::NetworkError, packet::op::SendOpcode};
use db::keybinding::Keybinding;
use packet::{io::write::PktWrite, Packet};
use std::collections::HashMap;

pub fn build_keymap(binds: HashMap<i16, Keybinding>) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();

    let op = SendOpcode::KeyMap as i16;
    packet.write_short(op)?;
    packet.write_byte(0)?;

    for i in 0..90 {
        if let Some(bind) = binds.get(&i) {
            packet.write_byte(bind.bind_type.ord())?;
            packet.write_int(bind.action as i32)?;
        } else {
            packet.write_byte(0)?;
            packet.write_int(0)?;
        }
    }

    Ok(packet)
}
