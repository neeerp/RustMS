//! Skill templates (`Skill.nx`), loaded once.

use crate::load::{self, asset_path};
use game::{BuffStat, InventoryType, MobStat, Player};
use game_data::skills::{SkillData, SkillInfo, SkillLevel};
use game_data::{FieldTemplate, FIELD_LIMIT_MYSTIC_DOOR, PORTAL_TYPE_START_POINT};
use std::sync::OnceLock;
use std::time::Duration;

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
/// The diseases Dispel cures, as Cosmic has them: not Stun, Seduce, or
/// Confuse.
pub const DISPELLED: [BuffStat; 6] = [
    BuffStat::Poison,
    BuffStat::Seal,
    BuffStat::Darkness,
    BuffStat::Weakness,
    BuffStat::Curse,
    BuffStat::Slow,
];
pub const RESURRECTION: i32 = 2321006;
pub const CHAKRA: i32 = 4211001;
/// The Arch Mages' and Bishop's Mana Reflection share their levels' data;
/// v83 looks them all up as the Fire/Poison one's (0x958db2).
pub const MANA_REFLECTION: i32 = 2121002;
pub const TIME_LEAP: i32 = 5121010;

pub const MYSTIC_DOOR: i32 = 2311002;

/// The Teleport skills (v83 FUN_00957b74): the client moves the player and
/// asks only for the MP.
pub const TELEPORTS: [i32; 5] = [2101002, 2201002, 2301001, 12101003, 22101001];

/// Skills that neither buff nor debuff: the handler does their work.
/// Chakra heals the user, Resurrection raises dead party members, Time Leap
/// ends cooldowns, and Mystic Door opens a door. Teleport has no work.
pub fn acts_alone(skill_id: i32) -> bool {
    matches!(skill_id, MYSTIC_DOOR | CHAKRA | RESURRECTION | TIME_LEAP)
        || TELEPORTS.contains(&skill_id)
}

/// The HP Chakra heals: from 2.3 to 3.5 times the user's LUK, with
/// equipment, times the level's `y` percent; `roll`, from 0 to 1, picks
/// between them. The v83 client leaves it to the server, so this is
/// OdinMS-family servers' formula.
pub fn chakra_heal(player: &Player, y: i32, roll: f64) -> i32 {
    let (_, _, luk) = total_stats(player);
    let rate = f64::from(y) / 100.0;
    let low = (f64::from(luk) * 2.3 * rate) as i32;
    let high = (f64::from(luk) * 3.5 * rate) as i32;
    (low + (roll * f64::from(high - low + 1)) as i32).min(high)
}

/// How close to a portal Mystic Door may not open (v83 FUN_00712ab1).
const DOOR_PORTAL_REACH: i32 = 50;

/// The town a Mystic Door cast at `at` leads to, if one may open there
/// (v83 FUN_00969bf9): not where the map forbids it, nor in a town or an
/// event map (`1x9xxxxxx`), nor within 50 of a portal that is not a spawn
/// point; and the map must have a town to return to.
pub fn door_town(field: &FieldTemplate, at: (i16, i16)) -> Option<i32> {
    if field.field_limit & FIELD_LIMIT_MYSTIC_DOOR != 0
        || field.town
        || (field.map_id / 1_000_000) % 100 == 9
    {
        return None;
    }
    let (x, y) = (i32::from(at.0), i32::from(at.1));
    // The client's PtInRect takes the box's left and top edges, not its
    // right and bottom.
    let reach = -DOOR_PORTAL_REACH..DOOR_PORTAL_REACH;
    let near_portal = field.portals().any(|portal| {
        portal.portal_type != PORTAL_TYPE_START_POINT
            && reach.contains(&(x - portal.x))
            && reach.contains(&(y - portal.y))
    });
    if near_portal {
        return None;
    }
    field.return_map.filter(|&town| town != field.map_id)
}

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

/// What a skill puts on the mobs it reaches or hits: statuses, with the
/// skill's chance for each mob, for its time.
#[derive(Clone, Debug, PartialEq)]
pub struct Inflict {
    /// The skill behind it, the statuses' reason.
    pub skill_id: i32,
    /// The chance in percent; 100 for always.
    pub prop: i32,
    pub duration: Duration,
    pub statuses: Vec<(MobStat, Amount)>,
}

impl Inflict {
    fn new(skill: &Learned, statuses: Vec<(MobStat, Amount)>) -> Self {
        let effect = &skill.effect;
        Self {
            skill_id: skill.skill_id,
            prop: if effect.prop > 0 { effect.prop } else { 100 },
            duration: Duration::from_secs(effect.duration.max(0) as u64),
            statuses,
        }
    }

    /// Whether it takes hold this time.
    pub fn takes(&self, roll: f64) -> bool {
        self.prop >= 100 || roll * 100.0 < f64::from(self.prop)
    }
}

/// A status's value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Amount {
    Value(i16),
    /// Poison from a skill at `level`: each second the mob's max HP over
    /// 70 less the level, rounded up (OdinMS-family servers; v83's server
    /// isn't available, and the client only shows the value it's sent).
    Poison {
        level: i32,
    },
}

impl Amount {
    /// The value on a mob with `max_hp`.
    pub fn against(self, max_hp: i32) -> i16 {
        match self {
            Amount::Value(value) => value,
            Amount::Poison { level } => {
                let per = (70 - level).max(1);
                (max_hp.max(0) as u32)
                    .div_ceil(per as u32)
                    .clamp(1, i16::MAX as u32) as i16
            }
        }
    }
}

/// A player's STR, DEX, and LUK with what they wear.
pub(crate) fn total_stats(player: &Player) -> (i32, i32, i32) {
    let stats = &player.stats;
    player
        .inventory
        .items(InventoryType::Equipped)
        .filter_map(|(_, item)| item.equip.as_ref())
        .fold(
            (
                i32::from(stats.str),
                i32::from(stats.dex),
                i32::from(stats.luk),
            ),
            |(str, dex, luk), equip| {
                (
                    str + i32::from(equip.str),
                    dex + i32::from(equip.dex),
                    luk + i32::from(equip.luk),
                )
            },
        )
}

/// What a skill that reaches mobs puts on each it takes hold of. None for
/// one that doesn't (yet); no statuses for one that only takes buffs away.
pub fn mob_debuffs(skill: &Learned, player: &Player) -> Option<Inflict> {
    let effect = &skill.effect;
    let value = |stat, value: i32| (stat, Amount::Value(value as i16));
    let statuses = match skill.skill_id {
        // Threaten.
        1201006 => vec![value(MobStat::Pad, effect.x), value(MobStat::Pdr, effect.y)],
        // Slow.
        2101003 | 2201003 | 12101001 => vec![value(MobStat::Speed, effect.x)],
        // Seal.
        2111004 | 2211004 | 12111002 => vec![value(MobStat::Seal, 1)],
        // Doom.
        2311005 => vec![value(MobStat::Doom, 1)],
        // Shadow Web.
        4111003 | 14111001 => vec![value(MobStat::Web, 1)],
        // Ninja Ambush: each second (STR + LUK) x (1.5 + level / 20) x
        // its damage percent (OdinMS-family servers).
        4121004 | 4221004 => {
            let (str, _, luk) = total_stats(player);
            let damage = f64::from(str + luk)
                * (1.5 + f64::from(skill.level) * 0.05)
                * f64::from(effect.damage)
                / 100.0;
            vec![value(
                MobStat::Ambush,
                (damage as i32).clamp(1, i16::MAX.into()),
            )]
        }
        _ if !mob_cancels(skill.skill_id).is_empty() => vec![],
        _ => return None,
    };
    Some(Inflict::new(skill, statuses))
}

/// What an attack with `skill` puts on the mobs it hits, as the skill
/// descriptions say: stuns, freezes, poisons, and Disorder's and Taunt's
/// weakening.
fn hit_statuses(skill: &Learned) -> Option<Vec<(MobStat, Amount)>> {
    let one = |stat| vec![(stat, Amount::Value(1))];
    let value = |stat, value: i32| (stat, Amount::Value(value as i16));
    let effect = &skill.effect;
    let statuses = match skill.skill_id {
        // Coma, Shout, Charged Blow, Arrow Bomb, Assaulter, Boomerang Step,
        // Backspin Blow, Double Uppercut, Demolition, Snatch, Blank Shot,
        // Dawn Warrior Coma, Rolling Spin.
        1111005 | 1111006 | 1111008 | 1211002 | 3101005 | 4211002 | 4221007 | 5101002 | 5101003
        | 5121004 | 5121005 | 5201004 | 11111003 | 21110006 => one(MobStat::Stun),
        // Cold Beam, Ice Strike, Element Composition (I/L), Blizzard,
        // Blizzard (crossbow), Ice Splitter, Paralyze.
        2201004 | 2211002 | 2211006 | 2221007 | 3211003 | 5211005 | 2121006 => one(MobStat::Freeze),
        // Poison Breath, Element Composition (F/P), Fire Demon, Ice Demon,
        // Flamethrower. The demons' reason also turns the mob's element
        // weakness on the client (0x78c7ab).
        2101005 | 2111006 | 2121003 | 2221003 | 5211004 => {
            vec![(MobStat::Poison, Amount::Poison { level: skill.level })]
        }
        // Disorder: less attack (x) and defence (y), both negative.
        4001002 | 14001002 => vec![value(MobStat::Pad, effect.x), value(MobStat::Pdr, effect.y)],
        // Taunt: more defence, but more EXP and drops, all by x (OdinMS
        // family).
        4121003 | 4221003 => vec![
            value(MobStat::Showdown, effect.x),
            value(MobStat::Pdr, effect.x),
            value(MobStat::Mdr, effect.x),
        ],
        _ => return None,
    };
    Some(statuses)
}

/// Venomous Star (a claw's shots), Venomous Stab (a dagger's swings), and
/// the Night Walker's Venom.
const VENOMS: [(i32, bool); 3] = [(4120005, true), (4220005, false), (14110004, true)];

/// What an attack puts on each mob it hits, besides its damage: its
/// skill's status, and the poison of a thief's Venom, each second a roll
/// between (8 (STR + LUK) + 2 DEX) / 100 and (18.5 (STR + LUK) + 2 DEX) /
/// 100 times the Venom's `mad` (the old community damage formulas).
pub fn on_hit(player: &Player, skill: Option<&Learned>, ranged: bool, roll: f64) -> Vec<Inflict> {
    let mut inflicts = Vec::new();
    if let Some(skill) = skill {
        if let Some(statuses) = hit_statuses(skill) {
            inflicts.push(Inflict::new(skill, statuses));
        }
    }
    let venom = VENOMS
        .iter()
        .filter(|&&(_, shot)| shot == ranged)
        .find_map(|&(skill_id, _)| learned(player, skill_id).filter(|venom| venom.level > 0));
    if let Some(venom) = venom {
        let (str, dex, luk) = total_stats(player);
        let at = |rate: f64| (rate * f64::from(str + luk) + 2.0 * f64::from(dex)) / 100.0;
        let damage = (at(8.0) + (at(18.5) - at(8.0)) * roll) * f64::from(venom.effect.magic);
        let damage = (damage as i32).clamp(1, i16::MAX.into()) as i16;
        inflicts.push(Inflict::new(
            &venom,
            vec![(MobStat::Venom, Amount::Value(damage))],
        ));
    }
    inflicts
}

/// Poison Mist: a cloud where it's cast that poisons the mobs in it.
pub const POISON_MIST: i32 = 2111003;

/// What the mist a spell leaves puts on the mobs in it, each with the
/// skill's chance, for as long as the mist's `time`; None for a spell that
/// leaves none. v83's client draws the mist (0x431a63); what it does is the
/// server's (OdinMS-family: Poison Mist poisons).
pub fn mist(skill: &Learned) -> Option<Inflict> {
    (skill.skill_id == POISON_MIST).then(|| {
        Inflict::new(
            skill,
            vec![(MobStat::Poison, Amount::Poison { level: skill.level })],
        )
    })
}

/// The mob buffs a skill that reaches mobs takes away: the Crashes, and
/// Dispel the lot.
pub fn mob_cancels(skill_id: i32) -> &'static [MobStat] {
    match skill_id {
        1111007 => &[MobStat::PGuardUp],
        1211009 => &[MobStat::MGuardUp],
        1311007 => &[MobStat::PowerUp],
        DISPEL => &[
            MobStat::PowerUp,
            MobStat::MagicUp,
            MobStat::PGuardUp,
            MobStat::MGuardUp,
            MobStat::PImmune,
            MobStat::MImmune,
        ],
        _ => &[],
    }
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

/// A skill's area used at `origin`, as left, top, right, bottom: `lt` and
/// `rb` as they are facing left, mirrored facing right.
pub fn area_box(effect: &SkillLevel, origin: (i16, i16), left: bool) -> (i32, i32, i32, i32) {
    let (x, y) = (i32::from(origin.0), i32::from(origin.1));
    let (l, r) = if left {
        (effect.lt.0, effect.rb.0)
    } else {
        (-effect.rb.0, -effect.lt.0)
    };
    (x + l, y + effect.lt.1, x + r, y + effect.rb.1)
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
/// isn't one. A summon's buff is its icon (`summons::stat`). Every buff gives the stats its level names (attack, defense,
/// accuracy, avoidability, speed, jump), and some a stat of their own. A
/// weapon charge's own `mad` is the strength of its element, not a buff.
pub fn buffs(skill: &Learned) -> Option<Vec<(BuffStat, i32)>> {
    // A summon's `pad` is its own attack, not the player's.
    if crate::summons::kind(skill.skill_id).is_some() {
        return Some(vec![(crate::summons::stat(skill.skill_id), 1)]);
    }
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
        2121002 | 2221002 | 2321002 => vec![(BuffStat::ManaReflection, skill.level)],
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

#[cfg(test)]
mod tests {
    use super::*;
    use game::SkillRecord;

    #[test]
    fn mystic_door_opens_away_from_towns_and_portals_and_leads_to_town() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/game-data/Map.nx");
        let data = game_data::GameData::load_from_nx_map(&path).unwrap();
        let henesys = data.field(100_000_000).unwrap();
        assert_eq!(door_town(henesys, (2786, 333)), None);
        let hunting = data.field(101_010_000).unwrap();
        assert_eq!(door_town(hunting, (-1000, 1900)), Some(101_000_000));
        // Not by its west portal, though a spawn point is no matter.
        assert_eq!(door_town(hunting, (-1600, 1561)), None);
        // Its box (-1642, 1561 ± 50) holds its left edge but not its right.
        assert_eq!(door_town(hunting, (-1692, 1561)), None);
        assert_eq!(door_town(hunting, (-1592, 1561)), Some(101_000_000));
        assert_eq!(door_town(hunting, (-1711, 1914)), Some(101_000_000));
    }

    fn knowing(skills: &[(i32, i32)]) -> Player {
        let mut player = crate::codec::stats::sample_player();
        for &(skill_id, level) in skills {
            player.skills.set(SkillRecord {
                skill_id,
                level,
                master_level: 0,
                expiration: None,
            });
        }
        player
    }

    #[test]
    fn poison_takes_a_share_of_max_hp_each_second() {
        let poison = Amount::Poison { level: 30 };
        assert_eq!(poison.against(1000), 25);
        assert_eq!(poison.against(1001), 26);
        assert_eq!(poison.against(0), 1);
        assert_eq!(poison.against(i32::MAX), i16::MAX);
        assert_eq!(Amount::Value(-20).against(1000), -20);
    }

    #[test]
    fn attacks_put_their_skills_statuses_and_venom_on_what_they_hit() {
        // Shout at 30: a stun with a 95% chance, for 10 seconds.
        let shout = at_level(1111008, 30).unwrap();
        let player = knowing(&[]);
        let inflicts = on_hit(&player, Some(&shout), false, 0.5);
        assert_eq!(
            inflicts,
            [Inflict {
                skill_id: 1111008,
                prop: 95,
                duration: Duration::from_secs(10),
                statuses: vec![(MobStat::Stun, Amount::Value(1))],
            }]
        );
        assert!(inflicts[0].takes(0.94) && !inflicts[0].takes(0.95));
        let breath = at_level(2101005, 30).unwrap();
        assert_eq!(
            on_hit(&player, Some(&breath), false, 0.5)[0].statuses,
            [(MobStat::Poison, Amount::Poison { level: 30 })]
        );
        assert!(on_hit(&player, None, true, 0.5).is_empty());

        // Venomous Star at 30 (mad 60, 30% for 4 seconds) poisons a
        // claw's shots only. STR 35, DEX 4, LUK 4: between 3.2 and 7.295
        // times 60.
        let hermit = knowing(&[(4120005, 30)]);
        assert!(on_hit(&hermit, None, false, 0.5).is_empty());
        let venom = |roll| on_hit(&hermit, None, true, roll).remove(0);
        assert_eq!(venom(0.0).skill_id, 4120005);
        assert_eq!(venom(0.0).prop, 30);
        assert_eq!(venom(0.0).duration, Duration::from_secs(4));
        assert_eq!(venom(0.0).statuses, [(MobStat::Venom, Amount::Value(192))]);
        assert_eq!(venom(1.0).statuses, [(MobStat::Venom, Amount::Value(437))]);
    }

    #[test]
    fn disorder_weakens_and_taunt_raises_exp_and_drops() {
        let player = knowing(&[]);
        let disorder = at_level(4001002, 20).unwrap();
        assert_eq!(
            on_hit(&player, Some(&disorder), false, 0.5),
            [Inflict {
                skill_id: 4001002,
                prop: 100,
                duration: Duration::from_secs(60),
                statuses: vec![
                    (MobStat::Pad, Amount::Value(-20)),
                    (MobStat::Pdr, Amount::Value(-20)),
                ],
            }]
        );
        let taunt = at_level(4121003, 30).unwrap();
        assert_eq!(
            on_hit(&player, Some(&taunt), false, 0.5),
            [Inflict {
                skill_id: 4121003,
                prop: 100,
                duration: Duration::from_secs(120),
                statuses: vec![
                    (MobStat::Showdown, Amount::Value(40)),
                    (MobStat::Pdr, Amount::Value(40)),
                    (MobStat::Mdr, Amount::Value(40)),
                ],
            }]
        );
    }

    #[test]
    fn poison_mist_poisons_what_is_in_its_box() {
        let mist = mist(&at_level(POISON_MIST, 30).unwrap()).unwrap();
        assert_eq!(mist.prop, 70);
        assert_eq!(mist.duration, Duration::from_secs(40));
        assert_eq!(
            mist.statuses,
            vec![(MobStat::Poison, Amount::Poison { level: 30 })]
        );
        assert!(super::mist(&at_level(2111006, 30).unwrap()).is_none());

        let mut effect = at_level(POISON_MIST, 30).unwrap().effect;
        assert_eq!(area_box(&effect, (10, 20), true), (-190, -130, 210, 170));
        effect.lt = (-300, -50);
        effect.rb = (100, 0);
        assert_eq!(area_box(&effect, (0, 0), true), (-300, -50, 100, 0));
        assert_eq!(area_box(&effect, (0, 0), false), (-100, -50, 300, 0));
    }
}
