//! The Aran's combo: the hits their polearm lands while they know Combo
//! Ability (v83ref reports/b360-aran-combo.md). The v83 client counts
//! nothing itself: it tells the server of each hit (0xA3), shows the count
//! the server answers (SHOW_COMBO), and forgets it 3 seconds after the last
//! one. The count, as temporary stat 68, adds to attack and criticals for
//! every ten, and the combo skills spend it all.

use std::time::{Duration, Instant};

use game::{Buff, BuffStat, Player};

use crate::skills::{self, Learned};

pub const COMBO_ABILITY: i32 = 21000000;
/// An Aran beginner's (job 2000) Combo Ability and Combo Critical.
const BEGINNER_COMBO_ABILITY: i32 = 20000017;
const COMBO_CRITICAL: i32 = 21110000;
const BEGINNER_COMBO_CRITICAL: i32 = 20000018;
const ARAN_BEGINNER: i16 = 2000;
pub const BODY_PRESSURE: i32 = 21101003;
const POLEARM: i32 = 144;

/// How long the count lasts after the last hit, as the client keeps it
/// (0x94bdba).
pub const IDLE: Duration = Duration::from_secs(3);
/// The most the count goes to; the client draws five digits.
const MOST: i16 = 30000;

/// Combo Ability for the player's job (0x77df86), once learned.
fn ability(player: &Player) -> Option<Learned> {
    let skill_id = if player.stats.job == ARAN_BEGINNER {
        BEGINNER_COMBO_ABILITY
    } else {
        COMBO_ABILITY
    };
    skills::learned(player, skill_id).filter(|skill| skill.level > 0)
}

/// The player's combo.
pub fn count(player: &Player) -> i32 {
    player.buffs.value(BuffStat::AranCombo).map_or(0, i32::from)
}

/// The combo a skill needs, which using it spends, or 0 (FUN_00766986):
/// Combo Smash and Combo Drain 30, Combo Penril 100, Combo Tempest and
/// Combo Barrier 200.
pub fn needs(skill_id: i32) -> i32 {
    match skill_id {
        21100004 | 21100005 => 30,
        21110004 => 100,
        21120006 | 21120007 => 200,
        _ => 0,
    }
}

/// Whether the player's combo is too short for `skill_id`. The client
/// says nothing; its key picks a skill the combo allows (0x74d314).
pub fn refused(player: &Player, skill_id: i32) -> bool {
    count(player) < needs(skill_id)
}

/// The combo after one more landed hit, if it counts: the client tells of
/// a hit only with Combo Ability and a polearm (0x668c11).
pub fn landed(player: &Player) -> Option<i16> {
    ability(player)?;
    if crate::combat::weapon(player) / 10_000 != POLEARM {
        return None;
    }
    Some((count(player) as i16).saturating_add(1).min(MOST))
}

/// Whether the client hears the combo as stat 68 too: once it starts, and
/// at every ten, where what it adds changes. SHOW_COMBO tells it the rest.
pub fn shown_as_stat(count: i16) -> bool {
    count == 1 || count % 10 == 0
}

/// Keep `count` for `IDLE` from now. The Combo Ability it is kept under.
pub fn set(player: &mut Player, count: i16, now: Instant) -> Option<i32> {
    let skill = ability(player)?;
    let combo = Buff::new(skill.skill_id, BuffStat::AranCombo, count, now, IDLE);
    player.buffs.give(combo);
    Some(skill.skill_id)
}

/// Spend the whole combo, as a combo skill does (0x9693b5). Whether there
/// was any.
pub fn spend(player: &mut Player) -> bool {
    let skill_id = match ability(player) {
        Some(skill) => skill.skill_id,
        None => return false,
    };
    !player.buffs.cancel(skill_id).is_empty()
}

/// What the combo adds to attack (0x77df48): Combo Ability's `y` for every
/// ten, up to its `x` tens.
pub fn attack(player: &Player) -> i32 {
    let combo = count(player);
    match ability(player) {
        Some(skill) if combo > 0 => skill.effect.x.min(combo / 10) * skill.effect.y,
        _ => 0,
    }
}

/// A critical's damage in percent with Combo Critical's added (0x78e254),
/// from `critical`, what the rest made it (0 for none): its `damage` for
/// every ten, up to its `x` tens, and another 100.
pub fn critical(player: &Player, critical: i32) -> i32 {
    let combo = count(player);
    let skill_id = if player.stats.job == ARAN_BEGINNER {
        BEGINNER_COMBO_CRITICAL
    } else {
        COMBO_CRITICAL
    };
    let Some(skill) = skills::learned(player, skill_id).filter(|_| combo > 0) else {
        return critical;
    };
    let tens = skill.effect.x.min(combo / 10);
    let added = skill.effect.damage * tens;
    if critical == 0 {
        added + 200
    } else {
        critical + 100 + added
    }
}

/// Whether `skill_id` is Body Pressure, which hits as the body attack, and
/// only while its buff is up (0x94b49d).
pub fn presses(skill_id: i32) -> bool {
    skill_id == BODY_PRESSURE
}

pub fn pressing(player: &Player) -> bool {
    player.buffs.has(BuffStat::BodyPressure)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::{self, Attack, AttackHit, AttackKind, AttackRejection};
    use game::inventory::equip_slot;
    use game::{EquipStats, InventoryType, Item, SkillRecord};

    const POLEARM_ITEM: i32 = 1442000;

    fn aran(skills: &[(i32, i32)]) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.stats.job = 2110;
        for &(skill_id, level) in skills {
            player.skills.set(SkillRecord {
                skill_id,
                level,
                master_level: 0,
                expiration: None,
            });
        }
        player
    }

    fn holding(mut player: Player, item_id: i32) -> Player {
        let weapon = Item::equipment(item_id, EquipStats::default());
        (player.inventory).insert(InventoryType::Equipped, equip_slot::WEAPON, weapon);
        player
    }

    #[test]
    fn a_polearm_hit_counts_once_combo_ability_is_learned() {
        if skills::info(COMBO_ABILITY).is_none() {
            return;
        }
        let now = Instant::now();
        let unarmed = aran(&[(COMBO_ABILITY, 1)]);
        assert_eq!(landed(&unarmed), None);
        assert_eq!(landed(&holding(aran(&[]), POLEARM_ITEM)), None);
        let mut player = holding(aran(&[(COMBO_ABILITY, 1)]), POLEARM_ITEM);
        assert_eq!(landed(&player), Some(1));
        set(&mut player, 29, now);
        assert_eq!(landed(&player), Some(30));
        assert!(shown_as_stat(1) && shown_as_stat(30) && !shown_as_stat(31));

        // The count goes 3 seconds after the last hit.
        let tick = player.buffs.advance(now + IDLE);
        assert_eq!(tick.expired, [BuffStat::AranCombo]);
        assert_eq!(count(&player), 0);
    }

    #[test]
    fn the_combo_skills_need_their_count_and_spend_it_all() {
        if skills::info(COMBO_ABILITY).is_none() {
            return;
        }
        let now = Instant::now();
        let mut player = aran(&[(COMBO_ABILITY, 1)]);
        set(&mut player, 29, now);
        assert!(refused(&player, 21100004) && !refused(&player, 21100002));
        set(&mut player, 30, now);
        assert!(!refused(&player, 21100004) && !refused(&player, 21100005));
        assert!(refused(&player, 21110004));
        set(&mut player, 199, now);
        assert!(!refused(&player, 21110004) && refused(&player, 21120006));
        set(&mut player, 200, now);
        assert!(!refused(&player, 21120006) && !refused(&player, 21120007));
        assert!(spend(&mut player));
        assert_eq!(count(&player), 0);
        assert!(!spend(&mut player));
    }

    #[test]
    fn every_ten_adds_to_attack_and_criticals() {
        if skills::info(COMBO_ABILITY).is_none() {
            return;
        }
        // Combo Ability 10 adds 1 attack for every ten up to ten tens;
        // Combo Critical 10 adds 5% for every ten up to five tens.
        let now = Instant::now();
        let mut player = aran(&[(COMBO_ABILITY, 10), (COMBO_CRITICAL, 10)]);
        assert_eq!((attack(&player), critical(&player, 0)), (0, 0));
        set(&mut player, 9, now);
        assert_eq!((attack(&player), critical(&player, 0)), (0, 200));
        set(&mut player, 57, now);
        assert_eq!((attack(&player), critical(&player, 0)), (5, 225));
        assert_eq!(critical(&player, 140), 265);
        set(&mut player, 300, now);
        assert_eq!((attack(&player), critical(&player, 0)), (10, 225));
    }

    fn swing(kind: AttackKind, skill: i32) -> Attack {
        Attack {
            kind,
            skill,
            hits: vec![AttackHit {
                object_id: 1,
                damage: vec![1],
            }],
            mob_count: 1,
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
    fn combo_smash_waits_for_thirty_and_body_pressure_for_its_buff() {
        if skills::info(COMBO_ABILITY).is_none() {
            return;
        }
        let now = Instant::now();
        let mut player = holding(
            aran(&[(COMBO_ABILITY, 1), (21100004, 1), (BODY_PRESSURE, 1)]),
            POLEARM_ITEM,
        );
        let rejection = |player: &Player, attack: &Attack| combat::validate(attack, player).err();
        let smash = swing(AttackKind::Close, 21100004);
        set(&mut player, 29, now);
        assert_eq!(
            rejection(&player, &smash),
            Some(AttackRejection::ComboShort)
        );
        set(&mut player, 30, now);
        assert_eq!(rejection(&player, &smash), None);

        // Body Pressure hits only as the body attack, while its buff lasts.
        let press = swing(AttackKind::Body, BODY_PRESSURE);
        assert_eq!(rejection(&player, &press), Some(AttackRejection::Skill));
        let lasts = Duration::from_secs(120);
        let buff = Buff::new(BODY_PRESSURE, BuffStat::BodyPressure, 1, now, lasts);
        player.buffs.give(buff);
        assert_eq!(rejection(&player, &press), None);
        assert_eq!(
            rejection(&player, &swing(AttackKind::Close, BODY_PRESSURE)),
            Some(AttackRejection::Skill)
        );
    }

    #[test]
    fn combo_barrier_keeps_its_share_off_and_combo_drain_heals() {
        if skills::info(COMBO_ABILITY).is_none() {
            return;
        }
        let now = Instant::now();
        let mut player = aran(&[]);
        let lasts = Duration::from_secs(120);
        // Combo Barrier 30 keeps 800 per mille, as Achilles does.
        player
            .buffs
            .give(Buff::new(21120007, BuffStat::ComboBarrier, 800, now, lasts));
        assert_eq!(combat::taken(&player, 1000, false, false).hp, 800);
        // Combo Drain 20 gives back 5% of a weapon hit, not a spell.
        player
            .buffs
            .give(Buff::new(21100005, BuffStat::ComboDrain, 5, now, lasts));
        assert_eq!(skills::takes(&player, None, false, 0.0).hp, 5);
        assert_eq!(skills::takes(&player, None, true, 0.0).hp, 0);
    }
}
