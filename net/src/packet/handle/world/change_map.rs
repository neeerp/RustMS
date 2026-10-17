use crate::error::NetworkError;
use crate::handler::{
    HandlerResult, SpawnPosition, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::packet::build;
use crate::packet::build::world::map::BlockedMap;
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
        // Leaving the Cash Shop sends the opcode alone.
        if packet.bytes.len() == 2 {
            return Ok(HandlerResult::empty().with(WorldAction::LeaveCashShop));
        }
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        reader.read_byte()?; // the key SET_FIELD gave the field
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
            // A portal to a map or spawn portal this server lacks is closed,
            // and the client may try again.
            let Some((destination_map, spawn_portal)) = source_portal.to_map.and_then(|map_id| {
                let field = game_data.field(map_id)?;
                Some((
                    map_id,
                    field.resolve_spawn_portal(source_portal.to_name.as_str())?,
                ))
            }) else {
                return Ok(HandlerResult::reply(build::world::map::build_blocked_map(
                    BlockedMap::PortalClosed,
                )?));
            };
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
        let warp_packet =
            build::world::map::build_warp_to_map(&player.stats, ctx.channel_id, None)?;
        let enable_actions = build::world::stat::build_stat_update(true, &[])?;
        Ok(HandlerResult::reply(warp_packet)
            .with(WorldAction::ChangeField { map_id, spawn })
            .with_reply(enable_actions))
    }
}

/// Send the player to `map_id`'s default spawn point (script warps).
pub fn warp(
    player: &mut Player,
    map_id: i32,
    channel_id: u8,
) -> Result<Vec<WorldAction>, NetworkError> {
    if !crate::game_data::get()?.field_exists(map_id) {
        return Err(NetworkError::PacketHandlerError("Target field not found"));
    }
    player.move_to(map_id, 0);
    Ok(vec![
        WorldAction::Reply(build::world::map::build_warp_to_map(
            &player.stats,
            channel_id,
            None,
        )?),
        WorldAction::ChangeField {
            map_id,
            spawn: crate::spawn_position(map_id, 0),
        },
    ])
}

/// Send the player through a Mystic Door to `map_id`: into town at the door
/// portal of the owner's party slot, or back to the door in its field.
pub fn door_warp(
    player: &mut Player,
    map_id: i32,
    to: DoorExit,
    channel_id: u8,
) -> Result<Vec<WorldAction>, NetworkError> {
    let (spawn_point, chase, spawn) = match to {
        DoorExit::Town { slot } => {
            // The client draws the door at its town's `slot`th door portal
            // (0x5377fa), any portal if the town has too few.
            let field = crate::game_data::get()?
                .field(map_id)
                .ok_or(NetworkError::PacketHandlerError("Door town not found"))?;
            let portal = field
                .door_portals()
                .nth(slot)
                .or_else(|| field.door_portals().next())
                .map_or(0, |portal| u8::try_from(portal.id).unwrap_or(0));
            (portal, None, crate::spawn_position(map_id, portal))
        }
        DoorExit::Field { x, y } => (
            crate::game_data::closest_spawn_point(map_id, x, y).unwrap_or(0),
            Some((x, y)),
            Some(SpawnPosition { x, y, stance: 2 }),
        ),
    };
    player.move_to(map_id, spawn_point);
    Ok(vec![
        WorldAction::Reply(build::world::map::build_warp_to_map(
            &player.stats,
            channel_id,
            chase,
        )?),
        WorldAction::ChangeField { map_id, spawn },
    ])
}

/// Where a Mystic Door lets the player out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DoorExit {
    /// In town, at the door of the owner's party slot (0 alone).
    Town { slot: usize },
    /// In the field, at the door.
    Field { x: i16, y: i16 },
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
    use packet::io::write::PktWrite;

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
        assert_eq!(
            direct_destination(&mut p, 180000000, |_| None).unwrap(),
            180000000
        );
    }

    #[test]
    fn dead_players_respawn_in_the_return_map_regardless_of_target() {
        let mut p = player(0, 0);
        let destination = direct_destination(&mut p, 180000000, |_| Some(100000000)).unwrap();
        assert_eq!(destination, 100000000);
        assert_eq!(p.stats.hp, RESPAWN_HP);
        assert!(p.is_dirty());
    }

    #[test]
    fn an_empty_change_map_leaves_the_cash_shop() {
        let mut p = player(50, 0);
        let mut packet = Packet::new_empty();
        packet
            .write_short(crate::packet::op::RecvOpcode::ChangeMap as i16)
            .unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut p,
        };
        let result = ChangeMapHandler.handle(&mut packet, &mut ctx).unwrap();
        assert!(matches!(
            result.actions.as_slice(),
            [WorldAction::LeaveCashShop]
        ));
        assert_eq!(p.stats.map_id, 100000000);
    }
}
