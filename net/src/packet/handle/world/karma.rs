//! The Scissors of Karma (5520000): an item whose data has
//! `tradeAvailable` may change hands once more (report c010). The client
//! sends USE_CASH_ITEM once an item dropped on the scissors' window is
//! confirmed, with the item's inventory type and slot, as the Item Guards
//! do.

use super::item_tag::{answer, restamp};
use crate::handler::{WorldContext, WorldResult};
use crate::items;
use game::inventory::FLAG_UNTRADEABLE;
use game::{InventoryType, Item};
use packet::io::read::PktRead;
use std::io::Cursor;

pub const KARMA_SCISSORS: i32 = 5_520_000;

/// Give the item the request names its Karma, with the scissors in Cash
/// `slot`; `r` is past the item id.
pub fn karma_scissors(ctx: &mut WorldContext, r: &mut Cursor<&Vec<u8>>, slot: i16) -> WorldResult {
    let tab = r.read_int()?;
    let target = r.read_int()?;
    // Only from the Item window.
    let kind = i16::try_from(tab)
        .ok()
        .and_then(InventoryType::from_i16)
        .filter(|kind| *kind != InventoryType::Equipped);
    let (Some(kind), Ok(target @ 1..)) = (kind, i16::try_from(target)) else {
        return answer(None);
    };
    answer(restamp(ctx.player, slot, kind, target, cut))
}

/// v83's test of an item dropped on the window (0x830ef7): not sealed, with
/// `tradeAvailable`, and not given its Karma yet. Equipment whose Karma was
/// used up stays bound.
fn cut(item: &mut Item) -> bool {
    let trade_available = items::info(item.item_id).is_some_and(|info| info.trade_available);
    let bound = item.equip.is_some() && item.flags & FLAG_UNTRADEABLE != 0;
    if item.sealed() || !trade_available || item.has_karma() || bound {
        return false;
    }
    item.give_karma();
    true
}

#[cfg(test)]
mod tests {
    use super::KARMA_SCISSORS;
    use crate::handler::{WorldContext, WorldHandler};
    use crate::packet::handle::world::UseCashItemHandler;
    use crate::packet::op::RecvOpcode;
    use game::inventory::{FLAG_LOCK, FLAG_UNTRADEABLE};
    use game::{InventoryType, Item, Player};
    use packet::{io::write::PktWrite, Packet};

    /// A weapon with `tradeAvailable`.
    const SWORD: i32 = 1_302_112;
    /// A skill book with it.
    const BOOK: i32 = 2_280_004;

    fn cut(player: &mut Player, tab: i32) {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
        p.write_short(1).unwrap();
        p.write_int(KARMA_SCISSORS).unwrap();
        p.write_int(tab).unwrap();
        p.write_int(1).unwrap();
        p.write_int(1234).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 2,
            player,
        };
        UseCashItemHandler.handle(&mut p, &mut ctx).unwrap();
    }

    fn holding(kind: InventoryType, item: Item) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.inventory.insert(kind, 1, item);
        player
            .inventory
            .insert(InventoryType::Cash, 1, Item::stack(KARMA_SCISSORS, 1));
        player
    }

    fn spent(player: &Player) -> bool {
        player.inventory.get(InventoryType::Cash, 1).is_none()
    }

    fn held(player: &Player, kind: InventoryType) -> &Item {
        player.inventory.get(kind, 1).unwrap()
    }

    #[test]
    fn scissors_let_an_item_change_hands_once() {
        let Some(sword) = crate::items::create(SWORD, 1) else {
            return;
        };
        let mut player = holding(InventoryType::Equip, sword);
        cut(&mut player, 1);
        assert!(spent(&player));
        assert!(held(&player, InventoryType::Equip).has_karma());

        let mut player = holding(InventoryType::Use, Item::stack(BOOK, 1));
        cut(&mut player, 2);
        assert!(spent(&player));
        assert_eq!(held(&player, InventoryType::Use).flags, 0x02);
    }

    #[test]
    fn scissors_leave_what_they_cant_cut() {
        let Some(sword) = crate::items::create(SWORD, 1) else {
            return;
        };
        let refused = |kind, item: Item, tab| {
            let flags = item.flags;
            let mut player = holding(kind, item);
            cut(&mut player, tab);
            assert!(!spent(&player));
            assert_eq!(held(&player, kind).flags, flags);
        };
        // No `tradeAvailable`.
        refused(InventoryType::Use, Item::stack(2_000_000, 5), 2);
        let mut sealed = sword.clone();
        sealed.flags |= FLAG_LOCK;
        refused(InventoryType::Equip, sealed, 1);
        let mut cut_once = sword.clone();
        cut_once.give_karma();
        refused(InventoryType::Equip, cut_once, 1);
        // Its Karma used up, an equip stays bound.
        let mut bound = sword.clone();
        bound.flags |= FLAG_UNTRADEABLE;
        refused(InventoryType::Equip, bound, 1);
        // The tab must be the item's, and not the worn equips'.
        let mut player = holding(InventoryType::Equip, sword);
        cut(&mut player, 2);
        cut(&mut player, -1);
        assert!(!spent(&player));
    }
}
