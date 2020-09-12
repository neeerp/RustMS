use std::io::BufReader;

use packet::{io::read::PktRead, Packet};

use crate::{
    error::NetworkError, io::client::MapleClient, packet::build, packet::handle::PacketHandler,
};

pub struct ChangeMapHandler {}

impl ChangeMapHandler {
    pub fn new() -> Self {
        Self {}
    }
}

impl PacketHandler for ChangeMapHandler {
    fn handle(&self, packet: &mut Packet, client: &mut MapleClient) -> Result<(), NetworkError> {
        let mut reader = BufReader::new(&**packet);
        let _op = reader.read_short()?;

        reader.read_byte()?;
        let target = reader.read_int()?;
        let _x = reader.read_str_with_length()?;

        reader.read_byte()?; // Padding

        let _wheel_of_destiny = reader.read_short()? > 0;

        let character = client.session.get_character()?;
        let mut character = &mut character.borrow_mut().character;

        if target != -1 {
            character.map_id = target;
            character.save()?;

            client.send(&mut build::world::map::build_warp_to_map(
                &character, target,
            )?)?;
        }

        client.send(&mut build::world::map::build_empty_stat_update()?)
    }
}
