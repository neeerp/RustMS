//! Skill templates (`Skill.nx`), loaded once.

use crate::load::{self, asset_path};
use game::{BuffStat, InventoryType, Player};
use game_data::skills::{SkillData, SkillInfo, SkillLevel};
use std::sync::OnceLock;

pub fn data() -> Result<&'static SkillData, &'static str> {
    static SKILLS: OnceLock<Result<SkillData, String>> = OnceLock::new();
    load::once(&SKILLS, || {
        SkillData::load(asset_path(
            "RUSTMS_SKILL_NX_PATH",
            "assets/game-data/Skill.nx",
        ))
    })
}

pub fn info(skill_id: i32) -> Option<SkillInfo> {
    data().ok()?.info(skill_id).ok().flatten()
}

/// A skill the player has, at the level they have it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Learned {
    pub skill_id: i32,
    pub level: i32,
    pub effect: SkillLevel,
}

pub fn learned(player: &Player, skill_id: i32) -> Option<Learned> {
    let level = player.skills.level(skill_id);
    let effect = info(skill_id)?.level(level)?.clone();
    Some(Learned {
        skill_id,
        level,
        effect,
    })
}

/// What the v83 client heals on its own at rest (FUN_0083007f), before the
/// map's `info/recovery` rate and a chair's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NaturalRecovery {
    /// HP per heal: 10, plus a warrior's Improved HP Recovery `hp` or a
    /// thief's Endure `hp`.
    pub hp: i32,
    /// MP per heal: 3, plus a magician's skill level in Improved MP Recovery
    /// times their level over 10, a thief's Endure `mp`, or a Crusader's or
    /// White Knight's Improved MP Recovery `mp`.
    pub mp: i32,
    /// Milliseconds between HP heals: 10000, or 5000 for a warrior with
    /// Improved HP Recovery. MP always heals every 10000.
    pub hp_interval: u64,
}

pub const IMPROVED_HP_RECOVERY: i32 = 1000000;
pub const IMPROVED_MP_RECOVERY: i32 = 2000000;
pub const ASSASSIN_ENDURE: i32 = 4100002;
pub const BANDIT_ENDURE: i32 = 4200001;
pub const FIGHTER_IMPROVED_MP_RECOVERY: i32 = 1110000;
pub const PAGE_IMPROVED_MP_RECOVERY: i32 = 1210000;

pub fn natural_recovery(player: &Player) -> NaturalRecovery {
    let job = i32::from(player.stats.job);
    let effect = |skill_id| learned(player, skill_id).map(|skill| skill.effect);
    let hp_skill = if job / 100 == 1 {
        effect(IMPROVED_HP_RECOVERY)
    } else if in_line(job, 410) {
        effect(ASSASSIN_ENDURE)
    } else if in_line(job, 420) {
        effect(BANDIT_ENDURE)
    } else {
        None
    };
    let mp_bonus = if job / 100 == 2 {
        player.skills.level(IMPROVED_MP_RECOVERY) * i32::from(player.stats.level) / 10
    } else {
        let skill_id = [
            (410, ASSASSIN_ENDURE),
            (420, BANDIT_ENDURE),
            (111, FIGHTER_IMPROVED_MP_RECOVERY),
            (121, PAGE_IMPROVED_MP_RECOVERY),
        ]
        .into_iter()
        .find(|&(line, _)| in_line(job, line))
        .map(|(_, skill_id)| skill_id);
        skill_id.and_then(effect).map_or(0, |effect| effect.mp)
    };
    let hp_interval = if job / 100 == 1 && player.skills.level(IMPROVED_HP_RECOVERY) > 0 {
        5000
    } else {
        10000
    };
    NaturalRecovery {
        hp: 10 + hp_skill.map_or(0, |effect| effect.hp),
        mp: 3 + mp_bonus,
        hp_interval,
    }
}

/// The percent Element Amplification scales the player's spells by (its
/// `y`), or 100 without it (v83 FUN_00765771).
pub fn amplification(player: &Player) -> i32 {
    let job = i32::from(player.stats.job);
    [(211, 2110001), (221, 2210001), (1211, 12110001)]
        .into_iter()
        .find(|&(line, _)| in_line(job, line))
        .and_then(|(_, skill_id)| learned(player, skill_id))
        .map_or(100, |skill| skill.effect.y)
}

/// The skill at `level`, whoever has it: what a party member gets from
/// another's skill.
pub fn at_level(skill_id: i32, level: i32) -> Option<Learned> {
    let effect = info(skill_id)?.level(level)?.clone();
    Some(Learned {
        skill_id,
        level,
        effect,
    })
}

pub const SHADOW_STARS: i32 = 4121006;
pub const DISPEL: i32 = 2311001;
pub const RESURRECTION: i32 = 2321006;

/// Whether USE_SKILL carries where the player stands (0x96d6ca): the skills
/// that place something, or reach the party around them.
pub fn sends_position(skill_id: i32) -> bool {
    matches!(
        skill_id,
        1001003
            | 1101006
            | 1111007
            | 1121000
            | 1201006
            | 1211009
            | 1221000
            | 1301006
            | 1301007
            | 1311007
            | 1321000
            | 2101001
            | 2101003
            | 2121000
            | 2201001
            | 2201003
            | 2221000
            | 2301004
            | 2311001
            | 2311003
            | 2321000
            | 2321005
            | 3121000
            | 3121002
            | 3221000
            | 4101004
            | 4111001
            | 4121000
            | 4201003
            | 4221000
            | 5121000
            | 5121009
            | 5221000
            | 11001001
            | 11101003
            | 12101000
            | 12101001
            | 14101003
            | 15111005
            | 21121000
            | 22171000
            | 22181000
    )
}

/// Whether a skill reaches the party members around the player (0x966f7a),
/// so that USE_SKILL carries a bit for each one it reached.
pub fn reaches_party(skill_id: i32) -> bool {
    matches!(
        skill_id,
        1101006
            | 1121000
            | 1221000
            | 1301006
            | 1301007
            | 1321000
            | 2101001
            | 2121000
            | 2201001
            | 2221000
            | 2301004
            | 2311003
            | 2321000
            | 2321005
            | RESURRECTION
            | 3121000
            | 3121002
            | 3221000
            | 3221002
            | 4101004
            | 4111001
            | 4121000
            | 4201003
            | 4221000
            | 5121000
            | 5121009
            | 5121010
            | 5221000
            | 11101003
            | 12101000
            | 14101003
            | 15111005
            | 21120007
            | 21121000
            | DISPEL
    )
}

/// Whether a skill reaches the mobs around the player, so that USE_SKILL
/// carries the ones it reached.
pub fn reaches_mobs(skill_id: i32) -> bool {
    matches!(
        skill_id,
        1111007
            | 1201006
            | 1211009
            | 1311007
            | 2101003
            | 2111004
            | 2201003
            | 2211004
            | 2311005
            | 4111003
            | 4121004
            | 4221004
            | 12101001
            | 12111002
            | 14111001
            | DISPEL
    )
}

/// The party a skill reached without one: the player alone.
pub const ALONE: u8 = 0x80;

/// The bit USE_SKILL sets for the party slot `slot`, the first highest
/// (0x96db3f).
pub fn party_bit(slot: usize) -> u8 {
    if slot < 6 {
        1 << (5 - slot)
    } else {
        0
    }
}

/// How far past a skill's area a party member may stand and still be
/// reached: the user's client sees them where they were a moment ago.
pub const AREA_SLACK: i32 = 100;

/// Whether someone at `at` stands in the area of a skill used at `origin`,
/// whichever way the user faced, give or take `AREA_SLACK`.
pub fn in_area(effect: &SkillLevel, origin: (i16, i16), at: (i16, i16)) -> bool {
    let reach_x = effect.lt.0.abs().max(effect.rb.0.abs()) + AREA_SLACK;
    let dx = i32::from(at.0) - i32::from(origin.0);
    let dy = i32::from(at.1) - i32::from(origin.1);
    dx.abs() <= reach_x && effect.lt.1 - AREA_SLACK <= dy && dy <= effect.rb.1 + AREA_SLACK
}

/// Heal: it heals the party around the cleric and hurts the undead.
pub const HEAL: i32 = 2301002;

/// What a spell's damage comes from, with the player's equipment: their INT,
/// their LUK, and their magic, INT plus magic attack, at most 1999.
pub fn spell_stats(player: &Player) -> (i32, i32, i32) {
    let (int, luk, matk) = player
        .inventory
        .items(InventoryType::Equipped)
        .filter_map(|(_, item)| item.equip.as_ref())
        .fold(
            (i32::from(player.stats.int), i32::from(player.stats.luk), 0),
            |(int, luk, matk), equip| {
                (
                    int + i32::from(equip.int),
                    luk + i32::from(equip.luk),
                    matk + i32::from(equip.matk),
                )
            },
        );
    (int, luk, (int + matk).clamp(0, 1999))
}

/// What Heal does to each of the `reached` it reaches, party members and
/// undead together, before the undead's defense: the v83 client's formula
/// for its hurt (FUN_00791617, 0x791aa8), from the skill's `hp` percent.
/// `roll`, from 0 to 1, picks between 20% and 80% of the INT. The more it
/// reaches, the less each gets.
pub fn heal_amount(player: &Player, hp: i32, reached: i32, roll: f64) -> f64 {
    let (int, luk, magic) = spell_stats(player);
    let reached = f64::from(reached.max(1));
    let int = f64::from(int);
    let rolled = int * 0.2 + (int * 0.8 - int * 0.2) * roll;
    (reached * 0.3 + 1.0)
        * f64::from(hp)
        * 0.01
        * (rolled * 1.5 + f64::from(luk))
        * f64::from(magic)
        * 0.005
        / reached
}

/// Whether `job` is `base` or advanced from it, as the v83 client checks:
/// 410 covers 410 to 412, and 100 every warrior.
fn in_line(job: i32, base: i32) -> bool {
    if base % 100 == 0 {
        base / 100 == job / 100
    } else {
        base / 10 == job / 10 && base % 10 <= job % 10
    }
}

/// Skills held down to charge or keep firing (the v83 client's
/// FUN_004fb08f).
pub const KEYDOWN: [i32; 16] = [
    1121001,  // Monster Magnet (Hero)
    1221001,  // Monster Magnet (Paladin)
    1321001,  // Monster Magnet (Dark Knight)
    2121001,  // Big Bang (Fire/Poison)
    2221001,  // Big Bang (Ice/Lightning)
    2321001,  // Big Bang (Bishop)
    3121004,  // Hurricane
    3221001,  // Piercing Arrow
    5101004,  // Corkscrew Blow
    5201002,  // Grenade
    5221004,  // Rapid Fire
    13111002, // Hurricane (Wind Archer)
    14111006, // Poison Bomb
    15101003, // Corkscrew Blow (Thunder Breaker)
    22121000, // Evan
    22151001, // Evan
];

/// The held spells, whose MAGIC_ATTACK says how long they were charged.
pub const CHARGED_SPELLS: [i32; 5] = [2121001, 2221001, 2321001, 22121000, 22151001];

pub fn keydown(skill_id: i32) -> bool {
    KEYDOWN.contains(&skill_id)
}

/// Monster Magnet, for each warrior branch: held, then let go with the mobs
/// it caught.
pub const MONSTER_MAGNETS: [i32; 3] = [1121001, 1221001, 1321001];

pub fn is_magnet(skill_id: i32) -> bool {
    MONSTER_MAGNETS.contains(&skill_id)
}

/// Grenade and Poison Bomb: held, then thrown, going off where they land
/// (0x95efc5).
pub const THROWS: [i32; 2] = [5201002, 14111006];

pub fn throws(skill_id: i32) -> bool {
    THROWS.contains(&skill_id)
}

/// Explosion and Chakra prepare too, but by a timer rather than a held key.
pub const TIMED: [i32; 2] = [2111002, 4211001];

/// Whether the client prepares the skill first, with SKILL_EFFECT.
pub fn prepares(skill_id: i32) -> bool {
    keydown(skill_id) || TIMED.contains(&skill_id)
}

/// The stats a buff skill gives at its level, or `None` for a skill that
/// isn't one. Every buff gives the stats its level names (attack, defense,
/// accuracy, avoidability, speed, jump), and some a stat of their own. A
/// weapon charge's own `mad` is the strength of its element, not a buff.
pub fn buffs(skill: &Learned) -> Option<Vec<(BuffStat, i32)>> {
    let effect = &skill.effect;
    let own = match skill.skill_id {
        1001 | 10001001 | 20001001 => vec![(BuffStat::Recovery, effect.x)],
        1002 | 10001002 | 20001002 => vec![],
        // Iron Body, Rage, Iron Will, Magic Armor, Meditation, Bless,
        // Focus, Haste.
        1001003 | 1101006 | 1301006 | 2001003 | 2101001 | 2201001 | 2301004 | 3001003 | 4101004
        | 4201003 => vec![],
        1101004 | 1101005 | 1201004 | 1201005 | 1301004 | 1301005 | 2111005 | 2211005 | 3101002
        | 3201002 | 4101003 | 4201002 | 5101006 | 5201003 => {
            vec![(BuffStat::Booster, effect.x)]
        }
        1101007 | 1201007 => vec![(BuffStat::PowerGuard, effect.x)],
        // The orbs start at none, which the client counts from 1.
        1111002 => vec![(BuffStat::Combo, 1)],
        1211003..=1211008 | 1221003 | 1221004 => {
            return Some(vec![(BuffStat::WeaponCharge, effect.x)]);
        }
        1301007 => vec![(BuffStat::MaxHp, effect.x), (BuffStat::MaxMp, effect.y)],
        1311008 => vec![(BuffStat::DragonBlood, effect.x)],
        1121000 | 1221000 | 1321000 | 2121000 | 2221000 | 2321000 | 3121000 | 3221000 | 4121000
        | 4221000 | 5121000 | 5221000 => vec![(BuffStat::MapleWarrior, effect.x)],
        1121002 | 1221002 | 1321002 => vec![(BuffStat::Stance, effect.prop)],
        2001002 => vec![(BuffStat::MagicGuard, effect.x)],
        2301003 => vec![(BuffStat::Invincible, effect.x)],
        2311003 => vec![(BuffStat::HolySymbol, effect.x)],
        2121004 | 2221004 | 2321004 => vec![(BuffStat::Infinity, effect.x)],
        3101004 | 3201004 => vec![(BuffStat::SoulArrow, effect.x)],
        3121008 => vec![(BuffStat::Concentrate, effect.x)],
        3121002 | 3221002 => vec![(BuffStat::SharpEyes, effect.x << 8 | effect.y)],
        4001003 => vec![(BuffStat::DarkSight, effect.x)],
        4111001 => vec![(BuffStat::MesoUp, effect.x)],
        4111002 => vec![(BuffStat::ShadowPartner, effect.x)],
        4211003 => vec![(BuffStat::PickPocket, effect.x)],
        4211005 => vec![(BuffStat::MesoGuard, effect.x)],
        _ => return None,
    };
    let leveled = [
        (BuffStat::Pad, effect.pad),
        (BuffStat::Pdd, effect.pdd),
        (BuffStat::Mad, effect.magic),
        (BuffStat::Mdd, effect.mdd),
        (BuffStat::Acc, effect.acc),
        (BuffStat::Eva, effect.eva),
        (BuffStat::Speed, effect.speed),
        (BuffStat::Jump, effect.jump),
    ];
    let mut stats: Vec<_> = leveled
        .into_iter()
        .filter(|&(_, value)| value != 0)
        .collect();
    stats.extend(own);
    Some(stats)
}
