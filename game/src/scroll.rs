//! Equipment upgrade scrolls (`Item.nx` `Consume/0204.img`): which equipment a
//! scroll accepts and what using one does. The rolls are injected so the rules
//! can be tested with fixed outcomes.

use crate::inventory::EquipStats;

/// The White Scroll, which keeps the slot when a scroll fails.
pub const WHITE_SCROLL: i32 = 2340000;
/// `Item.flags` bit 2040727 sets: spikes on shoes.
pub const FLAG_SPIKES: i16 = 0x02;
/// `Item.flags` bit 2041058 sets: cold protection on capes.
pub const FLAG_COLD: i16 = 0x04;

/// What a scroll does when it works.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollKind {
    /// Adds its `inc*` stats.
    #[default]
    Normal,
    /// Chaos (`randstat`): moves each of the equip's nonzero stats by -5..=5.
    Chaos,
    /// Clean Slate (`recover`): gives back a slot a failed scroll used.
    CleanSlate,
    /// `preventslip` (2040727): sets the spikes flag and uses no slot.
    Spikes,
    /// `warmsupport` (2041058): sets the cold flag and uses no slot.
    Cold,
}

impl ScrollKind {
    /// Whether a White Scroll may go with it. The client never offers one for
    /// Clean Slate and the flag scrolls (FUN_004f4bb7).
    pub fn takes_white_scroll(self) -> bool {
        matches!(self, Self::Normal | Self::Chaos)
    }
}

/// A scroll's `info` (and `req`) from `Item.nx`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScrollInfo {
    pub kind: ScrollKind,
    /// Percent chance it works (`success`).
    pub success: i32,
    /// Percent chance a failure destroys the equip (`cursed`).
    pub cursed: i32,
    /// The stats it adds (`incSTR` .. `incJump`, `incCraft`); only the 15
    /// stat fields are set.
    pub stats: EquipStats,
    /// The only equipment ids it accepts (`req`); empty means any that fits.
    pub req: Vec<i32>,
}

/// How a scroll went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollOutcome {
    Success,
    Fail,
    /// It failed and destroyed the equip.
    Cursed,
}

/// Whether `scroll_id` is meant for `equip_id`: the v83 client's check
/// before it sends a scroll (FUN_004f5497). Clean Slate and Chaos fit any
/// equipment but pet equipment; 20492xx fits rings, pendants and belts; any
/// other scroll fits the category its id names (2043001 → 130xxxx). The
/// [6th Anniversary] scrolls 2049105-2049110 fit nothing, as in the client.
pub fn fits(scroll_id: i32, equip_id: i32) -> bool {
    if scroll_id / 10_000 != 204 || equip_id / 1_000_000 != 1 {
        return false;
    }
    let group = scroll_id / 100;
    let any_equip = group == 20490 || (group == 20491 && !(2049105..=2049110).contains(&scroll_id));
    if any_equip && equip_id / 100_000 != 18 {
        return true;
    }
    let category = equip_id / 10_000 % 100;
    if group == 20492 {
        return (11..=13).contains(&category);
    }
    scroll_id % 10_000 / 100 == category
}

/// Whether `scroll` may be used on `equip` (`equip_id`), whose template has
/// `base_slots` upgrade slots: it fits, the equip is in its `req` list if it
/// has one, and a slot is left (for Clean Slate: one was used up by a
/// failure, counting the slots Vicious' Hammers added).
pub fn can_apply(
    scroll_id: i32,
    scroll: &ScrollInfo,
    equip_id: i32,
    equip: &EquipStats,
    base_slots: u8,
) -> bool {
    if !fits(scroll_id, equip_id) || !(scroll.req.is_empty() || scroll.req.contains(&equip_id)) {
        return false;
    }
    match scroll.kind {
        ScrollKind::CleanSlate => {
            i32::from(equip.upgrade_slots) + i32::from(equip.level)
                < i32::from(base_slots) + equip.vicious
        }
        _ => equip.upgrade_slots >= 1,
    }
}

/// Use `scroll` on `equip` (and its item `flags`), with or without a White
/// Scroll (`white`, ignored for kinds that take none). `roll(lo, hi)` returns
/// a number in `lo..hi`: first the percent roll for success, then on success
/// one -5..=5 roll per nonzero stat for Chaos, or on failure the percent roll
/// for `cursed` (only if the scroll has one). Rules as v83 servers
/// (HeavenMS `scrollEquipWithId`): a success uses a slot and adds a level,
/// except that Clean Slate gives a slot back and the flag scrolls use none; a
/// failure uses a slot unless the White Scroll saves it, and may still
/// destroy the equip.
pub fn apply(
    equip: &mut EquipStats,
    flags: &mut i16,
    scroll: &ScrollInfo,
    white: bool,
    roll: &mut impl FnMut(i32, i32) -> i32,
) -> ScrollOutcome {
    let white = white && scroll.kind.takes_white_scroll();
    let uses_slot = matches!(scroll.kind, ScrollKind::Normal | ScrollKind::Chaos);
    if roll(0, 100) < scroll.success {
        match scroll.kind {
            ScrollKind::Normal => {
                for (stat, bonus) in stats_mut(equip).into_iter().zip(stats(&scroll.stats)) {
                    *stat = stat.saturating_add(bonus);
                }
            }
            ScrollKind::Chaos => {
                for stat in stats_mut(equip) {
                    if *stat > 0 {
                        *stat = stat.saturating_add(roll(-5, 6) as i16).max(0);
                    }
                }
            }
            ScrollKind::CleanSlate => equip.upgrade_slots += 1,
            ScrollKind::Spikes => *flags |= FLAG_SPIKES,
            ScrollKind::Cold => *flags |= FLAG_COLD,
        }
        if scroll.kind != ScrollKind::CleanSlate {
            if uses_slot {
                equip.upgrade_slots = equip.upgrade_slots.saturating_sub(1);
            }
            equip.level = equip.level.saturating_add(1);
        }
        return ScrollOutcome::Success;
    }
    if uses_slot && !white {
        equip.upgrade_slots = equip.upgrade_slots.saturating_sub(1);
    }
    if scroll.cursed > 0 && roll(0, 100) < scroll.cursed {
        ScrollOutcome::Cursed
    } else {
        ScrollOutcome::Fail
    }
}

/// The 15 stats a scroll can change, in the wire order.
fn stats(equip: &EquipStats) -> [i16; 15] {
    [
        equip.str,
        equip.dex,
        equip.int,
        equip.luk,
        equip.hp,
        equip.mp,
        equip.watk,
        equip.matk,
        equip.wdef,
        equip.mdef,
        equip.acc,
        equip.avoid,
        equip.hands,
        equip.speed,
        equip.jump,
    ]
}

fn stats_mut(equip: &mut EquipStats) -> [&mut i16; 15] {
    [
        &mut equip.str,
        &mut equip.dex,
        &mut equip.int,
        &mut equip.luk,
        &mut equip.hp,
        &mut equip.mp,
        &mut equip.watk,
        &mut equip.matk,
        &mut equip.wdef,
        &mut equip.mdef,
        &mut equip.acc,
        &mut equip.avoid,
        &mut equip.hands,
        &mut equip.speed,
        &mut equip.jump,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sword(slots: u8, level: u8) -> EquipStats {
        EquipStats {
            upgrade_slots: slots,
            level,
            watk: 17,
            ..EquipStats::default()
        }
    }

    /// 2043001: one-handed sword attack 60%.
    fn sword_attack_60() -> ScrollInfo {
        ScrollInfo {
            success: 60,
            stats: EquipStats {
                watk: 2,
                str: 1,
                ..EquipStats::default()
            },
            ..ScrollInfo::default()
        }
    }

    /// 2043004: dark one-handed sword attack 70%.
    fn dark_sword_attack_70() -> ScrollInfo {
        ScrollInfo {
            cursed: 50,
            success: 70,
            ..sword_attack_60()
        }
    }

    fn scroll_of(kind: ScrollKind, success: i32, cursed: i32) -> ScrollInfo {
        ScrollInfo {
            kind,
            success,
            cursed,
            ..ScrollInfo::default()
        }
    }

    /// Apply `scroll` with `rolls` returned in order; checks their bounds and
    /// that every one was used.
    fn run(
        equip: &mut EquipStats,
        scroll: &ScrollInfo,
        white: bool,
        rolls: &[i32],
    ) -> (ScrollOutcome, i16) {
        let mut flags = 0;
        let mut next = rolls.iter();
        let outcome = apply(equip, &mut flags, scroll, white, &mut |lo, hi| {
            let value = *next.next().expect("an unexpected roll");
            assert!((lo..hi).contains(&value), "{value} not in {lo}..{hi}");
            value
        });
        assert_eq!(next.next(), None, "an unused roll");
        (outcome, flags)
    }

    #[test]
    fn scrolls_fit_the_category_their_id_names() {
        assert!(fits(2043001, 1302000));
        assert!(!fits(2043001, 1402000));
        assert!(fits(2049100, 1302000));
        assert!(fits(2049100, 1040002));
        assert!(!fits(2049100, 1802000));
        assert!(fits(2049003, 1302000));
        assert!(!fits(2049003, 1802000));
        assert!(fits(2049200, 1112000));
        assert!(fits(2049200, 1122000));
        assert!(fits(2049200, 1132000));
        assert!(!fits(2049200, 1142000));
        assert!(fits(2048000, 1802000));
        assert!(!fits(2049105, 1302000));
        assert!(fits(2049104, 1302000));
        assert!(fits(2049111, 1302000));
        assert!(!fits(2340000, 1302000));
        assert!(!fits(2043001, 2000000));
    }

    #[test]
    fn a_normal_scroll_adds_its_stats_or_uses_a_slot() {
        let scroll = sword_attack_60();
        let mut equip = sword(7, 0);
        assert_eq!(
            run(&mut equip, &scroll, false, &[59]).0,
            ScrollOutcome::Success
        );
        assert_eq!(
            (equip.watk, equip.str, equip.upgrade_slots, equip.level),
            (19, 1, 6, 1)
        );
        let mut equip = sword(7, 0);
        assert_eq!(
            run(&mut equip, &scroll, false, &[60]).0,
            ScrollOutcome::Fail
        );
        assert_eq!(equip, sword(6, 0));
    }

    #[test]
    fn a_white_scroll_keeps_the_slot_only_on_failure() {
        let scroll = sword_attack_60();
        let mut equip = sword(7, 0);
        assert_eq!(run(&mut equip, &scroll, true, &[60]).0, ScrollOutcome::Fail);
        assert_eq!(equip, sword(7, 0));
        let mut equip = sword(7, 0);
        assert_eq!(
            run(&mut equip, &scroll, true, &[0]).0,
            ScrollOutcome::Success
        );
        assert_eq!((equip.upgrade_slots, equip.level), (6, 1));
    }

    #[test]
    fn a_dark_scroll_may_destroy_the_equip_even_with_a_white_scroll() {
        let scroll = dark_sword_attack_70();
        let mut equip = sword(7, 0);
        assert_eq!(
            run(&mut equip, &scroll, false, &[70, 49]).0,
            ScrollOutcome::Cursed
        );
        let mut equip = sword(7, 0);
        assert_eq!(
            run(&mut equip, &scroll, false, &[99, 50]).0,
            ScrollOutcome::Fail
        );
        assert_eq!(equip.upgrade_slots, 6);
        let mut equip = sword(7, 0);
        assert_eq!(
            run(&mut equip, &scroll, true, &[70, 0]).0,
            ScrollOutcome::Cursed
        );
        // A scroll without `cursed` never rolls for it.
        let mut equip = sword(7, 0);
        assert_eq!(
            run(&mut equip, &sword_attack_60(), false, &[99]).0,
            ScrollOutcome::Fail
        );
    }

    #[test]
    fn chaos_moves_each_nonzero_stat_and_never_below_zero() {
        let scroll = scroll_of(ScrollKind::Chaos, 60, 0);
        let mut equip = EquipStats {
            upgrade_slots: 7,
            str: 3,
            watk: 17,
            acc: 2,
            ..EquipStats::default()
        };
        assert_eq!(
            run(&mut equip, &scroll, false, &[0, 5, -5, -5]).0,
            ScrollOutcome::Success
        );
        assert_eq!(
            (
                equip.str,
                equip.watk,
                equip.acc,
                equip.dex,
                equip.upgrade_slots,
                equip.level
            ),
            (8, 12, 0, 0, 6, 1)
        );
        let mut equip = sword(7, 0);
        assert_eq!(
            run(&mut equip, &scroll, false, &[60]).0,
            ScrollOutcome::Fail
        );
        assert_eq!(equip, sword(6, 0));
    }

    #[test]
    fn clean_slate_gives_back_a_used_slot() {
        let scroll = scroll_of(ScrollKind::CleanSlate, 20, 50);
        assert!(can_apply(2049003, &scroll, 1302000, &sword(5, 1), 7));
        assert!(!can_apply(2049003, &scroll, 1302000, &sword(6, 1), 7));
        let mut hammered = sword(7, 0);
        hammered.vicious = 1;
        assert!(can_apply(2049003, &scroll, 1302000, &hammered, 7));
        let mut equip = sword(5, 1);
        assert_eq!(
            run(&mut equip, &scroll, true, &[19]).0,
            ScrollOutcome::Success
        );
        assert_eq!((equip.upgrade_slots, equip.level), (6, 1));
        let mut equip = sword(5, 1);
        assert_eq!(
            run(&mut equip, &scroll, true, &[20, 49]).0,
            ScrollOutcome::Cursed
        );
        let mut equip = sword(5, 1);
        assert_eq!(
            run(&mut equip, &scroll, false, &[20, 50]).0,
            ScrollOutcome::Fail
        );
        assert_eq!(equip, sword(5, 1));
    }

    #[test]
    fn spikes_set_a_flag_and_use_no_slot() {
        let scroll = scroll_of(ScrollKind::Spikes, 10, 0);
        let mut equip = sword(5, 0);
        assert_eq!(
            run(&mut equip, &scroll, true, &[9]),
            (ScrollOutcome::Success, FLAG_SPIKES)
        );
        assert_eq!((equip.upgrade_slots, equip.level), (5, 1));
        let mut equip = sword(5, 0);
        assert_eq!(
            run(&mut equip, &scroll, false, &[10]),
            (ScrollOutcome::Fail, 0)
        );
        assert_eq!(equip, sword(5, 0));
        let scroll = scroll_of(ScrollKind::Cold, 10, 0);
        assert_eq!(run(&mut sword(5, 0), &scroll, false, &[0]).1, FLAG_COLD);
    }

    #[test]
    fn scrolls_need_a_slot_and_their_listed_equipment() {
        let scroll = sword_attack_60();
        assert!(can_apply(2043001, &scroll, 1302000, &sword(1, 6), 7));
        assert!(!can_apply(2043001, &scroll, 1302000, &sword(0, 7), 7));
        assert!(!can_apply(2043001, &scroll, 1402000, &sword(7, 0), 7));
        let dragon_stone = ScrollInfo {
            success: 100,
            req: vec![1122000],
            ..ScrollInfo::default()
        };
        assert!(can_apply(2041200, &dragon_stone, 1122000, &sword(3, 0), 3));
        assert!(!can_apply(2041200, &dragon_stone, 1122001, &sword(3, 0), 3));
    }
}
