//! Teleport rocks (504xxxx, report 179, 1.8 and 4): the maps kept for them,
//! edited with TROCK_ADD_MAP (0x66), and the rock itself, used through
//! USE_CASH_ITEM to go to a kept map or to a character on the channel. A
//! regular rock keeps five maps and stays on its continent; the VIP rock
//! keeps ten and crosses continents. Each use takes one rock. The USE-tab
//! rock (2320000) is a regular one, used through USE_TELEPORT_ROCK.

use super::change_map::{warp, warp_to_spawn_point};
use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::packet::build::world::inventory::build_inventory_operation;
use crate::packet::build::world::map::{build_rock_list, build_rock_refusal, RockRefusal};
use crate::packet::build::world::stat::build_stat_update;
use game::teleport_rock::{continent, is_vip, may_keep};
use game::{InventoryType, Player};
use game_data::{FIELD_LIMIT_MOVEMENT_SKILLS, FIELD_LIMIT_TELEPORT_ROCK};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// The level a character must reach to leave Maple Island by rock.
pub const MIN_LEVEL_OFF_ISLAND: u8 = 7;

/// TROCK_ADD_MAP (0x66, v83 FUN_00a261bc): keep the player's map in a list,
/// or drop one from it. The answer is the new list.
pub struct TeleportRockListHandler;

impl WorldHandler for TeleportRockListHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let keep = r.read_byte()? != 0;
        let vip = r.read_byte()? != 0;
        let player = &mut *ctx.player;
        if keep {
            let map_id = player.stats.map_id;
            if !may_keep(map_id)
                || limits(map_id) & FIELD_LIMIT_TELEPORT_ROCK != 0
                || !player.teleport_rocks.keep(vip, map_id)
            {
                return Ok(HandlerResult::reply(build_rock_refusal(
                    RockRefusal::NotForList,
                    vip,
                )?));
            }
        } else {
            let map_id = r.read_int()?;
            player.teleport_rocks.remove(vip, map_id);
        }
        player.mark_dirty();
        Ok(HandlerResult::reply(build_rock_list(
            !keep,
            vip,
            &player.teleport_rocks,
        )?))
    }
}

/// USE_TELEPORT_ROCK (0x54, v83 FUN_00a0a3bb): the USE-tab rock (232xxxx),
/// sent as a cash rock is, from its USE slot.
pub struct UseTeleportRockHandler;

impl WorldHandler for UseTeleportRockHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let slot = r.read_short()?;
        let item_id = r.read_int()?;
        let held = ctx
            .player
            .inventory
            .get(InventoryType::Use, slot)
            .is_some_and(|item| item.item_id == item_id);
        if !held || item_id / 10_000 != 232 {
            return Ok(HandlerResult::reply(build_stat_update(true, &[])?));
        }
        teleport_rock(ctx, &mut r, slot, item_id)
    }
}

/// A teleport rock in use, while the server looks for the character it
/// was used to reach.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rock {
    pub slot: i16,
    pub item_id: i32,
}

/// Where the character a rock looks for stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Whereabouts {
    pub map_id: i32,
    pub x: i16,
    pub y: i16,
}

/// Use the teleport rock `item_id` in `slot` of its bag; `r` is past the
/// item id.
/// A character target is looked for first (`WorldAction::RockToCharacter`).
pub fn teleport_rock(
    ctx: &mut WorldContext,
    r: &mut Cursor<&Vec<u8>>,
    slot: i16,
    item_id: i32,
) -> WorldResult {
    let vip = is_vip(item_id);
    let rock = Rock { slot, item_id };
    if r.read_byte()? != 0 {
        let name = r.read_str_with_length()?;
        if name.is_empty() || name.eq_ignore_ascii_case(ctx.player.name()) {
            return refuse(RockRefusal::NotFound, vip);
        }
        return Ok(HandlerResult::empty().with(WorldAction::RockToCharacter { name, rock }));
    }
    let map_id = r.read_int()?;
    if !ctx.player.teleport_rocks.maps(vip).contains(&map_id) {
        return refuse(RockRefusal::CannotGo, vip);
    }
    go(ctx.player, rock, map_id, None, ctx.channel_id)
}

/// Take the player to the character the rock looked for, found at `found`
/// or not at all.
pub fn to_character(
    player: &mut Player,
    rock: Rock,
    found: Option<Whereabouts>,
    channel_id: u8,
) -> WorldResult {
    let vip = is_vip(rock.item_id);
    let Some(found) = found else {
        return refuse(RockRefusal::NotFound, vip);
    };
    let near = crate::game_data::closest_spawn_point(found.map_id, found.x, found.y);
    go(
        player,
        rock,
        found.map_id,
        Some(near.unwrap_or(0)),
        channel_id,
    )
}

/// Take the player to `map_id` with the rock, at `spawn_point` (the map's
/// first spawn point without one), unless something forbids it.
fn go(
    player: &mut Player,
    rock: Rock,
    map_id: i32,
    spawn_point: Option<u8>,
    channel_id: u8,
) -> WorldResult {
    let vip = is_vip(rock.item_id);
    if let Some(why) = refusal(player, vip, map_id) {
        return refuse(why, vip);
    }
    let bag = InventoryType::for_item(rock.item_id).unwrap_or(InventoryType::Cash);
    let held = player
        .inventory
        .get(bag, rock.slot)
        .is_some_and(|item| item.item_id == rock.item_id);
    if !held {
        return Ok(HandlerResult::reply(build_stat_update(true, &[])?));
    }
    let Ok(changes) = player.inventory_mut().take(bag, rock.slot, 1) else {
        return Ok(HandlerResult::reply(build_stat_update(true, &[])?));
    };
    let mut result = HandlerResult::reply(build_inventory_operation(true, &changes)?);
    let actions = match spawn_point {
        Some(spawn_point) => warp_to_spawn_point(player, map_id, spawn_point, channel_id)?,
        None => warp(player, map_id, "sp", channel_id)?,
    };
    result.actions.extend(actions);
    Ok(result)
}

/// Why the player can't go to `map_id` by rock, if they can't.
fn refusal(player: &Player, vip: bool, map_id: i32) -> Option<RockRefusal> {
    let here = player.stats.map_id;
    let Some(there) = crate::game_data::get()
        .ok()
        .and_then(|data| data.field(map_id))
    else {
        return Some(RockRefusal::CannotGo);
    };
    // Maps that send the player elsewhere on login, but Maple Island's.
    let held_back = there.forced_return.is_some() && continent(map_id) != 0;
    if map_id == here {
        Some(RockRefusal::SameMap)
    } else if player.stats.hp <= 0
        || limits(here) & (FIELD_LIMIT_TELEPORT_ROCK | FIELD_LIMIT_MOVEMENT_SKILLS) != 0
    {
        Some(RockRefusal::CannotGo)
    } else if continent(here) == 0 && player.stats.level < MIN_LEVEL_OFF_ISLAND {
        Some(RockRefusal::MapleIsland)
    } else if (!vip && continent(map_id) != continent(here))
        || there.field_limit & FIELD_LIMIT_TELEPORT_ROCK != 0
        || held_back
    {
        Some(RockRefusal::CannotGo)
    } else {
        None
    }
}

/// The map's `info/fieldLimit`.
fn limits(map_id: i32) -> i32 {
    crate::game_data::get()
        .ok()
        .and_then(|data| data.field(map_id))
        .map_or(0, |field| field.field_limit)
}

/// MAP_TRANSFER_RESULT doesn't unlock the client, so a refusal comes with
/// an empty stat change.
fn refuse(why: RockRefusal, vip: bool) -> WorldResult {
    Ok(HandlerResult::reply(build_rock_refusal(why, vip)?)
        .with_reply(build_stat_update(true, &[])?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::handle::world::UseCashItemHandler;
    use crate::packet::op::{RecvOpcode, SendOpcode};
    use game::teleport_rock::EMPTY;
    use game::Item;
    use packet::io::write::PktWrite;

    const ROCK: i32 = 5040000;
    const VIP_ROCK: i32 = 5041000;
    const HENESYS: i32 = 100000000;
    const ELLINIA: i32 = 101000000;
    const ORBIS: i32 = 200000000;
    const AMHERST: i32 = 1000000;

    /// A player in Henesys holding three regular rocks in Cash slot 1 and a
    /// VIP rock in slot 2.
    fn traveller() -> Player {
        let mut player = crate::codec::stats::sample_player();
        let cash = &mut player.inventory;
        cash.insert(InventoryType::Cash, 1, Item::stack(ROCK, 3));
        cash.insert(InventoryType::Cash, 2, Item::stack(VIP_ROCK, 1));
        player
    }

    fn handle(handler: &dyn WorldHandler, player: &mut Player, p: &mut Packet) -> Vec<WorldAction> {
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        handler.handle(p, &mut ctx).unwrap().actions
    }

    fn edit_list(player: &mut Player, vip: bool, dropped: Option<i32>) -> Vec<WorldAction> {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::TeleportRockList as i16).unwrap();
        p.write_byte(u8::from(dropped.is_none())).unwrap();
        p.write_byte(u8::from(vip)).unwrap();
        if let Some(map_id) = dropped {
            p.write_int(map_id).unwrap();
        }
        handle(&TeleportRockListHandler, player, &mut p)
    }

    fn use_rock(player: &mut Player, item_id: i32, to: Result<i32, &str>) -> Vec<WorldAction> {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
        p.write_short(if is_vip(item_id) { 2 } else { 1 }).unwrap();
        p.write_int(item_id).unwrap();
        match to {
            Ok(map_id) => {
                p.write_byte(0).unwrap();
                p.write_int(map_id).unwrap();
            }
            Err(name) => {
                p.write_byte(1).unwrap();
                p.write_str_with_length(name).unwrap();
            }
        }
        p.write_int(1234).unwrap();
        handle(&UseCashItemHandler, player, &mut p)
    }

    fn opcode(packet: &Packet) -> i16 {
        i16::from_le_bytes([packet.bytes[0], packet.bytes[1]])
    }

    /// The MAP_TRANSFER_RESULT refusal `why` and the unlock after it.
    fn assert_refused(actions: &[WorldAction], why: RockRefusal, vip: bool) {
        let [WorldAction::Reply(refusal), WorldAction::Reply(unlock)] = actions else {
            panic!("expected a refusal and an unlock: {actions:?}");
        };
        assert_eq!(refusal.bytes, [0x2A, 0, why as u8, u8::from(vip)]);
        assert_eq!(opcode(unlock), SendOpcode::StatChange as i16);
    }

    #[test]
    fn the_players_map_is_kept_and_dropped() {
        let mut player = traveller();
        let actions = edit_list(&mut player, false, None);
        let [WorldAction::Reply(list)] = &actions[..] else {
            panic!("expected the new list: {actions:?}");
        };
        assert_eq!(list.bytes[..8], [0x2A, 0, 3, 0, 0x00, 0xE1, 0xF5, 0x05]);
        assert_eq!(player.teleport_rocks.maps(false), [HENESYS]);
        assert!(player.is_dirty());

        let actions = edit_list(&mut player, false, None);
        let [WorldAction::Reply(refusal)] = &actions[..] else {
            panic!("expected a refusal: {actions:?}");
        };
        assert_eq!(refusal.bytes, [0x2A, 0, 10, 0], "kept already");

        player.stats.map_id = AMHERST;
        let actions = edit_list(&mut player, true, None);
        let [WorldAction::Reply(refusal)] = &actions[..] else {
            panic!("expected a refusal: {actions:?}");
        };
        assert_eq!(refusal.bytes, [0x2A, 0, 10, 1], "not Maple Island");

        let actions = edit_list(&mut player, false, Some(HENESYS));
        let [WorldAction::Reply(list)] = &actions[..] else {
            panic!("expected the new list: {actions:?}");
        };
        assert_eq!(list.bytes[..4], [0x2A, 0, 2, 0]);
        assert_eq!(list.bytes[4..8], EMPTY.to_le_bytes());
        assert!(player.teleport_rocks.maps(false).is_empty());
    }

    #[test]
    fn a_rock_goes_only_to_a_kept_map() {
        let mut player = traveller();
        let actions = use_rock(&mut player, ROCK, Ok(ELLINIA));
        assert_refused(&actions, RockRefusal::CannotGo, false);
        assert_eq!(player.inventory.count(ROCK), 3, "not used");
    }

    #[test]
    fn a_rock_takes_the_player_to_a_kept_map() {
        if crate::get_game_data().is_err() {
            return;
        }
        let mut player = traveller();
        player.teleport_rocks.keep(false, ELLINIA);
        let actions = use_rock(&mut player, ROCK, Ok(ELLINIA));
        let [WorldAction::Reply(taken), WorldAction::Reply(_), WorldAction::ChangeField { map_id, .. }] =
            &actions[..]
        else {
            panic!("expected the rock taken and the warp: {actions:?}");
        };
        assert_eq!(opcode(taken), SendOpcode::InventoryOperation as i16);
        assert_eq!(*map_id, ELLINIA);
        assert_eq!(player.stats.map_id, ELLINIA);
        assert_eq!(player.inventory.count(ROCK), 2);
    }

    #[test]
    fn a_rock_is_refused_where_it_cannot_go() {
        if crate::get_game_data().is_err() {
            return;
        }
        let mut player = traveller();
        player.teleport_rocks.keep(false, HENESYS);
        player.teleport_rocks.keep(false, ORBIS);
        player.teleport_rocks.keep(true, ORBIS);
        let same_map = use_rock(&mut player, ROCK, Ok(HENESYS));
        assert_refused(&same_map, RockRefusal::SameMap, false);
        let overseas = use_rock(&mut player, ROCK, Ok(ORBIS));
        assert_refused(&overseas, RockRefusal::CannotGo, false);
        player.stats.hp = 0;
        let dead = use_rock(&mut player, VIP_ROCK, Ok(ORBIS));
        assert_refused(&dead, RockRefusal::CannotGo, true);
        player.stats.hp = 100;
        player.stats.map_id = AMHERST;
        player.stats.level = MIN_LEVEL_OFF_ISLAND - 1;
        let young = use_rock(&mut player, VIP_ROCK, Ok(ORBIS));
        assert_refused(&young, RockRefusal::MapleIsland, true);
        assert_eq!(player.inventory.count(ROCK), 3, "not used");
        assert_eq!(player.inventory.count(VIP_ROCK), 1, "not used");

        player.stats.level = MIN_LEVEL_OFF_ISLAND;
        use_rock(&mut player, VIP_ROCK, Ok(ORBIS));
        assert_eq!(player.stats.map_id, ORBIS, "the VIP rock crosses the sea");
        assert_eq!(player.inventory.count(VIP_ROCK), 0);
    }

    #[test]
    fn the_use_tab_rock_is_a_regular_one_taken_from_its_use_slot() {
        if crate::get_game_data().is_err() {
            return;
        }
        const USE_ROCK: i32 = 2_320_000;
        let mut player = traveller();
        player
            .inventory
            .insert(InventoryType::Use, 1, Item::stack(USE_ROCK, 1));
        player.teleport_rocks.keep(true, ORBIS);
        player.teleport_rocks.keep(false, ELLINIA);
        let use_rock = |player: &mut Player, slot: i16, map_id: i32| {
            let mut p = Packet::new_empty();
            p.write_short(RecvOpcode::UseTeleportRock as i16).unwrap();
            p.write_short(slot).unwrap();
            p.write_int(USE_ROCK).unwrap();
            p.write_byte(0).unwrap();
            p.write_int(map_id).unwrap();
            p.write_int(1234).unwrap();
            handle(&UseTeleportRockHandler, player, &mut p)
        };

        let [WorldAction::Reply(unlock)] = &use_rock(&mut player, 2, ELLINIA)[..] else {
            panic!("expected only an unlock for an empty slot");
        };
        assert_eq!(opcode(unlock), SendOpcode::StatChange as i16);
        let overseas = use_rock(&mut player, 1, ORBIS);
        assert_refused(&overseas, RockRefusal::CannotGo, false);
        assert_eq!(player.inventory.count(USE_ROCK), 1, "not used");

        use_rock(&mut player, 1, ELLINIA);
        assert_eq!(player.stats.map_id, ELLINIA);
        assert_eq!(player.inventory.count(USE_ROCK), 0);
        assert_eq!(player.inventory.count(ROCK), 3, "the cash rocks are kept");
    }

    #[test]
    fn a_rock_looks_for_a_character_by_name() {
        let mut player = traveller();
        let actions = use_rock(&mut player, VIP_ROCK, Err("Friend"));
        let [WorldAction::RockToCharacter { name, rock }] = &actions[..] else {
            panic!("expected a search for the character: {actions:?}");
        };
        assert_eq!(name, "Friend");
        assert_eq!(
            *rock,
            Rock {
                slot: 2,
                item_id: VIP_ROCK
            }
        );
        let own = use_rock(&mut player, VIP_ROCK, Err("tester"));
        assert_refused(&own, RockRefusal::NotFound, true);
        let lost = to_character(&mut player, rock.clone(), None, 0).unwrap();
        assert_refused(&lost.actions, RockRefusal::NotFound, true);
        assert_eq!(player.inventory.count(VIP_ROCK), 1, "not used");
    }

    #[test]
    fn a_rock_takes_the_player_to_a_character() {
        if crate::get_game_data().is_err() {
            return;
        }
        let mut player = traveller();
        let rock = Rock {
            slot: 1,
            item_id: ROCK,
        };
        let found = Whereabouts {
            map_id: ELLINIA,
            x: 0,
            y: 0,
        };
        to_character(&mut player, rock, Some(found), 0).unwrap();
        assert_eq!(player.stats.map_id, ELLINIA);
        assert_eq!(player.inventory.count(ROCK), 2);
    }
}
