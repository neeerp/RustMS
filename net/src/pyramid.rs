//! Nett's Pyramid's tally: a run's kills, cool kills and misses, and the
//! gauge they keep up on each stage (`agent_docs/pyramid.md`; Cosmic
//! `Pyramid.java`).

use game_data::mobs::MobInfo;
use game_data::{CountEffect, Massacre};

/// SESSION_VALUE keys for the run's tallies, which the client draws on its
/// Count panel and its result screen.
pub const HIT: &str = "massacre_hit";
pub const MISS: &str = "massacre_miss";
pub const COOL: &str = "massacre_cool";
/// Uses of Rage of Pharaoh left; the client allows 1020 only above 0.
pub const SKILL: &str = "massacre_skill";

/// What one attack did to a stage's mobs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Hits {
    pub kills: u16,
    pub cools: u16,
    pub misses: u16,
}

impl Hits {
    pub fn any(self) -> bool {
        self.kills + self.cools + self.misses > 0
    }
}

/// Whether a kill with a hit for `damage` is a cool one: as hard as the
/// mob's `coolDamage`, `coolDamageProb` percent of the time (`roll` in
/// 0..100).
pub fn cool(info: &MobInfo, damage: i64, roll: u32) -> bool {
    info.cool_damage > 0
        && damage >= i64::from(info.cool_damage)
        && i64::from(roll) < i64::from(info.cool_damage_prob)
}

/// A run's tally.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    pub hit: i32,
    pub miss: i32,
    pub cool: i32,
    pub skill: i32,
    /// What the stage's kills and cool kills have put back on its gauge,
    /// less what its misses took off: the PYRAMID_GAUGE value. The client
    /// draws the gauge as the stage's elapsed seconds times `decrease`,
    /// less this.
    pub count: i32,
    /// How many of the stage's `countEffect`s the run has reached (Cosmic
    /// `buffcount`).
    pub effects: usize,
}

impl Tally {
    /// A new stage's gauge starts full (Cosmic `startGaugeSchedule`).
    pub fn new_stage(&mut self) {
        self.count = 0;
    }

    /// How much of the gauge is gone `seconds` into the stage.
    pub fn lost(&self, gauge: &Massacre, seconds: i32) -> i32 {
        seconds * gauge.decrease - self.count
    }

    /// Count an attack's `hits`, `seconds` into the stage. A kill or cool
    /// kill puts back at most what is gone (Cosmic `kill`, `cool`); a miss
    /// always takes its share (`miss`).
    pub fn add(&mut self, hits: Hits, gauge: &Massacre, seconds: i32) {
        for _ in 0..hits.kills {
            self.hit += 1;
            self.count += gauge.hit_add.min(self.lost(gauge, seconds).max(0));
        }
        for _ in 0..hits.cools {
            self.cool += 1;
            self.count += gauge.cool_add.min(self.lost(gauge, seconds).max(0));
        }
        for _ in 0..hits.misses {
            self.miss += 1;
            self.count -= gauge.miss_sub;
        }
    }

    /// The `countEffect`s the run's kills and cool kills have newly
    /// reached, from any stage; each with `skillUse` gives a use of Rage of
    /// Pharaoh (Cosmic `checkBuffs`, which reads the same counts from code).
    pub fn reached(&mut self, gauge: &Massacre) -> Vec<CountEffect> {
        let kills = self.hit + self.cool;
        let new: Vec<_> = (gauge.count_effects.iter().skip(self.effects))
            .take_while(|effect| effect.count <= kills)
            .copied()
            .collect();
        self.effects += new.len();
        let uses = new.iter().filter(|effect| effect.skill_use).count();
        self.skill += i32::try_from(uses).unwrap_or(0);
        new
    }

    /// Spends a use of Rage of Pharaoh, if one is left (Cosmic `useSkill`).
    pub fn spend_rage(&mut self) -> bool {
        let left = self.skill > 0;
        if left {
            self.skill -= 1;
        }
        left
    }

    /// The whole seconds into the stage at which the gauge runs out, as
    /// things stand; never on a gauge that doesn't go down.
    pub fn runs_out(&self, gauge: &Massacre) -> Option<i32> {
        (gauge.decrease > 0).then(|| {
            let left = (gauge.total + self.count).max(0);
            (left + gauge.decrease - 1) / gauge.decrease
        })
    }

    /// The session values for what `hits` changed, with the uses of Rage
    /// of Pharaoh if `rage`, or for all four.
    pub fn values(&self, hits: Option<Hits>, rage: bool) -> Vec<(&'static str, i32)> {
        let all = [
            (HIT, self.hit, hits.is_none_or(|h| h.kills > 0)),
            (MISS, self.miss, hits.is_none_or(|h| h.misses > 0)),
            (COOL, self.cool, hits.is_none_or(|h| h.cools > 0)),
            (SKILL, self.skill, rage || hits.is_none()),
        ];
        all.into_iter()
            .filter(|&(_, _, changed)| changed)
            .map(|(key, value, _)| (key, value))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solo_normal() -> Massacre {
        Massacre {
            total: 100,
            decrease: 2,
            hit_add: 1,
            cool_add: 6,
            miss_sub: 5,
            ..Massacre::default()
        }
    }

    #[test]
    fn kills_put_back_only_what_the_gauge_has_lost() {
        let gauge = solo_normal();
        let mut tally = Tally::default();
        // At the start nothing is gone, so a kill counts but adds nothing.
        tally.add(
            Hits {
                kills: 1,
                ..Hits::default()
            },
            &gauge,
            0,
        );
        assert_eq!((tally.hit, tally.count), (1, 0));
        // 3 s in, 6 is gone: a cool kill gives back its 6, and the next
        // kill nothing.
        let hits = Hits {
            kills: 1,
            cools: 1,
            ..Hits::default()
        };
        tally.add(hits, &gauge, 3);
        assert_eq!((tally.hit, tally.cool, tally.count), (2, 1, 6));
        assert_eq!(tally.lost(&gauge, 3), 0);
        // A miss always takes its 5.
        tally.add(
            Hits {
                misses: 1,
                ..Hits::default()
            },
            &gauge,
            3,
        );
        assert_eq!((tally.miss, tally.count, tally.lost(&gauge, 3)), (1, 1, 5));
    }

    #[test]
    fn the_gauge_runs_out_when_all_it_holds_is_lost() {
        let gauge = solo_normal();
        let mut tally = Tally::default();
        assert_eq!(tally.runs_out(&gauge), Some(50));
        tally.add(
            Hits {
                misses: 3,
                ..Hits::default()
            },
            &gauge,
            0,
        );
        // 85 left at 2 a second: gone 43 s in.
        assert_eq!(tally.runs_out(&gauge), Some(43));
        tally.count = -200;
        assert_eq!(tally.runs_out(&gauge), Some(0));
        tally.new_stage();
        assert_eq!(tally.runs_out(&gauge), Some(50));
        let still = Massacre {
            decrease: 0,
            ..gauge
        };
        assert_eq!(tally.runs_out(&still), None);
    }

    #[test]
    fn a_kill_is_cool_as_hard_as_the_mobs_cool_damage_and_as_often_as_it_says() {
        let info = MobInfo {
            cool_damage: 200,
            cool_damage_prob: 9,
            ..MobInfo::default()
        };
        assert!(cool(&info, 200, 8));
        assert!(!cool(&info, 200, 9));
        assert!(!cool(&info, 199, 0));
        assert!(!cool(&MobInfo::default(), 10_000, 0));
    }

    #[test]
    fn only_what_an_attack_changed_is_sent_again() {
        let tally = Tally {
            hit: 4,
            miss: 2,
            cool: 1,
            skill: 1,
            ..Tally::default()
        };
        let misses = Hits {
            misses: 1,
            ..Hits::default()
        };
        assert_eq!(tally.values(Some(misses), false), [(MISS, 2)]);
        assert_eq!(tally.values(Some(misses), true), [(MISS, 2), (SKILL, 1)]);
        assert_eq!(
            tally.values(None, false),
            [(HIT, 4), (MISS, 2), (COOL, 1), (SKILL, 1)]
        );
    }

    fn effect(count: i32, buff: i32, skill_use: bool) -> CountEffect {
        CountEffect {
            count,
            buff,
            skill_use,
        }
    }

    #[test]
    fn kills_and_cool_kills_reach_the_count_effects_once_each() {
        let gauge = Massacre {
            count_effects: vec![
                effect(250, 2022585, false),
                effect(500, 2022586, true),
                effect(1000, 2022587, true),
                effect(1500, 0, true),
            ],
            ..solo_normal()
        };
        let mut tally = Tally {
            hit: 249,
            ..Tally::default()
        };
        assert!(tally.reached(&gauge).is_empty());
        tally.cool = 1;
        assert_eq!(tally.reached(&gauge), [effect(250, 2022585, false)]);
        assert!(tally.reached(&gauge).is_empty());
        assert_eq!(tally.skill, 0);
        // One attack may pass more than one.
        tally.hit = 1499;
        let reached = tally.reached(&gauge);
        assert_eq!(reached.len(), 3, "{reached:?}");
        assert_eq!((tally.effects, tally.skill), (4, 3));
        // A new stage keeps them.
        tally.new_stage();
        assert!(tally.reached(&gauge).is_empty());
        assert!(tally.spend_rage());
        assert_eq!(tally.skill, 2);
        tally.skill = 0;
        assert!(!tally.spend_rage());
        assert_eq!(tally.skill, 0);
    }
}
