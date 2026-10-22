//! Maple TV (5075000-5075005) through USE_CASH_ITEM (report 179, 1.6 and
//! 6). The world puts the message in the TV's line; the item goes once it
//! is in line, and the sender hears how it went (recv 343).

use crate::handler::{BroadcastScope, HandlerResult, WorldAction, WorldContext, WorldResult};
use crate::maple_tv::MapleTv;
use crate::packet::build::world::inventory::build_inventory_operation;
use crate::packet::build::world::maple_tv::{build_tv_result, TvRefusal};
use crate::packet::build::world::stat::build_stat_update;
use game::InventoryType;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// The longest line taken. The dialog's lines are the soft-wrapped rows of
/// one edit 125 or 170 px wide (report c056 section 3.2), far shorter.
pub const MAX_LINE: usize = 60;

/// Use the Maple TV `item_id` in Cash `slot`; `r` is past the item id.
/// Messenger and Megassenger start with a mode (3 with a receiver), the
/// three Mega kinds with the ear, and all but the Star kinds name a
/// receiver (the Heart kinds must). Five lines follow.
pub fn maple_tv(
    ctx: &mut WorldContext,
    r: &mut Cursor<&Vec<u8>>,
    slot: i16,
    item_id: i32,
) -> WorldResult {
    let kind = item_id % 10;
    if kind > 5 {
        return Ok(HandlerResult::reply(build_stat_update(true, &[])?));
    }
    if kind % 3 == 0 {
        r.read_byte()?;
    }
    let ear = kind >= 3 && r.read_byte()? != 0;
    let receiver = if kind % 3 == 1 {
        String::new()
    } else {
        r.read_str_with_length()?
    };
    let mut lines: [String; 5] = Default::default();
    for line in &mut lines {
        *line = r.read_str_with_length()?;
    }
    let heart = kind % 3 == 2;
    // The client sends empty text (it has no check of its own).
    if (heart && receiver.is_empty()) || lines.iter().any(|line| line.len() > MAX_LINE) {
        return Ok(HandlerResult::reply(build_stat_update(true, &[])?));
    }
    let tv = MapleTv {
        item_id,
        sender: ctx.player.name().to_string(),
        look: ctx.player.look(),
        receiver: (!receiver.is_empty()).then_some(receiver),
        lines,
        scope: if ear {
            BroadcastScope::World
        } else {
            BroadcastScope::Channel(ctx.channel_id)
        },
    };
    Ok(HandlerResult::empty().with(WorldAction::MapleTv { slot, tv }))
}

/// Use up the Maple TV the world has put `item_id`'s message in line for,
/// or tell the sender why it didn't.
pub fn spend_maple_tv(
    player: &mut game::Player,
    slot: i16,
    item_id: i32,
    queued: Result<(), TvRefusal>,
) -> Result<Vec<Packet>, crate::error::NetworkError> {
    if let Err(why) = queued {
        return Ok(vec![
            build_tv_result(Some(why))?,
            build_stat_update(true, &[])?,
        ]);
    }
    let held = player
        .inventory
        .get(InventoryType::Cash, slot)
        .is_some_and(|item| item.item_id == item_id);
    let taken = held
        .then(|| {
            player
                .inventory_mut()
                .take(InventoryType::Cash, slot, 1)
                .ok()
        })
        .flatten();
    let used = match taken {
        Some(changes) => build_inventory_operation(true, &changes)?,
        None => build_stat_update(true, &[])?,
    };
    Ok(vec![build_tv_result(None)?, used])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;
    use crate::handler::WorldHandler;
    use crate::packet::build::world::maple_tv::build_send_tv;
    use crate::packet::handle::world::UseCashItemHandler;
    use crate::packet::op::RecvOpcode;
    use game::{Item, Player};
    use packet::io::write::PktWrite;

    /// "Tester" with `item_id` in Cash slot 2.
    fn sender(item_id: i32) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player
            .inventory
            .insert(InventoryType::Cash, 2, Item::stack(item_id, 3));
        player
    }

    /// The fields before the lines (`head`), then the five lines.
    fn use_tv(
        player: &mut Player,
        item_id: i32,
        head: impl Fn(&mut Packet),
        lines: [&str; 5],
    ) -> Vec<WorldAction> {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
        p.write_short(2).unwrap();
        p.write_int(item_id).unwrap();
        head(&mut p);
        for line in lines {
            p.write_str_with_length(line).unwrap();
        }
        p.write_int(1234).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 2,
            player,
        };
        UseCashItemHandler.handle(&mut p, &mut ctx).unwrap().actions
    }

    fn queued(actions: Vec<WorldAction>) -> MapleTv {
        let [WorldAction::MapleTv { slot: 2, tv }] = <[_; 1]>::try_from(actions).unwrap() else {
            panic!("expected the message put in line");
        };
        tv
    }

    fn refused(actions: &[WorldAction]) {
        let [WorldAction::Reply(packet)] = actions else {
            panic!("expected a lone reply: {actions:?}");
        };
        assert_eq!(packet.bytes[..2], [0x1F, 0], "a stat change to unlock");
    }

    const LINES: [&str; 5] = ["hello", "", "world", "", ""];

    #[test]
    fn a_messenger_names_its_receiver_after_the_mode() {
        let mut player = sender(5075000);
        let tv = queued(use_tv(
            &mut player,
            5075000,
            |p| {
                p.write_byte(3).unwrap();
                p.write_str_with_length("Friend").unwrap();
            },
            LINES,
        ));
        assert_eq!(tv.sender, "Tester");
        assert_eq!(tv.receiver.as_deref(), Some("Friend"));
        assert_eq!(tv.lines[2], "world");
        assert_eq!(tv.scope, BroadcastScope::Channel(2));
        assert_eq!(tv.screen(), 0);
    }

    #[test]
    fn a_star_has_no_receiver_and_a_messenger_may_not() {
        let mut player = sender(5075001);
        let tv = queued(use_tv(&mut player, 5075001, |_| {}, LINES));
        assert_eq!((tv.screen(), tv.receiver), (1, None));
        let mut player = sender(5075000);
        let tv = queued(use_tv(
            &mut player,
            5075000,
            |p| {
                p.write_byte(1).unwrap();
                p.write_str_with_length("").unwrap();
            },
            LINES,
        ));
        assert_eq!(tv.receiver, None);
    }

    #[test]
    fn the_mega_kinds_reach_the_world_with_the_ear() {
        let mut player = sender(5075003);
        let tv = queued(use_tv(
            &mut player,
            5075003,
            |p| {
                p.write_byte(1).unwrap();
                p.write_byte(1).unwrap();
                p.write_str_with_length("").unwrap();
            },
            LINES,
        ));
        assert_eq!(tv.scope, BroadcastScope::World);
        let mut player = sender(5075004);
        let tv = queued(use_tv(
            &mut player,
            5075004,
            |p| {
                p.write_byte(0).unwrap();
            },
            LINES,
        ));
        assert_eq!(tv.scope, BroadcastScope::Channel(2));
        let mut player = sender(5075005);
        let tv = queued(use_tv(
            &mut player,
            5075005,
            |p| {
                p.write_byte(1).unwrap();
                p.write_str_with_length("Friend").unwrap();
            },
            LINES,
        ));
        assert_eq!((tv.scope, tv.screen()), (BroadcastScope::World, 2));
    }

    #[test]
    fn a_heart_needs_a_receiver_and_a_line_must_fit() {
        let mut player = sender(5075002);
        let no_one = |p: &mut Packet| {
            p.write_str_with_length("").unwrap();
        };
        refused(&use_tv(&mut player, 5075002, no_one, LINES));
        let friend = |p: &mut Packet| {
            p.write_str_with_length("Friend").unwrap();
        };
        let long = "x".repeat(MAX_LINE + 1);
        refused(&use_tv(
            &mut player,
            5075002,
            friend,
            ["hi", &long, "", "", ""],
        ));
        assert_eq!(
            player
                .inventory
                .get(InventoryType::Cash, 2)
                .unwrap()
                .quantity,
            3
        );
    }

    #[test]
    fn a_message_in_line_uses_the_item_up_and_a_refused_one_keeps_it() {
        let mut player = sender(5075001);
        let refused = spend_maple_tv(&mut player, 2, 5075001, Err(TvRefusal::WrongName)).unwrap();
        Expect::new(&refused[0].bytes)
            .short(0x157)
            .byte(1)
            .byte(2)
            .end();
        assert_eq!(refused[1].bytes[..2], [0x1F, 0]);
        assert_eq!(
            player
                .inventory
                .get(InventoryType::Cash, 2)
                .unwrap()
                .quantity,
            3
        );
        let used = spend_maple_tv(&mut player, 2, 5075001, Ok(())).unwrap();
        Expect::new(&used[0].bytes).short(0x157).byte(0).end();
        assert_eq!(used[1].bytes[..2], [0x1D, 0], "an inventory operation");
        assert_eq!(
            player
                .inventory
                .get(InventoryType::Cash, 2)
                .unwrap()
                .quantity,
            2
        );
    }

    #[test]
    fn the_tv_shows_both_looks_and_the_wait() {
        let player = sender(5075002);
        let tv = MapleTv {
            item_id: 5075002,
            sender: "Tester".into(),
            look: player.look(),
            receiver: Some("Friend".into()),
            lines: ["a", "b", "c", "d", "e"].map(String::from),
            scope: BroadcastScope::Channel(0),
        };
        let packet = build_send_tv(&tv, Some(&player.look()), 75).unwrap();
        Expect::new(&packet.bytes)
            .short(0x155)
            .byte(3)
            .byte(2)
            .look(&player.look(), false)
            .string("Tester")
            .string("Friend")
            .string("a")
            .string("b")
            .string("c")
            .string("d")
            .string("e")
            .int(75)
            .look(&player.look(), false)
            .end();
        let alone = MapleTv {
            receiver: None,
            ..tv
        };
        let packet = build_send_tv(&alone, None, 15).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(0x155)
            .byte(1)
            .byte(2)
            .look(&player.look(), false);
        expect.string("Tester").string("");
        for line in ["a", "b", "c", "d", "e"] {
            expect.string(line);
        }
        expect.int(15).end();
    }
}
