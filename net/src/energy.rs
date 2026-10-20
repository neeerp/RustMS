//! Energy Charge: the gauge a Brawler's (or a Thunder Breaker's) close
//! attacks fill. The server owns the energy; the v83 client only draws the
//! gauge and, once it is full, the glow (report 194, Energy Charge). Full,
//! it empties after the skill's `time`, which the client leaves to the
//! server.

use std::time::{Duration, Instant};

use game::buff::ENERGY_FULL;
use game::{Buff, BuffStat, Player};

use crate::combat::{Attack, AttackKind};
use crate::skills::{self, Learned};

const ENERGY_CHARGE: i32 = 5110001;
const DAWN_ENERGY_CHARGE: i32 = 15100004;

/// How long a gauge that isn't full yet keeps its energy: it doesn't drain.
const CHARGING: Duration = Duration::from_secs(24 * 60 * 60);

/// Energy Charge for the player's line (the client's `job / 1000 == 1`
/// test, 0x94b44b), once learned.
fn skill(player: &Player) -> Option<Learned> {
    let skill_id = if player.stats.job / 1000 == 1 {
        DAWN_ENERGY_CHARGE
    } else {
        ENERGY_CHARGE
    };
    skills::learned(player, skill_id)
}

/// The energy an accepted attack leaves, if it changes it: a close attack
/// gains Energy Charge's `x` for each mob it reaches (as Cosmic has it), up
/// to full. A full gauge gains nothing until it empties.
pub fn after_attack(player: &Player, attack: &Attack) -> Option<i16> {
    if attack.kind != AttackKind::Close || attack.hits.is_empty() {
        return None;
    }
    let skill = skill(player)?;
    let energy = i32::from(player.buffs.value(BuffStat::EnergyCharge).unwrap_or(0));
    if energy >= i32::from(ENERGY_FULL) {
        return None;
    }
    let gained = skill.effect.x * attack.hits.len() as i32;
    Some((energy + gained).min(i32::from(ENERGY_FULL)) as i16)
}

/// Energy Blast, Energy Drain and Energy Orb, and the Thunder Breaker's
/// two: v83 casts them only on a full gauge, telling the player "This
/// skill is available only when your energy is fully charged." otherwise
/// (FUN_00969465, FUN_009696a2).
const NEED_FULL: [i32; 5] = [5111002, 5111004, 5121002, 15101005, 15111001];

/// Whether `skill_id` is Energy Charge, which hits as the body attack.
pub fn charges(skill_id: i32) -> bool {
    skill_id == ENERGY_CHARGE || skill_id == DAWN_ENERGY_CHARGE
}

/// Whether `skill_id` needs a full gauge the player hasn't got: the body
/// attack too (0x94b3aa).
pub fn refused(player: &Player, skill_id: i32) -> bool {
    (NEED_FULL.contains(&skill_id) || charges(skill_id)) && !full(player)
}

/// Whether the player's gauge is full and they still have Energy Charge.
pub fn full(player: &Player) -> bool {
    player.buffs.value(BuffStat::EnergyCharge).unwrap_or(0) >= ENERGY_FULL
        && skill(player).is_some()
}

/// What buffs add to attack, accuracy or avoidability (`buffed`), raised
/// to Energy Charge's own `pad`, `acc` or `eva` while the gauge is full:
/// the larger of the two, not their sum (0x77dd8c, 0x77de1c, 0x77deb2).
pub fn raise(player: &Player, stat: BuffStat, buffed: i32) -> i32 {
    let Some(skill) = skill(player).filter(|_| full(player)) else {
        return buffed;
    };
    let own = match stat {
        BuffStat::Pad => skill.effect.pad,
        BuffStat::Acc => skill.effect.acc,
        BuffStat::Eva => skill.effect.eva,
        _ => return buffed,
    };
    buffed.max(own)
}

/// Fill the gauge to `energy`. A full one empties after the skill's `time`.
pub fn charge(player: &mut Player, energy: i16, now: Instant) {
    let Some(skill) = skill(player) else {
        return;
    };
    let lasts = if energy >= ENERGY_FULL {
        Duration::from_secs(skill.effect.duration.max(0) as u64)
    } else {
        CHARGING
    };
    let gauge = Buff::new(skill.skill_id, BuffStat::EnergyCharge, energy, now, lasts);
    player.buffs.give(gauge);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::{self, AttackHit, AttackRejection};
    use game::SkillRecord;

    fn brawler(level: i32) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.stats.job = 510;
        player.skills.set(SkillRecord {
            skill_id: ENERGY_CHARGE,
            level,
            master_level: 0,
            expiration: None,
        });
        player
    }

    fn attack(kind: AttackKind, mobs: usize) -> Attack {
        Attack {
            kind,
            skill: 0,
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
    fn close_attacks_fill_the_gauge_by_x_for_each_mob() {
        if skills::info(ENERGY_CHARGE).is_none() {
            return;
        }
        // Energy Charge 1 gains 75 a mob.
        let mut player = brawler(1);
        assert_eq!(
            after_attack(&player, &attack(AttackKind::Close, 1)),
            Some(75)
        );
        assert_eq!(
            after_attack(&player, &attack(AttackKind::Close, 2)),
            Some(150)
        );
        assert_eq!(after_attack(&player, &attack(AttackKind::Close, 0)), None);
        assert_eq!(after_attack(&player, &attack(AttackKind::Ranged, 1)), None);
        assert_eq!(
            after_attack(&brawler(0), &attack(AttackKind::Close, 1)),
            None
        );

        let now = Instant::now();
        charge(&mut player, 9990, now);
        assert_eq!(
            after_attack(&player, &attack(AttackKind::Close, 1)),
            Some(10000)
        );
        charge(&mut player, 10000, now);
        assert_eq!(after_attack(&player, &attack(AttackKind::Close, 1)), None);
    }

    #[test]
    fn a_full_gauge_raises_attack_accuracy_and_avoid_and_frees_the_energy_skills() {
        if skills::info(ENERGY_CHARGE).is_none() {
            return;
        }
        // Energy Charge 40 has 20 attack, accuracy and avoidability.
        let mut player = brawler(40);
        let now = Instant::now();
        charge(&mut player, 9999, now);
        assert!(!full(&player));
        assert_eq!(raise(&player, BuffStat::Pad, 12), 12);
        assert!(refused(&player, 5111002) && !refused(&player, 5001001));

        charge(&mut player, 10000, now);
        assert!(full(&player));
        assert_eq!(raise(&player, BuffStat::Pad, 12), 20);
        assert_eq!(raise(&player, BuffStat::Pad, 25), 25);
        assert_eq!(raise(&player, BuffStat::Acc, 0), 20);
        assert_eq!(raise(&player, BuffStat::Eva, 0), 20);
        assert_eq!(raise(&player, BuffStat::Mad, 3), 3);
        assert!(!refused(&player, 5111002) && !refused(&player, 5121002));
    }

    #[test]
    fn a_full_gauge_hits_one_touching_mob_with_energy_charge() {
        if skills::info(ENERGY_CHARGE).is_none() {
            return;
        }
        let body = |kind, mobs, skill| Attack {
            skill,
            ..attack(kind, mobs)
        };
        let mut player = brawler(40);
        let rejection = |player: &Player, attack: &Attack| combat::validate(attack, player).err();
        let touch = body(AttackKind::Body, 1, ENERGY_CHARGE);
        assert_eq!(
            rejection(&player, &touch),
            Some(AttackRejection::NotCharged)
        );

        charge(&mut player, 10000, Instant::now());
        assert_eq!(rejection(&player, &touch), None);
        assert_eq!(
            rejection(&player, &body(AttackKind::Body, 2, ENERGY_CHARGE)),
            Some(AttackRejection::TooManyTargets)
        );
        // Energy Charge is no swing, and the body attack is nothing else.
        assert_eq!(
            rejection(&player, &body(AttackKind::Close, 1, ENERGY_CHARGE)),
            Some(AttackRejection::Skill)
        );
        assert_eq!(
            rejection(&player, &body(AttackKind::Body, 1, 0)),
            Some(AttackRejection::Skill)
        );
    }

    #[test]
    fn a_full_gauge_empties_after_the_skills_time() {
        if skills::info(ENERGY_CHARGE).is_none() {
            return;
        }
        // Energy Charge 1 lasts 31 seconds once full.
        let mut player = brawler(1);
        let now = Instant::now();
        charge(&mut player, 75, now);
        assert!(player
            .buffs
            .advance(now + Duration::from_secs(3600))
            .expired
            .is_empty());
        charge(&mut player, 10000, now);
        assert!(player
            .buffs
            .advance(now + Duration::from_secs(30))
            .expired
            .is_empty());
        let tick = player.buffs.advance(now + Duration::from_secs(31));
        assert_eq!(tick.expired, vec![BuffStat::EnergyCharge]);
        assert_eq!(player.buffs.energy(), None);
    }
}
