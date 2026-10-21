use super::use_item::restores;
use crate::handler::{HandlerResult, WorldContext, WorldHandler, WorldResult};
use game::keymap::KEY_COUNT;
use game::{KeyBinding, Player};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// `CHANGE_KEYMAP` (0x87). Mode 0 changes key bindings; modes 1 and 2 set
/// the potion the first pet gives from the HP or MP pouch's slot. Nothing
/// answers.
pub struct ChangeKeybindsHandler;

impl WorldHandler for ChangeKeybindsHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;

        match reader.read_int()? {
            0 => {}
            mode @ (1 | 2) => {
                let item_id = reader.read_int()?;
                set_auto_potion(ctx.player, mode == 2, item_id);
                return Ok(HandlerResult::empty());
            }
            _ => return Ok(HandlerResult::empty()),
        }

        let count = reader.read_int()?;
        let keymap = ctx.player.keymap_mut();
        for _ in 0..count {
            let key = reader.read_int()?;
            let kind = reader.read_byte()?;
            let action = reader.read_int()?;
            if (0..i32::from(KEY_COUNT)).contains(&key) {
                keymap.bind(key as i16, KeyBinding { kind, action });
            }
        }
        Ok(HandlerResult::empty())
    }
}

/// Set the potion beside the HP pouch, or the MP pouch if `mp`, to
/// `item_id`, or clear it for 0 (v83 FUN_007ff0d6, FUN_007ff5fe). The slot
/// takes only an item restoring what its pouch is for. A v83 client clears
/// both whenever it draws the pet panel without a pet out or without the
/// pouch, so zeros come often. Returns whether the item was taken.
fn set_auto_potion(player: &mut Player, mp: bool, item_id: i32) -> bool {
    if item_id != 0 && !restores(item_id, mp) {
        return false;
    }
    let potions = &mut player.auto_potions;
    let slot = if mp { &mut potions.mp } else { &mut potions.hp };
    if std::mem::replace(slot, item_id) != item_id {
        player.mark_dirty();
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items;
    use crate::packet::op::RecvOpcode;
    use game::AutoPotions;
    use packet::io::write::PktWrite;

    fn change(player: &mut Player, mode: i32, item_id: i32) {
        let mut packet = Packet::new_empty();
        packet
            .write_short(RecvOpcode::ChangeKeybinds as i16)
            .unwrap();
        packet.write_int(mode).unwrap();
        packet.write_int(item_id).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        let result = ChangeKeybindsHandler.handle(&mut packet, &mut ctx).unwrap();
        assert!(result.actions.is_empty());
    }

    #[test]
    fn the_pouch_slots_take_potions_for_what_their_pouch_is_for() {
        if items::data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        // A Red Potion restores HP, a Blue Potion MP, an Elixir both.
        change(&mut player, 1, 2000000);
        change(&mut player, 2, 2000003);
        assert_eq!(
            player.auto_potions,
            AutoPotions {
                hp: 2000000,
                mp: 2000003
            }
        );
        assert!(player.is_dirty());
        change(&mut player, 1, 2000003);
        change(&mut player, 2, 2000000);
        change(&mut player, 2, 4000000);
        assert_eq!(player.auto_potions.hp, 2000000);
        assert_eq!(player.auto_potions.mp, 2000003);
        change(&mut player, 1, 2000004);
        change(&mut player, 2, 2000004);
        assert_eq!(
            player.auto_potions,
            AutoPotions {
                hp: 2000004,
                mp: 2000004
            }
        );
    }

    #[test]
    fn zero_clears_a_pouch_slot() {
        let mut player = crate::codec::stats::sample_player();
        player.auto_potions = AutoPotions {
            hp: 2000000,
            mp: 2000003,
        };
        change(&mut player, 1, 0);
        assert_eq!(player.auto_potions, AutoPotions { hp: 0, mp: 2000003 });
        assert!(player.is_dirty());
    }
}
