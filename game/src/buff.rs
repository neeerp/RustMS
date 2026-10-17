//! Temporary skill effects, the diseases mobs inflict, and skill
//! cooldowns. They last only while the character is online and are never
//! saved.

use std::time::{Duration, Instant};

/// How often Recovery heals.
pub const RECOVERY_INTERVAL: Duration = Duration::from_secs(5);
/// How often Poison hurts.
pub const POISON_INTERVAL: Duration = Duration::from_secs(1);

/// A temporary stat, numbered by its bit in the v83 client's 128-bit
/// temporary-stat flags (built as `1 << n` at startup, e.g. 0x78d9ad).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum BuffStat {
    /// Weapon attack, weapon defense, magic attack, magic defense,
    /// accuracy, and avoidability.
    Pad = 0,
    Pdd = 1,
    Mad = 2,
    Mdd = 3,
    Acc = 4,
    Eva = 5,
    Speed = 7,
    Jump = 8,
    MagicGuard = 9,
    DarkSight = 10,
    Booster = 11,
    PowerGuard = 12,
    MaxHp = 13,
    MaxMp = 14,
    Invincible = 15,
    SoulArrow = 16,
    /// The diseases a mob skill (or attack) inflicts, from `Stun` on: their
    /// skill is the mob skill and its level, `skill | level << 16`.
    Stun = 17,
    /// Loses `value` HP every `POISON_INTERVAL`, down to 1.
    Poison = 18,
    /// No skills but Dispel.
    Seal = 19,
    Darkness = 20,
    Combo = 21,
    WeaponCharge = 22,
    DragonBlood = 23,
    HolySymbol = 24,
    MesoUp = 25,
    ShadowPartner = 26,
    PickPocket = 27,
    MesoGuard = 28,
    /// No jumping.
    Weakness = 30,
    /// Less EXP.
    Curse = 31,
    /// Walks at `value` speed.
    Slow = 32,
    /// Heals `value` HP every `RECOVERY_INTERVAL`.
    Recovery = 34,
    MapleWarrior = 35,
    Stance = 36,
    SharpEyes = 37,
    /// Seduce: walks the way the mob wants, pattern `value`, and can't
    /// attack (the client's Attract).
    Seduce = 39,
    Infinity = 41,
    Concentrate = 45,
    /// A summon that stays where it was put: a Puppet or an Octopus. The
    /// client only shows its icon (Cosmic's PUPPET).
    Puppet = 46,
    /// The arrow keys are swapped (the client's ReverseInput).
    Confuse = 51,
}

impl BuffStat {
    /// Its bit in the client's flags.
    pub fn bit(self) -> u8 {
        self as u8
    }

    /// Stats the rest of the map is told about, in the order the client
    /// reads them (FUN_00788156); the others only matter to the player.
    pub const SHOWN: [BuffStat; 14] = [
        BuffStat::Speed,
        BuffStat::Combo,
        BuffStat::WeaponCharge,
        BuffStat::Stun,
        BuffStat::Darkness,
        BuffStat::Seal,
        BuffStat::Weakness,
        BuffStat::Curse,
        BuffStat::Poison,
        BuffStat::ShadowPartner,
        BuffStat::DarkSight,
        BuffStat::SoulArrow,
        BuffStat::Seduce,
        BuffStat::Confuse,
    ];

    pub const DISEASES: [BuffStat; 9] = [
        BuffStat::Stun,
        BuffStat::Poison,
        BuffStat::Seal,
        BuffStat::Darkness,
        BuffStat::Weakness,
        BuffStat::Curse,
        BuffStat::Slow,
        BuffStat::Seduce,
        BuffStat::Confuse,
    ];

    pub fn disease(self) -> bool {
        Self::DISEASES.contains(&self)
    }

    /// How often the stat heals or hurts, if it does.
    fn interval(self) -> Option<Duration> {
        match self {
            BuffStat::Recovery => Some(RECOVERY_INTERVAL),
            BuffStat::Poison => Some(POISON_INTERVAL),
            _ => None,
        }
    }

    pub fn shown(self) -> bool {
        Self::SHOWN.contains(&self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Buff {
    pub skill_id: i32,
    pub stat: BuffStat,
    pub value: i16,
    pub expires: Instant,
    /// When Recovery next heals, or Poison next hurts.
    next_tick: Option<Instant>,
}

/// A disease a mob skill inflicts on a player: its stat and value, the mob
/// skill and level (`skill | level << 16`), and how long it lasts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Disease {
    pub stat: BuffStat,
    pub value: i16,
    pub skill_id: i32,
    pub lasts: Duration,
}

/// A buff others see, with what they're told about it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShownBuff {
    pub stat: BuffStat,
    pub value: i16,
    /// Its skill: for a weapon charge, the element; for a disease, the mob
    /// skill.
    pub skill_id: i32,
}

impl Buff {
    pub fn new(
        skill_id: i32,
        stat: BuffStat,
        value: i16,
        now: Instant,
        duration: Duration,
    ) -> Self {
        Self {
            skill_id,
            stat,
            value,
            expires: now + duration,
            next_tick: stat.interval().map(|interval| now + interval),
        }
    }

    fn next_deadline(&self) -> Instant {
        self.next_tick
            .map_or(self.expires, |at| at.min(self.expires))
    }
}

/// What happened when the buffs were brought up to date.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct BuffTick {
    pub expired: Vec<BuffStat>,
    /// Skills whose cooldowns ran out. The client keeps a skill on cooldown
    /// until it is told.
    pub cooled: Vec<i32>,
    /// HP that Recovery heals now.
    pub heal: i32,
    /// HP that Poison takes now.
    pub poison: i32,
}

#[derive(Clone, Debug, Default)]
pub struct Buffs {
    active: Vec<Buff>,
    /// (skill id, usable again at)
    cooldowns: Vec<(i32, Instant)>,
}

impl Buffs {
    /// Start a buff, replacing one on the same stat.
    pub fn give(&mut self, buff: Buff) {
        self.active.retain(|b| b.stat != buff.stat);
        self.active.push(buff);
    }

    pub fn active(&self) -> &[Buff] {
        &self.active
    }

    /// The running buffs the rest of the map sees, in the client's order.
    pub fn shown(&self) -> Vec<ShownBuff> {
        BuffStat::SHOWN
            .iter()
            .filter_map(|&stat| {
                let buff = self.active.iter().find(|buff| buff.stat == stat)?;
                Some(ShownBuff {
                    stat,
                    value: buff.value,
                    skill_id: buff.skill_id,
                })
            })
            .collect()
    }

    pub fn has(&self, stat: BuffStat) -> bool {
        self.active.iter().any(|buff| buff.stat == stat)
    }

    /// The value of a running buff on this stat.
    pub fn value(&self, stat: BuffStat) -> Option<i16> {
        self.active
            .iter()
            .find(|buff| buff.stat == stat)
            .map(|buff| buff.value)
    }

    pub fn cooling_down(&self, skill_id: i32, now: Instant) -> bool {
        self.cooldowns
            .iter()
            .any(|&(id, until)| id == skill_id && until > now)
    }

    pub fn start_cooldown(&mut self, skill_id: i32, now: Instant, length: Duration) {
        self.cooldowns.retain(|&(id, _)| id != skill_id);
        self.cooldowns.push((skill_id, now + length));
    }

    /// End every cooldown but `kept`'s, as Time Leap does: the skills that
    /// were cooling down.
    pub fn end_cooldowns_except(&mut self, kept: i32) -> Vec<i32> {
        let mut ended = Vec::new();
        self.cooldowns.retain(|&(id, _)| {
            if id == kept {
                return true;
            }
            ended.push(id);
            false
        });
        ended
    }

    /// When `advance` next has something to do.
    pub fn next_deadline(&self) -> Option<Instant> {
        let buffs = self.active.iter().map(Buff::next_deadline);
        let cooldowns = self.cooldowns.iter().map(|&(_, until)| until);
        buffs.chain(cooldowns).min()
    }

    /// Heal or hurt for each Recovery or Poison interval that has passed,
    /// then end the buffs and cooldowns that ran out.
    pub fn advance(&mut self, now: Instant) -> BuffTick {
        let mut tick = BuffTick::default();
        for buff in &mut self.active {
            let Some(interval) = buff.stat.interval() else {
                continue;
            };
            while let Some(at) = buff.next_tick.filter(|&at| at <= now && at <= buff.expires) {
                match buff.stat {
                    BuffStat::Poison => tick.poison += i32::from(buff.value),
                    _ => tick.heal += i32::from(buff.value),
                }
                buff.next_tick = Some(at + interval);
            }
        }
        self.active.retain(|buff| {
            let running = buff.expires > now;
            if !running {
                tick.expired.push(buff.stat);
            }
            running
        });
        self.cooldowns.retain(|&(id, until)| {
            let running = until > now;
            if !running {
                tick.cooled.push(id);
            }
            running
        });
        tick
    }

    /// End the buffs a skill gave, e.g. when the player right-clicks its
    /// icon. Its cooldown keeps running. Diseases aren't the player's to
    /// end.
    pub fn cancel(&mut self, skill_id: i32) -> Vec<BuffStat> {
        let mut ended = Vec::new();
        self.active.retain(|buff| {
            let cancelled = buff.skill_id == skill_id && !buff.stat.disease();
            if cancelled {
                ended.push(buff.stat);
            }
            !cancelled
        });
        ended
    }

    /// End the diseases among `stats`, as Dispel or a cure potion does.
    pub fn cure(&mut self, stats: &[BuffStat]) -> Vec<BuffStat> {
        let mut ended = Vec::new();
        self.active.retain(|buff| {
            let cured = buff.stat.disease() && stats.contains(&buff.stat);
            if cured {
                ended.push(buff.stat);
            }
            !cured
        });
        ended
    }

    /// End every buff and disease, e.g. on death. Cooldowns keep running.
    pub fn clear(&mut self) -> Vec<BuffStat> {
        self.active.drain(..).map(|buff| buff.stat).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: Duration = Duration::from_secs(1);

    #[test]
    fn recovery_heals_every_five_seconds_until_it_ends() {
        let start = Instant::now();
        let mut buffs = Buffs::default();
        buffs.give(Buff::new(1001, BuffStat::Recovery, 4, start, 12 * SECOND));
        assert_eq!(buffs.next_deadline(), Some(start + 5 * SECOND));
        assert_eq!(buffs.advance(start + 4 * SECOND), BuffTick::default());
        assert_eq!(buffs.advance(start + 5 * SECOND).heal, 4);
        assert_eq!(buffs.next_deadline(), Some(start + 10 * SECOND));
        // Missed ticks are caught up; it ends before the third heal.
        let tick = buffs.advance(start + 13 * SECOND);
        assert_eq!((tick.heal, tick.expired), (4, vec![BuffStat::Recovery]));
        assert_eq!(buffs.next_deadline(), None);
    }

    #[test]
    fn a_new_buff_replaces_one_on_the_same_stat() {
        let start = Instant::now();
        let mut buffs = Buffs::default();
        buffs.give(Buff::new(1002, BuffStat::Speed, 10, start, 4 * SECOND));
        buffs.give(Buff::new(1001, BuffStat::Recovery, 4, start, 30 * SECOND));
        buffs.give(Buff::new(1002, BuffStat::Speed, 20, start, 12 * SECOND));
        assert_eq!(buffs.active().len(), 2);
        assert_eq!(buffs.next_deadline(), Some(start + 5 * SECOND));
        assert!(buffs.advance(start + 4 * SECOND).expired.is_empty());
        assert_eq!(buffs.clear(), vec![BuffStat::Recovery, BuffStat::Speed]);
    }

    #[test]
    fn cancelling_a_skill_ends_only_its_buffs() {
        let start = Instant::now();
        let mut buffs = Buffs::default();
        buffs.give(Buff::new(1001, BuffStat::Recovery, 4, start, 30 * SECOND));
        buffs.give(Buff::new(1002, BuffStat::Speed, 20, start, 12 * SECOND));
        buffs.start_cooldown(1002, start, 60 * SECOND);
        assert_eq!(buffs.cancel(1002), vec![BuffStat::Speed]);
        assert_eq!(buffs.cancel(1002), vec![]);
        assert_eq!(buffs.active()[0].stat, BuffStat::Recovery);
        assert!(buffs.cooling_down(1002, start + SECOND));
    }

    #[test]
    fn poison_hurts_every_second_and_only_a_cure_ends_a_disease_early() {
        let start = Instant::now();
        let mut buffs = Buffs::default();
        let reason = 125 | 1 << 16;
        buffs.give(Buff::new(reason, BuffStat::Poison, 15, start, 3 * SECOND));
        buffs.give(Buff::new(reason, BuffStat::Stun, 1, start, 10 * SECOND));
        buffs.give(Buff::new(1002, BuffStat::Speed, 20, start, 12 * SECOND));
        assert_eq!(buffs.next_deadline(), Some(start + SECOND));
        let tick = buffs.advance(start + 2 * SECOND);
        assert_eq!((tick.poison, tick.heal), (30, 0));
        assert!(buffs.cancel(reason).is_empty());
        let tick = buffs.advance(start + 3 * SECOND);
        assert_eq!((tick.poison, tick.expired), (15, vec![BuffStat::Poison]));
        assert!(buffs.cure(&[BuffStat::Speed, BuffStat::Poison]).is_empty());
        assert_eq!(buffs.cure(&BuffStat::DISEASES), vec![BuffStat::Stun]);
        assert!(buffs.has(BuffStat::Speed) && !buffs.has(BuffStat::Stun));
    }

    #[test]
    fn cooldowns_run_out() {
        let start = Instant::now();
        let mut buffs = Buffs::default();
        buffs.start_cooldown(1002, start, 60 * SECOND);
        assert!(buffs.cooling_down(1002, start + 59 * SECOND));
        assert!(!buffs.cooling_down(1001, start));
        assert!(!buffs.cooling_down(1002, start + 60 * SECOND));
    }

    #[test]
    fn advancing_reports_each_cooldown_that_ran_out() {
        let start = Instant::now();
        let mut buffs = Buffs::default();
        buffs.start_cooldown(1002, start, 60 * SECOND);
        buffs.start_cooldown(1001003, start, 30 * SECOND);
        assert_eq!(buffs.next_deadline(), Some(start + 30 * SECOND));
        assert!(buffs.advance(start + 29 * SECOND).cooled.is_empty());
        // Another skill's cooldown starting doesn't drop one that ran out.
        buffs.start_cooldown(1001, start + 31 * SECOND, 60 * SECOND);
        assert_eq!(buffs.advance(start + 31 * SECOND).cooled, vec![1001003]);
        assert_eq!(buffs.next_deadline(), Some(start + 60 * SECOND));
        assert_eq!(buffs.advance(start + 60 * SECOND).cooled, vec![1002]);
        // Death ends buffs but not cooldowns.
        assert!(buffs.clear().is_empty());
        assert_eq!(buffs.next_deadline(), Some(start + 91 * SECOND));
    }

    #[test]
    fn time_leap_ends_every_other_cooldown() {
        let start = Instant::now();
        let mut buffs = Buffs::default();
        buffs.start_cooldown(1002, start, 60 * SECOND);
        buffs.start_cooldown(5121010, start, 600 * SECOND);
        buffs.start_cooldown(1001003, start, 30 * SECOND);
        assert_eq!(buffs.end_cooldowns_except(5121010), vec![1002, 1001003]);
        assert!(!buffs.cooling_down(1002, start));
        assert!(buffs.cooling_down(5121010, start));
    }
}
