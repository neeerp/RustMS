//! What a mob skill (`MobSkill.img/<id>`) does, for the skills the server
//! lets mobs cast.

use crate::MobStat;

/// Heal: the mob and those around it regain HP and MP.
pub const HEAL: i32 = 114;
/// Summon: the mob calls up more mobs.
pub const SUMMON: i32 = 200;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MobSkillEffect {
    /// Statuses on the caster, or on every mob in its area.
    Statuses(Vec<(MobStat, i16)>),
    Heal,
    Summon,
}

/// What `skill` does with its value `x`, or `None` for the ones not cast
/// yet: the diseases (120-136), dispel, banish, poison mist, and the damage
/// counters (143-145).
///
/// The v83 client multiplies a mob's attack by its PowerUp and MagicUp as
/// percents, and adds Pad and Mad, so Power Up (100, 115%) is PowerUp, not
/// Pad as OdinMS-family servers have it; the guards (102, 103) likewise.
pub fn effect(skill: i32, x: i32) -> Option<MobSkillEffect> {
    let x = i16::try_from(x).unwrap_or(i16::MAX);
    let stat = match skill {
        100 | 110 => MobStat::PowerUp,
        101 | 111 => MobStat::MagicUp,
        102 | 112 => MobStat::PGuardUp,
        103 | 113 => MobStat::MGuardUp,
        115 | 156 => MobStat::Speed,
        140 => MobStat::PImmune,
        141 => MobStat::MImmune,
        142 => MobStat::HardSkin,
        150 => MobStat::Pad,
        151 => MobStat::Mad,
        152 => MobStat::Pdr,
        153 => MobStat::Mdr,
        154 => MobStat::Acc,
        155 => MobStat::Eva,
        157 => MobStat::SealSkill,
        HEAL => return Some(MobSkillEffect::Heal),
        SUMMON => return Some(MobSkillEffect::Summon),
        _ => return None,
    };
    Some(MobSkillEffect::Statuses(vec![(stat, x)]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffs_heals_and_summons() {
        assert_eq!(
            effect(110, 115),
            Some(MobSkillEffect::Statuses(vec![(MobStat::PowerUp, 115)]))
        );
        assert_eq!(effect(HEAL, 700), Some(MobSkillEffect::Heal));
        assert_eq!(effect(SUMMON, 0), Some(MobSkillEffect::Summon));
        assert_eq!(effect(123, 1), None);
        assert_eq!(effect(145, 1), None);
    }
}
