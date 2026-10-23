//! Scripted items (243xxxx), used from the Use tab as USE_SCRIPTED_ITEM
//! (0x4E, v83 FUN_00a09b26; report c083): the tick, the slot and the item.
//! The item names a script (`spec/script`) that runs where the player
//! stands; `scripts::item` says what it does.

use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::items;
use crate::packet::build::world::stat::build_stat_update;
use game::{InventoryType, Player};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::debug;

pub struct ScriptedItemHandler;

impl WorldHandler for ScriptedItemHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        r.read_int()?;
        let slot = r.read_short()?;
        let item_id = r.read_int()?;
        match script(ctx.player, slot, item_id) {
            Some(script) => Ok(HandlerResult::empty().with(WorldAction::ScriptedItem { script })),
            None => {
                debug!(slot, item_id, "Scripted item refused");
                // The client waits for this before it uses another item.
                Ok(HandlerResult::reply(build_stat_update(true, &[])?))
            }
        }
    }
}

/// The script of the scripted item `item_id` in Use `slot`, if the player
/// is alive and holds it there.
fn script(player: &Player, slot: i16, item_id: i32) -> Option<String> {
    if item_id / 10_000 != 243
        || player.stats.hp <= 0
        || player.inventory.get(InventoryType::Use, slot)?.item_id != item_id
    {
        return None;
    }
    items::info(item_id)?.spec?.script
}

#[cfg(test)]
mod tests {
    use super::ScriptedItemHandler;
    use crate::handler::{WorldAction, WorldContext, WorldHandler};
    use crate::packet::op::RecvOpcode;
    use game::{InventoryType, Item, Player};
    use packet::{io::write::PktWrite, Packet};

    const KILLER_MUSHROOM_SPORE: i32 = 2_430_014;
    const THORN_REMOVER: i32 = 2_430_015;

    fn use_item(player: &mut Player, slot: i16, item_id: i32) -> Vec<WorldAction> {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseScriptedItem as i16).unwrap();
        p.write_int(1234).unwrap();
        p.write_short(slot).unwrap();
        p.write_int(item_id).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 2,
            player,
        };
        ScriptedItemHandler
            .handle(&mut p, &mut ctx)
            .unwrap()
            .actions
    }

    fn holding(item_id: i32) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player
            .inventory
            .insert(InventoryType::Use, 3, Item::stack(item_id, 1));
        player
    }

    fn script(actions: &[WorldAction]) -> Option<&str> {
        match actions {
            [WorldAction::ScriptedItem { script }] => Some(script),
            _ => None,
        }
    }

    #[test]
    fn runs_the_items_script() {
        let mut player = holding(KILLER_MUSHROOM_SPORE);
        let actions = use_item(&mut player, 3, KILLER_MUSHROOM_SPORE);
        assert_eq!(script(&actions), Some("killarmush"));
        let mut player = holding(THORN_REMOVER);
        let actions = use_item(&mut player, 3, THORN_REMOVER);
        assert_eq!(script(&actions), Some("removethorns"));
        // Nothing is taken here: the script decides.
        assert_eq!(player.inventory.count(THORN_REMOVER), 1);
    }

    #[test]
    fn only_the_item_in_its_slot_and_only_alive() {
        let mut player = holding(THORN_REMOVER);
        for (slot, item_id) in [(4, THORN_REMOVER), (3, KILLER_MUSHROOM_SPORE)] {
            let actions = use_item(&mut player, slot, item_id);
            assert!(
                matches!(actions[..], [WorldAction::Reply(_)]),
                "only the unlock"
            );
        }
        player.stats.hp = 0;
        let actions = use_item(&mut player, 3, THORN_REMOVER);
        assert!(matches!(actions[..], [WorldAction::Reply(_)]), "the dead");
    }

    #[test]
    fn not_other_items() {
        let mut player = holding(2_000_000);
        let actions = use_item(&mut player, 3, 2_000_000);
        assert!(matches!(actions[..], [WorldAction::Reply(_)]));
    }
}
