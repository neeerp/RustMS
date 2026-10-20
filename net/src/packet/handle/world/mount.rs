//! Mounts (report 201): getting off one, and the Revitalizer. Getting on
//! is a skill, in `use_skill`.

use crate::handler::{
    FieldRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::items;
use crate::packet::build::world::buff;
use crate::packet::build::world::inventory::build_inventory_operation;
use crate::packet::build::world::stat::build_stat_update;
use game::{BuffStat, InventoryChange, InventoryType, Player};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// End the player's ride, if they're riding. The client waits for a
/// CANCEL_BUFF with riding's bit after asking to get on or off (v83
/// FUN_0095f065), so it gets one either way.
pub fn dismount(player: &mut Player, result: HandlerResult<WorldAction>) -> WorldResult {
    let result = result.with_reply(buff::build_cancel_buff(&[BuffStat::Riding])?);
    let Some(skill_id) = player.ride_skill() else {
        return Ok(result);
    };
    player.buffs.cancel(skill_id);
    let foreign = buff::build_cancel_foreign_buff(player.id(), &[BuffStat::Riding])?;
    Ok(result.with(WorldAction::BuffsChanged(foreign)))
}

/// USE_MOUNT_FOOD (0x4D, FUN_00a09a64): the tick, the USE slot and the
/// food (226xxxx), which the client sends only while the player rides a
/// 190xxxx mount. It takes tiredness off the mount, earning it EXP; the map
/// sees the mount's new state. The food is used up even if the mount wasn't
/// tired (Cosmic's UseMountFoodHandler).
pub struct MountFoodHandler;

impl WorldHandler for MountFoodHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        r.read_int()?;
        let slot = r.read_short()?;
        let item_id = r.read_int()?;
        let relief = items::info(item_id)
            .and_then(|info| info.spec)
            .map_or(0, |spec| -spec.inc_fatigue);
        let Some(fed) = feed_mount(ctx.player, slot, item_id, relief) else {
            return Ok(HandlerResult::reply(build_stat_update(true, &[])?));
        };
        let result = HandlerResult::reply(build_inventory_operation(true, &fed.changes)?);
        if fed.taken == 0 {
            return Ok(result);
        }
        let info = buff::build_taming_mob_info(ctx.player.id(), &ctx.player.mount, fed.level_up)?;
        Ok(result
            .with_reply(info.clone())
            .with(WorldAction::Field(FieldRequest::Broadcast(info))))
    }
}

struct FedMount {
    /// One food fewer.
    changes: Vec<InventoryChange>,
    /// The tiredness it took off.
    taken: i32,
    level_up: bool,
}

/// Feed the mount the player rides the food `item_id` in USE `slot`, which
/// takes up to `relief` tiredness off. `None` if they aren't riding or the
/// slot holds no such food.
fn feed_mount(player: &mut Player, slot: i16, item_id: i32, relief: i32) -> Option<FedMount> {
    if item_id / 10_000 != 226
        || player.ride().is_none()
        || player.inventory.get(InventoryType::Use, slot)?.item_id != item_id
    {
        return None;
    }
    let changes = player
        .inventory_mut()
        .take(InventoryType::Use, slot, 1)
        .ok()?;
    let (taken, level_up) = player.mount.feed(relief);
    Some(FedMount {
        changes,
        taken,
        level_up,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::inventory::equip_slot;
    use game::{Buff, EquipStats, Item, Mount};
    use std::time::Instant;

    const REVITALIZER: i32 = 2260000;

    fn rider() -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.inventory.insert(
            InventoryType::Equipped,
            equip_slot::MOUNT,
            Item::equipment(1902000, EquipStats::default()),
        );
        player
            .inventory
            .insert(InventoryType::Use, 3, Item::stack(REVITALIZER, 2));
        player.mount = Mount {
            level: 1,
            exp: 20,
            tiredness: 45,
        };
        let riding = Buff::new(
            1004,
            BuffStat::Riding,
            0,
            Instant::now(),
            game::mount::RIDE_DURATION,
        );
        player.buffs.give(riding);
        player
    }

    #[test]
    fn food_rests_the_mount_ridden() {
        let mut player = rider();
        let fed = feed_mount(&mut player, 3, REVITALIZER, 30).unwrap();
        assert_eq!((fed.taken, fed.level_up), (30, true));
        assert_eq!(player.mount.tiredness, 15);
        let left = player.inventory.get(InventoryType::Use, 3).unwrap();
        assert_eq!(left.quantity, 1);
    }

    #[test]
    fn food_is_used_up_on_a_rested_mount() {
        let mut player = rider();
        player.mount.tiredness = 0;
        let fed = feed_mount(&mut player, 3, REVITALIZER, 30).unwrap();
        assert_eq!((fed.taken, fed.level_up), (0, false));
        assert_eq!(fed.changes.len(), 1);
    }

    #[test]
    fn only_a_rider_feeds_their_mount() {
        let mut player = rider();
        assert!(
            feed_mount(&mut player, 4, REVITALIZER, 30).is_none(),
            "no food there"
        );
        player.buffs.cancel(1004);
        assert!(
            feed_mount(&mut player, 3, REVITALIZER, 30).is_none(),
            "not riding"
        );
        assert_eq!(player.mount.tiredness, 45);
    }

    #[test]
    fn getting_off_answers_with_riding_either_way() {
        let mut player = rider();
        let result = dismount(&mut player, HandlerResult::empty()).unwrap();
        assert!(player.ride().is_none());
        assert_eq!(result.actions.len(), 2, "the cancel and the others told");

        let result = dismount(&mut player, HandlerResult::empty()).unwrap();
        assert_eq!(result.actions.len(), 1, "just the cancel");
    }
}
