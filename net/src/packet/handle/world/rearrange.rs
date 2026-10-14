use crate::handler::{HandlerResult, WorldContext, WorldHandler, WorldResult};
use crate::packet::build::world::{inventory, stat};
use game::{InventoryChange, InventoryError, InventoryType, Player};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::debug;

/// `GATHER_ITEMS`: tick, inventory type. Moves the bag's items up into its
/// empty slots, keeping their order.
pub struct GatherItemsHandler;

/// `SORT_ITEMS`: tick, inventory type. Orders the bag's items by item id.
pub struct SortItemsHandler;

impl WorldHandler for GatherItemsHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        rearrange(packet, ctx.player, false)
    }
}

impl WorldHandler for SortItemsHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        rearrange(packet, ctx.player, true)
    }
}

/// Answer with the moves, then the result that flips the item window's
/// Gather/Sort button.
fn rearrange(packet: &Packet, player: &mut Player, sort: bool) -> WorldResult {
    let mut reader = Cursor::new(&packet.bytes);
    reader.read_short()?;
    reader.read_int()?; // client tick
    let kind = reader.read_byte()?;

    let changes = InventoryType::from_i16(i16::from(kind))
        .filter(|kind| *kind != InventoryType::Equipped)
        .ok_or(InventoryError::InvalidSlot)
        .and_then(|kind| {
            let changes = if sort {
                player.inventory_mut().sort(kind)
            } else {
                player.inventory_mut().gather(kind)
            };
            changes.map(|changes| (kind, changes))
        });
    let (kind, changes): (_, Vec<InventoryChange>) = match changes {
        Ok(result) => result,
        Err(error) => {
            debug!(kind, sort, %error, "Gather or sort refused");
            return Ok(HandlerResult::reply(stat::build_stat_update(true, &[])?));
        }
    };

    let result = if sort {
        inventory::build_sort_result(kind)?
    } else {
        inventory::build_gather_result(kind)?
    };
    Ok(if changes.is_empty() {
        HandlerResult::reply(result).with_reply(stat::build_stat_update(true, &[])?)
    } else {
        HandlerResult::reply(inventory::build_inventory_operation(true, &changes)?)
            .with_reply(result)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::WorldAction;
    use crate::packet::op::{RecvOpcode, SendOpcode};
    use game::Item;
    use packet::io::write::PktWrite;

    fn request(opcode: RecvOpcode, kind: u8) -> Packet {
        let mut p = Packet::new_empty();
        p.write_short(opcode as i16).unwrap();
        p.write_int(0).unwrap();
        p.write_byte(kind).unwrap();
        p
    }

    fn handle(player: &mut Player, handler: &dyn WorldHandler, packet: Packet) -> Vec<i16> {
        let mut packet = packet;
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        handler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
            .into_iter()
            .map(|action| match action {
                WorldAction::Reply(packet) => {
                    i16::from_le_bytes([packet.bytes[0], packet.bytes[1]])
                }
                other => panic!("unexpected action {other:?}"),
            })
            .collect()
    }

    fn slots(player: &Player) -> Vec<(i16, i32)> {
        player
            .inventory
            .items(InventoryType::Etc)
            .map(|(slot, item)| (slot, item.item_id))
            .collect()
    }

    #[test]
    fn gather_then_sort_rearrange_the_bag_and_answer_each() {
        let mut player = crate::codec::stats::sample_player();
        player
            .inventory
            .insert(InventoryType::Etc, 3, Item::stack(4000019, 5));
        player
            .inventory
            .insert(InventoryType::Etc, 7, Item::stack(4000000, 2));

        let replies = handle(
            &mut player,
            &GatherItemsHandler,
            request(RecvOpcode::GatherItems, InventoryType::Etc as u8),
        );
        assert_eq!(
            replies,
            vec![
                SendOpcode::InventoryOperation as i16,
                SendOpcode::GatherItemResult as i16
            ]
        );
        assert_eq!(slots(&player), vec![(1, 4000019), (2, 4000000)]);

        let replies = handle(
            &mut player,
            &SortItemsHandler,
            request(RecvOpcode::SortItems, InventoryType::Etc as u8),
        );
        assert_eq!(
            replies,
            vec![
                SendOpcode::InventoryOperation as i16,
                SendOpcode::SortItemResult as i16
            ]
        );
        assert_eq!(slots(&player), vec![(1, 4000000), (2, 4000019)]);
    }

    #[test]
    fn a_tidy_bag_still_gets_its_result() {
        let mut player = crate::codec::stats::sample_player();
        let replies = handle(
            &mut player,
            &SortItemsHandler,
            request(RecvOpcode::SortItems, InventoryType::Use as u8),
        );
        assert_eq!(
            replies,
            vec![
                SendOpcode::SortItemResult as i16,
                SendOpcode::StatChange as i16
            ]
        );
    }

    #[test]
    fn unknown_inventories_are_refused() {
        let mut player = crate::codec::stats::sample_player();
        for kind in [0, 6, -1i8 as u8] {
            let replies = handle(
                &mut player,
                &GatherItemsHandler,
                request(RecvOpcode::GatherItems, kind),
            );
            assert_eq!(replies, vec![SendOpcode::StatChange as i16]);
        }
    }
}
