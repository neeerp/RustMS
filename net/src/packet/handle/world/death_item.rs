//! USE_DEATHITEM (0x35): a dead player holding a Wheel of Destiny is asked
//! whether to use it, and their client sends the item and where they lie,
//! playing `BasicEff/TombUpgrade` there itself (FUN_0095af8e, report b334).
//! The rest of the map sees SHOW_UPGRADE_TOMB_EFFECT. Whether the wheel is
//! used is the revive's choice, made on CHANGE_MAP.

use crate::handler::{
    FieldRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::packet::build::world::field::build_upgrade_tomb_effect;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

const WHEEL_OF_DESTINY: i32 = 5510000;

pub struct UseDeathItemHandler;

impl WorldHandler for UseDeathItemHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let item_id = r.read_int()?;
        let x = r.read_int()?;
        let y = r.read_int()?;
        let dead = ctx.player.stats.hp <= 0;
        if item_id != WHEEL_OF_DESTINY || !dead || ctx.player.inventory.count(item_id) == 0 {
            return Ok(HandlerResult::empty());
        }
        let shown = build_upgrade_tomb_effect(ctx.player.id(), item_id, x, y)?;
        Ok(HandlerResult::empty().with(WorldAction::Field(FieldRequest::Broadcast(shown))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use game::{InventoryType, Item, Player};
    use packet::io::write::PktWrite;

    fn use_wheel(player: &mut Player, item_id: i32) -> Vec<WorldAction> {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseDeathItem as i16).unwrap();
        p.write_int(item_id).unwrap();
        p.write_int(-120).unwrap();
        p.write_int(34).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        UseDeathItemHandler
            .handle(&mut p, &mut ctx)
            .unwrap()
            .actions
    }

    fn fallen() -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.stats.hp = 0;
        player
            .inventory
            .insert(InventoryType::Cash, 1, Item::stack(WHEEL_OF_DESTINY, 1));
        player
    }

    #[test]
    fn the_map_sees_the_tomb_where_the_player_lies() {
        let mut player = fallen();
        let actions = use_wheel(&mut player, WHEEL_OF_DESTINY);
        let [WorldAction::Field(FieldRequest::Broadcast(packet))] = &actions[..] else {
            panic!("expected the effect for the map: {actions:?}");
        };
        let mut expected = vec![0xC3, 0];
        for value in [player.id(), WHEEL_OF_DESTINY, -120, 34] {
            expected.extend(value.to_le_bytes());
        }
        assert_eq!(packet.bytes, expected);
        assert_eq!(player.inventory.count(WHEEL_OF_DESTINY), 1, "not used yet");
    }

    #[test]
    fn nothing_without_a_wheel_or_while_alive() {
        let mut player = fallen();
        assert!(use_wheel(&mut player, 5510001).is_empty());
        player.stats.hp = 50;
        assert!(use_wheel(&mut player, WHEEL_OF_DESTINY).is_empty());
        let mut player = crate::codec::stats::sample_player();
        player.stats.hp = 0;
        assert!(use_wheel(&mut player, WHEEL_OF_DESTINY).is_empty());
    }
}
