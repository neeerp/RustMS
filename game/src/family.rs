//! Families: a senior guides up to two juniors, who may have juniors of
//! their own. The senior at the top leads, and the family goes by their
//! name (report b039).

use std::time::{Duration, SystemTime};

/// Juniors a character may have. INFO and CHART tell the client, which
/// hides Add Junior at this many.
pub const MAX_JUNIORS: u16 = 2;
/// The lowest level a junior may be (pool 0x11fd: "must be over Level 10").
pub const MIN_JUNIOR_LEVEL: u8 = 10;
/// How far below the senior a junior may be (0x11fc: "within 20 levels").
pub const MAX_LEVEL_GAP: u8 = 20;
/// The most generations a family may span, top to bottom (0x1202).
pub const MAX_GENERATIONS: usize = 1000;
/// The longest precepts the client lets the leader type (b039).
pub const MAX_PRECEPTS: usize = 200;

/// Why a junior wasn't added, or an entitlement didn't apply:
/// FAMILY_RESULT's code, which picks the client's line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// 0x11f5: "You cannot add this character as a Junior."
    CannotAdd,
    /// 0x11f4: the name is wrong or they aren't online.
    NotFound,
    /// 0x11f6: "You belong to the same family."
    SameFamily,
    /// 0x11f9: they must be in the same map.
    NotSameMap,
    /// 0x11fa: they are already another's junior.
    HasSenior,
    /// 0x11fb: the junior must be of a lower level.
    NotLower,
    /// 0x11fc: more than 20 levels apart.
    LevelGap,
    /// 0x11fe: someone else is asking them already.
    Busy,
    /// 0x1202: the family would span more than 1000 generations.
    Generations,
    /// 0x11fd: the junior is below level 10.
    TooLow,
    /// 0x11f7: "You do not belong to the same family."
    NotSameFamily,
    /// 0x11ff: someone else is summoning them.
    SummonPending,
    /// 0x1201, a chat line: "The summons has failed. Your current location
    /// or state does not allow a summons." (report b050)
    CannotSummon,
}

impl Refusal {
    pub fn code(self) -> i32 {
        match self {
            Refusal::CannotAdd => 0x40,
            Refusal::NotFound => 0x41,
            Refusal::SameFamily => 0x42,
            Refusal::NotSameMap => 0x45,
            Refusal::HasSenior => 0x46,
            Refusal::NotLower => 0x47,
            Refusal::LevelGap => 0x48,
            Refusal::Busy => 0x49,
            Refusal::Generations => 0x4c,
            Refusal::TooLow => 0x4d,
            Refusal::NotSameFamily => 0x43,
            Refusal::SummonPending => 0x4a,
            Refusal::CannotSummon => 0x4b,
        }
    }
}

/// Whether a senior of level `senior` may take a junior of level `junior`.
pub fn check_levels(senior: u8, junior: u8) -> Result<(), Refusal> {
    if junior < MIN_JUNIOR_LEVEL {
        Err(Refusal::TooLow)
    } else if junior >= senior {
        Err(Refusal::NotLower)
    } else if senior - junior > MAX_LEVEL_GAP {
        Err(Refusal::LevelGap)
    } else {
        Ok(())
    }
}

/// What parting a senior of level `senior` from a junior of level `junior`
/// costs whoever asks, in mesos: `(d + 2500) * d` for the levels between
/// them, as the client's confirmation shows it (report b048).
pub fn separation_fee(senior: u8, junior: u8) -> i32 {
    let gap = i32::from(senior.saturating_sub(junior));
    (gap + 2500) * gap
}

/// The Rep that parting from a junior of level `junior` takes from the
/// senior's own senior; the senior loses twice it (report b048).
pub fn separation_rep(junior: u8) -> i32 {
    let level = i32::from(junior);
    (level / 20 + 10) * level
}

/// Rep a junior earns their senior and their senior's senior for each mob
/// they share in killing, each boss, and each level they gain. v83's own
/// rule isn't known; these are Cosmic's settings (report b049).
pub const REP_PER_KILL: i32 = 4;
pub const REP_PER_BOSS: i32 = 20;
pub const REP_PER_LEVEL: i32 = 200;

/// What `gain` Rep from a junior of level `junior` comes to for a senior of
/// level `senior`: half, once the junior has passed them (the client's
/// warnings 0x121b, 0x121c and 0x1246).
pub fn rep_gain(gain: i32, senior: u8, junior: u8) -> i32 {
    if senior < junior {
        gain / 2
    } else {
        gain
    }
}

/// What spending an entitlement does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Benefit {
    /// Go to a family member.
    Teleport,
    /// Bring a family member to the player.
    Summon,
    /// EXP or drops, or both, at `percent` for `minutes`, for the player or
    /// for their party.
    Rate {
        exp: bool,
        drop: bool,
        percent: u16,
        minutes: u16,
        party: bool,
    },
    /// EXP and drops at `percent` for `minutes`, once enough of the family
    /// below the player is online.
    Bonding { percent: u16, minutes: u16 },
}

/// An entitlement the Family window offers, bought with Rep.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entitlement {
    /// The client asks for a family member's name first for 0 and 1.
    pub kind: u8,
    pub cost: i32,
    /// Uses a day.
    pub limit: i32,
    pub benefit: Benefit,
}

impl Entitlement {
    /// Whether USE_FAMILY names a family member.
    pub fn targeted(&self) -> bool {
        self.kind <= 1
    }
}

const fn rate(cost: i32, exp: bool, percent: u16, minutes: u16, party: bool) -> Entitlement {
    Entitlement {
        kind: 2,
        cost,
        limit: 1,
        benefit: Benefit::Rate {
            exp,
            drop: !exp,
            percent,
            minutes,
            party,
        },
    }
}

/// The entitlements that go to a family member, by their index.
pub const TELEPORT: usize = 0;
pub const SUMMON: usize = 1;

/// How many of the juniors and juniors' juniors, the six the Family Tree
/// shows below the player, must be online for the bonding entitlement
/// (pool 0x1200, which the client never shows).
pub const BONDING_JUNIORS: usize = 6;

/// The entitlements, by the index USE_FAMILY sends. v83's own list isn't
/// known; these costs, limits and benefits are Cosmic's (report b049), and
/// their names and descriptions are content-pack text.
pub const ENTITLEMENTS: [Entitlement; 11] = [
    Entitlement {
        kind: 1,
        cost: 300,
        limit: 1,
        benefit: Benefit::Teleport,
    },
    Entitlement {
        kind: 1,
        cost: 500,
        limit: 1,
        benefit: Benefit::Summon,
    },
    rate(700, false, 150, 15, false),
    rate(800, true, 150, 15, false),
    Entitlement {
        kind: 2,
        cost: 1000,
        limit: 1,
        benefit: Benefit::Bonding {
            percent: 200,
            minutes: 30,
        },
    },
    rate(1200, false, 200, 15, false),
    rate(1500, true, 200, 15, false),
    rate(2000, false, 200, 30, false),
    rate(2500, true, 200, 30, false),
    rate(4000, false, 200, 30, true),
    rate(5000, true, 200, 30, true),
];

impl Benefit {
    /// The EXP and drop rates it gives, 1 for none.
    pub fn rates(&self) -> (f64, f64) {
        let rate = |on: bool, percent: u16| if on { f64::from(percent) / 100.0 } else { 1.0 };
        match *self {
            Benefit::Rate {
                exp, drop, percent, ..
            } => (rate(exp, percent), rate(drop, percent)),
            Benefit::Bonding { percent, .. } => (rate(true, percent), rate(true, percent)),
            Benefit::Teleport | Benefit::Summon => (1.0, 1.0),
        }
    }

    /// How long it lasts; 0 for a trip.
    pub fn duration(&self) -> Duration {
        match *self {
            Benefit::Rate { minutes, .. } | Benefit::Bonding { minutes, .. } => {
                Duration::from_secs(u64::from(minutes) * 60)
            }
            Benefit::Teleport | Benefit::Summon => Duration::ZERO,
        }
    }

    /// The icon `FAMILY_SET_PRIVILEGE` shows, `RightIcon/<n>`: 2 drops, 3
    /// EXP, 4 both (report b042).
    pub fn icon(&self) -> u8 {
        match *self {
            Benefit::Rate { exp: true, .. } => 3,
            Benefit::Rate { .. } => 2,
            _ => 4,
        }
    }
}

/// A rate entitlement working for a character until `until`. One at a
/// time: a new one takes the old one's place, as its icon does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FamilyBuff {
    /// Its index in `ENTITLEMENTS`.
    pub index: usize,
    pub until: SystemTime,
}

impl FamilyBuff {
    /// Entitlement `index`, starting `now`.
    pub fn new(index: usize, now: SystemTime) -> Self {
        FamilyBuff {
            index,
            until: now + ENTITLEMENTS[index].benefit.duration(),
        }
    }

    pub fn benefit(&self) -> Benefit {
        ENTITLEMENTS[self.index].benefit
    }

    /// The EXP and drop rates it gives at `now`.
    pub fn rates(&self, now: SystemTime) -> (f64, f64) {
        if now < self.until {
            self.benefit().rates()
        } else {
            (1.0, 1.0)
        }
    }
}

/// A character's senior and juniors, by character id; both empty out of a
/// family.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Relatives {
    pub senior: Option<i32>,
    pub juniors: Vec<i32>,
}

impl Relatives {
    /// The senior, then the juniors.
    pub fn ids(&self) -> impl Iterator<Item = i32> + '_ {
        self.senior.into_iter().chain(self.juniors.iter().copied())
    }
}

/// What the Family window shows of the character's family (FAMILY_INFO).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FamilyInfo {
    /// Rep to spend, and all ever earned.
    pub reputation: i32,
    pub total_reputation: i32,
    /// Rep earned today.
    pub today_reputation: i32,
    pub juniors: u16,
    /// The leader; 0 out of a family.
    pub leader_id: i32,
    /// The leader's name; empty out of a family.
    pub name: String,
    /// The leader's message to the family.
    pub precepts: String,
    /// Entitlements used today: (index, times).
    pub used: Vec<(i32, i32)>,
}

/// A member as the Family Tree draws them (FAMILY_CHART).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChartMember {
    pub id: i32,
    /// 0 for the leader.
    pub senior_id: i32,
    pub job: i16,
    pub level: u8,
    pub online: bool,
    pub reputation: i32,
    pub total_reputation: i32,
    /// Rep recorded to their senior.
    pub senior_reputation: i32,
    pub today_reputation: i32,
    pub map_id: i32,
    pub name: String,
}

/// The Family Tree around `viewed`: the leader, the two seniors above them,
/// their sibling, juniors and juniors' juniors, parents before children.
/// Empty out of a family.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FamilyChart {
    pub viewed: i32,
    pub members: Vec<ChartMember>,
    /// Head counts: -1 for the whole family, 0 for `viewed`'s seniors, and a
    /// junior's id for their branch.
    pub counts: Vec<(i32, i32)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn juniors_are_at_least_ten_and_within_twenty_levels_below() {
        assert_eq!(check_levels(30, 9), Err(Refusal::TooLow));
        assert_eq!(check_levels(30, 10), Ok(()));
        assert_eq!(check_levels(30, 29), Ok(()));
        assert_eq!(check_levels(30, 30), Err(Refusal::NotLower));
        assert_eq!(check_levels(30, 31), Err(Refusal::NotLower));
        assert_eq!(check_levels(50, 30), Ok(()));
        assert_eq!(check_levels(51, 30), Err(Refusal::LevelGap));
    }

    #[test]
    fn separating_costs_by_the_gap_and_the_juniors_level() {
        assert_eq!(separation_fee(60, 50), 25_100);
        assert_eq!(separation_fee(50, 50), 0);
        // A junior who has outgrown the senior costs nothing.
        assert_eq!(separation_fee(50, 60), 0);
        assert_eq!(separation_rep(50), 600);
        assert_eq!(separation_rep(19), 190);
        assert_eq!(separation_rep(20), 220);
    }

    #[test]
    fn rate_entitlements_raise_their_rates_while_they_last() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let drops = FamilyBuff::new(2, now);
        assert_eq!(drops.until, now + Duration::from_secs(15 * 60));
        assert_eq!(drops.rates(now), (1.0, 1.5));
        assert_eq!(drops.rates(drops.until), (1.0, 1.0));
        assert_eq!(drops.benefit().icon(), 2);
        let exp = FamilyBuff::new(10, now);
        assert_eq!(exp.rates(now), (2.0, 1.0));
        assert_eq!(exp.benefit().icon(), 3);
        let bonding = FamilyBuff::new(4, now);
        assert_eq!(bonding.rates(now), (2.0, 2.0));
        assert_eq!(bonding.benefit().icon(), 4);
    }

    #[test]
    fn a_junior_who_has_passed_their_senior_earns_them_half() {
        assert_eq!(rep_gain(REP_PER_KILL, 30, 20), 4);
        assert_eq!(rep_gain(REP_PER_KILL, 30, 30), 4);
        assert_eq!(rep_gain(REP_PER_KILL, 30, 31), 2);
        assert_eq!(rep_gain(5, 30, 31), 2);
    }
}
