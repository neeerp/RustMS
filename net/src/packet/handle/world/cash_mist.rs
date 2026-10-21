//! Cash mists (528xxxx: 5281000 Passed Gas, 5281001 Floral Scent), used
//! through USE_CASH_ITEM with nothing after the item (v83 FUN_00a0a63f, type
//! 0x22). The item goes, and the map's field leaves its cloud around the
//! player for everyone to see (reports leftovers-145-157-160 3.5, c007).

use crate::handler::{FieldRequest, HandlerResult, WorldAction, WorldContext, WorldResult};
use crate::packet::build::world::inventory::build_inventory_operation;
use crate::packet::build::world::stat::build_stat_update;
use game::InventoryType;
use std::time::Duration;

/// Use the cash mist `item_id` in Cash `slot`.
pub fn cash_mist(ctx: &mut WorldContext, slot: i16, item_id: i32) -> WorldResult {
    let Some(mist) = crate::items::info(item_id).and_then(|info| info.mist) else {
        return Ok(HandlerResult::reply(build_stat_update(true, &[])?));
    };
    let Ok(changes) = ctx
        .player
        .inventory_mut()
        .take(InventoryType::Cash, slot, 1)
    else {
        return Ok(HandlerResult::reply(build_stat_update(true, &[])?));
    };
    ctx.player.mist_face = Some(item_id);
    Ok(
        HandlerResult::reply(build_inventory_operation(true, &changes)?).with(WorldAction::Field(
            FieldRequest::ItemMist {
                item_id,
                area: mist.area,
                lasts: Duration::from_secs(mist.seconds.max(0) as u64),
            },
        )),
    )
}

#[cfg(test)]
mod tests {
    use crate::handler::{FieldRequest, WorldAction, WorldContext, WorldHandler};
    use crate::packet::handle::world::UseCashItemHandler;
    use crate::packet::op::RecvOpcode;
    use game::{InventoryType, Item, Player};
    use packet::{io::write::PktWrite, Packet};
    use std::time::Duration;

    const PASSED_GAS: i32 = 5_281_000;

    fn use_mist(player: &mut Player, item_id: i32) -> Vec<WorldAction> {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
        p.write_short(1).unwrap();
        p.write_int(item_id).unwrap();
        p.write_int(1234).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 2,
            player,
        };
        UseCashItemHandler.handle(&mut p, &mut ctx).unwrap().actions
    }

    fn holding(item_id: i32) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player
            .inventory
            .insert(InventoryType::Cash, 1, Item::stack(item_id, 2));
        player
    }

    #[test]
    fn a_cash_mist_is_spent_on_its_cloud() {
        if crate::items::info(PASSED_GAS).is_none() {
            return;
        }
        let mut player = holding(PASSED_GAS);
        let actions = use_mist(&mut player, PASSED_GAS);
        assert!(actions.iter().any(|action| matches!(
            action,
            WorldAction::Field(FieldRequest::ItemMist {
                item_id: PASSED_GAS,
                area: (-110, -82, 110, 83),
                lasts,
            }) if *lasts == Duration::from_secs(60)
        )));
        let left = player.inventory.get(InventoryType::Cash, 1).unwrap();
        assert_eq!(left.quantity, 1);
    }

    #[test]
    fn an_item_with_no_cloud_is_kept() {
        if crate::items::info(PASSED_GAS).is_none() {
            return;
        }
        // 5280000 is no item; nothing is spent.
        let mut player = holding(5_280_000);
        let actions = use_mist(&mut player, 5_280_000);
        assert!(!actions
            .iter()
            .any(|action| matches!(action, WorldAction::Field(_))));
        let kept = player.inventory.get(InventoryType::Cash, 1).unwrap();
        assert_eq!(kept.quantity, 2);
    }
}
