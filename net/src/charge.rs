//! A White Knight's or Paladin's weapon charges, and the Charged Blow that
//! spends them (report 218). The client puts no status on the mob for a
//! charged hit; the server only gates Charged Blow, allows for the charge's
//! and Advanced Charge's damage, and ends the charge.

use game::{BuffStat, Player};

use crate::combat::Attack;
use crate::skills::{self, Learned};

pub const CHARGED_BLOW: i32 = 1211002;
const ADVANCED_CHARGE: i32 = 1220010;
const PALADIN: i16 = 122;

/// Whether `skill_id` is Charged Blow without a charge on, which v83
/// refuses without a word (0x969545).
pub fn refused(player: &Player, skill_id: i32) -> bool {
    skill_id == CHARGED_BLOW && !player.buffs.has(BuffStat::WeaponCharge)
}

/// The charge's skill, if one is on.
fn charge(player: &Player) -> Option<i32> {
    let buffs = player.buffs.active();
    let buff = buffs
        .iter()
        .find(|buff| buff.stat == BuffStat::WeaponCharge)?;
    Some(buff.skill_id)
}

/// The percent a charge raises every weapon hit by, its `damage`, before
/// its element counts (FUN_007907ec); 100 without one.
pub fn damage_percent(player: &Player) -> i32 {
    charge(player)
        .and_then(|skill_id| skills::learned(player, skill_id))
        .map_or(100, |skill| skill.effect.damage.max(100))
}

/// A Paladin's Charged Blow hits by Advanced Charge's `damage` instead of
/// its own once Advanced Charge is learned (0x951cf9).
pub fn blow_percent(player: &Player, skill: &Learned) -> Option<i32> {
    if skill.skill_id != CHARGED_BLOW || player.stats.job != PALADIN {
        return None;
    }
    skills::learned(player, ADVANCED_CHARGE).map(|advanced| advanced.effect.damage)
}

/// The charge an accepted attack ends, by its skill: Charged Blow that
/// reached a mob spends it, unless Advanced Charge's `x` percent keeps it
/// (`roll` in [0, 1)). Nothing else ends a charge but its time.
pub fn after_attack(player: &Player, attack: &Attack, roll: f64) -> Option<i32> {
    if attack.skill != CHARGED_BLOW || attack.hits.is_empty() {
        return None;
    }
    let charge = charge(player)?;
    let kept = skills::learned(player, ADVANCED_CHARGE).map_or(0, |advanced| advanced.effect.x);
    (roll * 100.0 >= f64::from(kept)).then_some(charge)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::{AttackHit, AttackKind};
    use game::{Buff, SkillRecord};
    use std::time::{Duration, Instant};

    const FIRE_CHARGE: i32 = 1211003;

    fn knight(job: i16, advanced: i32, charged: bool) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.stats.job = job;
        for (skill_id, level) in [
            (CHARGED_BLOW, 30),
            (FIRE_CHARGE, 30),
            (ADVANCED_CHARGE, advanced),
        ] {
            player.skills.set(SkillRecord {
                skill_id,
                level,
                master_level: 0,
                expiration: None,
            });
        }
        if charged {
            let lasts = Duration::from_secs(120);
            (player.buffs).give(Buff::new(
                FIRE_CHARGE,
                BuffStat::WeaponCharge,
                1,
                Instant::now(),
                lasts,
            ));
        }
        player
    }

    fn blow(skill: i32, mobs: usize) -> Attack {
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
            explosion: None,
        }
    }

    #[test]
    fn charged_blow_needs_a_charge() {
        assert!(refused(&knight(121, 0, false), CHARGED_BLOW));
        assert!(!refused(&knight(121, 0, true), CHARGED_BLOW));
        assert!(!refused(&knight(121, 0, false), FIRE_CHARGE));
    }

    #[test]
    fn a_charge_raises_hits_by_its_damage() {
        // Fire Charge 30's damage is 120.
        assert_eq!(damage_percent(&knight(121, 0, true)), 120);
        assert_eq!(damage_percent(&knight(121, 0, false)), 100);
    }

    #[test]
    fn advanced_charge_replaces_a_paladins_blow_damage() {
        let percent = |player: &Player, skill_id| {
            let skill = skills::learned(player, skill_id).unwrap();
            blow_percent(player, &skill)
        };
        // Advanced Charge 10's damage is 350.
        assert_eq!(percent(&knight(122, 10, true), CHARGED_BLOW), Some(350));
        assert_eq!(percent(&knight(122, 0, true), CHARGED_BLOW), None);
        assert_eq!(percent(&knight(121, 10, true), CHARGED_BLOW), None);
        assert_eq!(percent(&knight(122, 10, true), ADVANCED_CHARGE), None);
    }

    #[test]
    fn charged_blow_spends_the_charge_unless_advanced_charge_keeps_it() {
        // Without Advanced Charge, always.
        let white = knight(121, 0, true);
        assert_eq!(
            after_attack(&white, &blow(CHARGED_BLOW, 1), 0.99),
            Some(FIRE_CHARGE)
        );
        // Advanced Charge 5 keeps it half the time.
        let paladin = knight(122, 5, true);
        assert_eq!(after_attack(&paladin, &blow(CHARGED_BLOW, 1), 0.49), None);
        assert_eq!(
            after_attack(&paladin, &blow(CHARGED_BLOW, 1), 0.5),
            Some(FIRE_CHARGE)
        );
        // A miss, or another skill, leaves it.
        assert_eq!(after_attack(&white, &blow(CHARGED_BLOW, 0), 0.99), None);
        assert_eq!(after_attack(&white, &blow(0, 1), 0.99), None);
    }
}
