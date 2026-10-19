//! AP Reset (5050000): one AP moves from one stat to another, as the v83
//! client's three dialogs allow it (FUN_008cbddb, report 195). Max HP and MP
//! move by their own amounts, not by what an AP adds when spent.

use crate::character::{ApStat, CharacterStats, MAX_BASE_STAT, MAX_HP_MP};

/// The job's branch: 0 beginner, 1 warrior, 2 magician, 3 bowman, 4 thief,
/// 5 pirate, the same for Cygnus.
fn branch(job: i16) -> i16 {
    (job % 1000) / 100
}

/// Max HP an AP gives back (FUN_008cbddb).
pub fn hp_loss(job: i16) -> i16 {
    match branch(job) {
        1 => 54,
        2 => 10,
        3 | 4 => 20,
        5 => 42,
        _ => 12,
    }
}

/// Max MP an AP gives back; a magician's follows their INT with what they
/// wear (FUN_008ce5bd).
pub fn mp_loss(job: i16, total_int: i32) -> i16 {
    match branch(job) {
        1 => 4,
        2 => (total_int * 3 / 40 + 30).clamp(0, i32::from(MAX_HP_MP)) as i16,
        3 | 4 => 12,
        5 => 16,
        _ => 8,
    }
}

/// Max HP an AP put there adds.
pub fn hp_gain(job: i16) -> i16 {
    match branch(job) {
        1 => 20,
        2 => 6,
        3 | 4 => 16,
        5 => 18,
        _ => 8,
    }
}

/// Max MP an AP put there adds.
pub fn mp_gain(job: i16) -> i16 {
    match branch(job) {
        1 => 2,
        2 => 18,
        3 | 4 => 10,
        5 => 14,
        _ => 6,
    }
}

/// The least Max HP a character of `job` and `level` keeps (FUN_008cd10a).
pub fn min_hp(job: i16, level: u8) -> i32 {
    let level = i32::from(level);
    match job % 1000 {
        0 => 12 * level + 38,
        100 | 120..=122 | 130..=132 => 24 * level + 118,
        110..=112 => 24 * level + 418,
        200..=299 => 10 * level + 54,
        300 | 400 => 20 * level + 58,
        310..=312 | 320..=322 | 410..=412 | 420..=422 => 20 * level + 358,
        500 => 22 * level + 38,
        510..=512 | 520..=522 => 22 * level + 338,
        _ => 0,
    }
}

/// The least Max MP a character of `job` and `level` keeps (FUN_008cd369).
pub fn min_mp(job: i16, level: u8) -> i32 {
    let level = i32::from(level);
    match job % 1000 {
        0 => 10 * level - 5,
        100 | 110..=112 => 4 * level + 55,
        120..=122 | 130..=132 => 4 * level + 155,
        200 => 22 * level - 1,
        210..=212 | 220..=222 | 230..=232 => 22 * level + 449,
        300 | 400 => 14 * level - 15,
        310..=312 | 320..=322 | 410..=412 | 420..=422 => 14 * level + 135,
        500 => 18 * level - 55,
        510..=512 | 520..=522 => 18 * level + 95,
        _ => 0,
    }
}

/// Whether any AP went into Max HP or MP: the main stats and unspent AP
/// account for less than every AP the character earned.
pub fn has_hp_mp_ap(stats: &CharacterStats) -> bool {
    let level = i32::from(stats.level);
    let mut earned = 20 + level * 5;
    if stats.job / 1000 == 1 {
        earned += (level - 1).min(69);
    }
    earned += match stats.job % 10 {
        1 => 5,
        2 => 10,
        _ => 0,
    };
    let held = [stats.str, stats.dex, stats.int, stats.luk, stats.ap]
        .into_iter()
        .map(i32::from)
        .sum::<i32>();
    held < earned
}

/// The least a main stat keeps: 4, or what the job's advancement asked for.
fn main_stat_floor(job: i16, stat: ApStat) -> i16 {
    let required = match (branch(job), stat) {
        (1, ApStat::Str) => 35,
        (2, ApStat::Int) => 20,
        (3 | 4, ApStat::Dex) => 25,
        (5, ApStat::Dex) => 20,
        _ => 0,
    };
    required.max(4)
}

/// Whether an AP can be taken out of `stat`.
pub fn may_take(stats: &CharacterStats, stat: ApStat, total_int: i32) -> bool {
    let (job, level) = (stats.job, stats.level);
    match stat {
        ApStat::MaxHp => {
            has_hp_mp_ap(stats) && i32::from(stats.max_hp - hp_loss(job)) >= min_hp(job, level)
        }
        ApStat::MaxMp => {
            has_hp_mp_ap(stats)
                && i32::from(stats.max_mp - mp_loss(job, total_int)) >= min_mp(job, level)
        }
        ApStat::Str => stats.str > main_stat_floor(job, stat),
        ApStat::Dex => stats.dex > main_stat_floor(job, stat),
        ApStat::Int => stats.int > main_stat_floor(job, stat),
        ApStat::Luk => stats.luk > main_stat_floor(job, stat),
    }
}

/// Whether an AP can go into `stat`. The client is stricter about Max HP and
/// MP, counting the gain twice.
pub fn may_give(stats: &CharacterStats, stat: ApStat) -> bool {
    match stat {
        ApStat::MaxHp => stats.max_hp + hp_gain(stats.job) <= MAX_HP_MP,
        ApStat::MaxMp => stats.max_mp + mp_gain(stats.job) <= MAX_HP_MP,
        ApStat::Str => stats.str < MAX_BASE_STAT,
        ApStat::Dex => stats.dex < MAX_BASE_STAT,
        ApStat::Int => stats.int < MAX_BASE_STAT,
        ApStat::Luk => stats.luk < MAX_BASE_STAT,
    }
}

/// Move an AP from `from` to `to`, keeping HP and MP within their max.
/// Returns false, changing nothing, if either end refuses.
pub fn reset(stats: &mut CharacterStats, from: ApStat, to: ApStat, total_int: i32) -> bool {
    if from == to || !may_take(stats, from, total_int) || !may_give(stats, to) {
        return false;
    }
    let job = stats.job;
    match from {
        ApStat::MaxHp => stats.max_hp -= hp_loss(job),
        ApStat::MaxMp => stats.max_mp -= mp_loss(job, total_int),
        ApStat::Str => stats.str -= 1,
        ApStat::Dex => stats.dex -= 1,
        ApStat::Int => stats.int -= 1,
        ApStat::Luk => stats.luk -= 1,
    }
    match to {
        ApStat::MaxHp => stats.max_hp += hp_gain(job),
        ApStat::MaxMp => stats.max_mp += mp_gain(job),
        ApStat::Str => stats.str += 1,
        ApStat::Dex => stats.dex += 1,
        ApStat::Int => stats.int += 1,
        ApStat::Luk => stats.luk += 1,
    }
    stats.hp = stats.hp.min(stats.max_hp);
    stats.mp = stats.mp.min(stats.max_mp);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::test_stats;

    fn warrior() -> CharacterStats {
        let mut stats = test_stats();
        stats.level = 30;
        stats.job = 110;
        // 20 + 150 earned; 160 held, so 10 AP are in HP or MP.
        (stats.str, stats.dex, stats.int, stats.luk, stats.ap) = (120, 30, 4, 4, 2);
        (stats.max_hp, stats.hp) = (1500, 1400);
        (stats.max_mp, stats.mp) = (200, 200);
        stats
    }

    #[test]
    fn an_ap_moves_between_main_stats() {
        let mut stats = warrior();
        assert!(reset(&mut stats, ApStat::Dex, ApStat::Str, 4));
        assert_eq!((stats.str, stats.dex, stats.ap), (121, 29, 2));
    }

    #[test]
    fn max_hp_gives_back_the_jobs_loss_and_keeps_hp_within_it() {
        let mut stats = warrior();
        stats.hp = 1500;
        assert!(reset(&mut stats, ApStat::MaxHp, ApStat::Str, 4));
        assert_eq!((stats.max_hp, stats.hp, stats.str), (1446, 1446, 121));
    }

    #[test]
    fn max_mp_takes_the_reset_gain() {
        let mut stats = warrior();
        assert!(reset(&mut stats, ApStat::Str, ApStat::MaxMp, 4));
        assert_eq!((stats.str, stats.max_mp), (119, 202));
    }

    #[test]
    fn hp_and_mp_need_an_ap_in_them() {
        let mut stats = warrior();
        stats.ap = 12;
        assert!(!has_hp_mp_ap(&stats));
        assert!(!reset(&mut stats, ApStat::MaxHp, ApStat::Str, 4));
        assert!(!reset(&mut stats, ApStat::MaxMp, ApStat::Str, 4));
    }

    #[test]
    fn hp_keeps_the_jobs_least() {
        let mut stats = warrior();
        // 24 * 30 + 418 = 1138
        stats.max_hp = 1138 + 53;
        assert!(!may_take(&stats, ApStat::MaxHp, 4));
        stats.max_hp = 1138 + 54;
        assert!(may_take(&stats, ApStat::MaxHp, 4));
    }

    #[test]
    fn a_magicians_mp_loss_follows_int() {
        assert_eq!(mp_loss(230, 400), 60);
        assert_eq!(mp_loss(200, 4), 30);
    }

    #[test]
    fn main_stats_keep_four_and_the_jobs_requirement() {
        let mut stats = warrior();
        stats.str = 35;
        assert!(!may_take(&stats, ApStat::Str, 4));
        stats.str = 36;
        assert!(may_take(&stats, ApStat::Str, 4));
        assert!(!may_take(&stats, ApStat::Int, 4));
        stats.int = 5;
        assert!(may_take(&stats, ApStat::Int, 4));
    }

    #[test]
    fn the_same_stat_or_a_full_one_is_refused() {
        let mut stats = warrior();
        assert!(!reset(&mut stats, ApStat::Dex, ApStat::Dex, 4));
        stats.str = MAX_BASE_STAT;
        assert!(!reset(&mut stats, ApStat::Dex, ApStat::Str, 4));
        stats.max_mp = MAX_HP_MP - 1;
        assert!(!reset(&mut stats, ApStat::Dex, ApStat::MaxMp, 4));
        assert_eq!(stats.dex, 30);
    }
}
