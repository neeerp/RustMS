use crate::error::NetworkError;
use crate::handler::{HandlerContext, HandlerResult, PacketHandler};
use crate::packet::build;
use packet::{io::read::PktRead, Packet};
use std::convert::TryFrom;
use std::io::BufReader;

pub struct ChangeMapHandler;

impl ChangeMapHandler {
    pub fn new() -> Self {
        Self
    }
}

impl PacketHandler for ChangeMapHandler {
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

        let character = ctx.session.get_character()?;
        let mut chr = character.lock().unwrap();

        let mut result = HandlerResult::empty();
        let old_map_id = chr.character.map_id;

        if target != -1 {
            let game_data = crate::game_data::get()?;
            if !game_data.field_exists(target) {
                return Err(NetworkError::PacketHandlerError("Target field not found"));
            }

            chr.character.map_id = target;
            chr.character.save()?;

            let warp_packet = build::world::map::build_warp_to_map(&chr.character, target, 0)?;
            result = result
                .with_reply(warp_packet)
                .with_map_changed(old_map_id, target);
        } else {
            let game_data = crate::game_data::get()?;
            let source_field = game_data
                .field(old_map_id)
                .ok_or(NetworkError::PacketHandlerError("Current field not found"))?;
            let source_portal = source_field
                .portal_by_name(portal_name.as_str())
                .ok_or(NetworkError::PacketHandlerError("Source portal not found"))?;
            let destination_map = source_portal
                .to_map
                .ok_or(NetworkError::PacketHandlerError(
                    "Portal has no destination",
                ))?;
            let destination_field =
                game_data
                    .field(destination_map)
                    .ok_or(NetworkError::PacketHandlerError(
                        "Destination field not found",
                    ))?;
            let spawn_portal = destination_field
                .resolve_spawn_portal(source_portal.to_name.as_str())
                .ok_or(NetworkError::PacketHandlerError(
                    "Destination spawn portal not found",
                ))?;
            let spawn_point = u8::try_from(spawn_portal.id).map_err(|_| {
                NetworkError::PacketHandlerError("Destination spawn portal id out of range")
            })?;

            chr.character.map_id = destination_map;
            chr.character.save()?;

            let warp_packet =
                build::world::map::build_warp_to_map(&chr.character, destination_map, spawn_point)?;
            result = result
                .with_reply(warp_packet)
                .with_map_changed(old_map_id, destination_map);
        }

        let stat_packet = build::world::map::build_empty_stat_update()?;
        Ok(result.with_reply(stat_packet))
    }
}
