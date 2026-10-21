//! Vega's Spell (5610000, 5610001), used through USE_CASH_ITEM with an
//! equip from the Equip tab and a scroll from the Use tab (v83 FUN_0082cbe2,
//! type 0x44; reports c004 and c004c). The 10% one makes a 10% scroll
//! work 30% of the time, the 60% one makes a 60% scroll work 90%. The
//! client lets only the scrolls in `Etc.nx`'s list into the window and
//! checks no slots or rates, so all of that is checked here.
//!
//! The roll is decided at once: the start code tells the window which
//! result to show, and the result, with the changed items, follows when its
//! gauge is full, so the equip's new stats don't show early.

use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldResult};
use crate::items;
use crate::packet::build::world::inventory::build_inventory_operation;
use crate::packet::build::world::stat::build_stat_update;
use crate::packet::build::world::vega::{build_vega_result, build_vega_start};
use game::scroll::{self, ScrollOutcome};
use game::{InventoryChange, InventoryType, Player};
use packet::io::read::PktRead;
use rand::Rng;
use std::io::Cursor;
use std::time::Duration;

/// When the result follows the start. The window's gauge fills in about
/// 2.2 s (140 px at 63 px/s); v83's server answered after about 2.5 s.
pub const ROLL: Duration = Duration::from_millis(2_600);

/// The wire types of the Equip and Use tabs.
const EQUIP_TAB: i32 = 1;
const USE_TAB: i32 = 2;

/// Cast the Vega's Spell `item_id` in Cash `slot`; `r` is past the item id.
pub fn vega_spell(
    ctx: &mut WorldContext,
    r: &mut Cursor<&Vec<u8>>,
    slot: i16,
    item_id: i32,
) -> WorldResult {
    let equip_tab = r.read_int()?;
    let equip_slot = r.read_int()?;
    let scroll_tab = r.read_int()?;
    let scroll_slot = r.read_int()?;
    let slots = (i16::try_from(equip_slot), i16::try_from(scroll_slot));
    let (Ok(equip_slot), Ok(scroll_slot)) = slots else {
        return refuse();
    };
    if (equip_tab, scroll_tab) != (EQUIP_TAB, USE_TAB) || ctx.player.stats.hp <= 0 {
        return refuse();
    }
    let cast = Cast {
        slot,
        item_id,
        equip_slot,
        scroll_slot,
    };
    let Some((changes, outcome)) = cast.apply(ctx.player, &mut |lo, hi| {
        rand::thread_rng().gen_range(lo, hi)
    }) else {
        return refuse();
    };
    ctx.player.mark_dirty();
    let success = outcome == ScrollOutcome::Success;
    let later = |packet| WorldAction::ReplyLater {
        after: ROLL,
        packet,
    };
    Ok(HandlerResult::reply(build_vega_start(success)?)
        .with(later(build_inventory_operation(true, &changes)?))
        .with(later(build_vega_result(success)?)))
}

struct Cast {
    slot: i16,
    item_id: i32,
    equip_slot: i16,
    scroll_slot: i16,
}

impl Cast {
    /// Use the scroll on the equip at the spell's rate; `roll` is
    /// `scroll::apply`'s. `None` (nothing changes) unless the spell, the
    /// scroll and the equip exist, the spell takes the scroll, and the
    /// scroll fits the equip. The changes are the spell and the scroll
    /// taken, then the equip removed and, unless destroyed, added back.
    fn apply(
        &self,
        player: &mut Player,
        roll: &mut impl FnMut(i32, i32) -> i32,
    ) -> Option<(Vec<InventoryChange>, ScrollOutcome)> {
        let scroll_id = player
            .inventory
            .get(InventoryType::Use, self.scroll_slot)?
            .item_id;
        let success = match (self.item_id, items::vega_rate(scroll_id)?) {
            (5_610_000, 10) => 30,
            (5_610_001, 60) => 90,
            _ => return None,
        };
        let mut scroll = items::info(scroll_id)?.scroll?;
        let mut item = player
            .inventory
            .get(InventoryType::Equip, self.equip_slot)?
            .clone();
        let base_slots = items::info(item.item_id)?.equip?.upgrade_slots;
        let fits = scroll::can_apply(
            scroll_id,
            &scroll,
            item.item_id,
            item.equip.as_ref()?,
            base_slots,
        );
        if item.is_cash() || !fits {
            return None;
        }
        scroll.success = success;
        let equip = item.equip.as_mut()?;
        let outcome = scroll::apply(equip, &mut item.flags, &scroll, false, roll);

        let mut next = player.inventory.clone();
        let mut changes = next.take(InventoryType::Cash, self.slot, 1).ok()?;
        changes.extend(next.take(InventoryType::Use, self.scroll_slot, 1).ok()?);
        changes.extend(next.take(InventoryType::Equip, self.equip_slot, 1).ok()?);
        if outcome != ScrollOutcome::Cursed {
            next.insert(InventoryType::Equip, self.equip_slot, item.clone());
            changes.push(InventoryChange::Add {
                kind: InventoryType::Equip,
                slot: self.equip_slot,
                item,
            });
        }
        *player.inventory_mut() = next;
        Some((changes, outcome))
    }
}

fn refuse() -> WorldResult {
    Ok(HandlerResult::reply(build_stat_update(true, &[])?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::WorldHandler;
    use crate::packet::handle::world::UseCashItemHandler;
    use crate::packet::op::{RecvOpcode, SendOpcode};
    use game::Item;
    use packet::{io::write::PktWrite, Packet};

    const VEGA_10: i32 = 5_610_000;
    const VEGA_60: i32 = 5_610_001;
    /// Work Gloves: 5 upgrade slots.
    const GLOVE: i32 = 1_082_002;
    /// Scrolls for Gloves for DEX, 60% and 10%.
    const DEX_60: i32 = 2_040_801;
    const DEX_10: i32 = 2_040_802;
    /// A Scroll for Helmet for DEF 10%: it doesn't fit gloves.
    const HELMET_10: i32 = 2_040_002;

    fn cast(player: &mut Player, vega: i32, equip_slot: i32, scroll_slot: i32) -> Vec<WorldAction> {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
        p.write_short(if vega == VEGA_10 { 5 } else { 6 }).unwrap();
        p.write_int(vega).unwrap();
        for value in [1, equip_slot, 2, scroll_slot, 1, 1234] {
            p.write_int(value).unwrap();
        }
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        UseCashItemHandler.handle(&mut p, &mut ctx).unwrap().actions
    }

    fn holder() -> Player {
        let mut player = crate::codec::stats::sample_player();
        let inventory = &mut player.inventory;
        inventory.insert(InventoryType::Cash, 5, Item::stack(VEGA_10, 1));
        inventory.insert(InventoryType::Cash, 6, Item::stack(VEGA_60, 1));
        inventory.insert(InventoryType::Equip, 2, items::create(GLOVE, 1).unwrap());
        inventory.insert(InventoryType::Use, 1, Item::stack(DEX_60, 5));
        inventory.insert(InventoryType::Use, 2, Item::stack(DEX_10, 5));
        inventory.insert(InventoryType::Use, 3, Item::stack(HELMET_10, 5));
        player
    }

    fn glove(player: &Player) -> game::EquipStats {
        let item = player.inventory.get(InventoryType::Equip, 2).unwrap();
        item.equip.clone().unwrap()
    }

    fn vega(code: u8) -> Vec<u8> {
        let mut bytes = (SendOpcode::VegaScroll as i16).to_le_bytes().to_vec();
        bytes.push(code);
        bytes
    }

    #[test]
    fn the_list_rates_match_the_spells() {
        assert_eq!(items::vega_rate(DEX_60), Some(60));
        assert_eq!(items::vega_rate(DEX_10), Some(10));
    }

    #[test]
    fn the_start_code_comes_at_once_and_the_result_after_the_roll() {
        let mut player = holder();
        let before = glove(&player);
        let actions = cast(&mut player, VEGA_60, 2, 1);
        let [WorldAction::Reply(start), WorldAction::ReplyLater {
            after,
            packet: items,
        }, WorldAction::ReplyLater {
            after: same,
            packet: result,
        }] = &actions[..]
        else {
            panic!("unexpected {actions:?}");
        };
        assert_eq!((*after, *same), (ROLL, ROLL));
        assert_eq!(
            items.bytes[..2],
            (SendOpcode::InventoryOperation as i16).to_le_bytes()
        );
        let after = glove(&player);
        assert_eq!(after.upgrade_slots, before.upgrade_slots - 1);
        let worked = after.dex > before.dex;
        assert_eq!(start.bytes, vega(if worked { 0x40 } else { 0x45 }));
        assert_eq!(result.bytes, vega(if worked { 0x41 } else { 0x43 }));
        assert!(player.inventory.get(InventoryType::Cash, 6).is_none());
        assert_eq!(
            player
                .inventory
                .get(InventoryType::Use, 1)
                .unwrap()
                .quantity,
            4
        );
    }

    #[test]
    fn the_spell_raises_the_scrolls_rate() {
        // With the roll fixed at 85, a plain 60% scroll fails and the spell's
        // 90% works.
        let mut player = holder();
        let cast = Cast {
            slot: 6,
            item_id: VEGA_60,
            equip_slot: 2,
            scroll_slot: 1,
        };
        let (_, outcome) = cast.apply(&mut player, &mut |_, _| 85).unwrap();
        assert_eq!(outcome, ScrollOutcome::Success);
        let cast = Cast {
            slot: 5,
            item_id: VEGA_10,
            equip_slot: 2,
            scroll_slot: 2,
        };
        assert_eq!(
            cast.apply(&mut player, &mut |_, _| 29).unwrap().1,
            ScrollOutcome::Success
        );
        assert_eq!(glove(&player).level, 2);
    }

    #[test]
    fn a_failed_spell_uses_a_slot_and_keeps_the_equip() {
        let mut player = holder();
        let before = glove(&player);
        let cast = Cast {
            slot: 5,
            item_id: VEGA_10,
            equip_slot: 2,
            scroll_slot: 2,
        };
        let (changes, outcome) = cast.apply(&mut player, &mut |_, _| 30).unwrap();
        assert_eq!(outcome, ScrollOutcome::Fail);
        assert!(matches!(
            changes.last(),
            Some(InventoryChange::Add { slot: 2, .. })
        ));
        let after = glove(&player);
        assert_eq!(
            (after.upgrade_slots, after.level, after.dex),
            (before.upgrade_slots - 1, 0, before.dex)
        );
    }

    #[test]
    fn the_spell_refuses_what_it_does_not_take() {
        // The wrong rate for the spell, a scroll that doesn't fit, a missing
        // equip, the wrong tabs, and an equip with no slots left.
        let mut player = holder();
        for (vega, equip_slot, scroll_slot) in [
            (VEGA_10, 2, 1),
            (VEGA_60, 2, 2),
            (VEGA_10, 2, 3),
            (VEGA_60, 7, 1),
            (VEGA_60, -8, 1),
        ] {
            let actions = cast(&mut player, vega, equip_slot, scroll_slot);
            assert!(
                matches!(&actions[..], [WorldAction::Reply(p)] if p.bytes[..2] == (SendOpcode::StatChange as i16).to_le_bytes()),
                "{vega} {equip_slot} {scroll_slot}: {actions:?}"
            );
        }
        let mut item = player
            .inventory
            .get(InventoryType::Equip, 2)
            .unwrap()
            .clone();
        item.equip.as_mut().unwrap().upgrade_slots = 0;
        player.inventory.insert(InventoryType::Equip, 2, item);
        assert_eq!(cast(&mut player, VEGA_60, 2, 1).len(), 1);
        assert_eq!(
            player
                .inventory
                .get(InventoryType::Use, 1)
                .unwrap()
                .quantity,
            5
        );
        assert!(player.inventory.get(InventoryType::Cash, 6).is_some());
    }
}
