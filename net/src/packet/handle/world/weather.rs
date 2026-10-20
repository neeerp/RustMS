//! Weather items (512xxxx), used through USE_CASH_ITEM with the message the
//! player attached (v83 FUN_00a0a63f, type 0x10). The map's field blows the
//! item's weather for everyone there, under the item's message with the
//! player's name and theirs (report 201).

use crate::handler::{FieldRequest, HandlerResult, WorldAction, WorldContext, WorldResult};
use crate::packet::build::world::inventory::build_inventory_operation;
use crate::packet::build::world::stat::build_stat_update;
use game::InventoryType;
use packet::io::read::PktRead;
use std::io::Cursor;

/// The longest message the v83 client lets a player attach.
pub const MAX_MESSAGE: usize = 35;

/// Use the weather item `item_id` in Cash `slot`; `r` is past the item id.
pub fn weather(
    ctx: &mut WorldContext,
    r: &mut Cursor<&Vec<u8>>,
    slot: i16,
    item_id: i32,
) -> WorldResult {
    let text = r.read_str_with_length()?;
    let allowed = crate::get_game_data()
        .ok()
        .and_then(|data| data.field(ctx.player.stats.map_id))
        .is_none_or(|field| field.may_blow_weather());
    if text.is_empty() || text.len() > MAX_MESSAGE || !allowed {
        return Ok(HandlerResult::reply(build_stat_update(true, &[])?));
    }
    let Ok(changes) = ctx
        .player
        .inventory_mut()
        .take(InventoryType::Cash, slot, 1)
    else {
        return Ok(HandlerResult::reply(build_stat_update(true, &[])?));
    };
    let message = crate::items::cash_message(item_id)
        .map(|template| fill(&template, &[ctx.player.name(), &text]))
        .unwrap_or_default();
    Ok(
        HandlerResult::reply(build_inventory_operation(true, &changes)?).with(WorldAction::Field(
            FieldRequest::Weather { item_id, message },
        )),
    )
}

/// `template` with its `%s`s, in turn, replaced by `with`.
fn fill(template: &str, with: &[&str]) -> String {
    let mut out = String::new();
    let mut rest = template;
    for value in with {
        let Some(at) = rest.find("%s") else {
            break;
        };
        out.push_str(&rest[..at]);
        out.push_str(value);
        rest = &rest[at + 2..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::WorldHandler;
    use crate::packet::handle::world::UseCashItemHandler;
    use crate::packet::op::RecvOpcode;
    use game::{Item, Player};
    use packet::{io::write::PktWrite, Packet};

    fn use_weather(player: &mut Player, text: &str) -> Vec<WorldAction> {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
        p.write_short(2).unwrap();
        p.write_int(5120000).unwrap();
        p.write_str_with_length(text).unwrap();
        p.write_int(1234).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 2,
            player,
        };
        UseCashItemHandler.handle(&mut p, &mut ctx).unwrap().actions
    }

    fn snowman() -> Player {
        let mut player = crate::codec::stats::sample_player();
        player
            .inventory
            .insert(InventoryType::Cash, 2, Item::stack(5120000, 3));
        player
    }

    fn left(player: &Player) -> i16 {
        player
            .inventory
            .get(InventoryType::Cash, 2)
            .map_or(0, |item| item.quantity)
    }

    #[test]
    fn a_weather_item_blows_over_the_map() {
        let mut player = snowman();
        let actions = use_weather(&mut player, "let it snow");
        let [WorldAction::Reply(used), WorldAction::Field(FieldRequest::Weather { item_id, message })] =
            &actions[..]
        else {
            panic!("expected a reply and a weather: {actions:?}");
        };
        assert_eq!(used.bytes[..2], [0x1D, 0], "an inventory operation");
        assert_eq!(*item_id, 5120000);
        if crate::items::cash_message(5120000).is_some() {
            assert_eq!(message, "Tester's snow : let it snow");
        }
        assert_eq!(left(&player), 2);
    }

    #[test]
    fn a_weather_needs_a_message_of_at_most_35() {
        for text in ["", &"a".repeat(36)] {
            let mut player = snowman();
            let actions = use_weather(&mut player, text);
            let [WorldAction::Reply(packet)] = &actions[..] else {
                panic!("expected a lone reply: {actions:?}");
            };
            assert_eq!(packet.bytes[..2], [0x1F, 0], "a stat change to unlock");
            assert_eq!(left(&player), 3);
        }
    }

    #[test]
    fn fill_replaces_each_placeholder_in_turn() {
        assert_eq!(fill("%s's snow : %s", &["Al", "hi"]), "Al's snow : hi");
        assert_eq!(fill("%s", &["Al", "hi"]), "Al");
        assert_eq!(fill("no name", &["Al"]), "no name");
    }
}
