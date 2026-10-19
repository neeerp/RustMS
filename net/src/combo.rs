//! Combo Attack: the orbs a Crusader's (or a Dawn Warrior's) swings gather,
//! which make every attack stronger until a finisher spends them. The Combo
//! value is one more than the orbs.

use game::{BuffStat, Player};

use crate::combat::{Attack, AttackKind};
use crate::skills;

const COMBO: i32 = 1111002;
const ADVANCED_COMBO: i32 = 1120003;
const DAWN_COMBO: i32 = 11111001;
const DAWN_ADVANCED_COMBO: i32 = 11110005;
const SHOUT: i32 = 1111008;
/// A Hero's Enrage, which spends the orbs.
pub const ENRAGE: i32 = 1121010;

/// Panic and Coma, a Crusader's and a Dawn Warrior's.
const FINISHERS: [i32; 6] = [1111003, 1111004, 1111005, 1111006, 11111002, 11111003];

pub fn finisher(skill_id: i32) -> bool {
    FINISHERS.contains(&skill_id)
}

/// Combo Attack and Advanced Combo for the player's line, and Combo
/// Attack's last level.
fn line(player: &Player) -> (i32, i32, i32) {
    if player.stats.job / 1000 == 1 {
        (DAWN_COMBO, DAWN_ADVANCED_COMBO, 20)
    } else {
        (COMBO, ADVANCED_COMBO, 30)
    }
}

/// The orbs the player has while Combo Attack is up.
pub fn orbs(player: &Player) -> Option<i32> {
    let buff = (player.buffs.active())
        .iter()
        .find(|buff| buff.stat == BuffStat::Combo)?;
    matches!(buff.skill_id, COMBO | DAWN_COMBO).then_some(i32::from(buff.value) - 1)
}

/// Enrage wants a Combo value of 10, nine orbs: the v83 client compares
/// the value, not the orbs (0x764379).
pub fn enrage_refused(player: &Player, skill_id: i32) -> bool {
    skill_id == ENRAGE && player.buffs.value(BuffStat::Combo).unwrap_or(0) < 10
}

/// The percent the orbs make an attack's damage, after its skill's, or 0
/// when they leave it alone (the v83 client's FUN_007653fa). Its base is
/// Combo Attack's `damage`, or Advanced Combo's once Combo Attack is at its
/// last level.
pub fn damage_percent(player: &Player, skill_id: i32) -> i32 {
    let (combo, advanced, last) = line(player);
    let level = player.skills.level(combo);
    if level == 0 {
        return 0;
    }
    let entry = if level == last && player.skills.level(advanced) > 0 {
        advanced
    } else {
        combo
    };
    let base = skills::learned(player, entry).map_or(0, |skill| skill.effect.damage);
    let orbs = i32::from(player.buffs.value(BuffStat::Combo).unwrap_or(0)) - 1;
    percent(finisher(skill_id), level, base, orbs)
}

fn percent(finisher: bool, level: i32, base: i32, orbs: i32) -> i32 {
    match (finisher, orbs) {
        (false, i32::MIN..=-1) => 0,
        (_, 1) => base,
        (false, 2..=5) => base + 5 * (orbs - 1) * (level - 1) / 29,
        (true, 2..=5) => {
            let step = [6, 16, 30, 46][orbs as usize - 2];
            base + step + 4 * step * (level - 1) / 29
        }
        (false, 6..=10) => base + [24, 28, 32, 36, 40][orbs as usize - 6],
        (true, 6..=10) => base + [250, 270, 290, 305, 320][orbs as usize - 6],
        _ => 100,
    }
}

/// The Combo value an accepted attack leaves, if it changes it. A finisher
/// spends every orb. A swing that reaches a mob, other than Shout's, gains
/// one up to Combo Attack's `x` (Advanced Combo's, once learned), and with
/// Advanced Combo, on its `prop` chance, one more (as Cosmic has it).
pub fn after_attack(player: &Player, attack: &Attack, roll: f64) -> Option<i16> {
    let orbs = orbs(player)?;
    if finisher(attack.skill) {
        return Some(1);
    }
    if attack.kind != AttackKind::Close || attack.hits.is_empty() || attack.skill == SHOUT {
        return None;
    }
    let (combo, advanced, _) = line(player);
    let (effect, advanced) = match skills::learned(player, advanced) {
        Some(advanced) => (advanced.effect, true),
        None => (skills::learned(player, combo)?.effect, false),
    };
    let most = effect.x;
    if orbs >= most {
        return None;
    }
    let mut gained = orbs + 1;
    if advanced && gained < most && roll * 100.0 < f64::from(effect.prop) {
        gained += 1;
    }
    Some(gained as i16 + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::AttackHit;
    use game::{Buff, SkillRecord};
    use std::time::{Duration, Instant};

    fn crusader(combo: i32, advanced: i32, value: i16) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.stats.job = 111;
        for (skill_id, level) in [(COMBO, combo), (ADVANCED_COMBO, advanced)] {
            player.skills.set(SkillRecord {
                skill_id,
                level,
                master_level: 0,
                expiration: None,
            });
        }
        let lasts = Duration::from_secs(120);
        (player.buffs).give(Buff::new(
            COMBO,
            BuffStat::Combo,
            value,
            Instant::now(),
            lasts,
        ));
        player
    }

    fn swing(skill: i32, mobs: usize) -> Attack {
        Attack {
            kind: AttackKind::Close,
            skill,
            hits: (0..mobs)
                .map(|object_id| AttackHit {
                    object_id: object_id as i32,
                    damage: vec![1],
                })
                .collect(),
            mob_count: mobs as u8,
            hit_count: 1,
            display: 0,
            action: 0,
            speed: 4,
            charge: 0,
            ammo_slot: 0,
        }
    }

    #[test]
    fn orbs_raise_damage_by_the_clients_table() {
        // Combo Attack 30's damage is 120.
        let percent = |value, skill| damage_percent(&crusader(30, 0, value), skill);
        assert_eq!(percent(1, 0), 100);
        assert_eq!(percent(2, 0), 120);
        assert_eq!(percent(3, 0), 125);
        assert_eq!(percent(6, 0), 140);
        // A finisher's: 350% more with five orbs.
        assert_eq!(percent(6, 1111003), 120 + 46 + 184);
        assert_eq!(percent(3, 1111005), 120 + 6 + 24);
        assert_eq!(percent(4, 1111005), 120 + 16 + 64);
        // Advanced Combo 30 (150) once Combo Attack is at 30.
        assert_eq!(damage_percent(&crusader(30, 30, 11), 0), 190);
        assert_eq!(damage_percent(&crusader(30, 30, 11), 1111004), 470);
        assert_eq!(damage_percent(&crusader(29, 30, 2), 0), 119);
        // Without Combo Attack, or with the buff off, nothing.
        assert_eq!(damage_percent(&crusader(0, 0, 3), 0), 0);
        let mut off = crusader(30, 0, 3);
        off.buffs.clear();
        assert_eq!(damage_percent(&off, 0), 0);
    }

    #[test]
    fn the_tables_ends() {
        assert_eq!(percent(false, 1, 100, 2), 100);
        assert_eq!(percent(false, 30, 120, 11), 100);
        assert_eq!(percent(true, 30, 120, 0), 100);
        assert_eq!(percent(true, 30, 120, -1), 100);
        assert_eq!(percent(false, 30, 120, -1), 0);
    }

    #[test]
    fn swings_gather_orbs_up_to_combo_attacks_x() {
        // Combo Attack 30 holds five orbs.
        let after = |value, attack: Attack| after_attack(&crusader(30, 0, value), &attack, 0.0);
        assert_eq!(after(1, swing(0, 1)), Some(2));
        assert_eq!(after(5, swing(0, 1)), Some(6));
        assert_eq!(after(6, swing(0, 1)), None);
        // Not without a mob, nor with Shout.
        assert_eq!(after(1, swing(0, 0)), None);
        assert_eq!(after(1, swing(SHOUT, 1)), None);
        // Combo Attack 1 holds three.
        assert_eq!(after_attack(&crusader(1, 0, 4), &swing(0, 1), 0.0), None);
        // A finisher spends them.
        assert_eq!(after(4, swing(1111004, 0)), Some(1));
    }

    #[test]
    fn advanced_combo_holds_ten_and_may_gain_two() {
        // Advanced Combo 30: ten orbs, 60% for two at once.
        let after = |value, roll| after_attack(&crusader(30, 30, value), &swing(0, 1), roll);
        assert_eq!(after(6, 0.59), Some(8));
        assert_eq!(after(6, 0.6), Some(7));
        assert_eq!(after(10, 0.0), Some(11));
        assert_eq!(after(11, 0.0), None);
    }

    #[test]
    fn a_summons_combo_stat_has_no_orbs() {
        let mut player = crusader(30, 0, 1);
        player.buffs.clear();
        let lasts = Duration::from_secs(60);
        (player.buffs).give(Buff::new(
            3111005,
            BuffStat::Combo,
            1,
            Instant::now(),
            lasts,
        ));
        assert_eq!(orbs(&player), None);
        assert_eq!(after_attack(&player, &swing(0, 1), 0.0), None);
    }
}
