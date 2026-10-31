//! Temporary skill effects, the diseases mobs inflict, and skill
//! cooldowns. They last only while the character is online and are never
//! saved.

use std::time::{Duration, Instant};

/// How often Recovery heals.
pub const RECOVERY_INTERVAL: Duration = Duration::from_secs(5);
/// How often Poison hurts.
pub const POISON_INTERVAL: Duration = Duration::from_secs(1);
/// How often Dragon Blood drinks HP.
pub const DRAGON_BLOOD_INTERVAL: Duration = Duration::from_secs(4);
/// The energy that fills Energy Charge's gauge (the v83 client's 10000).
pub const ENERGY_FULL: i16 = 10000;

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
    /// skill is the mob skill and its level, `skill | level << 16` (but
    /// Dragon Roar's stun of its user is under Dragon Roar).
    Stun = 17,
    /// Loses `value` HP every `POISON_INTERVAL`, down to 1.
    Poison = 18,
    /// No skills but Dispel.
    Seal = 19,
    Darkness = 20,
    Combo = 21,
    WeaponCharge = 22,
    /// Drinks `value` HP every `DRAGON_BLOOD_INTERVAL`.
    DragonBlood = 23,
    HolySymbol = 24,
    MesoUp = 25,
    ShadowPartner = 26,
    PickPocket = 27,
    MesoGuard = 28,
    /// Keeps `value` HP of each of a map's hazard hits off (an item's
    /// `thaw`): in the cold when positive, under water when negative.
    Thaw = 29,
    /// No jumping.
    Weakness = 30,
    /// Less EXP.
    Curse = 31,
    /// Walks at `value` speed.
    Slow = 32,
    /// Looks like the template `Morph.nx/<value>.img`. Below 100, an item's
    /// morph, the player can't attack (Cosmic's rule; the client refuses
    /// too).
    Morph = 33,
    /// Heals `value` HP every `RECOVERY_INTERVAL`.
    Recovery = 34,
    MapleWarrior = 35,
    Stance = 36,
    SharpEyes = 37,
    /// Mana Reflection, at its skill level (the client looks up its chance
    /// and share).
    ManaReflection = 38,
    /// Seduce: walks the way the mob wants, pattern `value`, and can't
    /// attack (the client's Attract).
    Seduce = 39,
    /// Shadow Stars: claw throws use the star `value` + 2069999 and spend
    /// none (the client's SpiritJavelin).
    ShadowStars = 40,
    Infinity = 41,
    /// Holy Shield: mobs' skills give no diseases, and their attacks only
    /// Stun and Seduce (Cosmic's rule). The client never reads it.
    HolyShield = 42,
    /// Hamstring and Blind arm the player's hits with their mob status
    /// while they last. The client only shows their icons.
    Hamstring = 43,
    Blind = 44,
    Concentrate = 45,
    /// A summon that stays where it was put: a Puppet or an Octopus. The
    /// client only shows its icon (Cosmic's PUPPET).
    Puppet = 46,
    /// Echo of Hero (the client's MaxLevelBuff): attack and magic rise by
    /// `value` percent of their whole.
    EchoOfHero = 47,
    /// A ghost: the client draws the body's `ghost*` stances and only the
    /// head's items, and lets it neither attack nor use skills (a ghost
    /// item's `ghost`; report c039).
    Ghost = 49,
    /// The client's Barrier: mobs that would take items from the player
    /// take none (a `barrier` item, the Ariant Coliseum's 2022269, as
    /// Cosmic's AURA has it).
    Barrier = 50,
    /// The arrow keys are swapped (the client's ReverseInput).
    Confuse = 51,
    /// The Dojo's Power Explosion (1011): the client's weapon hits are
    /// `value` percent and its spells twice, and it plays the skill's
    /// `repeat` loop (FUN_0078df87, FUN_00791617; others see only the loop).
    DojangBerserk = 59,
    /// No mob can hurt the player: the client dodges every hit
    /// (Invincibility, 1010).
    DivineBody = 60,
    /// The Aran's combo, the count of hits their polearm has landed: Combo
    /// Ability adds to attack, and Combo Critical to criticals, for every
    /// ten (0x77df48, report b360). Never shown to others.
    AranCombo = 68,
    /// Combo Drain (21100005): each weapon hit heals `value` percent of its
    /// damage, which the client leaves to the server.
    ComboDrain = 69,
    /// Combo Barrier (21120007): the player keeps `value` per mille of each
    /// hit after Achilles (report 193, step 18).
    ComboBarrier = 70,
    /// Body Pressure (21101003): a touching mob is hit by the body attack.
    BodyPressure = 71,
    /// Smart Knockback (21111001): only hits of `value` percent of a mob's
    /// `pushed` knock it back, which only the client draws.
    SmartKnockback = 72,
    /// An item's `berserk` (Pharaoh's Blessing, Power Buff): the client only
    /// draws `Effect/ItemEff.img/<item>` about the player, the item coming
    /// as `-skill_id`, and hides Power Explosion's loop meanwhile (report
    /// b339). The value changes nothing.
    ItemAura = 73,
    /// Kills give `value` percent of their EXP: Hunter's Luck (2450000).
    /// The client only shows its icon.
    ExpBuff = 74,
    /// No potions or other use items.
    StopPotion = 75,
    /// Standing still for two seconds costs `value` HP, never the last
    /// (the client takes it and reports it).
    StopMotion = 76,
    /// Darkness all around but a ring about the player.
    Fear = 77,
    /// Energy Charge's gauge (a two-state stat): `value` is the energy, full
    /// at `ENERGY_FULL`. It fills slowly and, once full, empties after the
    /// skill's `time`.
    EnergyCharge = 82,
    /// Dash's speed (a two-state stat): the player walks at their base
    /// speed plus the larger of `value` and their Speed buff.
    DashSpeed = 83,
    /// Dash's jump, as `DashSpeed` with the Jump buff.
    DashJump = 84,
    /// Riding the tamed monster worn at `equip_slot::MOUNT` (the client's
    /// RideVehicle, a two-state stat). Its value isn't used; the mount
    /// tires every `mount::TIRE_INTERVAL`.
    Riding = 85,
    /// Speed Infusion (the client's PartyBooster, a two-state stat): `value`
    /// adds to the player's attack speed, Booster's included.
    SpeedInfusion = 86,
    /// Homing Beacon's mark (the client's GuidedBullet, a two-state stat):
    /// the player's shots seek the mob in `Buffs::beacon`. Never shown to
    /// the map.
    GuidedBullet = 87,
    /// Items heal half as much HP. It shares Puppet's bit 46 in the client
    /// (as Cosmic's ZOMBIFY and PUPPET do); 128 + 46 keeps it apart here.
    Undead = 174,
}

impl BuffStat {
    /// Its bit in the client's flags.
    pub fn bit(self) -> u8 {
        self as u8 % 128
    }

    /// Stats the rest of the map is told about, in the order the client
    /// reads them (FUN_00788156); the others only matter to the player.
    pub const SHOWN: [BuffStat; 24] = [
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
        BuffStat::Morph,
        BuffStat::Ghost,
        BuffStat::Seduce,
        BuffStat::ShadowStars,
        BuffStat::Undead,
        BuffStat::Confuse,
        BuffStat::ItemAura,
        BuffStat::StopPotion,
        BuffStat::StopMotion,
        BuffStat::Fear,
        BuffStat::DojangBerserk,
        BuffStat::DivineBody,
    ];

    pub const DISEASES: [BuffStat; 13] = [
        BuffStat::Stun,
        BuffStat::Poison,
        BuffStat::Seal,
        BuffStat::Darkness,
        BuffStat::Weakness,
        BuffStat::Curse,
        BuffStat::Slow,
        BuffStat::Seduce,
        BuffStat::Confuse,
        BuffStat::Undead,
        BuffStat::StopPotion,
        BuffStat::StopMotion,
        BuffStat::Fear,
    ];

    pub fn disease(self) -> bool {
        Self::DISEASES.contains(&self)
    }

    /// How often the stat heals or hurts, if it does.
    fn interval(self) -> Option<Duration> {
        match self {
            BuffStat::Recovery => Some(RECOVERY_INTERVAL),
            BuffStat::Poison => Some(POISON_INTERVAL),
            BuffStat::DragonBlood => Some(DRAGON_BLOOD_INTERVAL),
            BuffStat::Riding => Some(crate::mount::TIRE_INTERVAL),
            _ => None,
        }
    }

    /// Whether the rest of the map sees it: one of `SHOWN`, or energy, a
    /// dash or riding, which they get in two-state blocks of their own.
    pub fn shown(self) -> bool {
        Self::SHOWN.contains(&self)
            || matches!(
                self,
                BuffStat::EnergyCharge
                    | BuffStat::DashSpeed
                    | BuffStat::DashJump
                    | BuffStat::Riding
            )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Buff {
    pub skill_id: i32,
    pub stat: BuffStat,
    pub value: i16,
    pub expires: Instant,
    /// When Recovery next heals, or Poison or Dragon Blood next hurts.
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
    /// HP that Dragon Blood drinks now.
    pub blood: i32,
    /// How many times the ridden mount tires now.
    pub tire: i32,
}

#[derive(Clone, Debug, Default)]
pub struct Buffs {
    active: Vec<Buff>,
    /// (skill id, usable again at)
    cooldowns: Vec<(i32, Instant)>,
    /// When Dark Sight was last cancelled: Assassinate cancels it just
    /// before it strikes.
    pub dark_sight_cancelled: Option<Instant>,
    /// The mob Assassinate's second strike must hit, and by when.
    pub assassinate: Option<(i32, Instant)>,
    /// The mob Homing Beacon marked, until the player leaves the map.
    pub beacon: Option<Beacon>,
    /// What is left of the Battleship's HP. It stays damaged between rides
    /// and is whole again once the ship was destroyed (`None`).
    pub battleship_hp: Option<i32>,
}

/// A mob Homing Beacon or Bullseye marked: the skill and the mob's object
/// id (the client's GuidedBullet data).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Beacon {
    pub skill_id: i32,
    pub mob: i32,
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

    /// Energy Charge's gauge, while it has energy.
    pub fn energy(&self) -> Option<ShownBuff> {
        self.two_state(BuffStat::EnergyCharge)
    }

    /// A two-state stat, as others see it, while it lasts.
    pub fn two_state(&self, stat: BuffStat) -> Option<ShownBuff> {
        let buff = self.active.iter().find(|buff| buff.stat == stat)?;
        Some(ShownBuff {
            stat: buff.stat,
            value: buff.value,
            skill_id: buff.skill_id,
        })
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

    /// Heal or hurt for each Recovery, Poison or Dragon Blood interval that
    /// has passed, and tire a ridden mount, then end the buffs and
    /// cooldowns that ran out.
    pub fn advance(&mut self, now: Instant) -> BuffTick {
        let mut tick = BuffTick::default();
        for buff in &mut self.active {
            let Some(interval) = buff.stat.interval() else {
                continue;
            };
            while let Some(at) = buff.next_tick.filter(|&at| at <= now && at <= buff.expires) {
                match buff.stat {
                    BuffStat::Poison => tick.poison += i32::from(buff.value),
                    BuffStat::DragonBlood => tick.blood += i32::from(buff.value),
                    // Only a tamed monster tires: the ship wears down by its
                    // hits instead, and an event vehicle never does.
                    BuffStat::Riding => {
                        tick.tire += i32::from(crate::mount::RIDER_SKILLS.contains(&buff.skill_id))
                    }
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

    /// End every buff but the diseases and Energy Charge's gauge, as a mob's
    /// Dispel does. Cooldowns keep running.
    pub fn dispel(&mut self) -> Vec<BuffStat> {
        let mut ended = Vec::new();
        self.active.retain(|buff| {
            if buff.stat.disease() || buff.stat == BuffStat::EnergyCharge {
                return true;
            }
            ended.push(buff.stat);
            false
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
    fn dispel_leaves_the_diseases() {
        let start = Instant::now();
        let mut buffs = Buffs::default();
        buffs.give(Buff::new(1001, BuffStat::Recovery, 4, start, 30 * SECOND));
        buffs.give(Buff::new(126, BuffStat::Slow, 80, start, 30 * SECOND));
        buffs.give(Buff::new(1002, BuffStat::Speed, 20, start, 30 * SECOND));
        assert_eq!(buffs.dispel(), [BuffStat::Recovery, BuffStat::Speed]);
        assert!(buffs.has(BuffStat::Slow));
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
    fn dragon_blood_drinks_every_four_seconds() {
        let start = Instant::now();
        let mut buffs = Buffs::default();
        buffs.give(Buff::new(1311008, BuffStat::Pad, 12, start, 160 * SECOND));
        buffs.give(Buff::new(
            1311008,
            BuffStat::DragonBlood,
            20,
            start,
            160 * SECOND,
        ));
        assert_eq!(buffs.next_deadline(), Some(start + 4 * SECOND));
        assert_eq!(buffs.advance(start + 3 * SECOND).blood, 0);
        assert_eq!(buffs.advance(start + 8 * SECOND).blood, 40);
        assert_eq!(buffs.next_deadline(), Some(start + 12 * SECOND));
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
