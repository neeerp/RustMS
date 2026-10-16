use crate::handler::{
    FieldRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::packet::build::world::field::build_cancel_chair;
use crate::packet::build::world::stat::build_stat_update;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// Portable chairs are the setup items 301xxxx.
fn is_portable_chair(item_id: i32) -> bool {
    item_id / 10000 == 301
}

/// USE_CHAIR: the client has already sat down on a portable chair it owns;
/// show the others on the map. Every answer unlocks the client, which waits
/// on it before sitting or using an item again.
pub struct UseChairHandler;

impl WorldHandler for UseChairHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let item_id = reader.read_int()?;
        let unlock = HandlerResult::reply(build_stat_update(true, &[])?);

        if !is_portable_chair(item_id) || ctx.player.inventory.count(item_id) == 0 {
            return Ok(unlock);
        }
        ctx.player.chair = item_id;
        Ok(unlock.with(WorldAction::Field(FieldRequest::Chair(item_id))))
    }
}

/// CANCEL_CHAIR: the player wants to stand up. Seat -1 is a portable chair;
/// map seats are handled by the client alone.
pub struct CancelChairHandler;

impl WorldHandler for CancelChairHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let seat = reader.read_short()?;

        if seat != -1 {
            return Ok(HandlerResult::empty());
        }
        ctx.player.chair = 0;
        Ok(HandlerResult::empty()
            .with(WorldAction::Reply(build_cancel_chair()?))
            .with(WorldAction::Field(FieldRequest::Chair(0))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::{RecvOpcode, SendOpcode};
    use game::{InventoryType, Item};
    use packet::io::write::PktWrite;

    fn handle(handler: &dyn WorldHandler, opcode: RecvOpcode, body: &[u8]) -> Vec<WorldAction> {
        let mut player = crate::codec::stats::sample_player();
        player
            .inventory
            .insert(InventoryType::Setup, 1, Item::stack(3010000, 1));
        player
            .inventory
            .insert(InventoryType::Setup, 2, Item::stack(3994000, 1));
        let mut packet = Packet::new_empty();
        packet.write_short(opcode as i16).unwrap();
        packet.write_bytes(body).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        handler.handle(&mut packet, &mut ctx).unwrap().actions
    }

    fn use_chair(item_id: i32) -> Vec<WorldAction> {
        handle(
            &UseChairHandler,
            RecvOpcode::UseChair,
            &item_id.to_le_bytes(),
        )
    }

    fn unlocks(action: &WorldAction) -> bool {
        matches!(action, WorldAction::Reply(p)
            if p.bytes[..2] == (SendOpcode::StatChange as i16).to_le_bytes() && p.bytes[2] == 1)
    }

    #[test]
    fn sitting_needs_an_owned_portable_chair() {
        let actions = use_chair(3010000);
        assert!(matches!(
            actions.as_slice(),
            [_, WorldAction::Field(FieldRequest::Chair(3010000))]
        ));
        assert!(unlocks(&actions[0]));
        // Not owned, or owned but not a chair: only unlocked.
        for item_id in [3010001, 3994000] {
            let actions = use_chair(item_id);
            assert!(matches!(actions.as_slice(), [a] if unlocks(a)));
        }
    }

    #[test]
    fn standing_up_from_a_portable_chair_is_confirmed() {
        let actions = handle(
            &CancelChairHandler,
            RecvOpcode::CancelChair,
            &(-1i16).to_le_bytes(),
        );
        assert!(matches!(
            actions.as_slice(),
            [
                WorldAction::Reply(_),
                WorldAction::Field(FieldRequest::Chair(0))
            ]
        ));
        assert!(handle(
            &CancelChairHandler,
            RecvOpcode::CancelChair,
            &3i16.to_le_bytes()
        )
        .is_empty());
    }
}
