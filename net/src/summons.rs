//! Summons: what Puppet, Silver Hawk, the Octopus, Elquines, and the like
//! call up. A summon lasts as long as its skill's buff; the owner's client
//! moves it and decides when it attacks (v83 CSummoned, report 158), and
//! the server shows the others and deals the damage.

use crate::error::NetworkError;
use crate::skills::{self, Amount, Inflict, Learned};
use game::{BuffStat, InventoryType, MobStat, Player};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use std::time::Duration;

pub const PUPPETS: [i32; 3] = [3111002, 3211002, 13111004];
pub const OCTOPUS: i32 = 5211001;
pub const GAVIOTA: i32 = 5211002;
pub const WRATH_OF_THE_OCTOPI: i32 = 5220002;
pub const BEHOLDER: i32 = 1321007;
pub const AURA_OF_THE_BEHOLDER: i32 = 1320008;
pub const HEX_OF_THE_BEHOLDER: i32 = 1320009;

/// How a summon moves (its spawn's moveAbility, v83 CVecCtrlSummoned).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Moves {
    /// Never moves; also follows nobody to another map.
    Stationary = 0,
    /// Glides after its owner.
    Follow = 1,
    /// Flies around its owner.
    CircleFollow = 3,
}

/// What a summon does on its own (its spawn's assistType).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Assist {
    /// Only stands there (Puppets).
    Nothing = 0,
    Attack = 1,
    /// Heals and buffs its owner (Beholder).
    Support = 2,
}

/// How a skill's summon moves and what it does, or `None` for a skill
/// that summons nothing (the OdinMS-family tables; v83 reads both from the
/// spawn). Beginners' Echo-like 10001013/20001013 summon nothing here.
pub fn kind(skill_id: i32) -> Option<(Moves, Assist)> {
    let kind = match skill_id {
        3111002 | 3211002 | 13111004 => (Moves::Stationary, Assist::Nothing),
        OCTOPUS | WRATH_OF_THE_OCTOPI => (Moves::Stationary, Assist::Attack),
        // Silver Hawk, Golden Eagle, Summon Dragon, Phoenix, Frostprey,
        // Gaviota.
        3111005 | 3211005 | 2311006 | 3121006 | 3221005 | GAVIOTA => {
            (Moves::CircleFollow, Assist::Attack)
        }
        BEHOLDER => (Moves::Follow, Assist::Support),
        // Elquines, Ifrit, Bahamut, the Cygnus Knights' souls, and Blaze
        // Wizard's Ifrit.
        2121005 | 2221005 | 2321003 | 11001004 | 12001004 | 13001004 | 14001005 | 15001004
        | 12111004 => (Moves::Follow, Assist::Attack),
        _ => return None,
    };
    Some(kind)
}

pub fn stationary(skill_id: i32) -> bool {
    matches!(kind(skill_id), Some((Moves::Stationary, _)))
}

/// The buff a summon's skill gives, its icon: Puppet's own stat for the
/// summons that stay put, the combo counter at no orbs for the others
/// (as OdinMS-family servers do). A Ranger can keep a Puppet and a Silver
/// Hawk at once, each with its own stat.
pub fn stat(skill_id: i32) -> BuffStat {
    if stationary(skill_id) {
        BuffStat::Puppet
    } else {
        BuffStat::Combo
    }
}

/// How much damage a Puppet takes before it breaks: its skill's `x`. The
/// others can't be hurt (FUN_007a6040).
pub fn hp(skill: &Learned) -> Option<i32> {
    PUPPETS.contains(&skill.skill_id).then_some(skill.effect.x)
}

/// The most mobs one attack of a summon reaches: its `summon/attack1/info`
/// mobCount (1 if none), or the level's if more (Bahamut's grows).
pub fn mob_count(skill: &Learned) -> i32 {
    let info = match skill.skill_id {
        3121006 | 3221005 => 4,
        2121005 | 2221005 | 2321003 | 12111004 | GAVIOTA | 11001004 | 12001004 | 13001004
        | 14001005 | 15001004 => 3,
        _ => 1,
    };
    info.max(skill.effect.mob_count)
}

/// Whether a summon's attacks are magic (the list at 0x7ad4e0).
fn magic(skill_id: i32) -> bool {
    matches!(
        skill_id,
        2121005 | 2221005 | 2321003 | 12111004 | 2311006 | 12001004
    )
}

/// The most one of a summon's hits can do, twice what the owner's client
/// can roll before its multipliers (the mob's element, criticals). A
/// physical hit is the owner's main stat times 2.5 plus the other
/// (FUN_0079216d), a magic one the v83 spell formula on their magic
/// capped at 1999 (FUN_00792595), each times the summon's `pad` or `mad`
/// percent.
pub fn max_damage(player: &Player, skill: &Learned) -> i32 {
    let mut stat = [
        player.stats.str,
        player.stats.dex,
        player.stats.int,
        player.stats.luk,
    ]
    .map(f64::from);
    let mut matk = 0.0;
    for (_, item) in player.inventory.items(InventoryType::Equipped) {
        if let Some(equip) = &item.equip {
            for (total, add) in stat
                .iter_mut()
                .zip([equip.str, equip.dex, equip.int, equip.luk])
            {
                *total += f64::from(add);
            }
            matk += f64::from(equip.matk);
        }
    }
    let [str, dex, int, luk] = stat;
    let roll = if magic(skill.skill_id) {
        let mad = player.buffs.value(BuffStat::Mad).map_or(0.0, f64::from);
        let magic = (int + matk + mad).min(1999.0);
        let squared = magic * 0.058;
        (magic * 3.3 + int * 0.5 + squared * squared) * f64::from(skill.effect.magic)
    } else {
        let main = str.max(dex).max(luk);
        (main * 2.5 + str + dex + luk - main) * f64::from(skill.effect.pad)
    };
    ((roll / 100.0 * 2.0).ceil() as i32).max(10)
}

/// What a summon's hits put on the mobs: Silver Hawk's and Golden Eagle's
/// stun, at their `prop` (50% to 99%), and Elquines' and Frostprey's
/// freeze, always (their data has no `prop`). The client shows only what
/// it's sent and v83's server isn't available, so each lasts the skill's
/// `x` seconds, the value that grows from 1 to 3 on the freezes
/// (inferred; OdinMS-family servers hold every summon's status 4 s).
pub fn inflict(skill: &Learned) -> Option<Inflict> {
    let stat = match skill.skill_id {
        3111005 | 3211005 => MobStat::Stun,
        2121005 | 3221005 => MobStat::Freeze,
        _ => return None,
    };
    let effect = &skill.effect;
    Some(Inflict {
        skill_id: skill.skill_id,
        prop: if effect.prop > 0 { effect.prop } else { 100 },
        duration: Duration::from_secs(effect.x.max(1) as u64),
        statuses: vec![(stat, Amount::Value(1))],
    })
}

/// What the owner's summons may do in one attack, for each summon skill
/// they have running: the field checks the attack against its summon's.
#[derive(Clone, Debug, PartialEq)]
pub struct Limit {
    pub skill_id: i32,
    pub mob_count: i32,
    pub max_damage: i32,
    /// What its hits put on the mobs.
    pub inflict: Option<Inflict>,
}

/// The limits for each summon the player has up.
pub fn limits(player: &Player) -> Vec<Limit> {
    player
        .buffs
        .active()
        .iter()
        .filter(|buff| kind(buff.skill_id).is_some())
        .filter_map(|buff| skills::learned(player, buff.skill_id))
        .map(|skill| Limit {
            skill_id: skill.skill_id,
            mob_count: mob_count(&skill),
            max_damage: max_damage(player, &skill),
            inflict: inflict(&skill),
        })
        .collect()
}

/// One mob a summon hit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SummonHit {
    pub object_id: i32,
    /// How the mob reacts (its hit action).
    pub action: u8,
    pub damage: i32,
}

/// SUMMON_ATTACK (recv 0xB0, built at 0x7a57d4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SummonAttack {
    pub object_id: i32,
    /// The animation, 4 for `attack1` or 5 for `attack2`, with 0x80 when
    /// it faces left.
    pub action: u8,
    pub hits: Vec<SummonHit>,
}

pub fn parse_attack(packet: &Packet) -> Result<SummonAttack, NetworkError> {
    let mut r = Cursor::new(&packet.bytes);
    r.read_short()?;
    let object_id = r.read_int()?;
    r.read_int()?; // tick
    let action = r.read_byte()?;
    let count = r.read_byte()?;
    r.read_bytes(8)?; // owner and summon positions
    let mut hits = Vec::with_capacity(usize::from(count));
    for _ in 0..count {
        let object_id = r.read_int()?;
        r.read_int()?; // mob template
        let action = r.read_byte()?;
        // Facing and frame, the damage stat, positions, and delay.
        r.read_bytes(1 + 1 + 1 + 8 + 2)?;
        let damage = r.read_int()?;
        hits.push(SummonHit {
            object_id,
            action,
            damage,
        });
    }
    Ok(SummonAttack {
        object_id,
        action,
        hits,
    })
}

/// DAMAGE_SUMMON (recv 0xB1, 0x7a627a): a mob hurt a Puppet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SummonDamage {
    pub object_id: i32,
    /// The mob's attack: -1 for touching it, -2 for no mob.
    pub attack: i8,
    pub damage: i32,
    /// The mob's template and whether it hit from the left.
    pub mob: Option<(i32, bool)>,
}

pub fn parse_damage(packet: &Packet) -> Result<SummonDamage, NetworkError> {
    let mut r = Cursor::new(&packet.bytes);
    r.read_short()?;
    let object_id = r.read_int()?;
    let attack = r.read_byte()? as i8;
    let damage = r.read_int()?;
    let mob = if attack > -2 {
        Some((r.read_int()?, r.read_byte()? != 0))
    } else {
        None
    };
    Ok(SummonDamage {
        object_id,
        attack,
        damage,
        mob,
    })
}

/// BEHOLDER (recv 0xB2, FUN_007a5ab4 / FUN_007a5c99): the player's
/// Beholder heals them (Aura) or buffs them (Hex, one of its stats).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BeholderAct {
    pub object_id: i32,
    pub skill_id: i32,
    /// `left << 7 | animation`: 6 for the heal, 7 + the stat for the buff.
    pub action: u8,
    /// Hex's stat, 0 to 4.
    pub buff: Option<u8>,
}

pub fn parse_beholder(packet: &Packet) -> Result<BeholderAct, NetworkError> {
    let mut r = Cursor::new(&packet.bytes);
    r.read_short()?;
    let object_id = r.read_int()?;
    let skill_id = r.read_int()?;
    let action = r.read_byte()?;
    let buff = if skill_id == HEX_OF_THE_BEHOLDER {
        Some(r.read_byte()?)
    } else {
        None
    };
    Ok(BeholderAct {
        object_id,
        skill_id,
        action,
        buff,
    })
}

/// What a Beholder does for its owner, at most once every `every`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Support {
    /// Aura of the Beholder: heal `hp`.
    Heal { hp: i32, every: Duration },
    /// Hex of the Beholder: one stat for `lasts`.
    Hex {
        stat: BuffStat,
        value: i16,
        lasts: Duration,
        every: Duration,
    },
}

impl Support {
    pub fn skill_id(&self) -> i32 {
        match self {
            Support::Heal { .. } => AURA_OF_THE_BEHOLDER,
            Support::Hex { .. } => HEX_OF_THE_BEHOLDER,
        }
    }

    pub fn every(&self) -> Duration {
        match *self {
            Support::Heal { every, .. } | Support::Hex { every, .. } => every,
        }
    }
}

/// Hex's stats in the order its levels add them, which is how the client
/// numbers them (FUN_007a5c99, level data +0x28, +0x40, +0x4c, +0x58,
/// +0x1c).
const HEX_STATS: [BuffStat; 5] = [
    BuffStat::Pdd,
    BuffStat::Mdd,
    BuffStat::Acc,
    BuffStat::Eva,
    BuffStat::Pad,
];

/// What the player's Beholder may do for them: an Aura or Hex they know,
/// and for Hex a stat its level gives. Nothing while they are dead.
pub fn support(player: &Player, act: &BeholderAct) -> Option<Support> {
    if player.stats.hp <= 0 || player.skills.level(BEHOLDER) <= 0 {
        return None;
    }
    let skill = skills::learned(player, act.skill_id).filter(|skill| skill.level > 0)?;
    let effect = &skill.effect;
    let every = Duration::from_secs(effect.x.max(0) as u64);
    match act.skill_id {
        AURA_OF_THE_BEHOLDER => Some(Support::Heal {
            hp: effect.hp,
            every,
        }),
        HEX_OF_THE_BEHOLDER => {
            let stat = *HEX_STATS.get(usize::from(act.buff?))?;
            let value = match stat {
                BuffStat::Pdd => effect.pdd,
                BuffStat::Mdd => effect.mdd,
                BuffStat::Acc => effect.acc,
                BuffStat::Eva => effect.eva,
                _ => effect.pad,
            };
            (value > 0).then(|| Support::Hex {
                stat,
                value: value as i16,
                lasts: Duration::from_secs(effect.duration.max(0) as u64),
                every,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::SkillRecord;
    use packet::io::write::PktWrite;

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
    fn summons_that_stay_put_show_the_puppet_icon() {
        assert_eq!(kind(3111002), Some((Moves::Stationary, Assist::Nothing)));
        assert_eq!(kind(OCTOPUS), Some((Moves::Stationary, Assist::Attack)));
        assert_eq!(kind(3111005), Some((Moves::CircleFollow, Assist::Attack)));
        assert_eq!(kind(BEHOLDER), Some((Moves::Follow, Assist::Support)));
        assert_eq!(kind(3111003), None);
        assert_eq!(stat(3211002), BuffStat::Puppet);
        assert_eq!(stat(3211005), BuffStat::Combo);
    }

    #[test]
    fn a_summon_hits_by_its_pad_or_mad() {
        // STR 35, DEX 4, LUK 4: (35 * 2.5 + 8) * 100% * 2.
        let hawk = skills::at_level(3111005, 30).unwrap();
        let player = knowing(&[]);
        assert_eq!(max_damage(&player, &hawk), 191);
        let phoenix = skills::at_level(3121006, 30).unwrap();
        assert_eq!(mob_count(&phoenix), 4);
        assert_eq!(max_damage(&player, &phoenix), 1051);
        // Bahamut at 30 reaches 6, more than its attack's 3.
        assert_eq!(mob_count(&skills::at_level(2321003, 30).unwrap()), 6);
        let puppet = skills::at_level(3111002, 20).unwrap();
        assert_eq!(hp(&puppet), Some(6000));
        assert_eq!(hp(&hawk), None);
    }

    #[test]
    fn limits_cover_the_summons_up() {
        let mut player = knowing(&[(3111005, 30), (3111002, 20)]);
        let now = std::time::Instant::now();
        let lasts = std::time::Duration::from_secs(60);
        player
            .buffs
            .give(game::Buff::new(3111005, BuffStat::Combo, 1, now, lasts));
        let limits = limits(&player);
        assert_eq!(limits.len(), 1);
        assert_eq!(limits[0].skill_id, 3111005);
        assert_eq!(limits[0].mob_count, 1);
    }

    #[test]
    fn hawks_stun_and_the_ice_summons_freeze() {
        let player = knowing(&[
            (3111005, 1),
            (3211005, 30),
            (2121005, 30),
            (3221005, 1),
            (3121006, 30),
        ]);
        let inflict = |skill_id| inflict(&skills::learned(&player, skill_id).unwrap());
        let hawk = inflict(3111005).unwrap();
        assert_eq!(
            (hawk.prop, hawk.duration, hawk.statuses),
            (
                50,
                Duration::from_secs(1),
                vec![(MobStat::Stun, Amount::Value(1))]
            )
        );
        assert_eq!(inflict(3211005).unwrap().prop, 99);
        let elquines = inflict(2121005).unwrap();
        assert_eq!(
            (elquines.prop, elquines.duration, elquines.statuses),
            (
                100,
                Duration::from_secs(3),
                vec![(MobStat::Freeze, Amount::Value(1))]
            )
        );
        assert_eq!(inflict(3221005).unwrap().duration, Duration::from_secs(1));
        assert_eq!(inflict(3121006), None);
    }

    #[test]
    fn a_summon_attack_names_each_mob_and_its_damage() {
        let mut p = Packet::new_empty();
        p.write_short(0xB0).unwrap();
        p.write_int(900).unwrap();
        p.write_int(1234).unwrap();
        p.write_byte(0x84).unwrap();
        p.write_byte(1).unwrap();
        p.write_bytes(&[0; 8]).unwrap();
        p.write_int(2_000_000_001).unwrap();
        p.write_int(100100).unwrap();
        p.write_byte(7).unwrap();
        p.write_bytes(&[0; 13]).unwrap();
        p.write_int(321).unwrap();
        p.write_int(0).unwrap();
        assert_eq!(
            parse_attack(&p).unwrap(),
            SummonAttack {
                object_id: 900,
                action: 0x84,
                hits: vec![SummonHit {
                    object_id: 2_000_000_001,
                    action: 7,
                    damage: 321,
                }],
            }
        );

        let mut p = Packet::new_empty();
        p.write_short(0xB1).unwrap();
        p.write_int(900).unwrap();
        p.write_byte(0xFF).unwrap();
        p.write_int(55).unwrap();
        p.write_int(100100).unwrap();
        p.write_byte(1).unwrap();
        let damage = parse_damage(&p).unwrap();
        assert_eq!(damage.attack, -1);
        assert_eq!(damage.damage, 55);
        assert_eq!(damage.mob, Some((100100, true)));
    }

    fn beholder(skill_id: i32, action: u8, buff: Option<u8>) -> Packet {
        let mut p = Packet::new_empty();
        p.write_short(0xB2).unwrap();
        p.write_int(900).unwrap();
        p.write_int(skill_id).unwrap();
        p.write_byte(action).unwrap();
        if let Some(buff) = buff {
            p.write_byte(buff).unwrap();
        }
        p
    }

    #[test]
    fn a_beholder_names_its_skill_and_hex_its_stat() {
        assert_eq!(
            parse_beholder(&beholder(AURA_OF_THE_BEHOLDER, 0x86, None)).unwrap(),
            BeholderAct {
                object_id: 900,
                skill_id: AURA_OF_THE_BEHOLDER,
                action: 0x86,
                buff: None,
            }
        );
        let hex = parse_beholder(&beholder(HEX_OF_THE_BEHOLDER, 11, Some(4))).unwrap();
        assert_eq!((hex.action, hex.buff), (11, Some(4)));
    }

    #[test]
    fn a_beholder_heals_and_hexes_by_what_its_owner_knows() {
        let aura = parse_beholder(&beholder(AURA_OF_THE_BEHOLDER, 6, None)).unwrap();
        let hex =
            |stat| parse_beholder(&beholder(HEX_OF_THE_BEHOLDER, 7 + stat, Some(stat))).unwrap();
        assert_eq!(support(&knowing(&[(AURA_OF_THE_BEHOLDER, 1)]), &aura), None);

        let player = knowing(&[
            (BEHOLDER, 1),
            (AURA_OF_THE_BEHOLDER, 1),
            (HEX_OF_THE_BEHOLDER, 1),
        ]);
        assert_eq!(
            support(&player, &aura),
            Some(Support::Heal {
                hp: 40,
                every: Duration::from_secs(10),
            })
        );
        // Level 1 Hex raises only weapon defense.
        assert_eq!(
            support(&player, &hex(0)),
            Some(Support::Hex {
                stat: BuffStat::Pdd,
                value: 20,
                lasts: Duration::from_secs(20),
                every: Duration::from_secs(48),
            })
        );
        assert_eq!(support(&player, &hex(4)), None);
        assert_eq!(support(&player, &hex(5)), None);

        let player = knowing(&[(BEHOLDER, 1), (HEX_OF_THE_BEHOLDER, 25)]);
        assert_eq!(support(&player, &aura), None);
        let stats: Vec<_> = (0..5)
            .map(|stat| match support(&player, &hex(stat)) {
                Some(Support::Hex { stat, value, .. }) => (stat, value),
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(
            stats,
            vec![
                (BuffStat::Pdd, 100),
                (BuffStat::Mdd, 100),
                (BuffStat::Acc, 25),
                (BuffStat::Eva, 50),
                (BuffStat::Pad, 15),
            ]
        );

        let mut dead = player;
        dead.stats.hp = 0;
        assert_eq!(support(&dead, &hex(0)), None);
    }
}
