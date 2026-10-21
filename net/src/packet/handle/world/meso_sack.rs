//! Meso sacks (520xxxx), used through USE_CASH_ITEM with nothing after the
//! item (v83 FUN_00a0a63f, type 0x13). The client has asked "Will you use
//! the meso bag to receive %d mesos?" with the item's `info/meso`; the sack
//! goes and its mesos land, with the chat line quest rewards print
//! (report c005).

use crate::handler::{HandlerResult, WorldContext, WorldResult};
use crate::items;
use crate::packet::build::world::drop::build_meso_gain_in_chat;
use crate::packet::build::world::inventory::build_inventory_operation;
use crate::packet::build::world::stat::{build_stat_update, StatUpdate};
use game::InventoryType;

/// Open the meso sack `item_id` in Cash `slot`. One that would take the
/// player past the most mesos anyone holds stays shut.
pub fn meso_sack(ctx: &mut WorldContext, slot: i16, item_id: i32) -> WorldResult {
    let gain = items::info(item_id).map_or(0, |info| info.meso);
    let Some(meso) = ctx.player.meso.checked_add(gain).filter(|_| gain > 0) else {
        return Ok(HandlerResult::reply(build_stat_update(true, &[])?));
    };
    let Ok(changes) = ctx
        .player
        .inventory_mut()
        .take(InventoryType::Cash, slot, 1)
    else {
        return Ok(HandlerResult::reply(build_stat_update(true, &[])?));
    };
    ctx.player.meso = meso;
    ctx.player.mark_dirty();
    Ok(
        HandlerResult::reply(build_stat_update(true, &[StatUpdate::Meso(meso)])?)
            .with_reply(build_inventory_operation(true, &changes)?)
            .with_reply(build_meso_gain_in_chat(gain)?),
    )
}

#[cfg(test)]
mod tests {
    use crate::handler::{WorldAction, WorldContext, WorldHandler};
    use crate::packet::handle::world::UseCashItemHandler;
    use crate::packet::op::RecvOpcode;
    use game::{InventoryType, Item, Player};
    use packet::{io::write::PktWrite, Packet};

    const SACK: i32 = 5_200_000;

    fn open(player: &mut Player, slot: i16) -> Vec<WorldAction> {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
        p.write_short(slot).unwrap();
        p.write_int(SACK).unwrap();
        p.write_int(1234).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 2,
            player,
        };
        UseCashItemHandler.handle(&mut p, &mut ctx).unwrap().actions
    }

    fn with_sacks(meso: i32) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.meso = meso;
        player
            .inventory
            .insert(InventoryType::Cash, 1, Item::stack(SACK, 2));
        player
    }

    #[test]
    fn the_sacks_hold_what_their_data_says() {
        let meso = |id| crate::items::info(id).unwrap().meso;
        assert_eq!(
            [meso(5_200_000), meso(5_200_001), meso(5_200_002)],
            [1_000_000, 5_000_000, 10_000_000]
        );
    }

    #[test]
    fn a_sack_turns_into_its_mesos() {
        let mut player = with_sacks(100);
        let actions = open(&mut player, 1);
        assert_eq!(actions.len(), 3, "mesos, the sack, the chat line");
        assert_eq!(player.meso, 1_000_100);
        let left = player.inventory.get(InventoryType::Cash, 1).unwrap();
        assert_eq!(left.quantity, 1);
    }

    #[test]
    fn a_sack_past_the_meso_cap_stays_shut() {
        let mut player = with_sacks(i32::MAX - 10);
        let actions = open(&mut player, 1);
        assert_eq!(actions.len(), 1, "only the unlock");
        assert_eq!(player.meso, i32::MAX - 10);
        let left = player.inventory.get(InventoryType::Cash, 1).unwrap();
        assert_eq!(left.quantity, 2);
    }

    #[test]
    fn no_sack_in_the_slot_opens_nothing() {
        let mut player = with_sacks(100);
        let actions = open(&mut player, 2);
        assert_eq!(actions.len(), 1, "only the unlock");
        assert_eq!(player.meso, 100);
    }
}
