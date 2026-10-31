//! Homing Beacon (5211006) and the Corsair's Bullseye (5220011), report
//! 222. Each is a plain shot; the server marks the first mob it hits
//! (GuidedBullet, bit 87), and the client's later shots and single-target
//! spells seek that mob first. Bullseye's mark takes its `x` percent more
//! from every hit (FUN_0078dea6); the client counts it.

use game::{Beacon, Player};

use crate::combat::Attack;
use crate::skills;

pub const HOMING_BEACON: i32 = 5211006;
pub const BULLSEYE: i32 = 5220011;

/// After a beacon shot, the mark it leaves and the one it replaces. Nothing
/// changes when it marks the same mob again.
pub fn after_attack(player: &mut Player, attack: &Attack) -> Option<(Option<Beacon>, Beacon)> {
    if !matches!(attack.skill, HOMING_BEACON | BULLSEYE) {
        return None;
    }
    let beacon = Beacon {
        skill_id: attack.skill,
        mob: attack.hits.first()?.object_id,
    };
    let replaced = player.buffs.beacon.replace(beacon);
    (replaced != Some(beacon)).then_some((replaced, beacon))
}

/// The percent a hit on `mob` is raised by: Bullseye's 100 plus `x` on
/// the marked mob, 100 on any other.
pub fn damage_percent(player: &Player, mob: i32) -> i32 {
    if player.buffs.beacon.is_none_or(|beacon| beacon.mob != mob) {
        return 100;
    }
    skills::learned(player, BULLSEYE).map_or(100, |skill| 100 + skill.effect.x)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::{AttackHit, AttackKind};
    use game::SkillRecord;

    fn corsair(bullseye: i32) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.stats.job = 522;
        player.skills.set(SkillRecord {
            skill_id: BULLSEYE,
            level: bullseye,
            master_level: 0,
            expiration: None,
        });
        player
    }

    fn shot(skill: i32, mobs: &[i32]) -> Attack {
        Attack {
            kind: AttackKind::Ranged,
            skill,
            hits: (mobs.iter())
                .map(|&object_id| AttackHit {
                    object_id,
                    damage: vec![1],
                })
                .collect(),
            mob_count: mobs.len() as u8,
            hit_count: 1,
            display: 0,
            action: 0,
            speed: 4,
            charge: 0,
            ammo_slot: 0,
            explosion: None,
            grenade: None,
        }
    }

    #[test]
    fn a_beacon_marks_the_first_mob_it_hits() {
        let mut player = corsair(0);
        let first = Beacon {
            skill_id: HOMING_BEACON,
            mob: 7,
        };
        assert_eq!(
            after_attack(&mut player, &shot(HOMING_BEACON, &[7])),
            Some((None, first))
        );
        // The same mob again changes nothing.
        assert_eq!(after_attack(&mut player, &shot(HOMING_BEACON, &[7])), None);
        let second = Beacon { mob: 9, ..first };
        assert_eq!(
            after_attack(&mut player, &shot(HOMING_BEACON, &[9])),
            Some((Some(first), second))
        );
        // A miss, or any other shot, leaves the mark.
        assert_eq!(after_attack(&mut player, &shot(HOMING_BEACON, &[])), None);
        assert_eq!(after_attack(&mut player, &shot(5201001, &[3])), None);
        assert_eq!(player.buffs.beacon, Some(second));
    }

    #[test]
    fn bullseye_raises_hits_on_the_marked_mob() {
        let mut player = corsair(20);
        after_attack(&mut player, &shot(BULLSEYE, &[7]));
        // Bullseye 20's x is 20.
        assert_eq!(damage_percent(&player, 7), 120);
        assert_eq!(damage_percent(&player, 8), 100);
        // Homing Beacon alone gives nothing.
        let mut player = corsair(0);
        after_attack(&mut player, &shot(HOMING_BEACON, &[7]));
        assert_eq!(damage_percent(&player, 7), 100);
    }
}
