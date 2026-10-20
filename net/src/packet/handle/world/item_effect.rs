//! USE_ITEMEFFECT (0x34): the player shows an effect item's effect, or
//! stops with item 0 (report 201, item effects). The client draws it at
//! once; the rest of the map sees SHOW_ITEM_EFFECT, and those who come later
//! the effect in SPAWN_PLAYER. It lasts as long as the session.

use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::packet::build::world::field::build_show_item_effect;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// Whether `item_id` is an effect item: a 501xxxx cash item or a 429xxxx
/// etc item.
fn is_effect(item_id: i32) -> bool {
    matches!(item_id / 10000, 501 | 429)
}

pub struct UseItemEffectHandler;

impl WorldHandler for UseItemEffectHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let item_id = r.read_int()?;
        let owned = is_effect(item_id) && ctx.player.inventory.count(item_id) > 0;
        if (item_id != 0 && !owned) || item_id == ctx.player.item_effect {
            return Ok(HandlerResult::empty());
        }
        ctx.player.item_effect = item_id;
        let shown = build_show_item_effect(ctx.player.id(), item_id)?;
        Ok(HandlerResult::empty().with(WorldAction::ItemEffectChanged(shown)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use game::{InventoryType, Item, Player};
    use packet::io::write::PktWrite;

    const SUNNY: i32 = 5010000;
    const ETC: i32 = 4290000;

    fn use_effect(player: &mut Player, item_id: i32) -> Vec<WorldAction> {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseItemEffect as i16).unwrap();
        p.write_int(item_id).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        UseItemEffectHandler
            .handle(&mut p, &mut ctx)
            .unwrap()
            .actions
    }

    fn holder() -> Player {
        let mut player = crate::codec::stats::sample_player();
        player
            .inventory
            .insert(InventoryType::Cash, 1, Item::stack(SUNNY, 1));
        player
            .inventory
            .insert(InventoryType::Etc, 1, Item::stack(ETC, 1));
        player
    }

    fn shown(actions: &[WorldAction]) -> Vec<u8> {
        let [WorldAction::ItemEffectChanged(packet)] = actions else {
            panic!("expected the effect for the map: {actions:?}");
        };
        packet.bytes[2..].to_vec()
    }

    #[test]
    fn an_owned_effect_item_shows_to_the_map() {
        let mut player = holder();
        let id = player.id().to_le_bytes();
        let body = shown(&use_effect(&mut player, SUNNY));
        assert_eq!(body[..4], id);
        assert_eq!(body[4..], SUNNY.to_le_bytes());
        assert_eq!(player.item_effect, SUNNY);

        shown(&use_effect(&mut player, ETC));
        assert_eq!(player.item_effect, ETC, "etc effect items too");
    }

    #[test]
    fn item_zero_takes_the_effect_away() {
        let mut player = holder();
        assert!(
            use_effect(&mut player, 0).is_empty(),
            "nothing to take away"
        );
        use_effect(&mut player, SUNNY);
        assert!(use_effect(&mut player, SUNNY).is_empty(), "already shown");
        assert_eq!(shown(&use_effect(&mut player, 0))[4..], [0, 0, 0, 0]);
        assert_eq!(player.item_effect, 0);
    }

    #[test]
    fn an_item_not_held_or_not_an_effect_is_ignored() {
        let mut player = holder();
        player
            .inventory
            .insert(InventoryType::Cash, 2, Item::stack(5120000, 1));
        assert!(use_effect(&mut player, 5010001).is_empty(), "not held");
        assert!(use_effect(&mut player, 5120000).is_empty(), "a weather");
        assert_eq!(player.item_effect, 0);
    }
}
