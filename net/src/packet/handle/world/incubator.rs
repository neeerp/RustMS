//! The Incubator (5060002), which hatches a Pigmy Egg (report c035). The
//! client's Incubator window sends USE_CASH_ITEM with the egg's inventory
//! type and slot once its OK is confirmed. The answer is what the egg held
//! (INCUBATOR_RESULT), then the inventory changes, which unlock the client.

use crate::handler::{HandlerResult, WorldContext, WorldResult};
use crate::items;
use crate::packet::build::world::inventory::{build_incubator_result, build_inventory_operation};
use crate::packet::build::world::stat::build_stat_update;
use game::{InventoryChange, InventoryType, Player};
use packet::io::read::PktRead;
use std::io::Cursor;

pub const INCUBATOR: i32 = 5_060_002;
/// The Pigmy Eggs the window takes (0x4f46d2); not the DS Egg 4170012.
const EGGS: std::ops::RangeInclusive<i32> = 4_170_000..=4_170_009;
/// The wire type of the Etc tab.
const ETC_TAB: i32 = 4;

/// What an egg may hold, each as likely as the next. Neither the client nor
/// any server we can read has the real table; this is Cosmic's list
/// (lanes/QUESTIONS.md, 2026-10-22).
const PRIZES: [(i32, i16); 20] = [
    (1_012_070, 1),   // Strawberry Icecream Bar
    (1_302_049, 1),   // Glowing Whip
    (1_302_063, 1),   // Flaming Katana
    (1_322_027, 1),   // Frying Pan
    (2_000_004, 240), // Elixir
    (2_000_005, 200), // Power Elixir
    (2_020_013, 200), // Reindeer Milk
    (2_020_015, 200), // Sunset Dew
    (2_040_307, 1),   // Dark Scroll for Earring for DEX
    (2_040_509, 1),   // Dark Scroll for Overall Armor for DEX
    (2_040_519, 1),   // Dark Scroll for Overall Armor for INT
    (2_040_521, 1),   // Dark Scroll for Overall Armor for LUK
    (2_040_533, 1),   // Scroll for Overall Armor for STR 30%
    (2_040_715, 1),   // Dark Scroll for Shoes for Jump
    (2_040_717, 1),   // Dark Scroll for Shoes for Speed
    (2_040_810, 1),   // Dark Scroll for Gloves for ATT
    (2_040_811, 1),   // Dark Scroll for Gloves for ATT
    (2_070_005, 1),   // Steely Throwing-Knives
    (2_070_006, 1),   // Ilbi Throwing-Stars
    (4_020_009, 3),   // Piece of Time
];

/// The free slots the window's question asks for (SP 0xd63), counted with
/// the egg and the incubator gone.
const ROOM: [(InventoryType, usize); 3] = [
    (InventoryType::Equip, 1),
    (InventoryType::Use, 3),
    (InventoryType::Etc, 1),
];

/// Hatch the egg the request names with the incubator in Cash `slot`; `r`
/// is past the item id.
pub fn incubator(ctx: &mut WorldContext, r: &mut Cursor<&Vec<u8>>, slot: i16) -> WorldResult {
    let tab = r.read_int()?;
    let egg_slot = r.read_int()?;
    let prize = PRIZES[rand::random::<usize>() % PRIZES.len()];
    let unlock = || Ok(HandlerResult::reply(build_stat_update(true, &[])?));
    let Ok(egg_slot) = i16::try_from(egg_slot) else {
        return unlock();
    };
    if tab != ETC_TAB || ctx.player.stats.hp <= 0 {
        return unlock();
    }
    match hatch(ctx.player, slot, egg_slot, prize) {
        Hatch::Bad => unlock(),
        Hatch::Full => Ok(HandlerResult::reply(build_incubator_result(0, 0)?)
            .with_reply(build_stat_update(true, &[])?)),
        Hatch::Done(changes) => {
            let (item_id, count) = prize;
            Ok(
                HandlerResult::reply(build_incubator_result(item_id, count)?)
                    .with_reply(build_inventory_operation(true, &changes)?),
            )
        }
    }
}

enum Hatch {
    /// Not an incubator and an egg: the client never sends it.
    Bad,
    Full,
    Done(Vec<InventoryChange>),
}

/// Spend the incubator in Cash `slot` and the egg in Etc `egg_slot` on
/// `prize`. Nothing changes unless it all fits.
fn hatch(player: &mut Player, slot: i16, egg_slot: i16, (item_id, count): (i32, i16)) -> Hatch {
    let held = |kind, slot, ids: &dyn Fn(i32) -> bool| {
        player
            .inventory
            .get(kind, slot)
            .is_some_and(|item| ids(item.item_id))
    };
    if !held(InventoryType::Cash, slot, &|id| id == INCUBATOR)
        || !held(InventoryType::Etc, egg_slot, &|id| EGGS.contains(&id))
    {
        return Hatch::Bad;
    }
    let mut next = player.inventory.clone();
    let Ok(mut changes) = next.take(InventoryType::Cash, slot, 1) else {
        return Hatch::Bad;
    };
    let Ok(egg) = next.take(InventoryType::Etc, egg_slot, 1) else {
        return Hatch::Bad;
    };
    changes.extend(egg);
    if ROOM.iter().any(|&(kind, room)| next.free(kind) < room) {
        return Hatch::Full;
    }
    let Ok(prize) = items::give(&mut next, item_id, count) else {
        return Hatch::Full;
    };
    changes.extend(prize);
    *player.inventory_mut() = next;
    Hatch::Done(changes)
}

#[cfg(test)]
mod tests {
    use super::{hatch, Hatch, INCUBATOR, PRIZES};
    use crate::handler::{WorldAction, WorldContext, WorldHandler};
    use crate::packet::handle::world::UseCashItemHandler;
    use crate::packet::op::{RecvOpcode, SendOpcode};
    use game::{InventoryType, Item, Player};
    use packet::{io::write::PktWrite, Packet};

    const EGG: i32 = 4_170_000;
    const ELIXIR: (i32, i16) = (2_000_004, 240);

    fn holding(egg: i32) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player
            .inventory
            .insert(InventoryType::Etc, 2, Item::stack(egg, 3));
        player
            .inventory
            .insert(InventoryType::Cash, 1, Item::stack(INCUBATOR, 1));
        player
    }

    /// The opcodes of the answer to the window's request.
    fn incubate(player: &mut Player, tab: i32, egg_slot: i32) -> Vec<i16> {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
        p.write_short(1).unwrap();
        p.write_int(INCUBATOR).unwrap();
        p.write_int(tab).unwrap();
        p.write_int(egg_slot).unwrap();
        p.write_int(1234).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 2,
            player,
        };
        UseCashItemHandler
            .handle(&mut p, &mut ctx)
            .unwrap()
            .actions
            .into_iter()
            .map(|action| match action {
                WorldAction::Reply(p) => i16::from_le_bytes([p.bytes[0], p.bytes[1]]),
                other => panic!("unexpected {other:?}"),
            })
            .collect()
    }

    fn eggs(player: &Player) -> i16 {
        player
            .inventory
            .get(InventoryType::Etc, 2)
            .map_or(0, |item| item.quantity)
    }

    fn spent(player: &Player) -> bool {
        player.inventory.get(InventoryType::Cash, 1).is_none()
    }

    #[test]
    fn every_prize_is_an_item() {
        if crate::items::info(EGG).is_none() {
            return;
        }
        for (item_id, _) in PRIZES {
            assert!(crate::items::info(item_id).is_some(), "{item_id}");
        }
    }

    #[test]
    fn an_egg_hatches_into_its_prize() {
        if crate::items::info(ELIXIR.0).is_none() {
            return;
        }
        let mut player = holding(EGG);
        assert!(matches!(hatch(&mut player, 1, 2, ELIXIR), Hatch::Done(_)));
        assert!(spent(&player));
        assert_eq!(eggs(&player), 2);
        assert_eq!(player.inventory.count(ELIXIR.0), 240);

        let mut player = holding(4_170_009);
        let opcodes = incubate(&mut player, 4, 2);
        assert_eq!(
            opcodes,
            [
                SendOpcode::IncubatorResult as i16,
                SendOpcode::InventoryOperation as i16
            ]
        );
        assert!(spent(&player));
        assert_eq!(eggs(&player), 2);
        assert!(PRIZES
            .iter()
            .any(|&(id, count)| player.inventory.count(id) == i32::from(count)));
    }

    #[test]
    fn a_full_inventory_keeps_the_egg() {
        let mut player = holding(EGG);
        player.inventory.set_slot_limit(InventoryType::Use, 4);
        for slot in 1..=2 {
            player
                .inventory
                .insert(InventoryType::Use, slot, Item::stack(2_000_000, 1));
        }
        assert!(matches!(
            hatch(&mut player, 1, 2, (4_020_009, 3)),
            Hatch::Full
        ));
        let opcodes = incubate(&mut player, 4, 2);
        assert_eq!(
            opcodes,
            [
                SendOpcode::IncubatorResult as i16,
                SendOpcode::StatChange as i16
            ]
        );
        assert!(!spent(&player));
        assert_eq!(eggs(&player), 3);
    }

    #[test]
    fn only_a_pigmy_egg_hatches() {
        let unlocked = [SendOpcode::StatChange as i16];
        // The DS Egg.
        let mut player = holding(4_170_012);
        assert_eq!(incubate(&mut player, 4, 2), unlocked);
        let mut player = holding(EGG);
        // Not the Etc tab, or not the egg's slot.
        assert_eq!(incubate(&mut player, 2, 2), unlocked);
        assert_eq!(incubate(&mut player, 4, 3), unlocked);
        assert_eq!(incubate(&mut player, 4, 70_000), unlocked);
        // No incubator.
        player.inventory.take(InventoryType::Cash, 1, 1).unwrap();
        assert_eq!(incubate(&mut player, 4, 2), unlocked);
        assert_eq!(eggs(&player), 3);
    }
}
