//! SP Reset (5050001-5050004): one skill level moves from a skill to another,
//! as the v83 client's three dialogs allow it (FUN_00a0a63f type 0x18,
//! report 195 section 4). Each item resets into one advancement's skills:
//! 5050001 the first job's, up to 5050004 the fourth's. A level may come from
//! that advancement or, while there is SP to spare, an earlier one.

use crate::character::CharacterStats;
use crate::skill::{is_fourth_job, SkillBook, SkillRecord};

pub const FIRST_ITEM: i32 = 5_050_001;
pub const LAST_ITEM: i32 = 5_050_004;

/// What the skill data says about a skill.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SkillRule {
    pub max_level: i32,
    pub invisible: bool,
    /// `(skill id, level)` pairs that must be reached first.
    pub requirements: Vec<(i32, i32)>,
}

/// A job's advancement (FUN_004e8f66): 1 for a first job (and a beginner),
/// 2 to 4 after, 0 for anything else.
pub fn advancement(job: i32) -> i32 {
    if job % 100 == 0 {
        return 1;
    }
    let advancement = job % 10 + 2;
    if (2..=4).contains(&advancement) {
        advancement
    } else {
        0
    }
}

/// The jobs a character advanced through, first job first (FUN_004a8c4f): a
/// Hero's are 100, 110, 111, 112. A beginner has none.
pub fn job_path(job: i16) -> Vec<i32> {
    let job = i32::from(job);
    let branch = job % 1000 / 100;
    if branch == 0 {
        return Vec::new();
    }
    let first = (branch + job / 1000 * 10) * 100;
    let mut path = vec![first];
    let second = job % 100 / 10;
    if second != 0 {
        let second = first + second * 10;
        path.extend((0..=job % 10).map(|n| second + n));
    }
    path
}

/// The advancement whose skills `item_id` resets into, as an index into the
/// job path: 0 for 5050001 to 3 for 5050004.
pub fn item_tab(item_id: i32) -> Option<usize> {
    (FIRST_ITEM..=LAST_ITEM)
        .contains(&item_id)
        .then(|| (item_id - FIRST_ITEM) as usize)
}

/// Whether a character of `job` may use the SP Reset for `tab`: not a
/// beginner or GM, and advanced that far.
pub fn may_use(job: i16, tab: usize) -> bool {
    (1..=8).contains(&(job % 1000 / 100)) && advancement(i32::from(job)) > tab as i32
}

/// The SP put into earlier advancements' skills beyond what they could
/// earn, which may move into `tab`'s (FUN_008b2ab7 and FUN_008b2918).
pub fn spare(stats: &CharacterStats, skills: &SkillBook, tab: usize) -> i32 {
    let mut sums = [0; 5];
    for skill in skills.iter() {
        if let Ok(advancement) = usize::try_from(advancement(skill.skill_id / 10_000)) {
            sums[advancement] += skill.level;
        }
    }
    let job = i32::from(stats.job);
    let advancement = advancement(job);
    let mage = job % 1000 / 100 == 2;
    let first = (i32::from(stats.level) - if mage { 8 } else { 10 }) * 3
        - if mage { 6 } else { 0 }
        - i32::from(stats.sp)
        - sums[2]
        - sums[3]
        - sums[4]
        + advancement
        + if advancement == 4 { 2 } else { 0 }
        - 61;
    let second = sums[2] - 121;
    let third = sums[3] - 151;
    match tab {
        1 => first,
        2 => first + second,
        3 => first + second + third,
        _ => 0,
    }
}

/// Move a level of `from` to `to` with the SP Reset `item_id`, if the
/// dialogs would let the character pick them: `to` is a skill of the item's
/// advancement that can go up, and `from` has a level that no other skill
/// relies on. `rule` looks up a skill's data. Returns the two new records,
/// `to` first.
pub fn reset(
    stats: &CharacterStats,
    skills: &mut SkillBook,
    item_id: i32,
    to: i32,
    from: i32,
    rule: impl Fn(i32) -> Option<SkillRule>,
) -> Option<[SkillRecord; 2]> {
    let tab = item_tab(item_id)?;
    if !may_use(stats.job, tab) || to == from {
        return None;
    }
    let path = job_path(stats.job);
    if path.get(tab) != Some(&(to / 10_000)) {
        return None;
    }
    let from_tab = path[..=tab].iter().position(|&job| job == from / 10_000)?;
    let source = skills.get(from).filter(|s| s.level >= 1)?.clone();
    rule(from)?;
    // FUN_008b29d4: lowering it must not take another skill below its `req`.
    let relied_on = skills.iter().filter(|s| s.level >= 1).any(|s| {
        rule(s.skill_id).is_some_and(|r| {
            r.requirements
                .iter()
                .any(|&(id, level)| id == from && source.level <= level)
        })
    });
    if relied_on || (from_tab != tab && spare(stats, skills, tab) <= 0) {
        return None;
    }
    // FUN_00a08e05: the target's `req` is met and it isn't at its cap.
    let target_rule = rule(to)?;
    let mut target = skills.get(to).cloned().unwrap_or(SkillRecord {
        skill_id: to,
        level: 0,
        master_level: 0,
        expiration: None,
    });
    if target_rule.invisible && skills.get(to).is_none() {
        return None;
    }
    let requirements_met = target_rule
        .requirements
        .iter()
        .all(|&(id, level)| skills.level(id) >= level);
    let capped = target.level >= target_rule.max_level
        || (is_fourth_job(to) && target.master_level <= target.level);
    if !requirements_met || capped {
        return None;
    }
    target.level += 1;
    let mut source = source;
    source.level -= 1;
    skills.set(target.clone());
    skills.set(source.clone());
    Some([target, source])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::test_stats;

    const POWER_STRIKE: i32 = 1001004;
    const SLASH_BLAST: i32 = 1001005;
    const SWORD_MASTERY: i32 = 1100000;
    const SWORD_BOOSTER: i32 = 1101004;
    const RAGE: i32 = 1101006;
    const RUSH: i32 = 1121006;
    const ACHILLES: i32 = 1120004;

    fn rule(skill_id: i32) -> Option<SkillRule> {
        let (max_level, requirements) = match skill_id {
            POWER_STRIKE | SLASH_BLAST => (20, vec![]),
            SWORD_MASTERY | RAGE => (20, vec![]),
            SWORD_BOOSTER => (20, vec![(SWORD_MASTERY, 5)]),
            RUSH | ACHILLES => (30, vec![]),
            _ => return None,
        };
        Some(SkillRule {
            max_level,
            invisible: false,
            requirements,
        })
    }

    fn skill(skill_id: i32, level: i32) -> SkillRecord {
        SkillRecord {
            skill_id,
            level,
            master_level: 0,
            expiration: None,
        }
    }

    fn fighter(level: u8, sp: i16, skills: &[(i32, i32)]) -> (CharacterStats, SkillBook) {
        let mut stats = test_stats();
        (stats.job, stats.level, stats.sp) = (110, level, sp);
        let book = SkillBook::from_records(skills.iter().map(|&(id, lv)| skill(id, lv)));
        (stats, book)
    }

    #[test]
    fn advancements_and_paths_follow_the_job_id() {
        assert_eq!(
            [0, 100, 110, 111, 112, 900].map(advancement),
            [1, 1, 2, 3, 4, 1]
        );
        assert_eq!(job_path(112), [100, 110, 111, 112]);
        assert_eq!(job_path(230), [200, 230]);
        assert_eq!(job_path(1111), [1100, 1110, 1111]);
        assert_eq!(job_path(2112), [2100, 2110, 2111, 2112]);
        assert!(job_path(0).is_empty() && job_path(2000).is_empty());
    }

    #[test]
    fn each_item_needs_its_advancement() {
        assert_eq!(item_tab(5_050_001), Some(0));
        assert_eq!(item_tab(5_050_004), Some(3));
        assert_eq!(item_tab(5_050_000), None);
        assert!(may_use(100, 0) && may_use(110, 1) && !may_use(110, 2));
        assert!(!may_use(0, 0) && !may_use(900, 0) && may_use(1100, 0));
    }

    #[test]
    fn a_level_moves_within_an_advancement() {
        let (stats, mut book) = fighter(30, 0, &[(POWER_STRIKE, 20), (SLASH_BLAST, 3)]);
        let [to, from] = reset(
            &stats,
            &mut book,
            5_050_001,
            SLASH_BLAST,
            POWER_STRIKE,
            rule,
        )
        .expect("reset");
        assert_eq!((to.level, from.level), (4, 19));
        assert_eq!((book.level(SLASH_BLAST), book.level(POWER_STRIKE)), (4, 19));
    }

    #[test]
    fn the_target_is_of_the_items_advancement_and_can_go_up() {
        let (stats, book) = fighter(30, 0, &[(POWER_STRIKE, 20), (SLASH_BLAST, 3)]);
        let tries = [
            (5_050_001, RAGE, POWER_STRIKE),
            (5_050_002, SLASH_BLAST, POWER_STRIKE),
            (5_050_001, POWER_STRIKE, SLASH_BLAST),
            (5_050_001, SLASH_BLAST, SLASH_BLAST),
            (5_050_003, RAGE, POWER_STRIKE),
            (5_050_001, SLASH_BLAST, 1000),
        ];
        for (item, to, from) in tries {
            let mut tried = book.clone();
            assert!(
                reset(&stats, &mut tried, item, to, from, rule).is_none(),
                "{to} from {from}"
            );
            assert_eq!(tried, book);
        }
    }

    #[test]
    fn a_level_others_rely_on_stays() {
        let (stats, mut book) = fighter(35, 0, &[(SWORD_MASTERY, 5), (SWORD_BOOSTER, 1)]);
        assert!(reset(&stats, &mut book, 5_050_002, RAGE, SWORD_MASTERY, rule).is_none());
        let (stats, mut book) = fighter(35, 0, &[(SWORD_MASTERY, 6), (SWORD_BOOSTER, 1)]);
        assert!(reset(&stats, &mut book, 5_050_002, RAGE, SWORD_MASTERY, rule).is_some());
        // Nothing learned on it yet, so the requirement doesn't hold it.
        let (stats, mut book) = fighter(35, 0, &[(SWORD_MASTERY, 5), (POWER_STRIKE, 1)]);
        assert!(reset(
            &stats,
            &mut book,
            5_050_002,
            SWORD_BOOSTER,
            SWORD_MASTERY,
            rule
        )
        .is_some());
    }

    #[test]
    fn an_earlier_advancement_gives_only_its_spare_sp() {
        // Level 30: 62 earned as a fighter, 61 of them as a warrior, all spent there.
        let first_job = |extra| {
            [
                (POWER_STRIKE, 20),
                (SLASH_BLAST, 20),
                (1000000, 20),
                (1001003, extra),
            ]
        };
        let (stats, mut book) = fighter(30, 1, &first_job(1));
        assert_eq!(spare(&stats, &book, 1), 0);
        assert!(reset(&stats, &mut book, 5_050_002, RAGE, POWER_STRIKE, rule).is_none());
        // Level 31's three went there too.
        let (stats, mut book) = fighter(31, 1, &first_job(4));
        assert_eq!(spare(&stats, &book, 1), 3);
        let [to, from] = reset(&stats, &mut book, 5_050_002, RAGE, POWER_STRIKE, rule).unwrap();
        assert_eq!((to.level, from.level), (1, 19));
        // Unspent SP isn't spare.
        let (stats, book) = fighter(31, 4, &first_job(1));
        assert_eq!(spare(&stats, &book, 1), 0);
    }

    #[test]
    fn a_fourth_job_target_stops_at_its_master_level() {
        let mut stats = test_stats();
        (stats.job, stats.level) = (112, 120);
        let mut book = SkillBook::from_records([
            SkillRecord {
                master_level: 10,
                ..skill(RUSH, 10)
            },
            SkillRecord {
                master_level: 20,
                ..skill(ACHILLES, 0)
            },
        ]);
        assert!(reset(&stats, &mut book, 5_050_004, ACHILLES, RUSH, rule).is_some());
        assert_eq!((book.level(ACHILLES), book.level(RUSH)), (1, 9));
        assert!(reset(&stats, &mut book, 5_050_004, RUSH, ACHILLES, rule).is_some());
        assert!(
            reset(&stats, &mut book, 5_050_004, RUSH, ACHILLES, rule).is_none(),
            "Achilles is at 0"
        );
        book.set(SkillRecord {
            master_level: 10,
            ..skill(RUSH, 10)
        });
        book.set(SkillRecord {
            master_level: 20,
            ..skill(ACHILLES, 5)
        });
        assert!(
            reset(&stats, &mut book, 5_050_004, RUSH, ACHILLES, rule).is_none(),
            "Rush is mastered"
        );
    }
}
