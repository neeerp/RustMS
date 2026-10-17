//! What a mob skill (`MobSkill.img/<id>`) does, for the skills the server
//! lets mobs cast.

use crate::{BuffStat, Disease, MobStat};
use std::time::Duration;

/// Heal: the mob and those around it regain HP and MP.
pub const HEAL: i32 = 114;
/// Poison Mist: the mob leaves a mist that hurts the players in it.
pub const POISON_MIST: i32 = 131;
/// Summon: the mob calls up more mobs.
pub const SUMMON: i32 = 200;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MobSkillEffect {
    /// Statuses on the caster, or on every mob in its area.
    Statuses(Vec<(MobStat, i16)>),
    Heal,
    Summon,
    /// A mist over its area; a v83 client in it loses `x` HP a second.
    Mist,
    /// Counters on the caster, or on every mob in its area: each attack on
    /// one hurts the attacker `x`, give or take half of `y`.
    Counters(Vec<MobStat>),
    /// A disease on every player in its area.
    Disease(BuffStat),
    /// Ends the skills' buffs of every player in its area.
    Dispel,
    /// Sends players in its area to the mob's `ban` map.
    Banish,
}

/// What `skill` does with its value `x`, or `None` for the ones not cast
/// yet (130, 133-136).
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
        POISON_MIST => return Some(MobSkillEffect::Mist),
        143 => return Some(MobSkillEffect::Counters(vec![MobStat::PCounter])),
        144 => return Some(MobSkillEffect::Counters(vec![MobStat::MCounter])),
        145 => {
            return Some(MobSkillEffect::Counters(vec![
                MobStat::PCounter,
                MobStat::MCounter,
            ]))
        }
        127 => return Some(MobSkillEffect::Dispel),
        129 => return Some(MobSkillEffect::Banish),
        _ => return disease(skill).map(MobSkillEffect::Disease),
    };
    Some(MobSkillEffect::Statuses(vec![(stat, x)]))
}

/// The disease a mob skill inflicts, cast or with an attack (`disease`).
pub fn disease(skill: i32) -> Option<BuffStat> {
    Some(match skill {
        120 => BuffStat::Seal,
        121 => BuffStat::Darkness,
        122 => BuffStat::Weakness,
        123 => BuffStat::Stun,
        124 => BuffStat::Curse,
        125 => BuffStat::Poison,
        126 => BuffStat::Slow,
        128 => BuffStat::Seduce,
        132 => BuffStat::Confuse,
        _ => return None,
    })
}

/// The disease mob skill `skill` at `level` inflicts, with its `x` and
/// `time` in seconds. A stat the client sees as set only when its value
/// isn't 0, so one without `x` gets 1.
pub fn inflicted(skill: i32, level: i32, x: i32, seconds: i32) -> Option<Disease> {
    Some(Disease {
        stat: disease(skill)?,
        value: i16::try_from(x.max(1)).unwrap_or(i16::MAX),
        skill_id: skill | level << 16,
        lasts: Duration::from_secs(seconds.max(0) as u64),
    })
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
        assert_eq!(effect(POISON_MIST, 30), Some(MobSkillEffect::Mist));
        assert_eq!(
            effect(123, 1),
            Some(MobSkillEffect::Disease(BuffStat::Stun))
        );
        assert_eq!(effect(127, 0), Some(MobSkillEffect::Dispel));
        assert_eq!(effect(129, 0), Some(MobSkillEffect::Banish));
        assert_eq!(effect(133, 1), None);
        let slow = inflicted(126, 2, 80, 10).unwrap();
        assert_eq!(
            (slow.stat, slow.value, slow.skill_id, slow.lasts),
            (BuffStat::Slow, 80, 126 | 2 << 16, Duration::from_secs(10))
        );
        assert_eq!(inflicted(123, 1, 0, 5).unwrap().value, 1);
        assert_eq!(inflicted(100, 1, 0, 5), None);
        assert_eq!(
            effect(145, 220),
            Some(MobSkillEffect::Counters(vec![
                MobStat::PCounter,
                MobStat::MCounter
            ]))
        );
    }
}
