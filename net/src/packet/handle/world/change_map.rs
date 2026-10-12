use crate::error::NetworkError;
use crate::handler::{
    HandlerResult, SpawnPosition, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::packet::build;
use crate::policy;
use game::Player;
use packet::{io::read::PktRead, Packet};
use std::convert::TryFrom;
use std::io::Cursor;

/// HP a character respawns with after dying.
const RESPAWN_HP: i16 = 50;

pub struct ChangeMapHandler;

impl WorldHandler for ChangeMapHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        reader.read_byte()?; // 1 when returning from the cash shop
        let target = reader.read_int()?;
        let portal_name = reader.read_str_with_length()?;

        let game_data = crate::game_data::get()?;
        let player = &mut *ctx.player;

        let (map_id, spawn_point, spawn) = if target != -1 {
            let destination = direct_destination(player, target, |map_id| {
                game_data.field(map_id).and_then(|field| field.return_map)
            })?;
            if !game_data.field_exists(destination) {
                return Err(NetworkError::PacketHandlerError("Target field not found"));
            }
            (destination, 0, crate::spawn_position(destination, 0))
        } else {
            let source_field = game_data
                .field(player.stats.map_id)
                .ok_or(NetworkError::PacketHandlerError("Current field not found"))?;
            let source_portal = source_field
                .portal_by_name(portal_name.as_str())
                .ok_or(NetworkError::PacketHandlerError("Source portal not found"))?;
            let destination_map = source_portal
                .to_map
                .ok_or(NetworkError::PacketHandlerError("Portal has no destination"))?;
            let destination_field = game_data
                .field(destination_map)
                .ok_or(NetworkError::PacketHandlerError("Destination field not found"))?;
            let spawn_portal = destination_field
                .resolve_spawn_portal(source_portal.to_name.as_str())
                .ok_or(NetworkError::PacketHandlerError(
                    "Destination spawn portal not found",
                ))?;
            let spawn_point = u8::try_from(spawn_portal.id).map_err(|_| {
                NetworkError::PacketHandlerError("Destination spawn portal id out of range")
            })?;
            let spawn = SpawnPosition {
                x: i16::try_from(spawn_portal.x).map_err(|_| {
                    NetworkError::PacketHandlerError("Destination spawn portal x out of range")
                })?,
                y: i16::try_from(spawn_portal.y).map_err(|_| {
                    NetworkError::PacketHandlerError("Destination spawn portal y out of range")
                })?,
                stance: 2,
            };
            (destination_map, spawn_point, Some(spawn))
        };

        player.move_to(map_id, spawn_point);
        let warp_packet = build::world::map::build_warp_to_map(&player.stats, ctx.channel_id)?;
        let enable_actions = build::world::stat::build_stat_update(true, &[])?;
        Ok(HandlerResult::reply(warp_packet)
            .with(WorldAction::ChangeField { map_id, spawn })
            .with_reply(enable_actions))
    }
}

/// Where a direct (map id) warp may take the player.
///
/// In v83 the client only sends a map id when a dead player respawns, which
/// always goes to the current map's return map. GMs (and debug builds, which
/// the integration harness uses as a teleport command) may warp anywhere.
fn direct_destination(
    player: &mut Player,
    target: i32,
    return_map_of: impl Fn(i32) -> Option<i32>,
) -> Result<i32, NetworkError> {
    if player.stats.hp <= 0 {
        let destination = return_map_of(player.stats.map_id).unwrap_or(player.stats.map_id);
        player.stats.hp = RESPAWN_HP.min(player.stats.max_hp);
        player.mark_dirty();
        return Ok(destination);
    }
    if policy::may_warp_anywhere(player) {
        return Ok(target);
    }
    Err(NetworkError::PacketHandlerError(
        "Direct map warp is not permitted",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::{CharacterStats, Inventory, Keymap, QuestLog, SkillBook};

    fn player(hp: i16, gm_level: i16) -> Player {
        let mut player = Player::new(
            1,
            0,
            CharacterStats {
                id: 1,
                name: "warper".into(),
                gender: 0,
                skin: 0,
                face: 20000,
                hair: 30000,
                level: 10,
                job: 0,
                str: 4,
                dex: 4,
                int: 4,
                luk: 4,
                hp,
                max_hp: 100,
                mp: 10,
                max_mp: 10,
                ap: 0,
                sp: 0,
                exp: 0,
                fame: 0,
                map_id: 100000000,
                spawn_point: 0,
            },
            0,
            Inventory::default(),
            SkillBook::default(),
            Keymap::default(),
            QuestLog::default(),
        );
        player.gm_level = gm_level;
        player
    }

    #[test]
    fn living_players_cannot_warp_by_map_id() {
        let mut p = player(50, 0);
        if !policy::debug_warps_enabled() {
            assert!(direct_destination(&mut p, 180000000, |_| None).is_err());
        }
    }

    #[test]
    fn gms_can_warp_by_map_id() {
        let mut p = player(50, 1);
        assert_eq!(direct_destination(&mut p, 180000000, |_| None).unwrap(), 180000000);
    }

    #[test]
    fn dead_players_respawn_in_the_return_map_regardless_of_target() {
        let mut p = player(0, 0);
        let destination = direct_destination(&mut p, 180000000, |_| Some(100000000)).unwrap();
        assert_eq!(destination, 100000000);
        assert_eq!(p.stats.hp, RESPAWN_HP);
        assert!(p.is_dirty());
    }
}
