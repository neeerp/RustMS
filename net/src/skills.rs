//! Skill templates (`Skill.nx`), loaded once.

use crate::load::{self, asset_path};
use crate::packet::build::world::mist::MistKind;
use game::{BuffStat, Disease, InventoryType, MobStat, Player, SkillRecord};
use game_data::skills::{SkillData, SkillInfo, SkillLevel};
use game_data::{
    FieldTemplate, FIELD_LIMIT_MYSTIC_DOOR, FIELD_TYPE_DOJO, FIELD_TYPE_DOJO_2, FIELD_TYPE_PYRAMID,
    PORTAL_TYPE_START_POINT,
};
use rand::Rng;
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
    if is_gm(skill_id) && player.gm_level == 0 {
        return None;
    }
    let level = match pq_skill(skill_id) {
        Some(kind) if pq_skill_allowed(kind, player) => 1,
        Some(_) => return None,
        None => player.skills.level(skill_id),
    };
    let mut effect = info(skill_id)?.level(level)?.clone();
    effect.morph = crate::morph::template(effect.morph, player.stats.gender);
    Some(Learned {
        skill_id,
        level,
        effect,
    })
}

/// The Beginner's secret skills of the Mu Lung Dojo and Nett's Pyramid, which
/// v83 casts at level 1 with no skill record (FUN_007616f6) and only on
/// their own maps (FUN_00587f5f).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PqSkill {
    /// Bamboo Rain (1009): every mob in reach loses its max HP, a boss 30%.
    BambooRain,
    /// Invincibility (1010) and Power Explosion (1011).
    DojoBuff,
    /// Rage of Pharaoh (1020): every mob in reach loses its max HP.
    RageOfPharaoh,
}

/// Which secret skill `skill_id` is, in the Beginner's, Noblesse's, Legend's,
/// or Evan's copy.
pub fn pq_skill(skill_id: i32) -> Option<PqSkill> {
    if !matches!(skill_id / 10000, 0 | 1000 | 2000 | 2001) {
        return None;
    }
    match skill_id % 10000 {
        1009 => Some(PqSkill::BambooRain),
        1010 | 1011 => Some(PqSkill::DojoBuff),
        1020 => Some(PqSkill::RageOfPharaoh),
        _ => None,
    }
}

/// Whether the player may use a secret skill where they are: the Dojo's
/// on its stages and Rage of Pharaoh in the Pyramid, and the two attacks
/// neither riding nor transformed (FUN_00966f7a).
fn pq_skill_allowed(kind: PqSkill, player: &Player) -> bool {
    let field_type = crate::get_game_data()
        .ok()
        .and_then(|data| data.field(player.stats.map_id))
        .map_or(0, |field| field.field_type);
    let on_its_map = match kind {
        PqSkill::BambooRain | PqSkill::DojoBuff => {
            matches!(field_type, FIELD_TYPE_DOJO | FIELD_TYPE_DOJO_2)
        }
        PqSkill::RageOfPharaoh => field_type == FIELD_TYPE_PYRAMID,
    };
    let attack = kind != PqSkill::DojoBuff;
    on_its_map && !(attack && (player.ride_skill().is_some() || player.buffs.has(BuffStat::Morph)))
}

/// The most a secret attack may take from a mob: all its max HP, but only
/// 30% of a boss's to Bamboo Rain (FUN_0078df87).
pub fn pq_strike_cap(skill_id: i32, max_hp: i64, boss: bool) -> Option<i64> {
    match pq_skill(skill_id)? {
        PqSkill::BambooRain if boss => Some(max_hp * 3 / 10),
        PqSkill::BambooRain | PqSkill::RageOfPharaoh => Some(max_hp),
        PqSkill::DojoBuff => None,
    }
}

/// The Maker skill of a player of `job`: 1007 for explorers, 10001007 for
/// Cygnus Knights, 20001007 for Aran and 20011007 for Evan.
pub fn maker_skill(job: i16) -> i32 {
    let job = i32::from(job);
    let base = if job / 100 == 22 || job == 2001 {
        2001
    } else {
        job / 1000 * 1000
    };
    base * 10_000 + 1007
}

/// What a new fourth job is taught at the advancement: each of `job`'s
/// skills with a `masterLevel`, at level 0 and that master level. The rest
/// are hidden until a quest or a book teaches them.
pub fn fourth_job_skills(job: i16) -> Vec<SkillRecord> {
    let Ok(data) = data() else {
        return Vec::new();
    };
    data.job_skills(job.into())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|skill_id| data.info(skill_id).ok().flatten())
        .filter(|info| info.master_level > 0)
        .map(|info| SkillRecord {
            skill_id: info.skill_id,
            level: 0,
            master_level: info.master_level,
            expiration: None,
        })
        .collect()
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

pub const IMPROVING_MAX_HP: i32 = 1000001;
pub const IMPROVING_MAX_MP: i32 = 2000001;
pub const BRAWLER_IMPROVE_MAX_HP: i32 = 5100000;

/// What the player adds to max HP and MP growth: a warrior's Improving
/// MaxHP Increase, a Brawler's Improve MaxHP and a magician's Improving
/// MaxMP Increase (`x` on a level up, or `y` for an AP, as their
/// descriptions say), and for MP a share of total INT: a tenth on a level
/// up, 3/40 for an AP, the share the client's AP Reset takes back.
pub fn growth_bonus(player: &Player, level_up: bool) -> (i16, i16) {
    let job = i32::from(player.stats.job);
    let bonus = |line, skill_id| {
        let skill = learned(player, skill_id).filter(|_| in_line(job, line));
        let value = skill.map_or(0, |skill| {
            if level_up {
                skill.effect.x
            } else {
                skill.effect.y
            }
        });
        i16::try_from(value).unwrap_or(0)
    };
    let int = total_int(player);
    let int = if level_up { int / 10 } else { int * 3 / 40 };
    (
        bonus(100, IMPROVING_MAX_HP) + bonus(510, BRAWLER_IMPROVE_MAX_HP),
        bonus(200, IMPROVING_MAX_MP) + i16::try_from(int).unwrap_or(i16::MAX),
    )
}

/// The random part of max HP/MP growth: 0 to `spread`.
pub fn growth_roll(spread: i16) -> i16 {
    rand::thread_rng().gen_range(0, spread + 1)
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
/// Shadow Stars' buff names its star as the id less this (0x949b5d).
pub const SHADOW_STARS_BASE: i32 = 2069999;

/// The star Shadow Stars throws, while it lasts.
pub fn shadow_star(player: &Player) -> Option<i32> {
    let value = player.buffs.value(BuffStat::ShadowStars)?;
    Some(i32::from(value) + SHADOW_STARS_BASE)
}
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
/// The Hero's Wills, Pirate's Rage and 5221010 among them (v83
/// FUN_00765e2d), which a seduced player may still use.
pub const HEROS_WILL: [i32; 14] = [
    1121011, 1221012, 1321010, 2121008, 2221008, 2321009, 3121009, 3221008, 4121009, 4221008,
    5121008, 5221010, 21121008, 22171004,
];
/// The diseases Hero's Will cures: Dispel's, and Seduce, Confuse, and
/// Undead (Cosmic's purgeDebuffs).
pub const PURGED: [BuffStat; 9] = [
    BuffStat::Poison,
    BuffStat::Seal,
    BuffStat::Darkness,
    BuffStat::Weakness,
    BuffStat::Curse,
    BuffStat::Slow,
    BuffStat::Seduce,
    BuffStat::Confuse,
    BuffStat::Undead,
];
pub const RESURRECTION: i32 = 2321006;
pub const CHAKRA: i32 = 4211001;
/// The Arch Mages' and Bishop's Mana Reflection share their levels' data;
/// v83 looks them all up as the Fire/Poison one's (0x958db2).
pub const MANA_REFLECTION: i32 = 2121002;
pub const TIME_LEAP: i32 = 5121010;
/// Echo of Hero, the Beginner's, the Noblesse's, and the Legend's: it
/// reaches everyone around, in the player's party or not (0x96a33a).
pub const ECHOES: [i32; 3] = [1005, 10001005, 20001005];

/// The GM skills (900.img, 910.img), which only a GM may use (Cosmic's
/// SpecialMoveHandler).
pub fn is_gm(skill_id: i32) -> bool {
    matches!(skill_id / 10000, 900 | 910)
}

pub const GM_HEAL: i32 = 9101000;
pub const GM_HIDE: i32 = 9101004;
pub const GM_RESURRECTION: i32 = 9101005;
/// Heal + Dispel, Haste (Super), Holy Symbol, Bless, Resurrection and Hyper
/// Body, which the client casts on everyone in their area as it does Echo
/// of Hero (0x96a33a).
pub const GM_AREA: [i32; 6] = [GM_HEAL, 9101001, 9101002, 9101003, GM_RESURRECTION, 9101008];

/// Whether a skill reaches everyone on the map around the player, in their
/// party or not.
pub fn reaches_everyone(skill_id: i32) -> bool {
    ECHOES.contains(&skill_id) || GM_AREA.contains(&skill_id)
}

/// Whether a skill reaches only the dead, and raises them.
pub fn raises_dead(skill_id: i32) -> bool {
    skill_id == RESURRECTION || skill_id == GM_RESURRECTION
}
pub const MP_RECOVERY: i32 = 5101005;

pub const SACRIFICE: i32 = 1311005;
pub const DRAGON_ROAR: i32 = 1311006;
pub const DRAGON_BLOOD: i32 = 1311008;

/// Final Attack's swings, which the client sends on its own after some of
/// its skill attacks (their `finalAttack`), and never pays for (0x96cd3e).
pub const FINAL_ATTACKS: [i32; 10] = [
    1100002, 1100003, 1200002, 1200003, 1300002, 1300003, 3100001, 3200001, 11101002, 13101002,
];

/// The Slash Blast whose mobs a Final Attack after it reaches: the Dawn
/// Warrior's for a Cygnus Knight (0x95150e).
pub fn slash_blast(player: &Player) -> i32 {
    if player.stats.job / 1000 == 1 {
        11001003
    } else {
        1001005
    }
}

/// The HP a skill costs, or `None` when the player hasn't enough
/// (FUN_00764256): its `hpCon`, which must stay under their HP. Dragon
/// Roar instead takes its `x` percent of their max HP, and needs at least
/// half of it; MP Recovery takes the same, and needs more.
pub fn hp_cost(player: &Player, skill: &Learned) -> Option<i32> {
    let hp = i32::from(player.stats.hp);
    let max_hp = player.effective_max_hp();
    if skill.skill_id == DRAGON_ROAR && hp < max_hp / 2 {
        return None;
    }
    let cost = if matches!(skill.skill_id, DRAGON_ROAR | MP_RECOVERY) {
        skill.effect.x * max_hp / 100
    } else {
        skill.effect.hp_cost
    };
    (cost == 0 || hp > cost).then_some(cost)
}

/// The MP a skill costs: its `mpCon`, or none under Infinity, which the
/// client zeroes for every skill (FUN_00764256, 0x76446d).
pub fn mp_cost(player: &Player, skill: &Learned) -> i32 {
    if player.buffs.has(BuffStat::Infinity) {
        0
    } else {
        skill.effect.mp_cost
    }
}

/// The stun Dragon Roar leaves its user in: `y` seconds of Stun, under the
/// skill's own id, so the client shows Dragon Roar's icon and won't let it
/// be cancelled (0xa203bf, 0x45ba47). The client never stuns the player
/// itself; it only stops them while the server has.
pub fn self_stun(skill: &Learned) -> Option<Disease> {
    (skill.skill_id == DRAGON_ROAR && skill.effect.y > 0).then(|| Disease {
        stat: BuffStat::Stun,
        value: 1,
        skill_id: DRAGON_ROAR,
        lasts: Duration::from_secs(skill.effect.y as u64),
    })
}

pub const MYSTIC_DOOR: i32 = 2311002;

/// The Teleport skills, the GM's among them (v83 FUN_00957b74): the client
/// moves the player and asks only for the MP.
pub const TELEPORTS: [i32; 6] = [2101002, 2201002, 2301001, 12101003, 22101001, 9001002];

/// The Flash Jump skills (v83 FUN_0096be38): the client throws the player on
/// through the air and asks only for the MP.
pub const FLASH_JUMPS: [i32; 2] = [4111006, 14101004];

/// The Dash skills (v83 FUN_009433a2): a double tap on an arrow casts them,
/// and the handler gives their speed and jump as two-state stats.
pub const DASHES: [i32; 2] = [5001005, 15001003];

/// Wings (v83 FUN_00930b27): the client jumps and glides down, and asks
/// only for the MP.
pub const WINGS: i32 = 5201005;

/// Skills that neither buff nor debuff: the handler does their work.
/// Chakra heals the user, MP Recovery turns HP into MP, Resurrection raises
/// dead party members, Time Leap ends cooldowns, Mystic Door opens a door,
/// and Smokescreen leaves its smoke. The GM's Heal + Dispel heals and cures
/// everyone around, and their Resurrection raises the dead around them.
/// Teleport, Flash Jump and Wings have no work, and Dash gives its own
/// buffs.
pub fn acts_alone(skill_id: i32) -> bool {
    matches!(
        skill_id,
        MYSTIC_DOOR
            | CHAKRA
            | MP_RECOVERY
            | RESURRECTION
            | TIME_LEAP
            | WINGS
            | SMOKESCREEN
            | GM_HEAL
            | GM_HIDE
            | GM_RESURRECTION
    ) || TELEPORTS.contains(&skill_id)
        || FLASH_JUMPS.contains(&skill_id)
        || DASHES.contains(&skill_id)
        || HEROS_WILL.contains(&skill_id)
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

/// The MP MP Recovery gives: its `y` percent of the HP it took. The v83
/// client leaves it to the server, so this is OdinMS-family servers' rule.
pub fn recovered_mp(player: &Player, skill: &Learned) -> i32 {
    skill.effect.x * player.effective_max_hp() / 100 * skill.effect.y / 100
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

/// A player's STR, DEX, and LUK with what they wear, and Maple Warrior's
/// percent of their own (BasicStat::SetFrom, 0x77f289).
pub(crate) fn total_stats(player: &Player) -> (i32, i32, i32) {
    let stats = &player.stats;
    let warrior = i32::from(player.buffs.value(BuffStat::MapleWarrior).unwrap_or(0));
    let own = |stat: i16| i32::from(stat) + i32::from(stat) * warrior / 100;
    player
        .inventory
        .items(InventoryType::Equipped)
        .filter_map(|(_, item)| item.equip.as_ref())
        .fold(
            (own(stats.str), own(stats.dex), own(stats.luk)),
            |(str, dex, luk), equip| {
                (
                    str + i32::from(equip.str),
                    dex + i32::from(equip.dex),
                    luk + i32::from(equip.luk),
                )
            },
        )
}

/// A player's INT with what they wear and Maple Warrior's percent of their
/// own, as `total_stats` counts the others.
pub(crate) fn total_int(player: &Player) -> i32 {
    let int = i32::from(player.stats.int);
    let warrior = i32::from(player.buffs.value(BuffStat::MapleWarrior).unwrap_or(0));
    player
        .inventory
        .items(InventoryType::Equipped)
        .filter_map(|(_, item)| item.equip.as_ref())
        .fold(int + int * warrior / 100, |total, equip| {
            total + i32::from(equip.int)
        })
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
        // Dragon Roar "stuns them" for its `y` seconds (it has no `time`).
        DRAGON_ROAR => one(MobStat::Stun),
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
        // Hypnotize: the mob stops attacking players and other mobs may hit
        // it (MobStat bit 28, 0x6663d2), by its `prop` for its `time`.
        HYPNOTIZE => one(MobStat::Dazzle),
        _ => return None,
    };
    Some(statuses)
}

/// The Corsair's Hypnotize, a close gun shot that puts Dazzle on the mob.
pub const HYPNOTIZE: i32 = 5221009;

/// Elemental Boost, which adds its `damage` percent to Flamethrower's and
/// Ice Splitter's (0x79004c) and, as its description says, its `y` seconds
/// to Ice Splitter's freeze.
pub const ELEMENTAL_BOOST: i32 = 5220001;
const FLAMETHROWER: i32 = 5211004;
const ICE_SPLITTER: i32 = 5211005;

/// The player's Elemental Boost, if they know it and it boosts `skill_id`.
pub fn elemental_boost(player: &Player, skill_id: i32) -> Option<Learned> {
    if skill_id != FLAMETHROWER && skill_id != ICE_SPLITTER {
        return None;
    }
    learned(player, ELEMENTAL_BOOST).filter(|boost| boost.level > 0)
}

/// Venomous Star (a claw's shots), Venomous Stab (a dagger's swings), and
/// the Night Walker's Venom.
const VENOMS: [(i32, bool); 3] = [(4120005, true), (4220005, false), (14110004, true)];

/// The buffs that arm the player's hits, with what they put on the mobs:
/// Hamstring slows them, and Blind makes their hits miss by its percent.
const ARMED: [(i32, BuffStat, MobStat); 2] = [
    (3121007, BuffStat::Hamstring, MobStat::Speed),
    (3221006, BuffStat::Blind, MobStat::Blind),
];

/// What an attack puts on each mob it hits, besides its damage: its
/// skill's status, Hamstring's or Blind's `x` by their `prop` for `y`
/// seconds while they last (Cosmic's AbstractDealDamageHandler), and the
/// poison of a thief's Venom, each second a roll
/// between (8 (STR + LUK) + 2 DEX) / 100 and (18.5 (STR + LUK) + 2 DEX) /
/// 100 times the Venom's `mad` (the old community damage formulas).
pub fn on_hit(player: &Player, skill: Option<&Learned>, ranged: bool, roll: f64) -> Vec<Inflict> {
    let mut inflicts = Vec::new();
    if let Some(skill) = skill {
        if let Some(statuses) = hit_statuses(skill) {
            let mut inflict = Inflict::new(skill, statuses);
            if skill.skill_id == DRAGON_ROAR {
                inflict.duration = Duration::from_secs(skill.effect.y.max(0) as u64);
            }
            if skill.skill_id == ICE_SPLITTER {
                if let Some(boost) = elemental_boost(player, ICE_SPLITTER) {
                    inflict.duration += Duration::from_secs(boost.effect.y.max(0) as u64);
                }
            }
            inflicts.push(inflict);
        }
    }
    for (skill_id, buff, status) in ARMED {
        let armed = learned(player, skill_id).filter(|_| player.buffs.has(buff));
        if let Some(skill) = armed {
            let effect = &skill.effect;
            inflicts.push(Inflict {
                skill_id,
                prop: if effect.prop > 0 { effect.prop } else { 100 },
                duration: Duration::from_secs(effect.y.max(0) as u64),
                statuses: vec![(status, Amount::Value(effect.x as i16))],
            });
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

/// Drain, the Marauder's and the Thunder Breaker's Energy Drain, and
/// Vampire: each gives back its `x` percent of the damage as HP.
pub const DRAINS: [i32; 4] = [4101005, 5111004, 15111001, 14101006];

/// Mortal Blow, the Ranger's and the Sniper's close shot.
pub const MORTAL_BLOWS: [i32; 2] = [3110001, 3210001];

/// The wizards' and the cleric's MP Eater.
pub const MP_EATERS: [i32; 3] = [2100000, 2200000, 2300000];

/// What an attack takes from the mobs it hits besides their HP.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Takes {
    /// Drain and its kin: this percent of the damage comes back as HP, no
    /// more than each mob's max HP, nor half the player's (Drain's
    /// description).
    pub hp: i32,
    /// MP Eater, for a spell.
    pub mp: Option<MpEater>,
    /// Mortal Blow's kill came up: a mob, not a boss, at or under this
    /// percent of its HP dies.
    pub life: i32,
    /// Steal: each mob hit, not yet stolen from, drops one of its items with
    /// this percent chance, then never again (Cosmic's rule; the v83 client
    /// sends a plain swing and sees a plain drop).
    pub steal: i32,
}

/// MP Eater: with its chance (`prop`), each mob a spell hits, not a boss,
/// loses its `x` percent of its max MP to the player, as much as it has,
/// and everyone sees the skill's effect (as Cosmic's applyPassive).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MpEater {
    pub skill_id: i32,
    pub level: u8,
    pub prop: i32,
    pub x: i32,
}

impl MpEater {
    /// Whether it eats this time.
    pub fn eats(&self, roll: f64) -> bool {
        roll * 100.0 < f64::from(self.prop)
    }
}

/// What this attack takes. A spell has the MP Eater of the player's own
/// branch; Mortal Blow rolls its `y` percent chance of a kill here.
pub fn takes(player: &Player, skill: Option<&Learned>, magic: bool, roll: f64) -> Takes {
    let mut takes = Takes::default();
    if let Some(skill) = skill {
        if DRAINS.contains(&skill.skill_id) {
            takes.hp = skill.effect.x;
        }
        if MORTAL_BLOWS.contains(&skill.skill_id) && roll * 100.0 < f64::from(skill.effect.y) {
            takes.life = skill.effect.x;
        }
        if skill.skill_id == STEAL {
            takes.steal = skill.effect.prop;
        }
    }
    let eater = i32::from(player.stats.job) / 10 * 100_000;
    if magic && MP_EATERS.contains(&eater) {
        takes.mp = learned(player, eater)
            .filter(|eater| eater.level > 0)
            .map(|eater| MpEater {
                skill_id: eater.skill_id,
                level: eater.level as u8,
                prop: eater.effect.prop,
                x: eater.effect.x,
            });
    }
    takes
}

pub const STEAL: i32 = 4201004;
pub const PICKPOCKET: i32 = 4211003;

/// The attacks Pickpocket picks with: a plain swing and the Bandit's own
/// (Cosmic's list).
const PICKS: [i32; 8] = [
    0, 4001334, 4201005, 4211002, 4211004, 4221001, 4221003, 4221007,
];

/// Pickpocket's mesos for an attack, per mob hit: each line that hurts
/// shakes out, with the skill's chance (`prop`), a line/20000 share of its
/// `x`, at least 1 (Cosmic's rule; v83's client only shows the drops).
pub fn pickpocket(
    player: &Player,
    attack: &crate::combat::Attack,
    mut roll: impl FnMut() -> f64,
) -> Vec<(i32, Vec<i32>)> {
    let Some(most) = player.buffs.value(BuffStat::PickPocket) else {
        return Vec::new();
    };
    let Some(skill) = learned(player, PICKPOCKET).filter(|skill| skill.level > 0) else {
        return Vec::new();
    };
    if !PICKS.contains(&attack.skill) {
        return Vec::new();
    }
    let most = i64::from(most);
    attack
        .hits
        .iter()
        .map(|hit| {
            let mesos = hit
                .damage
                .iter()
                .filter(|&&line| line > 0)
                .filter(|_| roll() * 100.0 < f64::from(skill.effect.prop))
                .map(|&line| (i64::from(line) * most / 20_000).clamp(1, most) as i32)
                .collect();
            (hit.object_id, mesos)
        })
        .filter(|(_, mesos): &(i32, Vec<i32>)| !mesos.is_empty())
        .collect()
}

/// Poison Mist: a cloud where it's cast that poisons the mobs in it.
pub const POISON_MIST: i32 = 2111003;

/// Smokescreen: smoke around the caster where mobs can't hurt them or
/// their party, which the v83 client works out itself (FUN_0043193a).
pub const SMOKESCREEN: i32 = 4221006;

/// What the mist a skill leaves puts on the mobs in it, each with the
/// skill's chance, for as long as the mist's `time`; None for a skill that
/// leaves none. v83's client draws the mist (0x431a63); what it does is the
/// server's (OdinMS-family: Poison Mist poisons, Smokescreen nothing).
pub fn mist(skill: &Learned) -> Option<Inflict> {
    match skill.skill_id {
        POISON_MIST => Some(Inflict::new(
            skill,
            vec![(MobStat::Poison, Amount::Poison { level: skill.level })],
        )),
        SMOKESCREEN => Some(Inflict::new(skill, Vec::new())),
        _ => None,
    }
}

/// The kind of mist a skill leaves: smoke for Smokescreen, else poison.
pub fn mist_kind(skill_id: i32) -> MistKind {
    if skill_id == SMOKESCREEN {
        MistKind::Smoke
    } else {
        MistKind::Poison
    }
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
        // Invincibility: no mob can hurt the player (FUN_00959708).
        1010 | 10001010 | 20001010 | 20011010 => vec![(BuffStat::DivineBody, effect.x)],
        1002 | 10001002 | 20001002 => vec![],
        // Iron Body, Rage, Iron Will, Magic Armor, Meditation, Bless,
        // Focus, Haste, Enrage.
        1001003 | 1101006 | 1301006 | 2001003 | 2101001 | 2201001 | 2301004 | 3001003 | 4101004
        | 4201003 | 1121010 => vec![],
        1101004 | 1101005 | 1201004 | 1201005 | 1301004 | 1301005 | 2111005 | 2211005 | 3101002
        | 3201002 | 4101003 | 4201002 | 5101006 | 5201003 => {
            vec![(BuffStat::Booster, effect.x)]
        }
        1101007 | 1201007 => vec![(BuffStat::PowerGuard, effect.x)],
        5121009 | 15111005 => vec![(BuffStat::SpeedInfusion, effect.x)],
        // The orbs start at none, which the client counts from 1.
        1111002 => vec![(BuffStat::Combo, 1)],
        1211003..=1211008 | 1221003 | 1221004 => {
            return Some(vec![(BuffStat::WeaponCharge, effect.x)]);
        }
        1301007 | 9101008 => vec![(BuffStat::MaxHp, effect.x), (BuffStat::MaxMp, effect.y)],
        // The GM's Haste (Normal) and (Super), and Bless.
        9001000 | 9101001 | 9101003 => vec![],
        9101002 => vec![(BuffStat::HolySymbol, effect.x)],
        DRAGON_BLOOD => vec![(BuffStat::DragonBlood, effect.x)],
        1121000 | 1221000 | 1321000 | 2121000 | 2221000 | 2321000 | 3121000 | 3221000 | 4121000
        | 4221000 | 5121000 | 5221000 => vec![(BuffStat::MapleWarrior, effect.x)],
        1121002 | 1221002 | 1321002 => vec![(BuffStat::Stance, effect.prop)],
        2001002 => vec![(BuffStat::MagicGuard, effect.x)],
        2301003 => vec![(BuffStat::Invincible, effect.x)],
        2311003 => vec![(BuffStat::HolySymbol, effect.x)],
        2121004 | 2221004 | 2321004 => vec![(BuffStat::Infinity, effect.x)],
        2321005 => vec![(BuffStat::HolyShield, effect.x)],
        3121007 => vec![(BuffStat::Hamstring, effect.x)],
        3221006 => vec![(BuffStat::Blind, effect.x)],
        1005 | 10001005 | 20001005 => vec![(BuffStat::EchoOfHero, effect.x)],
        2121002 | 2221002 | 2321002 => vec![(BuffStat::ManaReflection, skill.level)],
        3101004 | 3201004 | 13101003 => vec![(BuffStat::SoulArrow, effect.x)],
        // The caster names the star, which the buff's value carries.
        SHADOW_STARS => vec![(BuffStat::ShadowStars, 0)],
        3121008 => vec![(BuffStat::Concentrate, effect.x)],
        3121002 | 3221002 => vec![(BuffStat::SharpEyes, effect.x << 8 | effect.y)],
        4001003 => vec![(BuffStat::DarkSight, effect.x)],
        4111001 => vec![(BuffStat::MesoUp, effect.x)],
        4111002 | 14111000 => vec![(BuffStat::ShadowPartner, effect.x)],
        4211003 => vec![(BuffStat::PickPocket, effect.x)],
        4211005 => vec![(BuffStat::MesoGuard, effect.x)],
        crate::morph::OAK_BARREL
        | crate::morph::TRANSFORMATION
        | crate::morph::SUPER_TRANSFORMATION => vec![(BuffStat::Morph, effect.morph)],
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

    #[test]
    fn fourth_jobs_are_taught_the_skills_with_a_master_level() {
        let taught = |job| {
            let mut ids: Vec<_> = fourth_job_skills(job)
                .into_iter()
                .inspect(|record| assert_eq!((record.level, record.master_level), (0, 10)))
                .map(|record| record.skill_id)
                .collect();
            ids.sort();
            ids
        };
        assert_eq!(taught(112), [1120004, 1121001, 1121008]);
        assert_eq!(taught(232), [2321001, 2321002, 2321005]);
        assert_eq!(taught(412), [4120002, 4120005, 4121006]);
        assert_eq!(taught(512), [5121001, 5121002, 5121007, 5121009]);
        assert_eq!(taught(522), [5220001, 5220002, 5220011, 5221004]);
        assert!(taught(111).is_empty());
    }

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
    fn the_secret_skills_need_no_record_but_their_own_maps() {
        if data().is_err() || crate::get_game_data().is_err() {
            return;
        }
        let mut player = knowing(&[]);
        let level = |player: &Player, skill_id| learned(player, skill_id).map(|s| s.level);
        player.stats.map_id = 100_000_000;
        assert_eq!(level(&player, 1010), None);
        assert_eq!(level(&player, 1020), None);
        // A Dojo stage (fieldType 14).
        player.stats.map_id = 925_020_100;
        assert_eq!(level(&player, 1010), Some(1));
        assert_eq!(level(&player, 1009), Some(1));
        assert_eq!(level(&player, 1020), None);
        let invincibility = learned(&player, 1010).unwrap();
        assert_eq!(buffs(&invincibility), Some(vec![(BuffStat::DivineBody, 1)]));
        assert_eq!(invincibility.effect.duration, 30);
        // Transformed, the attack is refused but the buff isn't.
        let now = std::time::Instant::now();
        let morph = game::Buff::new(-2210000, BuffStat::Morph, 1, now, Duration::from_secs(60));
        player.buffs.give(morph);
        assert_eq!(level(&player, 1009), None);
        assert_eq!(level(&player, 1010), Some(1));
        // Nett's Pyramid (fieldType 23).
        let mut player = knowing(&[]);
        player.stats.map_id = 926_010_100;
        assert_eq!(level(&player, 1020), Some(1));
        assert_eq!(level(&player, 1009), None);
    }

    #[test]
    fn the_secret_attacks_take_a_mobs_max_hp_and_bamboo_a_bosss_30_percent() {
        assert_eq!(pq_strike_cap(1009, 1000, false), Some(1000));
        assert_eq!(pq_strike_cap(1009, 1000, true), Some(300));
        assert_eq!(pq_strike_cap(10001020, 1000, true), Some(1000));
        assert_eq!(pq_strike_cap(1010, 1000, false), None);
        assert_eq!(pq_strike_cap(1001009, 1000, false), None);
        assert_eq!(pq_skill(20011011), Some(PqSkill::DojoBuff));
    }

    #[test]
    fn the_gm_skills_are_a_gms_alone() {
        if data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player.skills.set(game::SkillRecord {
            skill_id: 9101003,
            level: 1,
            master_level: 0,
            expiration: None,
        });
        assert_eq!(learned(&player, 9101003), None);
        player.gm_level = 1;
        assert_eq!(learned(&player, 9101003).map(|skill| skill.level), Some(1));
    }

    #[test]
    fn the_gm_buffs_give_their_stats() {
        if data().is_err() {
            return;
        }
        let stats = |skill_id| buffs(&at_level(skill_id, 1).unwrap()).unwrap();
        assert!(stats(9101002).contains(&(BuffStat::HolySymbol, 50)));
        assert!(stats(9101003).contains(&(BuffStat::Pad, 20)));
        assert!(stats(9101008).contains(&(BuffStat::MaxHp, 60)));
        assert!(stats(9001000).contains(&(BuffStat::Speed, 40)));
        assert!(reaches_everyone(9101000) && !reaches_everyone(9001000));
        assert!(raises_dead(9101005) && acts_alone(9101005) && acts_alone(9001002));
    }

    #[test]
    fn the_knights_soul_arrow_and_shadow_partner_are_buffs() {
        let stat = |skill_id| buffs(&at_level(skill_id, 1).unwrap()).unwrap()[0].0;
        assert_eq!(stat(13101003), BuffStat::SoulArrow);
        assert_eq!(stat(14111000), BuffStat::ShadowPartner);
    }

    #[test]
    fn maple_warrior_adds_its_percent_of_the_players_own_stats() {
        let mut player = knowing(&[]);
        (player.stats.str, player.stats.dex, player.stats.luk) = (99, 50, 4);
        let ring = game::EquipStats {
            str: 100,
            ..Default::default()
        };
        let ring = game::Item::equipment(1112000, ring);
        player.inventory.insert(InventoryType::Equipped, -12, ring);
        assert_eq!(total_stats(&player), (199, 50, 4));
        let now = std::time::Instant::now();
        let long = Duration::from_secs(60);
        let warrior = game::Buff::new(1121000, BuffStat::MapleWarrior, 10, now, long);
        player.buffs.give(warrior);
        // Ten percent of 99 is 9, of 4 none; the ring's 100 isn't counted.
        assert_eq!(total_stats(&player), (208, 55, 4));
    }

    #[test]
    fn improving_max_hp_and_mp_add_to_their_jobs_growth() {
        let mut player = knowing(&[
            (IMPROVING_MAX_HP, 10),
            (IMPROVING_MAX_MP, 3),
            (BRAWLER_IMPROVE_MAX_HP, 2),
        ]);
        player.stats.job = 110;
        player.stats.int = 9;
        assert_eq!(growth_bonus(&player, true), (40, 0));
        assert_eq!(growth_bonus(&player, false), (30, 0));
        player.stats.job = 230;
        assert_eq!(growth_bonus(&player, true), (0, 6));
        assert_eq!(growth_bonus(&player, false), (0, 3));
        player.stats.job = 510;
        assert_eq!(growth_bonus(&player, true), (6, 0));
        // A Gunslinger has no Improve MaxHP.
        player.stats.job = 520;
        assert_eq!(growth_bonus(&player, false), (0, 0));
    }

    #[test]
    fn int_adds_to_max_mp_growth() {
        // A tenth of total INT on a level up, 3/40 for an AP, whatever the
        // job.
        let mut player = knowing(&[]);
        player.stats.job = 230;
        player.stats.int = 400;
        assert_eq!(growth_bonus(&player, true), (0, 40));
        assert_eq!(growth_bonus(&player, false), (0, 30));
        player.stats.job = 100;
        player.stats.int = 4;
        assert_eq!(growth_bonus(&player, true), (0, 0));
        assert!((0..=4).contains(&growth_roll(4)));
    }

    #[test]
    fn skills_cost_their_hp_and_dragon_roar_a_share_of_max_hp() {
        let mut player = knowing(&[]);
        player.stats.max_hp = 1000;
        player.stats.hp = 9;
        // Slash Blast at 1 costs 8, and needs more than that.
        let slash_blast = at_level(1001005, 1).unwrap();
        assert_eq!(hp_cost(&player, &slash_blast), Some(8));
        player.stats.hp = 8;
        assert_eq!(hp_cost(&player, &slash_blast), None);
        assert_eq!(hp_cost(&player, &at_level(1001004, 1).unwrap()), Some(0));
        // Dragon Roar at 30 takes 30 percent, from half of max HP up.
        let roar = at_level(DRAGON_ROAR, 30).unwrap();
        player.stats.hp = 500;
        assert_eq!(hp_cost(&player, &roar), Some(300));
        player.stats.hp = 499;
        assert_eq!(hp_cost(&player, &roar), None);
    }

    #[test]
    fn infinity_makes_skills_free_of_mp() {
        let mut player = knowing(&[]);
        let slash_blast = at_level(1001005, 1).unwrap();
        assert_eq!(mp_cost(&player, &slash_blast), slash_blast.effect.mp_cost);
        assert!(slash_blast.effect.mp_cost > 0);
        let now = std::time::Instant::now();
        let long = Duration::from_secs(40);
        player
            .buffs
            .give(game::Buff::new(2121004, BuffStat::Infinity, 1, now, long));
        assert_eq!(mp_cost(&player, &slash_blast), 0);
        // HP costs stay.
        player.stats.hp = 100;
        assert_eq!(hp_cost(&player, &slash_blast), Some(8));
    }

    #[test]
    fn dragon_roar_stuns_its_user_for_its_y_seconds() {
        let stun = |level| self_stun(&at_level(DRAGON_ROAR, level).unwrap());
        let stun_30 = stun(30).unwrap();
        assert_eq!((stun_30.stat, stun_30.value), (BuffStat::Stun, 1));
        assert_eq!(stun_30.skill_id, DRAGON_ROAR);
        assert_eq!(stun_30.lasts, Duration::from_secs(2));
        assert_eq!(stun(1).unwrap().lasts, Duration::from_secs(4));
        assert_eq!(self_stun(&at_level(1001005, 20).unwrap()), None);
    }

    #[test]
    fn drains_eaters_and_mortal_blows_take_their_shares() {
        let mut player = knowing(&[(2100000, 20), (2200000, 20)]);
        player.stats.job = 211;
        let drain = at_level(4101005, 30).unwrap();
        assert_eq!(takes(&player, Some(&drain), false, 0.0).hp, 45);
        // A fire wizard eats with the fire wizard's skill, from a spell only.
        let eater = takes(&player, None, true, 0.0).mp.unwrap();
        assert_eq!((eater.skill_id, eater.level), (2100000, 20));
        assert_eq!((eater.prop, eater.x), (30, 40));
        assert!(eater.eats(0.29) && !eater.eats(0.3));
        assert_eq!(takes(&player, None, false, 0.0).mp, None);
        player.stats.job = 200;
        assert_eq!(takes(&player, None, true, 0.0).mp, None);
        player.stats.job = 230;
        assert_eq!(takes(&player, None, true, 0.0).mp, None);
        // Mortal Blow at 20 kills at half HP, one time in ten.
        let blow = at_level(3110001, 20).unwrap();
        assert_eq!(takes(&player, Some(&blow), false, 0.09).life, 50);
        assert_eq!(takes(&player, Some(&blow), false, 0.1).life, 0);
        // Steal at 30 steals four times in ten.
        let steal = at_level(STEAL, 30).unwrap();
        assert_eq!(takes(&player, Some(&steal), false, 0.99).steal, 40);
        assert_eq!(takes(&player, Some(&drain), false, 0.0).steal, 0);
    }

    #[test]
    fn pickpocket_shakes_mesos_out_of_the_lines_that_hurt() {
        use crate::combat::{AttackHit, AttackKind};
        let attack = |skill, damage: Vec<i32>| crate::combat::Attack {
            kind: AttackKind::Close,
            skill,
            hits: vec![AttackHit {
                object_id: 7,
                damage,
            }],
            mob_count: 1,
            hit_count: 1,
            display: 0,
            action: 0,
            speed: 4,
            charge: 0,
            ammo_slot: 0,
            explosion: None,
        };
        let mut player = knowing(&[(PICKPOCKET, 20)]);
        let lines = vec![10_000, 0, 1, 50_000, -5];
        // Without the buff, nothing.
        assert!(pickpocket(&player, &attack(0, lines.clone()), || 0.0).is_empty());
        (player.buffs).give(game::Buff::new(
            PICKPOCKET,
            BuffStat::PickPocket,
            200,
            std::time::Instant::now(),
            Duration::from_secs(120),
        ));
        // Level 20 picks 60% of the time, up to 200 a line.
        assert_eq!(
            pickpocket(&player, &attack(0, lines.clone()), || 0.59),
            vec![(7, vec![100, 1, 200])]
        );
        assert!(pickpocket(&player, &attack(0, lines.clone()), || 0.6).is_empty());
        // Savage Blow picks; Lucky Seven doesn't.
        assert_eq!(
            pickpocket(&player, &attack(4201005, vec![4_000]), || 0.0)[0].1,
            [40]
        );
        assert!(pickpocket(&player, &attack(4001344, vec![4_000]), || 0.0).is_empty());
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
    fn hamstring_and_blind_arm_hits_while_they_last() {
        if data().is_err() {
            return;
        }
        let mut player = knowing(&[(3121007, 30), (3221006, 30)]);
        assert!(on_hit(&player, None, true, 0.5).is_empty());
        let now = std::time::Instant::now();
        let lasts = Duration::from_secs(180);
        for (skill_id, stat) in [(3121007, BuffStat::Hamstring), (3221006, BuffStat::Blind)] {
            player
                .buffs
                .give(game::Buff::new(skill_id, stat, 1, now, lasts));
        }
        // At 30: a 40% chance of -60 speed or a 30% miss, for 15 seconds.
        let armed = |skill_id, status| Inflict {
            skill_id,
            prop: 40,
            duration: Duration::from_secs(15),
            statuses: vec![(
                status,
                Amount::Value(if skill_id == 3121007 { -60 } else { 30 }),
            )],
        };
        assert_eq!(
            on_hit(&player, None, true, 0.5),
            [
                armed(3121007, MobStat::Speed),
                armed(3221006, MobStat::Blind)
            ]
        );
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
        // Dragon Roar always stuns, for 4 seconds at 1 and 2 at 30.
        let roar = |level| {
            on_hit(
                &player,
                Some(&at_level(DRAGON_ROAR, level).unwrap()),
                false,
                0.5,
            )
        };
        assert_eq!(
            roar(1),
            [Inflict {
                skill_id: DRAGON_ROAR,
                prop: 100,
                duration: Duration::from_secs(4),
                statuses: vec![(MobStat::Stun, Amount::Value(1))],
            }]
        );
        assert_eq!(roar(30)[0].duration, Duration::from_secs(2));
        let breath = at_level(2101005, 30).unwrap();
        assert_eq!(
            on_hit(&player, Some(&breath), false, 0.5)[0].statuses,
            [(MobStat::Poison, Amount::Poison { level: 30 })]
        );
        assert!(on_hit(&player, None, true, 0.5).is_empty());

        // Ice Splitter at 30 freezes for 3 seconds, and 2 more with
        // Elemental Boost at 30.
        let splitter = at_level(ICE_SPLITTER, 30).unwrap();
        let freeze = |player: &Player| on_hit(player, Some(&splitter), true, 0.5)[0].duration;
        assert_eq!(freeze(&player), Duration::from_secs(3));
        let corsair = knowing(&[(ELEMENTAL_BOOST, 30)]);
        assert_eq!(freeze(&corsair), Duration::from_secs(5));
        assert_eq!(
            elemental_boost(&corsair, FLAMETHROWER)
                .unwrap()
                .effect
                .damage,
            200
        );
        assert!(elemental_boost(&corsair, 5221004).is_none());

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
    fn hypnotize_dazzles_by_its_prop_for_its_time() {
        let player = knowing(&[]);
        let hypnotize = at_level(HYPNOTIZE, 1).unwrap();
        let [inflict] = &on_hit(&player, Some(&hypnotize), false, 0.5)[..] else {
            panic!("one inflict");
        };
        assert_eq!(inflict.skill_id, HYPNOTIZE);
        assert_eq!(inflict.prop, 43);
        assert_eq!(inflict.duration, Duration::from_secs(21));
        assert_eq!(inflict.statuses, [(MobStat::Dazzle, Amount::Value(1))]);
        assert!(inflict.takes(0.42) && !inflict.takes(0.43));
        assert!(MobStat::Dazzle.spares_bosses());
    }

    #[test]
    fn smokescreen_leaves_smoke_and_poison_mist_poison() {
        assert_eq!(mist_kind(SMOKESCREEN), MistKind::Smoke);
        assert_eq!(mist_kind(POISON_MIST), MistKind::Poison);
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
