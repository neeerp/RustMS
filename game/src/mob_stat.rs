//! A mob's temporary statuses: the debuffs players put on it and the buffs
//! it gives itself.

/// A temporary status, by its bit in the v83 client's 128-bit mob flags
/// (the initializers from 0x78973c). The names follow later clients; the
/// bits are the v83 ones. Each carries a value, the skill that caused it,
/// and how long it lasts. Burned (27), the counters (29, 30), and
/// Disable (31) carry more, and aren't here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MobStat {
    Pad = 0,
    Pdr = 1,
    Mad = 2,
    Mdr = 3,
    Acc = 4,
    Eva = 5,
    Speed = 6,
    Stun = 7,
    Freeze = 8,
    Poison = 9,
    Seal = 10,
    Darkness = 11,
    PowerUp = 12,
    MagicUp = 13,
    PGuardUp = 14,
    MGuardUp = 15,
    Doom = 16,
    Web = 17,
    PImmune = 18,
    MImmune = 19,
    HardSkin = 21,
    Ambush = 22,
    Venom = 24,
    Blind = 25,
    SealSkill = 26,
    Dazzle = 28,
    RiseByToss = 32,
    BodyPressure = 33,
    Weakness = 34,
}

impl MobStat {
    pub fn bit(self) -> u8 {
        self as u8
    }

    /// The order the client reads the statuses in (FUN_0078b0b1), which
    /// isn't quite that of their bits.
    pub const DECODE_ORDER: [MobStat; 29] = [
        MobStat::Pad,
        MobStat::Pdr,
        MobStat::Mad,
        MobStat::Mdr,
        MobStat::Acc,
        MobStat::Eva,
        MobStat::Speed,
        MobStat::Stun,
        MobStat::Freeze,
        MobStat::Poison,
        MobStat::Seal,
        MobStat::Darkness,
        MobStat::PowerUp,
        MobStat::MagicUp,
        MobStat::PGuardUp,
        MobStat::MGuardUp,
        MobStat::PImmune,
        MobStat::MImmune,
        MobStat::Doom,
        MobStat::Web,
        MobStat::HardSkin,
        MobStat::Ambush,
        MobStat::Venom,
        MobStat::Blind,
        MobStat::SealSkill,
        MobStat::Dazzle,
        MobStat::RiseByToss,
        MobStat::BodyPressure,
        MobStat::Weakness,
    ];

    /// Whether the client, after setting or resetting this status, reads a
    /// byte more and has the mob act on it (FUN_0068ad7f): the ones that
    /// stop it.
    pub fn stops(self) -> bool {
        matches!(
            self,
            MobStat::Stun | MobStat::Freeze | MobStat::Doom | MobStat::RiseByToss
        )
    }

    /// Whether a boss shrugs this status off.
    pub fn spares_bosses(self) -> bool {
        matches!(
            self,
            MobStat::Stun | MobStat::Freeze | MobStat::Seal | MobStat::Doom | MobStat::Web
        )
    }
}
