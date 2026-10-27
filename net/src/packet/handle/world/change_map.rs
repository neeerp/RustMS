use crate::error::NetworkError;
use crate::handler::{
    HandlerResult, SpawnPosition, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::packet::build;
use crate::packet::build::world::map::BlockedMap;
use crate::packet::build::world::stat::StatUpdate;
use crate::policy;
use game::buff::BuffStat;
use game::Player;
use packet::{io::read::PktRead, Packet};
use std::convert::TryFrom;
use std::io::Cursor;

/// HP a character respawns with after dying.
const RESPAWN_HP: i16 = 50;

/// Revives a dead player on the map they died on.
const WHEEL_OF_DESTINY: i32 = 5510000;

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
        reader.read_byte()?;
        // The death dialog's answer to "use a Wheel of Destiny?".
        let wheel = reader.read_byte()? != 0;
        if target == -1 && held_at(ctx.player, &portal_name) {
            return Ok(HandlerResult::reply(build::world::stat::build_stat_update(
                true,
                &[],
            )?));
        }

        let game_data = crate::game_data::get()?;
        let player = &mut *ctx.player;

        let mut revived = Vec::new();
        let (map_id, spawn_point, spawn) = if target != -1 {
            let traps_send_to = game_data
                .field(player.stats.map_id)
                .map_or(&[][..], |field| field.traps.target_fields.as_slice());
            let (destination, spawn_point, replies) =
                direct_destination(player, target, wheel, traps_send_to, |map_id| {
                    game_data.field(map_id).and_then(|field| field.return_map)
                })?;
            if !game_data.field_exists(destination) {
                return Err(NetworkError::PacketHandlerError("Target field not found"));
            }
            revived = replies;
            (
                destination,
                spawn_point,
                crate::spawn_position(destination, spawn_point),
            )
        } else {
            let source_field = game_data
                .field(player.stats.map_id)
                .ok_or(NetworkError::PacketHandlerError("Current field not found"))?;
            let source_portal = source_field
                .portal_by_name(portal_name.as_str())
                .ok_or(NetworkError::PacketHandlerError("Source portal not found"))?;
            // A portal to a map or spawn portal this server lacks is closed,
            // and the client may try again, unless its link is known lost.
            let lost = crate::reactors::lost_link(player.stats.map_id, &portal_name);
            let Some((destination_map, spawn_portal)) = source_portal.to_map.and_then(|map_id| {
                let field = game_data.field(map_id)?;
                let to = if lost {
                    ""
                } else {
                    source_portal.to_name.as_str()
                };
                Some((map_id, field.resolve_spawn_portal(to)?))
            }) else {
                return Ok(HandlerResult::reply(build::world::map::build_blocked_map(
                    BlockedMap::PortalClosed,
                )?));
            };
            // Below the map's `lvLimit`: "You cannot go to that place."
            if !game_data
                .field(destination_map)
                .is_none_or(|field| crate::map_rules::may_walk_in(player, field))
            {
                return Ok(HandlerResult::reply(build::world::map::build_blocked_map(
                    BlockedMap::CannotGo,
                )?));
            }
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
        let mut result =
            HandlerResult::reply(warp_packet).with(WorldAction::ChangeField { map_id, spawn });
        for reply in revived {
            result = result.with_reply(reply);
        }
        Ok(result.with_reply(enable_actions))
    }
}

/// Whether a Stun or Seduce holds the player at `portal`. Their client
/// takes no input then, so only a forged packet enters a portal by the Up
/// key; but v83 still enters the touch portals a Seduce walks them into
/// (FUN_0094dac6 checks those every update).
pub fn held_at(player: &Player, portal: &str) -> bool {
    let held = player.buffs.has(BuffStat::Stun) || player.buffs.has(BuffStat::Seduce);
    held && !crate::game_data::get()
        .ok()
        .and_then(|data| data.field(player.stats.map_id))
        .and_then(|field| field.portal_by_name(portal))
        .is_some_and(|portal| by_touch(portal.portal_type))
}

/// Touch portals (3) and touch script portals (9): the client enters them
/// on contact.
fn by_touch(portal_type: i32) -> bool {
    matches!(portal_type, 3 | 9)
}

/// The portal a CHANGE_MAP packet walks through, or `None` for a map-id
/// warp, a revival, or leaving the Cash Shop. Lets the client actor refuse
/// a portal (a party quest stage's closed gate) before the handler runs.
pub fn portal_name(packet: &Packet) -> Option<String> {
    let mut reader = Cursor::new(&packet.bytes);
    reader.read_short().ok()?;
    reader.read_byte().ok()?;
    if reader.read_int().ok()? != -1 {
        return None;
    }
    reader.read_str_with_length().ok()
}

/// Send the player to `map_id`'s portal named `portal`, or its default
/// spawn point without one (script warps, Banish).
pub fn warp(
    player: &mut Player,
    map_id: i32,
    portal: &str,
    channel_id: u8,
) -> Result<Vec<WorldAction>, NetworkError> {
    let field = crate::game_data::get()?
        .field(map_id)
        .ok_or(NetworkError::PacketHandlerError("Target field not found"))?;
    let spawn_point = field
        .portal_by_name(portal)
        .map_or(0, |portal| u8::try_from(portal.id).unwrap_or(0));
    warp_to_spawn_point(player, map_id, spawn_point, channel_id)
}

/// Send the player to `map_id`'s portal `spawn_point`.
pub fn warp_to_spawn_point(
    player: &mut Player,
    map_id: i32,
    spawn_point: u8,
    channel_id: u8,
) -> Result<Vec<WorldAction>, NetworkError> {
    player.move_to(map_id, spawn_point);
    Ok(vec![
        WorldAction::Reply(build::world::map::build_warp_to_map(
            &player.stats,
            channel_id,
            None,
        )?),
        WorldAction::ChangeField {
            map_id,
            spawn: crate::spawn_position(map_id, spawn_point),
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

/// Where a direct (map id) warp may take the player: the map, its spawn
/// point, and what a revival tells them.
///
/// In v83 the client only sends a map id when a dead player respawns. A
/// Wheel of Destiny, if they chose to use one, keeps them on the map they
/// died on, at the spawn point nearest where they fell (Cosmic); otherwise
/// they go to its return map. A trap may send a live player to one of
/// `traps_send_to`, as the Witch Tower's snacks send them back to its
/// entrance (v83 0x94b767), and a scene shown on the map (`Effect::Scene`)
/// ends by sending them to the field it leads to. GMs (and debug builds, which the integration
/// harness uses as a teleport command) may warp anywhere.
fn direct_destination(
    player: &mut Player,
    target: i32,
    wheel: bool,
    traps_send_to: &[i32],
    return_map_of: impl Fn(i32) -> Option<i32>,
) -> Result<(i32, u8, Vec<Packet>), NetworkError> {
    if player.stats.hp <= 0 {
        let mut replies = Vec::new();
        let destination = if wheel && player.inventory.count(WHEEL_OF_DESTINY) > 0 {
            let changes = player
                .inventory_mut()
                .remove(WHEEL_OF_DESTINY, 1)
                .map_err(|_| NetworkError::PacketHandlerError("Wheel of Destiny not found"))?;
            let left = player.inventory.count(WHEEL_OF_DESTINY);
            replies.push(build::world::inventory::build_inventory_operation(
                true, &changes,
            )?);
            replies.push(build::world::effect::build_wheel_used(
                u8::try_from(left).unwrap_or(u8::MAX),
            )?);
            (player.stats.map_id, player.stats.spawn_point)
        } else {
            let map_id = return_map_of(player.stats.map_id).unwrap_or(player.stats.map_id);
            (map_id, 0)
        };
        replies.extend(revive(player)?);
        return Ok((destination.0, destination.1, replies));
    }
    // A scene's end asks for the field it leads to (`Effect::Scene`), and
    // gets its UI back there, which also starts that field's music.
    if player.scene_exit == Some(target) {
        let unlock = build::world::effect::build_lock_ui(false)?;
        return Ok((target, 0, vec![unlock]));
    }
    if policy::may_warp_anywhere(player) || traps_send_to.contains(&target) {
        return Ok((target, 0, Vec::new()));
    }
    Err(NetworkError::PacketHandlerError(
        "Direct map warp is not permitted",
    ))
}

/// Bring a dead player back with 50 HP, or with 30% of their max HP and MP
/// more after a charm kept their EXP (Cosmic).
fn revive(player: &mut Player) -> Result<Option<Packet>, NetworkError> {
    if !std::mem::take(&mut player.charmed) {
        player.stats.hp = RESPAWN_HP.min(player.stats.max_hp);
        player.mark_dirty();
        return Ok(None);
    }
    let share = |most: i32| (3 * most + 9) / 10;
    let hp = player.add_hp(share(player.effective_max_hp()));
    let mp = player.add_mp(share(player.effective_max_mp()));
    Ok(Some(build::world::stat::build_stat_update(
        false,
        &[StatUpdate::Hp(hp), StatUpdate::Mp(mp)],
    )?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::{CharacterStats, Inventory, InventoryType, Item, Keymap, QuestLog, SkillBook};
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
                gacha_exp: 0,
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
            assert!(direct_destination(&mut p, 180000000, false, &[], |_| None).is_err());
        }
    }

    #[test]
    fn traps_send_living_players_on() {
        let mut p = player(50, 0);
        let to =
            |p: &mut Player, target| direct_destination(p, target, false, &[980040000], |_| None);
        assert_eq!(to(&mut p, 980040000).unwrap().0, 980040000);
        if !policy::debug_warps_enabled() {
            assert!(to(&mut p, 180000000).is_err());
        }
    }

    #[test]
    fn a_scene_ends_in_the_field_it_leads_to_with_the_ui_back() {
        let mut p = player(50, 0);
        assert!(direct_destination(&mut p, 106020000, false, &[], |_| None).is_err());
        p.scene_exit = Some(106020000);
        let (map, _, replies) =
            direct_destination(&mut p, 106020000, false, &[], |_| None).unwrap();
        assert_eq!(map, 106020000);
        let unlock = build::world::effect::build_lock_ui(false).unwrap();
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0].bytes, unlock.bytes);
        assert!(direct_destination(&mut p, 100000000, false, &[], |_| None).is_err());
    }

    #[test]
    fn gms_can_warp_by_map_id() {
        let mut p = player(50, 1);
        assert_eq!(
            direct_destination(&mut p, 180000000, false, &[], |_| None)
                .unwrap()
                .0,
            180000000
        );
    }

    #[test]
    fn dead_players_respawn_in_the_return_map_regardless_of_target() {
        let mut p = player(0, 0);
        let (destination, spawn_point, replies) =
            direct_destination(&mut p, 180000000, false, &[], |_| Some(100000000)).unwrap();
        assert_eq!((destination, spawn_point), (100000000, 0));
        assert!(replies.is_empty());
        assert_eq!(p.stats.hp, RESPAWN_HP);
        assert!(p.is_dirty());
    }

    #[test]
    fn a_wheel_of_destiny_revives_where_the_player_fell() {
        let mut p = player(0, 0);
        p.stats.map_id = 104000100;
        p.stats.spawn_point = 3;
        p.inventory
            .insert(InventoryType::Cash, 1, Item::stack(WHEEL_OF_DESTINY, 2));
        let (map_id, spawn_point, replies) =
            direct_destination(&mut p, 104000100, true, &[], |_| Some(104000000)).unwrap();
        assert_eq!((map_id, spawn_point), (104000100, 3));
        assert_eq!(p.inventory.count(WHEEL_OF_DESTINY), 1);
        assert_eq!(p.stats.hp, RESPAWN_HP);
        // The inventory change, then the chat line with one wheel left.
        assert_eq!(replies.len(), 2);
        assert_eq!(&replies[1].bytes[2..], &[0x15, 1]);
    }

    #[test]
    fn a_declined_wheel_is_kept() {
        let mut p = player(0, 0);
        p.inventory
            .insert(InventoryType::Cash, 1, Item::stack(WHEEL_OF_DESTINY, 1));
        let (map_id, _, replies) =
            direct_destination(&mut p, 100000000, false, &[], |_| Some(104000000)).unwrap();
        assert_eq!(map_id, 104000000);
        assert!(replies.is_empty());
        assert_eq!(p.inventory.count(WHEEL_OF_DESTINY), 1);
        // Asking for a wheel without one goes to the return map too.
        let mut p = player(0, 0);
        let (map_id, _, _) =
            direct_destination(&mut p, 100000000, true, &[], |_| Some(104000000)).unwrap();
        assert_eq!(map_id, 104000000);
    }

    #[test]
    fn a_charm_revives_with_thirty_percent_of_max_hp_and_mp() {
        let mut p = player(0, 0);
        p.stats.mp = 2;
        p.charmed = true;
        let (_, _, replies) =
            direct_destination(&mut p, 100000000, false, &[], |_| Some(104000000)).unwrap();
        assert_eq!((p.stats.hp, p.stats.mp), (30, 5));
        assert!(!p.charmed);
        assert_eq!(replies.len(), 1);
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

    fn change_map(target: i32, portal: &str) -> Packet {
        let mut p = Packet::new_empty();
        p.write_short(0x26).unwrap();
        p.write_byte(1).unwrap();
        p.write_int(target).unwrap();
        p.write_str_with_length(portal).unwrap();
        p.write_byte(0).unwrap();
        p.write_byte(0).unwrap();
        p
    }

    #[test]
    fn a_held_player_takes_no_portal() {
        let now = std::time::Instant::now();
        let long = std::time::Duration::from_secs(60);
        for stat in [BuffStat::Stun, BuffStat::Seduce] {
            let mut p = player(50, 0);
            p.buffs.give(game::Buff::new(0, stat, 1, now, long));
            let mut ctx = WorldContext {
                client_id: 1,
                channel_id: 0,
                player: &mut p,
            };
            let result = ChangeMapHandler
                .handle(&mut change_map(-1, "next00"), &mut ctx)
                .unwrap();
            // Only the update that frees the client comes back.
            assert!(matches!(result.actions.as_slice(), [WorldAction::Reply(_)]));
            assert_eq!(p.stats.map_id, 100000000);
        }
    }

    #[test]
    fn a_held_player_still_takes_touch_portals() {
        assert!(by_touch(3) && by_touch(9));
        assert!(!by_touch(2) && !by_touch(7) && !by_touch(10));
    }

    #[test]
    fn portal_name_is_the_portal_walked_through() {
        assert_eq!(
            portal_name(&change_map(-1, "next00")).as_deref(),
            Some("next00")
        );
        assert_eq!(portal_name(&change_map(103000801, "")), None);
        let mut cash_shop_exit = Packet::new_empty();
        cash_shop_exit.write_short(0x26).unwrap();
        assert_eq!(portal_name(&cash_shop_exit), None);
    }
}
