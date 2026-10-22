//! Families: a senior guides up to two juniors, who may have juniors of
//! their own. The senior at the top leads, and the family goes by their
//! name (report b039).

/// Juniors a character may have. INFO and CHART tell the client, which
/// hides Add Junior at this many.
pub const MAX_JUNIORS: u16 = 2;
/// The lowest level a junior may be (pool 0x11fd: "must be over Level 10").
pub const MIN_JUNIOR_LEVEL: u8 = 10;
/// How far below the senior a junior may be (0x11fc: "within 20 levels").
pub const MAX_LEVEL_GAP: u8 = 20;
/// The most generations a family may span, top to bottom (0x1202).
pub const MAX_GENERATIONS: usize = 1000;

/// Why a junior wasn't added: FAMILY_RESULT's code, which picks the
/// client's line.
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
}
