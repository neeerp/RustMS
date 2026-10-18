//! Megaphones (507xxxx), used through USE_CASH_ITEM (report 179, 1.1-1.4).
//! Each line goes out as `<name> : <text>`, as the v83 client expects
//! (FUN_00a22785 looks for the " : "), and the megaphone is used up.

use crate::handler::{BroadcastScope, HandlerResult, WorldAction, WorldContext, WorldResult};
use crate::packet::build::world::inventory::build_inventory_operation;
use crate::packet::build::world::messaging::{
    build_broadcast, build_item_megaphone, build_super_megaphone, build_triple_megaphone, Broadcast,
};
use crate::packet::build::world::stat::build_stat_update;
use game::InventoryType;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// The longest line the v83 client lets a megaphone send (StringPool 0x111).
pub const MAX_LINE: usize = 60;
/// The plain megaphone is for characters over this level (StringPool 0xea1).
pub const MIN_LEVEL: u8 = 10;

/// Use the megaphone `item_id` in Cash `slot`; `r` is past the item id.
pub fn megaphone(
    ctx: &mut WorldContext,
    r: &mut Cursor<&Vec<u8>>,
    slot: i16,
    item_id: i32,
) -> WorldResult {
    let Some((scope, packet)) = read_line(ctx, r, item_id)? else {
        return Ok(HandlerResult::reply(build_stat_update(true, &[])?));
    };
    let Ok(changes) = ctx
        .player
        .inventory_mut()
        .take(InventoryType::Cash, slot, 1)
    else {
        return Ok(HandlerResult::reply(build_stat_update(true, &[])?));
    };
    Ok(
        HandlerResult::reply(build_inventory_operation(true, &changes)?)
            .with(WorldAction::Broadcast { scope, packet }),
    )
}

/// What the megaphone says and to whom, or `None` when it can't be used
/// (FUN_0048645b's 507 kinds: 1 and 8 plain, 2 super, 5 above 5 and 6
/// item, 7 triple; 5 up to 5 is Maple TV).
fn read_line(
    ctx: &WorldContext,
    r: &mut Cursor<&Vec<u8>>,
    item_id: i32,
) -> Result<Option<(BroadcastScope, Packet)>, crate::error::NetworkError> {
    let channel = ctx.channel_id;
    let said = |text: &str| format!("{} : {}", ctx.player.name(), text);
    let read = match (item_id % 10_000 / 1_000, item_id % 10) {
        (1 | 8, _) => {
            let text = r.read_str_with_length()?;
            if ctx.player.stats.level <= MIN_LEVEL || !fits(&text) {
                return Ok(None);
            }
            (
                BroadcastScope::Channel(channel),
                build_broadcast(Broadcast::Megaphone, &said(&text))?,
            )
        }
        (2, _) => {
            let text = r.read_str_with_length()?;
            let ear = r.read_byte()? != 0;
            if !fits(&text) {
                return Ok(None);
            }
            (
                BroadcastScope::World,
                build_super_megaphone(&said(&text), channel, ear)?,
            )
        }
        (5, 6..) | (6, _) => {
            let text = r.read_str_with_length()?;
            let ear = r.read_byte()? != 0;
            let item = if r.read_byte()? != 0 {
                let kind = InventoryType::from_i16(r.read_int()? as i16)
                    .filter(|&kind| kind != InventoryType::Equipped);
                let slot = r.read_int()? as i16;
                match kind.and_then(|kind| ctx.player.inventory.get(kind, slot)) {
                    Some(item) => Some(item),
                    None => return Ok(None),
                }
            } else {
                None
            };
            if !fits(&text) {
                return Ok(None);
            }
            (
                BroadcastScope::World,
                build_item_megaphone(&said(&text), channel, ear, item)?,
            )
        }
        (7, _) => {
            let count = r.read_byte()?;
            if !(1..=3).contains(&count) {
                return Ok(None);
            }
            let mut lines = Vec::new();
            for _ in 0..count {
                lines.push(r.read_str_with_length()?);
            }
            let ear = r.read_byte()? != 0;
            if !lines.iter().all(|line| fits(line)) {
                return Ok(None);
            }
            let lines: Vec<String> = lines.iter().map(|line| said(line)).collect();
            (
                BroadcastScope::World,
                build_triple_megaphone(&lines, channel, ear)?,
            )
        }
        _ => return Ok(None),
    };
    Ok(Some(read))
}

/// A line the client would have sent: not empty and at most 60 long.
fn fits(text: &str) -> bool {
    !text.is_empty() && text.len() <= MAX_LINE
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;
    use crate::handler::WorldHandler;
    use crate::packet::handle::world::UseCashItemHandler;
    use crate::packet::op::RecvOpcode;
    use game::{Item, Player};
    use packet::io::write::PktWrite;

    /// "Tester", level `level`, with `item_id` in Cash slot 2 and a red
    /// potion in USE slot 4.
    fn speaker(level: u8, item_id: i32) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.stats.level = level;
        player
            .inventory
            .insert(InventoryType::Cash, 2, Item::stack(item_id, 3));
        player
            .inventory
            .insert(InventoryType::Use, 4, Item::stack(2000000, 7));
        player
    }

    fn use_item(player: &mut Player, item_id: i32, body: impl Fn(&mut Packet)) -> Vec<WorldAction> {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
        p.write_short(2).unwrap();
        p.write_int(item_id).unwrap();
        body(&mut p);
        p.write_int(1234).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 2,
            player,
        };
        UseCashItemHandler.handle(&mut p, &mut ctx).unwrap().actions
    }

    /// The line sent to `scope`, after the reply that used the megaphone up.
    fn broadcast(actions: &[WorldAction], expected: BroadcastScope) -> &Packet {
        let [WorldAction::Reply(used), WorldAction::Broadcast { scope, packet }] = actions else {
            panic!("expected a reply and a broadcast: {actions:?}");
        };
        assert_eq!(used.bytes[..2], [0x1D, 0], "an inventory operation");
        assert_eq!(*scope, expected);
        packet
    }

    fn refused(actions: &[WorldAction]) {
        let [WorldAction::Reply(packet)] = actions else {
            panic!("expected a lone reply: {actions:?}");
        };
        assert_eq!(packet.bytes[..2], [0x1F, 0], "a stat change to unlock");
    }

    fn left(player: &Player) -> i16 {
        player
            .inventory
            .get(InventoryType::Cash, 2)
            .map_or(0, |item| item.quantity)
    }

    #[test]
    fn a_megaphone_speaks_to_the_channel() {
        let mut player = speaker(11, 5071000);
        let actions = use_item(&mut player, 5071000, |p| {
            p.write_str_with_length("hi all").unwrap();
        });
        Expect::new(&broadcast(&actions, BroadcastScope::Channel(2)).bytes)
            .short(0x44)
            .byte(2)
            .string("Tester : hi all")
            .end();
        assert_eq!(left(&player), 2);
    }

    #[test]
    fn a_megaphone_needs_level_11() {
        let mut player = speaker(10, 5071000);
        refused(&use_item(&mut player, 5071000, |p| {
            p.write_str_with_length("hi").unwrap();
        }));
        assert_eq!(left(&player), 3);
    }

    #[test]
    fn a_super_megaphone_speaks_to_the_world() {
        let mut player = speaker(1, 5072000);
        let actions = use_item(&mut player, 5072000, |p| {
            p.write_str_with_length("buying").unwrap();
            p.write_byte(1).unwrap();
        });
        Expect::new(&broadcast(&actions, BroadcastScope::World).bytes)
            .short(0x44)
            .byte(3)
            .string("Tester : buying")
            .byte(2)
            .byte(1)
            .end();
    }

    #[test]
    fn a_line_longer_than_60_is_refused() {
        let mut player = speaker(30, 5072000);
        refused(&use_item(&mut player, 5072000, |p| {
            p.write_str_with_length(&"a".repeat(61)).unwrap();
            p.write_byte(0).unwrap();
        }));
        assert_eq!(left(&player), 3);
    }

    #[test]
    fn an_item_megaphone_shows_the_item() {
        let mut player = speaker(1, 5076000);
        let actions = use_item(&mut player, 5076000, |p| {
            p.write_str_with_length("look").unwrap();
            p.write_byte(0).unwrap();
            p.write_byte(1).unwrap();
            p.write_int(2).unwrap();
            p.write_int(4).unwrap();
        });
        let mut expect = Expect::new(&broadcast(&actions, BroadcastScope::World).bytes);
        expect
            .short(0x44)
            .byte(8)
            .string("Tester : look")
            .byte(2)
            .byte(0);
        expect.byte(1).byte(2).int(2000000);
    }

    #[test]
    fn an_item_megaphone_without_the_item_is_refused() {
        let mut player = speaker(1, 5076000);
        refused(&use_item(&mut player, 5076000, |p| {
            p.write_str_with_length("look").unwrap();
            p.write_byte(0).unwrap();
            p.write_byte(1).unwrap();
            p.write_int(2).unwrap();
            p.write_int(5).unwrap();
        }));
    }

    #[test]
    fn a_triple_megaphone_names_every_line() {
        let mut player = speaker(1, 5077000);
        let actions = use_item(&mut player, 5077000, |p| {
            p.write_byte(2).unwrap();
            p.write_str_with_length("one").unwrap();
            p.write_str_with_length("two").unwrap();
            p.write_byte(1).unwrap();
        });
        Expect::new(&broadcast(&actions, BroadcastScope::World).bytes)
            .short(0x44)
            .byte(10)
            .string("Tester : one")
            .byte(2)
            .string("Tester : two")
            .byte(2)
            .byte(1)
            .end();
    }

    #[test]
    fn maple_tv_is_not_a_megaphone() {
        let mut player = speaker(30, 5075000);
        refused(&use_item(&mut player, 5075000, |_| {}));
    }
}
