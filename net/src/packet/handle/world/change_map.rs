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
        let mut chr = character.lock().unwrap();

        if target != -1 {
            chr.character.map_id = target;
            chr.character.save()?;

            client.send(&mut build::world::map::build_warp_to_map(
                &chr.character, target,
            )?)?;
        }

        client.send(&mut build::world::map::build_empty_stat_update()?)
    }
}

// === ASYNC HANDLER ===
use crate::handler::{AsyncPacketHandler, HandlerContext, HandlerResult};

pub struct AsyncChangeMapHandler;

impl AsyncChangeMapHandler {
    pub fn new() -> Self {
        Self
    }
}

impl AsyncPacketHandler for AsyncChangeMapHandler {
    fn handle(
        &self,
        packet: &mut Packet,
        ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        let mut reader = BufReader::new(&**packet);
        let _op = reader.read_short()?;

        reader.read_byte()?;
        let target = reader.read_int()?;
        let portal_name = reader.read_str_with_length()?;
        reader.read_byte()?; // Padding
        let _wheel_of_destiny = reader.read_short()? > 0;

        // TODO: When target == -1, look up portal by name in map data
        let _ = portal_name;

        let character = ctx.session.get_character()?;
        let mut chr = character.lock().unwrap();

        let mut result = HandlerResult::empty();

        if target != -1 {
            chr.character.map_id = target;
            chr.character.save()?;

            let warp_packet = build::world::map::build_warp_to_map(&chr.character, target)?;
            result = result.with_reply(warp_packet);
        }

        let stat_packet = build::world::map::build_empty_stat_update()?;
        Ok(result.with_reply(stat_packet))
    }
}
