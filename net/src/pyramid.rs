//! Nett's Pyramid's tally: a run's kills, cool kills and misses, and the
//! gauge they keep up on each stage (`agent_docs/pyramid.md`; Cosmic
//! `Pyramid.java`).

use crate::events::EventKind;
use game::Player;
use game_data::mobs::MobInfo;
use game_data::{CountEffect, Massacre};

/// SESSION_VALUE keys for the run's tallies, which the client draws on its
/// Count panel and its result screen.
pub const HIT: &str = "massacre_hit";
pub const MISS: &str = "massacre_miss";
pub const COOL: &str = "massacre_cool";
/// Uses of Rage of Pharaoh left; the client allows 1020 only above 0.
pub const SKILL: &str = "massacre_skill";

/// Protector of Pharaoh (29932): Dalair's medal quest for 50,000 kills in
/// the Pyramid. Record 7760 counts each player's own kills and cool kills
/// (`Check/1 infoNumber`), and Duarte gives the medal (`QuestInfo` 1; the
/// quest's `Act/1` gives nothing).
pub const PROTECTOR_QUEST: i16 = 29932;
pub const PROTECTOR_KILLS: i16 = 7760;
pub const PROTECTOR_NEEDED: i32 = 50_000;
pub const PROTECTOR_MEDAL: i32 = 1142142;

/// The player's kills counted toward Protector of Pharaoh.
pub fn protector_kills(player: &Player) -> i32 {
    kills_in(player, PROTECTOR_KILLS)
}

/// Honorary Subway Worker (29931): Mr. Lim's medal quest for 10,000 kills
/// on the Dusty Platform's stages. Record 7662 counts them (`Check/1
/// infoNumber`, Cosmic `1052115`), and Mr. Lim gives the medal and starts
/// and ends the quest. The quest's text shows record 7762 instead, so the
/// count is kept there too (QUESTIONS b319).
pub const HONORARY_QUEST: i16 = 29931;
pub const HONORARY_KILLS: i16 = 7662;
pub const HONORARY_KILLS_SHOWN: i16 = 7762;
pub const HONORARY_NEEDED: i32 = 10_000;
pub const HONORARY_MEDAL: i32 = 1142141;

/// The player's kills counted toward Honorary Subway Worker.
pub fn honorary_kills(player: &Player) -> i32 {
    kills_in(player, HONORARY_KILLS)
}

/// The records a Massacre stage on `map_id` counts its player's kills in:
/// the Dusty Platform's for Honorary Subway Worker, or the Pyramid's for
/// Protector of Pharaoh. None off a stage, as for a kill heard after the
/// run moved its player on.
pub fn kill_records(map_id: i32) -> &'static [i16] {
    if crate::events::SUBWAY.stage_on(map_id).is_some() {
        &[HONORARY_KILLS, HONORARY_KILLS_SHOWN]
    } else if crate::events::pyramid_stage(map_id).is_some() {
        &[PROTECTOR_KILLS]
    } else {
        &[]
    }
}

/// The kills record `record` holds.
pub fn kills_in(player: &Player, record: i16) -> i32 {
    (player.quests.info(record)).map_or(0, |count| count.parse().unwrap_or(0))
}

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

/// How a Massacre run's kills and cool kills rank it: the fewest for S, A,
/// B and C, and the EXP each of them gives before the kills' own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ranks {
    pub least: [i32; 4],
    pub exp: [i32; 4],
}

/// Nett's Pyramid's ranks in the mode of index `mode` (Cosmic `sendScore`).
pub const fn pyramid_ranks(mode: i32) -> Ranks {
    Ranks {
        least: [3000, 2000, 1500, 500],
        exp: [
            60_500 + 5_500 * mode,
            55_000 + 5_000 * mode,
            46_750 + 4_250 * mode,
            22_000 + 2_000 * mode,
        ],
    }
}

/// The Dusty Platform's ranks, which no source gives (QUESTIONS b319):
/// the Pyramid's kills for its 14 minutes scaled to the platform's 9, and
/// for each rank a share of a level 25's EXP (a half for S down to a
/// tenth for C).
pub const SUBWAY_RANKS: Ranks = Ranks {
    least: [1930, 1290, 964, 321],
    exp: [13_500, 9_500, 5_400, 2_700],
};

/// The ranks of the Massacre run `kind`, if it is one.
pub fn ranks(kind: EventKind) -> Option<Ranks> {
    match kind {
        EventKind::Subway => Some(SUBWAY_RANKS),
        _ => Some(pyramid_ranks(kind.pyramid_mode()?.index())),
    }
}

/// A run's rank, 0 for S to 4 for D, and the EXP it gives, by its kills and
/// cool kills and its `ranks` (Cosmic `sendScore`). A run that `finished`,
/// outlasting the last stage's clock, ranks from S down; one whose gauge
/// ran out gets C at A's kills, or D.
pub fn score(tally: &Tally, ranks: &Ranks, finished: bool) -> (u8, i32) {
    let kills = tally.hit + tally.cool;
    let rank = match ranks.least.iter().position(|&least| kills >= least) {
        _ if !finished && kills >= ranks.least[1] => 3,
        _ if !finished => 4,
        Some(rank) => rank as u8,
        None => 4,
    };
    let base = ranks.exp.get(usize::from(rank)).copied().unwrap_or(0);
    (rank, base + tally.hit * 2 + tally.cool * 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_massacre_stage_counts_kills_toward_its_medal() {
        assert_eq!(kill_records(910320200), [7662, 7762]);
        assert_eq!(kill_records(926021300), [7760]);
        for off in [910320001, 926010001, 100000000] {
            assert!(kill_records(off).is_empty(), "{off}");
        }
    }

    #[test]
    fn the_score_ranks_the_runs_kills_and_gives_its_modes_exp() {
        let tally = |hit, cool| Tally {
            hit,
            cool,
            ..Tally::default()
        };
        let mode = pyramid_ranks;
        assert_eq!(
            score(&tally(2900, 100), &mode(0), true),
            (0, 60_500 + 5_800 + 1_000)
        );
        assert_eq!(score(&tally(2000, 0), &mode(3), true), (1, 70_000 + 4_000));
        assert_eq!(
            score(&tally(1499, 1), &mode(1), true),
            (2, 51_000 + 2_998 + 10)
        );
        assert_eq!(score(&tally(500, 0), &mode(2), true), (3, 26_000 + 1_000));
        assert_eq!(score(&tally(499, 0), &mode(3), true), (4, 998));
        // A gauge that ran out gives C at most.
        assert_eq!(score(&tally(3000, 0), &mode(0), false), (3, 22_000 + 6_000));
        assert_eq!(score(&tally(1, 0), &mode(3), false), (4, 2));
    }

    #[test]
    fn the_dusty_platforms_runs_rank_on_fewer_kills_for_less_exp() {
        let tally = |hit| Tally {
            hit,
            ..Tally::default()
        };
        let subway = ranks(EventKind::Subway).unwrap();
        assert_eq!(score(&tally(1930), &subway, true), (0, 13_500 + 3_860));
        assert_eq!(score(&tally(1929), &subway, true), (1, 9_500 + 3_858));
        assert_eq!(score(&tally(321), &subway, true), (3, 2_700 + 642));
        assert_eq!(score(&tally(320), &subway, true), (4, 640));
        assert_eq!(score(&tally(1290), &subway, false), (3, 2_700 + 2_580));
        assert_eq!(score(&tally(1289), &subway, false), (4, 2_578));
        // The Pyramid's, by mode; nothing else is a Massacre run.
        let hell = EventKind::PyramidParty(crate::events::PyramidMode::Hell);
        assert_eq!(ranks(hell), Some(pyramid_ranks(3)));
        assert_eq!(ranks(EventKind::Dojo), None);
    }

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
