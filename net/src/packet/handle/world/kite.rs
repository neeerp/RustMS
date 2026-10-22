//! Kites and banners (5080000-5080003), used through USE_CASH_ITEM with the
//! three lines the player wrote (v83 FUN_00a0a63f, type 0x12, report c014).
//! The field flies the kite above where the player stands for an hour.

use crate::handler::{FieldRequest, HandlerResult, WorldAction, WorldContext, WorldResult};
use crate::packet::build::world::inventory::build_inventory_operation;
use crate::packet::build::world::kite::build_cannot_spawn_kite;
use crate::packet::build::world::stat::build_stat_update;
use game::InventoryType;
use packet::io::read::PktRead;
use std::io::Cursor;

/// The lines a kite carries; the v83 client always sends all three.
pub const LINES: usize = 3;
/// The longest line the v83 client's fields take.
pub const MAX_LINE: usize = 60;

/// Whether kites may fly on `map_id`: the v83 client refuses them in the
/// Free Market (`map_id / 10,000,000 == 91`, "Not available in the Free
/// Market.").
pub fn allowed_on(map_id: i32) -> bool {
    map_id / 10_000_000 != 91
}

/// Fly the kite `item_id` in Cash `slot`; `r` is past the item id.
pub fn kite(
    ctx: &mut WorldContext,
    r: &mut Cursor<&Vec<u8>>,
    slot: i16,
    item_id: i32,
) -> WorldResult {
    let message = r.read_str_with_length()?;
    let unlock = build_stat_update(true, &[])?;
    if !allowed_on(ctx.player.stats.map_id) {
        return Ok(
            HandlerResult::reply(build_cannot_spawn_kite()?).with(WorldAction::Reply(unlock))
        );
    }
    let lines = message.split('\n');
    if lines.clone().count() > LINES
        || lines
            .into_iter()
            .any(|line| line.chars().count() > MAX_LINE)
    {
        return Ok(HandlerResult::reply(unlock));
    }
    let Ok(changes) = ctx
        .player
        .inventory_mut()
        .take(InventoryType::Cash, slot, 1)
    else {
        return Ok(HandlerResult::reply(unlock));
    };
    Ok(
        HandlerResult::reply(build_inventory_operation(true, &changes)?)
            .with(WorldAction::Field(FieldRequest::Kite { item_id, message })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::WorldHandler;
    use crate::packet::handle::world::UseCashItemHandler;
    use crate::packet::op::RecvOpcode;
    use game::{Item, Player};
    use packet::{io::write::PktWrite, Packet};

    fn fly(player: &mut Player, message: &str) -> Vec<WorldAction> {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
        p.write_short(3).unwrap();
        p.write_int(5080001).unwrap();
        p.write_str_with_length(message).unwrap();
        p.write_int(1234).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 2,
            player,
        };
        UseCashItemHandler.handle(&mut p, &mut ctx).unwrap().actions
    }

    fn flier() -> Player {
        let mut player = crate::codec::stats::sample_player();
        player
            .inventory
            .insert(InventoryType::Cash, 3, Item::stack(5080001, 2));
        player
    }

    fn left(player: &Player) -> i16 {
        player
            .inventory
            .get(InventoryType::Cash, 3)
            .map_or(0, |item| item.quantity)
    }

    #[test]
    fn a_kite_flies_with_its_lines() {
        let mut player = flier();
        let actions = fly(&mut player, "be\nmine\n");
        let [WorldAction::Reply(used), WorldAction::Field(FieldRequest::Kite { item_id, message })] =
            &actions[..]
        else {
            panic!("expected a reply and a kite: {actions:?}");
        };
        assert_eq!(used.bytes[..2], [0x1D, 0], "an inventory operation");
        assert_eq!((*item_id, message.as_str()), (5080001, "be\nmine\n"));
        assert_eq!(left(&player), 1);
    }

    #[test]
    fn a_kite_cannot_fly_in_the_free_market() {
        let mut player = flier();
        player.stats.map_id = 910_000_001;
        let actions = fly(&mut player, "hi\n\n");
        let [WorldAction::Reply(refused), WorldAction::Reply(unlock)] = &actions[..] else {
            panic!("expected a refusal and an unlock: {actions:?}");
        };
        assert_eq!(refused.bytes[..], [0x0E, 0x01]);
        assert_eq!(unlock.bytes[..2], [0x1F, 0], "a stat change to unlock");
        assert_eq!(left(&player), 2);
        assert!(allowed_on(100_000_000) && allowed_on(920_000_000));
    }

    #[test]
    fn a_kite_takes_three_lines_of_at_most_60() {
        for message in ["a\nb\nc\nd", &format!("{}\n\n", "a".repeat(61))] {
            let mut player = flier();
            let actions = fly(&mut player, message);
            let [WorldAction::Reply(packet)] = &actions[..] else {
                panic!("expected a lone reply: {actions:?}");
            };
            assert_eq!(packet.bytes[..2], [0x1F, 0], "a stat change to unlock");
            assert_eq!(left(&player), 2);
        }
    }
}
