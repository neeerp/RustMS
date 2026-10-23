//! Miu Miu the Traveling Merchant (5450000), used from the Cash tab as
//! USE_CASH_ITEM type 0x25 with nothing after the item but the tick (v83
//! FUN_00a0a63f; report c082). Each use opens the shop of Mu Mu (NPC
//! 9090000) wherever the player stands, and costs one of the stack.

use crate::cash_shop::MIU_MIU;
use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldResult};
use crate::packet::build::world::stat::build_stat_update;
use game::{InventoryChange, InventoryType, Player};
use tracing::debug;

/// Mu Mu, whose shop Miu Miu opens.
pub const MU_MU: i32 = 9_090_000;

/// The Miu Miu in Cash `slot`. The shop opens in the client actor, which
/// knows whether the player is trading or keeps a shop.
pub fn miu_miu(ctx: &mut WorldContext, slot: i16, item_id: i32) -> WorldResult {
    if !may_use(ctx.player, slot, item_id) {
        debug!(slot, item_id, "Miu Miu refused");
        return Ok(HandlerResult::reply(build_stat_update(true, &[])?));
    }
    Ok(HandlerResult::empty().with(WorldAction::MiuMiu { slot }))
}

/// Whether the player may use the Miu Miu in Cash `slot` here: they're
/// alive, it is there, and the map allows it.
pub fn may_use(player: &Player, slot: i16, item_id: i32) -> bool {
    if item_id != MIU_MIU || player.stats.hp <= 0 {
        return false;
    }
    if player
        .inventory
        .get(InventoryType::Cash, slot)
        .is_none_or(|item| item.item_id != MIU_MIU)
    {
        return false;
    }
    crate::get_game_data()
        .ok()
        .and_then(|data| data.field(player.stats.map_id))
        .is_none_or(|field| field.may_call_merchant())
}

/// Take one Miu Miu from Cash `slot`, once its shop is open.
pub fn take(player: &mut Player, slot: i16) -> Option<Vec<InventoryChange>> {
    if !player
        .inventory
        .get(InventoryType::Cash, slot)
        .is_some_and(|item| item.item_id == MIU_MIU)
    {
        return None;
    }
    player
        .inventory_mut()
        .take(InventoryType::Cash, slot, 1)
        .ok()
}

#[cfg(test)]
mod tests {
    use super::{may_use, take, MU_MU};
    use crate::cash_shop::MIU_MIU;
    use crate::handler::{WorldAction, WorldContext, WorldHandler};
    use crate::packet::handle::world::UseCashItemHandler;
    use crate::packet::op::RecvOpcode;
    use crate::shop::Shop;
    use game::{InventoryType, Item, Player};
    use packet::{io::write::PktWrite, Packet};

    fn call(player: &mut Player, slot: i16, item_id: i32) -> Vec<WorldAction> {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
        p.write_short(slot).unwrap();
        p.write_int(item_id).unwrap();
        p.write_int(1234).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 2,
            player,
        };
        UseCashItemHandler.handle(&mut p, &mut ctx).unwrap().actions
    }

    fn with_miu_miu(count: i16) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player
            .inventory
            .insert(InventoryType::Cash, 1, Item::stack(MIU_MIU, count));
        player
    }

    #[test]
    fn miu_miu_calls_the_merchant() {
        let mut player = with_miu_miu(2);
        let actions = call(&mut player, 1, MIU_MIU);
        assert!(matches!(actions[..], [WorldAction::MiuMiu { slot: 1 }]));
        // Nothing goes until the shop opens.
        assert_eq!(
            player
                .inventory
                .get(InventoryType::Cash, 1)
                .unwrap()
                .quantity,
            2
        );
    }

    #[test]
    fn only_a_miu_miu_in_its_slot_calls_him() {
        let mut player = with_miu_miu(1);
        assert!(!may_use(&player, 2, MIU_MIU), "an empty slot");
        assert!(!may_use(&player, 1, 5_451_000), "another item named");
        let actions = call(&mut player, 2, MIU_MIU);
        assert_eq!(actions.len(), 1, "only the unlock");
        assert!(matches!(actions[0], WorldAction::Reply(_)));
        player.stats.hp = 0;
        assert!(!may_use(&player, 1, MIU_MIU), "the dead");
    }

    #[test]
    fn not_where_the_map_keeps_him_away() {
        let data = crate::get_game_data().unwrap();
        let mut ids: Vec<i32> = data
            .field_ids()
            .filter(|&id| !data.field(id).unwrap().may_call_merchant())
            .collect();
        ids.sort();
        let mut player = with_miu_miu(1);
        player.stats.map_id = ids[0];
        assert!(!may_use(&player, 1, MIU_MIU), "{}", ids[0]);
        player.stats.map_id = 100_000_000;
        assert!(may_use(&player, 1, MIU_MIU), "Henesys");
    }

    #[test]
    fn each_call_takes_one() {
        let mut player = with_miu_miu(2);
        assert_eq!(take(&mut player, 1).unwrap().len(), 1);
        assert_eq!(
            player
                .inventory
                .get(InventoryType::Cash, 1)
                .unwrap()
                .quantity,
            1
        );
        take(&mut player, 1).unwrap();
        assert!(player.inventory.get(InventoryType::Cash, 1).is_none());
        assert!(take(&mut player, 1).is_none());
    }

    #[test]
    fn mu_mu_keeps_a_shop() {
        let player = with_miu_miu(1);
        let shop = Shop::open(MU_MU, &player).expect("Mu Mu's shop");
        assert_eq!(shop.items.len(), 26);
        assert_eq!(shop.items[0].item_id, 5_041_000);
    }
}
