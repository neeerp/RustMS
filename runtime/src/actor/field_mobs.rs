//! The mobs of one field: spawn points, live mobs, who controls each, and
//! the damage dealt to them. Pure state; `FieldActor` sends the packets.

use crate::actor::object_ids::{self, ObjectIds};
use game::mob_skill::{self, MobSkillEffect};
use game::MobStat;
use game_data::mobs::MobInfo;
use game_data::skills::MobSkillLevel;
use game_data::MapMobTemplate;
use net::packet::build::world::field::parse_movement;
use net::packet::build::world::mob::{MobStatusView, MobView};
use net::ClientId;
use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap};
use tokio::time::{Duration, Instant};

/// The soonest a killed mob comes back, when its spawn point has no
/// `mobTime` of its own.
pub const RESPAWN_DELAY: Duration = Duration::from_secs(7);

/// A v83 client chases its target for 180 to 240 s after being given a mob
/// with aggro (FUN_007f73d8). Within the shortest of those, another hit by
/// the same player needs no new aggro.
pub const AGGRO_DURATION: Duration = Duration::from_secs(180);

pub struct SpawnPoint {
    template: MapMobTemplate,
    info: MobInfo,
    alive: Option<i32>,
    next_spawn: Option<Instant>,
}

pub struct LiveMob {
    pub object_id: i32,
    /// Its spawn point, or none for a summoned mob, which never comes back.
    pub spawn: Option<usize>,
    /// The mob that summoned it.
    pub summoner: Option<i32>,
    pub info: MobInfo,
    pub x: i16,
    pub y: i16,
    pub stance: u8,
    pub foothold: i16,
    pub home_foothold: i16,
    pub hp: i64,
    /// Its MP, which its attacks cost. It doesn't come back: a v83 client
    /// never regains it either, and when the server should is unknown.
    pub mp: i32,
    pub controller: Option<ClientId>,
    /// Until when the controller's client is chasing its player with it.
    pub aggro_until: Option<Instant>,
    /// How far the controller last said it was from this firstAttack mob.
    pub controller_distance: Option<i32>,
    /// Damage per player, for sharing EXP.
    pub damage_by: HashMap<ClientId, i64>,
    pub statuses: BTreeMap<MobStat, MobStatus>,
    /// The skill its controller may cast with it next.
    pub granted: Option<Grant>,
    /// When each skill it cast may be cast again.
    pub cooldowns: HashMap<i32, Instant>,
    /// When it leaves on its own (`removeAfter`), if it does.
    pub leaves_at: Option<Instant>,
}

/// A mob skill and level the server let a mob cast, and when.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grant {
    pub skill: u8,
    pub level: u8,
    pub at: Instant,
}

/// How long a grant waits for its cast before the server may grant another
/// in its place: the client may never cast it, being stunned, say.
pub const GRANT_TIMEOUT: Duration = Duration::from_secs(10);

/// The longest a cast's animation may hold off what it does.
pub const MOST_CAST_DELAY: Duration = Duration::from_secs(3);

/// A skill a mob cast, to take effect at `at`, once its animation is done.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cast {
    pub object_id: i32,
    pub skill: i32,
    pub level: i32,
    pub effect: MobSkillLevel,
    pub at: Instant,
}

/// The box a mob skill covers, edges included.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CastArea {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl CastArea {
    pub fn contains(&self, (x, y): (i16, i16)) -> bool {
        let (x, y) = (i32::from(x), i32::from(y));
        self.left <= x && x <= self.right && self.top <= y && y <= self.bottom
    }
}

/// A status on a mob: its value, the skill that caused it, who put it on,
/// and when it wears off.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MobStatus {
    pub value: i16,
    pub skill_id: i32,
    pub from: ClientId,
    pub until: Instant,
    /// When a status that hurts the mob next does.
    pub next_hurt: Instant,
    /// A counter's spread (`MobStatusView::spread`), 0 for the rest.
    pub spread: i32,
}

/// A mob's attackN is action 12 + N - 1, sent as action << 1 | facing left
/// (FUN_0066a517); its skills follow at 21.
pub const FIRST_ATTACK_ACTION: u8 = 12;
pub const FIRST_SKILL_ACTION: u8 = 21;
/// No action.
pub const NO_ACTION: u8 = 0xFF;

/// What a controller's move of a mob let it do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MobAct {
    /// The action to show the others.
    pub action: u8,
    /// Whether it may attack on its next move.
    pub next_attack: bool,
    pub mp: i16,
    /// The skill granted with this move, if any.
    pub grant: Option<(u8, u8)>,
    /// The skill it cast, if the server allowed it.
    pub cast: Option<Cast>,
}

/// How often a status that hurts a mob does: every client shows it once a
/// second from when it was put on (0x6675a8).
pub const HURT_INTERVAL: Duration = Duration::from_secs(1);

/// A mob hurt by a status, the player who put it on, and the mob's HP
/// percent after.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hurt {
    pub object_id: i32,
    pub from: ClientId,
    pub percent: u8,
}

impl LiveMob {
    fn new(
        object_id: i32,
        info: MobInfo,
        at: (i16, i16),
        stance: u8,
        foothold: i16,
        now: Instant,
    ) -> Self {
        let leaves_at =
            (info.remove_after > 0).then(|| now + Duration::from_secs(info.remove_after as u64));
        Self {
            object_id,
            spawn: None,
            summoner: None,
            x: at.0,
            y: at.1,
            stance,
            foothold,
            home_foothold: foothold,
            hp: i64::from(info.max_hp),
            mp: info.max_mp,
            info,
            controller: None,
            aggro_until: None,
            controller_distance: None,
            damage_by: HashMap::new(),
            statuses: BTreeMap::new(),
            granted: None,
            cooldowns: HashMap::new(),
            leaves_at,
        }
    }

    /// Mob stances are even facing right, odd facing left.
    pub fn facing_left(&self) -> bool {
        self.stance & 1 == 1
    }

    /// Whether the mob can't use its skills: a player's Seal or its own
    /// SealSkill.
    fn sealed(&self) -> bool {
        self.statuses.contains_key(&MobStat::Seal)
            || self.statuses.contains_key(&MobStat::SealSkill)
    }

    /// Validate a cast of the skill in `skill_data` (id, level << 8, the
    /// animation's length << 16) with `action`: it must be the one granted,
    /// listed with that action. Pays for it and starts its cooldown.
    fn cast(
        &mut self,
        action: u8,
        skill_data: i32,
        now: Instant,
        levels: &dyn Fn(i32, i32) -> Option<MobSkillLevel>,
    ) -> Option<Cast> {
        let grant = self.granted?;
        if self.sealed() {
            return None;
        }
        let (skill, level) = (skill_data as u8, (skill_data >> 8) as u8);
        if (skill, level) != (grant.skill, grant.level) {
            return None;
        }
        let (skill, level) = (i32::from(skill), i32::from(level));
        self.info.skills.iter().find(|entry| {
            entry.skill == skill
                && entry.level == level
                && entry.action - 1 + i32::from(FIRST_SKILL_ACTION) == i32::from(action >> 1)
        })?;
        let effect = levels(skill, level)?;
        if effect.mp_cost > self.mp {
            return None;
        }
        self.mp -= effect.mp_cost;
        self.granted = None;
        self.cooldowns.insert(
            skill,
            now + Duration::from_secs(effect.interval.max(0) as u64),
        );
        let animation = Duration::from_millis(u64::from((skill_data >> 16) as u16));
        Some(Cast {
            object_id: self.object_id,
            skill,
            level,
            effect,
            at: now + animation.min(MOST_CAST_DELAY),
        })
    }

    /// Pick a skill for its controller to cast next, by `roll` in 0..1,
    /// from those it may: ones the server casts, that it can pay for, that
    /// are off cooldown, whose HP limit it is at or under, and, to summon,
    /// that have fewer than their limit of its `summons` alive. None while
    /// sealed or waiting on a grant.
    fn grant(
        &mut self,
        now: Instant,
        summons: usize,
        levels: &dyn Fn(i32, i32) -> Option<MobSkillLevel>,
        roll: f64,
    ) -> Option<(u8, u8)> {
        if self
            .granted
            .is_some_and(|grant| now < grant.at + GRANT_TIMEOUT)
        {
            return None;
        }
        self.granted = None;
        if self.sealed() {
            return None;
        }
        let max_hp = i64::from(self.info.max_hp.max(1));
        let hp_percent = self.hp * 100 / max_hp;
        let usable: Vec<(u8, u8)> = self
            .info
            .skills
            .iter()
            .filter(|entry| {
                let (Ok(_), Ok(_), Some(effect)) = (
                    u8::try_from(entry.skill),
                    u8::try_from(entry.level),
                    mob_skill::effect(entry.skill, 0),
                ) else {
                    return false;
                };
                let Some(level) = levels(entry.skill, entry.level) else {
                    return false;
                };
                level.mp_cost <= self.mp
                    && hp_percent <= i64::from(level.hp)
                    && self.cooldowns.get(&entry.skill).is_none_or(|&at| at <= now)
                    && (effect != MobSkillEffect::Summon
                        || (!level.summons.is_empty()
                            && (level.limit <= 0 || summons < level.limit as usize)))
            })
            .map(|entry| (entry.skill as u8, entry.level as u8))
            .collect();
        let index = ((roll * usable.len() as f64) as usize).min(usable.len().checked_sub(1)?);
        let (skill, level) = usable[index];
        self.granted = Some(Grant {
            skill,
            level,
            at: now,
        });
        Some((skill, level))
    }

    pub fn view(&self, fade_in: bool) -> MobView {
        MobView {
            object_id: self.object_id,
            mob_id: self.info.mob_id,
            x: self.x,
            y: self.y,
            stance: self.stance,
            foothold: self.foothold,
            home_foothold: self.home_foothold,
            fade_in,
            controlled: self.controller.is_some(),
            statuses: self.status_views(Instant::now()),
            summon_effect: None,
        }
    }

    /// The statuses on the mob, with the time each has left.
    pub fn status_views(&self, now: Instant) -> Vec<MobStatusView> {
        self.statuses
            .iter()
            .map(|(&stat, status)| MobStatusView {
                stat,
                value: status.value,
                skill_id: status.skill_id,
                left: status.until.saturating_duration_since(now),
                spread: status.spread,
            })
            .collect()
    }

    pub fn hp_percent(&self) -> u8 {
        let max = i64::from(self.info.max_hp.max(1));
        ((self.hp.max(0) * 100 + max - 1) / max).clamp(0, 100) as u8
    }
}

/// A mob killed by an attack. The field splits its EXP (`split_exp`),
/// since only the field knows who is in which party.
#[derive(Debug, PartialEq, Eq)]
pub struct Kill {
    pub object_id: i32,
    pub mob_id: i32,
    pub x: i16,
    pub y: i16,
    /// Raised by Taunt's Showdown, if it had it.
    pub exp: i32,
    pub level: i16,
    /// Damage per player still on the field, most first.
    pub damage_by: Vec<(ClientId, i64)>,
    /// Taunt's Showdown: the percent more EXP (already in `exp`) and drops
    /// it gives.
    pub showdown: i32,
}

pub struct MobSet {
    spawns: Vec<SpawnPoint>,
    live: BTreeMap<i32, LiveMob>,
    object_ids: ObjectIds,
    revivals: Vec<Revival>,
}

/// Mobs to come out of one that died (`revive`) once it is done dying,
/// where it died, facing its way, under its controller.
struct Revival {
    at: Instant,
    mob_ids: Vec<i32>,
    x: i16,
    y: i16,
    stance: u8,
    foothold: i16,
    controller: Option<ClientId>,
}

impl Default for MobSet {
    fn default() -> Self {
        Self::new([])
    }
}

impl MobSet {
    pub fn new(spawns: impl IntoIterator<Item = (MapMobTemplate, MobInfo)>) -> Self {
        Self {
            spawns: spawns
                .into_iter()
                .filter(|(template, _)| template.mob_time >= 0 && !template.hide)
                .map(|(template, info)| SpawnPoint {
                    template,
                    info,
                    alive: None,
                    next_spawn: None,
                })
                .collect(),
            live: BTreeMap::new(),
            object_ids: ObjectIds::new(object_ids::MOBS),
            revivals: Vec::new(),
        }
    }

    pub fn live(&self) -> impl Iterator<Item = &LiveMob> {
        self.live.values()
    }

    pub fn get(&self, object_id: i32) -> Option<&LiveMob> {
        self.live.get(&object_id)
    }

    /// Spawn every spawn point that is empty and due; returns the new mobs'
    /// object ids.
    pub fn respawn(&mut self, now: Instant) -> Vec<i32> {
        let mut spawned = Vec::new();
        for (index, spawn) in self.spawns.iter_mut().enumerate() {
            if spawn.alive.is_some() || spawn.next_spawn.is_some_and(|at| at > now) {
                continue;
            }
            let object_id = self.object_ids.next();
            spawn.alive = Some(object_id);
            spawn.next_spawn = None;
            let template = &spawn.template;
            let mut mob = LiveMob::new(
                object_id,
                spawn.info.clone(),
                (template.x, template.y),
                // Mob stances are 2 (moving) or 4 (standing), +1 facing left;
                // `flip` faces right.
                if template.flip { 4 } else { 5 },
                template.foothold,
                now,
            );
            mob.spawn = Some(index);
            self.live.insert(object_id, mob);
            spawned.push(object_id);
        }
        spawned
    }

    /// Give uncontrolled mobs to `candidate`; returns the ones it now controls.
    pub fn assign_uncontrolled(&mut self, candidate: ClientId) -> Vec<i32> {
        self.live
            .values_mut()
            .filter(|mob| mob.controller.is_none())
            .map(|mob| {
                mob.controller = Some(candidate);
                mob.controller_distance = None;
                mob.granted = None;
                mob.object_id
            })
            .collect()
    }

    /// `leaving` stopped controlling its mobs; hand them to `next` if any.
    /// Returns the mobs that changed hands.
    pub fn release(&mut self, leaving: ClientId, next: Option<ClientId>) -> Vec<i32> {
        for revival in &mut self.revivals {
            if revival.controller == Some(leaving) {
                revival.controller = next;
            }
        }
        self.live
            .values_mut()
            .filter(|mob| mob.controller == Some(leaving))
            .map(|mob| {
                mob.controller = next;
                mob.granted = None;
                mob.aggro_until = None;
                mob.controller_distance = None;
                mob.damage_by.remove(&leaving);
                mob.object_id
            })
            .collect()
    }

    /// `attacker` hit a mob that is still alive. As in v83 servers, it takes
    /// control with aggro so its client chases it, unless it already has
    /// aggro over the mob. Returns the previous controller when control
    /// must be sent.
    pub fn provoke(
        &mut self,
        attacker: ClientId,
        object_id: i32,
        now: Instant,
    ) -> Option<Option<ClientId>> {
        let mob = self.live.get_mut(&object_id)?;
        if mob.controller == Some(attacker) && mob.aggro_until.is_some_and(|until| until > now) {
            return None;
        }
        let previous = mob.controller.replace(attacker);
        if previous != Some(attacker) {
            mob.granted = None;
        }
        mob.aggro_until = Some(now + AGGRO_DURATION);
        mob.controller_distance = None;
        Some(previous)
    }

    /// `from` is `distance` from a firstAttack mob (|dx| / 10 + |dy| / 3,
    /// 100 more while dead). Unless a chase is on, the closest player
    /// controls it, so their client starts one once they come near. Returns
    /// the previous controller when control moves to `from`.
    pub fn report_distance(
        &mut self,
        from: ClientId,
        object_id: i32,
        distance: i32,
        now: Instant,
    ) -> Option<Option<ClientId>> {
        let mob = self.live.get_mut(&object_id)?;
        if !mob.info.first_attack || mob.aggro_until.is_some_and(|until| until > now) {
            return None;
        }
        if mob.controller == Some(from) {
            mob.controller_distance = Some(distance);
            return None;
        }
        if mob
            .controller_distance
            .is_some_and(|nearest| nearest <= distance)
        {
            return None;
        }
        let previous = mob.controller.replace(from);
        mob.granted = None;
        mob.aggro_until = None;
        mob.controller_distance = Some(distance);
        Some(previous)
    }

    /// Apply the controller's movement, a path from `start`. `false` if
    /// `from` doesn't control it.
    pub fn move_mob(
        &mut self,
        from: ClientId,
        object_id: i32,
        start: (i16, i16),
        movement: &[u8],
    ) -> bool {
        let Some(mob) = self.live.get_mut(&object_id) else {
            return false;
        };
        if mob.controller != Some(from) {
            return false;
        }
        if let Some((x, y, stance, foothold)) =
            parse_movement(movement, (start.0, start.1, mob.stance))
        {
            mob.x = x;
            mob.y = y;
            mob.stance = stance;
            // Spawns for players who join later must name the platform the
            // mob stands on now, not its spawn point's.
            if let Some(foothold) = foothold {
                mob.foothold = foothold;
            }
        }
        true
    }

    /// The controller's mob used `action` (action << 1 | facing left, or
    /// [`NO_ACTION`]) as it moved, with `skill_data`, and its client wants
    /// it to attack next (`want_attack`). An attack costs the mob its MP;
    /// one it lacks or can't pay for is shown as no action, as is a skill
    /// cast without the server's grant (see [`LiveMob::cast`]). Wanting to
    /// attack may get the mob a skill to cast next, picked by `roll` in
    /// 0..1; `levels` looks up mob skill levels.
    pub fn act(
        &mut self,
        object_id: i32,
        (want_attack, action, skill_data): (bool, u8, i32),
        now: Instant,
        levels: &dyn Fn(i32, i32) -> Option<MobSkillLevel>,
        roll: f64,
    ) -> Option<MobAct> {
        let summons = self
            .live
            .values()
            .filter(|mob| mob.summoner == Some(object_id))
            .count();
        let mob = self.live.get_mut(&object_id)?;
        let mut shown = action;
        let mut cast = None;
        if action != NO_ACTION && action >> 1 >= FIRST_SKILL_ACTION {
            cast = mob.cast(action, skill_data, now, levels);
            if cast.is_none() {
                shown = NO_ACTION;
            }
        } else if action != NO_ACTION && action >> 1 >= FIRST_ATTACK_ACTION {
            let index = usize::from((action >> 1) - FIRST_ATTACK_ACTION);
            match mob.info.attacks.get(index) {
                Some(attack) if attack.mp_cost <= mob.mp => mob.mp -= attack.mp_cost,
                _ => shown = NO_ACTION,
            }
        }
        let grant = if want_attack {
            mob.grant(now, summons, levels, roll)
        } else {
            None
        };
        let attacks = !mob.info.not_attack && !mob.info.attacks.is_empty();
        Some(MobAct {
            action: shown,
            next_attack: want_attack && (attacks || mob.granted.is_some()),
            mp: i16::try_from(mob.mp).unwrap_or(i16::MAX),
            grant,
            cast,
        })
    }

    /// The mobs a cast reaches: its caster, or, with an area, every mob in
    /// it.
    pub fn reached_by(&self, cast: &Cast) -> Vec<i32> {
        if !self.live.contains_key(&cast.object_id) {
            return Vec::new();
        }
        let Some(area) = self.area(cast) else {
            return vec![cast.object_id];
        };
        self.live
            .values()
            .filter(|mob| mob.object_id == cast.object_id || area.contains((mob.x, mob.y)))
            .map(|mob| mob.object_id)
            .collect()
    }

    /// A cast's area around its caster, `lt` and `rb` as it faces left,
    /// mirrored facing right; None without one, or once the caster is gone.
    pub fn area(&self, cast: &Cast) -> Option<CastArea> {
        let caster = self.live.get(&cast.object_id)?;
        let effect = &cast.effect;
        if effect.lt == (0, 0) && effect.rb == (0, 0) {
            return None;
        }
        let (x, y) = (i32::from(caster.x), i32::from(caster.y));
        let (left, right) = if caster.facing_left() {
            (x + effect.lt.0, x + effect.rb.0)
        } else {
            (x - effect.rb.0, x - effect.lt.0)
        };
        Some(CastArea {
            left,
            top: y + effect.lt.1,
            right,
            bottom: y + effect.rb.1,
        })
    }

    /// A mob regains `hp` and `mp`, up to its maxima. Returns the HP it
    /// gained, its HP, and its max HP.
    pub fn heal(&mut self, object_id: i32, hp: i32, mp: i32) -> Option<(i32, i32, i32)> {
        let mob = self.live.get_mut(&object_id)?;
        let max_hp = i64::from(mob.info.max_hp);
        let healed = i64::from(hp.max(0)).min((max_hp - mob.hp).max(0));
        mob.hp += healed;
        mob.mp = (mob.mp + mp.max(0)).min(mob.info.max_mp.max(mob.mp));
        Some((healed as i32, mob.hp as i32, mob.info.max_hp))
    }

    /// The mobs `summoner` calls up: as many of `summoned` as its `limit`
    /// (0 for none) leaves room for, where it stands, facing its way, and
    /// under its controller. They never respawn.
    pub fn summon(
        &mut self,
        summoner: i32,
        summoned: &[MobInfo],
        limit: i32,
        now: Instant,
    ) -> Vec<i32> {
        let Some(caster) = self.live.get(&summoner) else {
            return Vec::new();
        };
        let (at, stance, foothold, controller) = (
            (caster.x, caster.y),
            caster.stance,
            caster.foothold,
            caster.controller,
        );
        let alive = self
            .live
            .values()
            .filter(|mob| mob.summoner == Some(summoner))
            .count();
        let room = if limit > 0 {
            (limit as usize).saturating_sub(alive)
        } else {
            summoned.len()
        };
        let mut spawned = Vec::new();
        for info in summoned.iter().take(room) {
            let object_id = self.object_ids.next();
            let mut mob = LiveMob::new(object_id, info.clone(), at, stance, foothold, now);
            mob.summoner = Some(summoner);
            mob.controller = controller;
            self.live.insert(object_id, mob);
            spawned.push(object_id);
        }
        spawned
    }

    /// Bring out the mobs due to come out of ones that died, as `info`
    /// has them; returns their object ids. They never respawn.
    pub fn revive(&mut self, now: Instant, info: impl Fn(i32) -> Option<MobInfo>) -> Vec<i32> {
        let (due, waiting): (Vec<_>, _) = std::mem::take(&mut self.revivals)
            .into_iter()
            .partition(|revival| revival.at <= now);
        self.revivals = waiting;
        let mut spawned = Vec::new();
        for revival in due {
            for info in revival.mob_ids.iter().filter_map(|&mob_id| info(mob_id)) {
                let object_id = self.object_ids.next();
                let at = (revival.x, revival.y);
                let mut mob =
                    LiveMob::new(object_id, info, at, revival.stance, revival.foothold, now);
                mob.controller = revival.controller;
                self.live.insert(object_id, mob);
                spawned.push(object_id);
            }
        }
        spawned
    }

    /// Put statuses caused by `skill_id` on a mob from `now` until `until`,
    /// replacing any it has, save the ones a boss shrugs off and those its
    /// element wards off. Returns those it got.
    pub fn add_statuses(
        &mut self,
        object_id: i32,
        statuses: &[(MobStat, i16)],
        skill_id: i32,
        from: ClientId,
        now: Instant,
        until: Instant,
    ) -> Vec<MobStat> {
        let Some(mob) = self.live.get_mut(&object_id) else {
            return Vec::new();
        };
        let info = &mob.info;
        let wards = |stat: MobStat| {
            (info.boss && stat.spares_bosses())
                || stat.element().is_some_and(|letter| info.immune_to(letter))
        };
        let statuses: Vec<_> = statuses
            .iter()
            .filter(|(stat, _)| !wards(*stat))
            .copied()
            .collect();
        statuses
            .into_iter()
            .map(|(stat, value)| {
                let status = MobStatus {
                    value,
                    skill_id,
                    from,
                    until,
                    next_hurt: now + HURT_INTERVAL,
                    spread: 0,
                };
                mob.statuses.insert(stat, status);
                stat
            })
            .collect()
    }

    /// Give the counters a mob has among `stats` their spread.
    pub fn spread_counters(&mut self, object_id: i32, stats: &[MobStat], spread: i32) {
        let Some(mob) = self.live.get_mut(&object_id) else {
            return;
        };
        for stat in stats {
            if let Some(status) = mob.statuses.get_mut(stat) {
                status.spread = spread;
            }
        }
    }

    /// What a mob's counter (`stat`: MCounter for a spell, PCounter for
    /// the rest) hurts its attacker with for one attack, or 0: like the v83
    /// client (FUN_007937e3), its value, then with a spread, plus `roll`
    /// modulo the spread less half of it.
    pub fn counter(&self, object_id: i32, stat: MobStat, roll: u32) -> i32 {
        let Some(status) = self
            .live
            .get(&object_id)
            .and_then(|mob| mob.statuses.get(&stat))
        else {
            return 0;
        };
        let value = i32::from(status.value);
        match u32::try_from(status.spread) {
            Ok(spread) if spread > 0 => value + (roll % spread) as i32 - status.spread / 2,
            _ => value,
        }
    }

    /// Take these statuses off a mob; returns the ones it had.
    pub fn remove_statuses(&mut self, object_id: i32, stats: &[MobStat]) -> Vec<MobStat> {
        let Some(mob) = self.live.get_mut(&object_id) else {
            return Vec::new();
        };
        stats
            .iter()
            .copied()
            .filter(|stat| mob.statuses.remove(stat).is_some())
            .collect()
    }

    /// Take off every status that has worn off by `now`: each mob's lost
    /// statuses.
    pub fn expire_statuses(&mut self, now: Instant) -> Vec<(i32, Vec<MobStat>)> {
        let mut expired = Vec::new();
        for mob in self.live.values_mut() {
            let lost: Vec<_> = mob
                .statuses
                .iter()
                .filter(|(_, status)| status.until <= now)
                .map(|(&stat, _)| stat)
                .collect();
            if lost.is_empty() {
                continue;
            }
            for stat in &lost {
                mob.statuses.remove(stat);
            }
            expired.push((mob.object_id, lost));
        }
        expired
    }

    /// Each second a mob has a status that hurts it, it loses the status's
    /// value in HP (its `fixedDamage` instead, if it has one), as its
    /// clients show, but never its last HP (inferred, as OdinMS-family
    /// servers do). The damage counts as the player's who put it on.
    pub fn hurt_by_statuses(&mut self, now: Instant) -> Vec<Hurt> {
        let mut hurt = Vec::new();
        for mob in self.live.values_mut() {
            for (_, status) in mob.statuses.iter_mut().filter(|(stat, _)| stat.hurts()) {
                if status.next_hurt > now {
                    continue;
                }
                status.next_hurt += HURT_INTERVAL;
                let damage = if mob.info.fixed_damage > 0 {
                    i64::from(mob.info.fixed_damage)
                } else {
                    i64::from(status.value)
                };
                let dealt = damage.clamp(0, (mob.hp - 1).max(0));
                if dealt == 0 {
                    continue;
                }
                mob.hp -= dealt;
                *mob.damage_by.entry(status.from).or_default() += dealt;
                hurt.push(Hurt {
                    object_id: mob.object_id,
                    from: status.from,
                    percent: 0,
                });
            }
            let percent = mob.hp_percent();
            for entry in hurt.iter_mut().filter(|h| h.object_id == mob.object_id) {
                entry.percent = percent;
            }
        }
        hurt
    }

    /// Damage a mob. Returns its HP percent, or the kill if it died.
    pub fn damage(
        &mut self,
        from: ClientId,
        object_id: i32,
        damage: i64,
        now: Instant,
    ) -> Option<Result<u8, Kill>> {
        let mob = self.live.get_mut(&object_id)?;
        let dealt = damage.clamp(0, mob.hp);
        mob.hp -= dealt;
        *mob.damage_by.entry(from).or_default() += dealt;
        if mob.hp > 0 {
            return Some(Ok(mob.hp_percent()));
        }
        let mob = self.live.remove(&object_id)?;
        self.emptied(mob.spawn, now);
        let showdown = mob
            .statuses
            .get(&MobStat::Showdown)
            .filter(|status| status.until > now)
            .map_or(0, |status| i32::from(status.value).max(0));
        if !mob.info.revive.is_empty() {
            self.revivals.push(Revival {
                at: now + Duration::from_millis(mob.info.dying_ms.max(0) as u64),
                mob_ids: mob.info.revive.clone(),
                x: mob.x,
                y: mob.y,
                stance: mob.stance,
                foothold: mob.foothold,
                controller: mob.controller,
            });
        }
        Some(Err(Kill {
            object_id,
            mob_id: mob.info.mob_id,
            x: mob.x,
            y: mob.y,
            exp: (i64::from(mob.info.exp) * i64::from(100 + showdown) / 100) as i32,
            level: mob.info.level,
            damage_by: ranked_damage(&mob.damage_by),
            showdown,
        }))
    }

    /// Take away the mobs whose time is up (`removeAfter`); returns their
    /// object ids. As in Cosmic, they give no EXP or drops and let nothing
    /// out, and their spawn points fill again as after a kill.
    pub fn leave(&mut self, now: Instant) -> Vec<i32> {
        let gone: Vec<i32> = self
            .live
            .values()
            .filter(|mob| mob.leaves_at.is_some_and(|at| at <= now))
            .map(|mob| mob.object_id)
            .collect();
        for object_id in &gone {
            if let Some(mob) = self.live.remove(object_id) {
                self.emptied(mob.spawn, now);
            }
        }
        gone
    }

    /// The mob from spawn point `spawn`, if it had one, is gone: it spawns
    /// again after its `mobTime`, at least `RESPAWN_DELAY`.
    fn emptied(&mut self, spawn: Option<usize>, now: Instant) {
        let Some(spawn) = spawn.map(|index| &mut self.spawns[index]) else {
            return;
        };
        spawn.alive = None;
        let delay = if spawn.template.mob_time > 0 {
            Duration::from_secs(spawn.template.mob_time as u64).max(RESPAWN_DELAY)
        } else {
            RESPAWN_DELAY
        };
        spawn.next_spawn = Some(now + delay);
    }
}

fn ranked_damage(damage_by: &HashMap<ClientId, i64>) -> Vec<(ClientId, i64)> {
    let mut ranked: Vec<_> = damage_by
        .iter()
        .filter(|(_, damage)| **damage > 0)
        .map(|(&client, &damage)| (client, damage))
        .collect();
    ranked.sort_by_key(|&(client, damage)| (Reverse(damage), client));
    ranked
}

/// Someone on the field when a mob dies, who may share its EXP.
#[derive(Clone, Copy, Debug)]
pub struct Sharer {
    pub client: ClientId,
    pub level: u8,
    pub party: Option<i32>,
}

/// One player's part of a kill.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpShare {
    pub client: ClientId,
    pub exp: i32,
    /// Extra EXP for hunting with a party, shown separately.
    pub party_bonus: i32,
}

/// Party members share a party's EXP when within this many levels of the
/// mob or of a member who hit it.
const LEECH_LEVELS: i32 = 5;
/// Party EXP is split 80% by level, plus 20% to the member who hit hardest.
const LEVEL_SHARE: f64 = 0.8;
const TOP_DAMAGE_SHARE: f64 = 0.2;
/// Each member sharing a kill adds 5% party bonus.
const PARTY_BONUS_PER_MEMBER: f64 = 0.05;

/// Split a mob's EXP the v83 way (as reconstructed by Cosmic): each solo
/// attacker, and each party taken together, earns EXP for its part of the
/// damage. A party's EXP goes to its members on the field by level, with an
/// extra 20% to whoever dealt it the most, plus 5% per sharing member.
pub fn split_exp(
    exp: i32,
    mob_level: i16,
    damage_by: &[(ClientId, i64)],
    sharers: &[Sharer],
) -> Vec<ExpShare> {
    let total: i64 = damage_by.iter().map(|(_, damage)| damage).sum();
    if total <= 0 {
        return Vec::new();
    }
    let per_damage = f64::from(exp) / total as f64;
    let round = |exp: f64| exp.round() as i32;

    let mut shares = Vec::new();
    let mut parties: BTreeMap<i32, Vec<(Sharer, i64)>> = BTreeMap::new();
    for &(client, damage) in damage_by {
        let Some(&sharer) = sharers.iter().find(|s| s.client == client) else {
            continue;
        };
        match sharer.party {
            Some(party) => parties.entry(party).or_default().push((sharer, damage)),
            None => shares.push(ExpShare {
                client,
                exp: round(damage as f64 * per_damage),
                party_bonus: 0,
            }),
        }
    }

    for (party, attackers) in parties {
        // `damage_by` is ranked, so the first attacker hit hardest.
        let top = attackers[0].0.client;
        let party_exp = attackers.iter().map(|(_, d)| *d).sum::<i64>() as f64 * per_damage;
        let in_range = |level: u8| {
            let near = |other: i32| (i32::from(level) - other).abs() <= LEECH_LEVELS;
            near(i32::from(mob_level)) || attackers.iter().any(|(a, _)| near(i32::from(a.level)))
        };
        let members: Vec<&Sharer> = sharers
            .iter()
            .filter(|s| s.party == Some(party) && in_range(s.level))
            .collect();
        let total_level = members
            .iter()
            .map(|s| f64::from(s.level))
            .sum::<f64>()
            .max(1.0);
        let bonus_rate = if members.len() > 1 {
            PARTY_BONUS_PER_MEMBER * members.len() as f64
        } else {
            0.0
        };
        for member in members {
            let mut ratio = LEVEL_SHARE * f64::from(member.level) / total_level;
            if member.client == top {
                ratio += TOP_DAMAGE_SHARE;
            }
            let exp = ratio * party_exp;
            shares.push(ExpShare {
                client: member.client,
                exp: round(exp),
                party_bonus: round(exp * bonus_rate),
            });
        }
    }
    shares
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A move with `action`, in a world without mob skills.
    fn act(mobs: &mut MobSet, object_id: i32, action: u8, want_attack: bool) -> MobAct {
        mobs.act(
            object_id,
            (want_attack, action, 0),
            Instant::now(),
            &|_, _| None,
            0.0,
        )
        .unwrap()
    }

    fn snail_spawn(x: i16) -> (MapMobTemplate, MobInfo) {
        (
            MapMobTemplate {
                mob_id: 100100,
                x,
                y: -25,
                foothold: 45,
                flip: false,
                mob_time: 0,
                hide: false,
            },
            MobInfo {
                mob_id: 100100,
                max_hp: 8,
                exp: 3,
                attack: 12,
                body_attack: true,
                level: 1,
                ..MobInfo::default()
            },
        )
    }

    #[test]
    fn attacks_cost_mp() {
        let now = Instant::now();
        let (template, mut info) = snail_spawn(100);
        info.max_mp = 3;
        info.attacks = vec![game_data::mobs::MobAttack {
            mp_cost: 2,
            ..Default::default()
        }];
        let mut mobs = MobSet::new([(template, info)]);
        let mob = mobs.respawn(now)[0];
        let attack1_left = FIRST_ATTACK_ACTION << 1 | 1;
        let walk = act(&mut mobs, mob, NO_ACTION, true);
        assert_eq!(
            (walk.action, walk.next_attack, walk.mp),
            (NO_ACTION, true, 3)
        );
        let first = act(&mut mobs, mob, attack1_left, false);
        assert_eq!(
            (first.action, first.next_attack, first.mp),
            (attack1_left, false, 1)
        );
        // Out of MP, and with no attack2: shown as no action.
        assert_eq!(act(&mut mobs, mob, attack1_left, true).action, NO_ACTION);
        assert_eq!(
            act(&mut mobs, mob, (FIRST_ATTACK_ACTION + 1) << 1, true).action,
            NO_ACTION
        );
        // Skills await the server's grant.
        assert_eq!(
            act(&mut mobs, mob, FIRST_SKILL_ACTION << 1, true).action,
            NO_ACTION
        );
        // A hit (action 6) is shown as it is.
        assert_eq!(act(&mut mobs, mob, 6 << 1, true).action, 6 << 1);
        // A mob without attacks never may.
        let mut snails = MobSet::new([snail_spawn(100)]);
        let snail = snails.respawn(now)[0];
        assert!(!act(&mut snails, snail, NO_ACTION, true).next_attack);
    }

    /// Stumpy's skills: Power Up, Heal, and a summon.
    fn stumpy() -> (MapMobTemplate, MobInfo) {
        let (template, mut info) = snail_spawn(100);
        info.max_hp = 1000;
        info.max_mp = 12;
        let entry = |skill, level, action| game_data::mobs::MobSkillEntry {
            skill,
            level,
            action,
            effect_after: 0,
        };
        info.skills = vec![entry(100, 1, 1), entry(114, 1, 2), entry(200, 1, 1)];
        (template, info)
    }

    fn stumpy_levels(skill: i32, level: i32) -> Option<MobSkillLevel> {
        let base = MobSkillLevel {
            mp_cost: 5,
            interval: 40,
            hp: 100,
            prop: 100,
            ..MobSkillLevel::default()
        };
        match (skill, level) {
            (100, 1) => Some(MobSkillLevel {
                x: 115,
                duration: 30,
                ..base
            }),
            (114, 1) => Some(MobSkillLevel {
                x: 700,
                hp: 50,
                lt: (-100, -100),
                rb: (50, 10),
                ..base
            }),
            (200, 1) => Some(MobSkillLevel {
                limit: 2,
                summons: vec![100100, 100100, 100100],
                ..base
            }),
            _ => None,
        }
    }

    #[test]
    fn skills_are_granted_then_cast() {
        let now = Instant::now();
        let mut mobs = MobSet::new([stumpy()]);
        let mob = mobs.respawn(now)[0];
        mobs.assign_uncontrolled(1);
        let act = |mobs: &mut MobSet, action, skill_data, want_attack, at, roll| {
            mobs.act(
                mob,
                (want_attack, action, skill_data),
                at,
                &stumpy_levels,
                roll,
            )
            .unwrap()
        };
        let power_up = FIRST_SKILL_ACTION << 1;
        // An ungranted cast is shown as nothing.
        let early = act(&mut mobs, power_up, 100 | 1 << 8, false, now, 0.0);
        assert_eq!((early.action, early.cast), (NO_ACTION, None));
        // At full HP Heal (under 50%) can't be granted: Power Up or the summon.
        let granted = act(&mut mobs, NO_ACTION, 0, true, now, 0.0);
        assert_eq!((granted.grant, granted.next_attack), (Some((100, 1)), true));
        // While it waits, nothing more is granted, but it may still attack.
        let waiting = act(&mut mobs, NO_ACTION, 0, true, now, 0.0);
        assert_eq!((waiting.grant, waiting.next_attack), (None, true));
        // Cast with the wrong action, or another skill: refused.
        let wrong = act(&mut mobs, power_up + 2, 100 | 1 << 8, false, now, 0.0);
        assert_eq!(wrong.action, NO_ACTION);
        let other = act(&mut mobs, power_up, 200 | 1 << 8, false, now, 0.0);
        assert_eq!(other.action, NO_ACTION);
        // The real cast pays 5 MP and acts once its 900 ms animation ends.
        let cast = act(
            &mut mobs,
            power_up | 1,
            100 | 1 << 8 | 900 << 16,
            false,
            now,
            0.0,
        );
        assert_eq!((cast.action, cast.mp), (power_up | 1, 7));
        let cast = cast.cast.unwrap();
        assert_eq!((cast.skill, cast.level, cast.effect.x), (100, 1, 115));
        assert_eq!(cast.at, now + Duration::from_millis(900));
        assert_eq!(mobs.reached_by(&cast), [mob]);
        // Power Up cools down for 40 s; the summon is next.
        let next = act(&mut mobs, NO_ACTION, 0, true, now, 0.0);
        assert_eq!(next.grant, Some((200, 1)));
        // A new controller starts afresh.
        mobs.release(1, Some(2));
        assert!(mobs.get(mob).unwrap().granted.is_none());
        // Summon twice: its limit is 2 alive.
        assert_eq!(
            act(&mut mobs, NO_ACTION, 0, true, now, 0.0).grant,
            Some((200, 1))
        );
        let summoned = mobs.summon(
            mob,
            &[snail_spawn(0).1, snail_spawn(0).1, snail_spawn(0).1],
            2,
            now,
        );
        assert_eq!(summoned.len(), 2);
        let summon = mobs.get(summoned[0]).unwrap();
        assert_eq!(
            (summon.summoner, summon.spawn, summon.controller),
            (Some(mob), None, Some(2))
        );
        assert_eq!((summon.x, summon.y), (100, -25));
        assert!(mobs.summon(mob, &[snail_spawn(0).1], 2, now).is_empty());
        // Hurt below half, only Heal remains while the summons live, and
        // only with MP for it.
        let later = now + GRANT_TIMEOUT;
        mobs.damage(1, mob, 600, later);
        assert_eq!(
            act(&mut mobs, NO_ACTION, 0, true, later, 0.99).grant,
            Some((114, 1))
        );
        mobs.live.get_mut(&mob).unwrap().mp = 4;
        let broke = later + GRANT_TIMEOUT;
        assert_eq!(act(&mut mobs, NO_ACTION, 0, true, broke, 0.0).grant, None);
        // A sealed mob casts nothing.
        mobs.live.get_mut(&mob).unwrap().mp = 12;
        mobs.add_statuses(
            mob,
            &[(MobStat::SealSkill, 1)],
            157,
            1,
            broke,
            broke + GRANT_TIMEOUT,
        );
        assert_eq!(act(&mut mobs, NO_ACTION, 0, true, broke, 0.0).grant, None);
        // So does one under a player's Seal.
        mobs.live.get_mut(&mob).unwrap().statuses.clear();
        mobs.add_statuses(
            mob,
            &[(MobStat::Seal, 1)],
            2111004,
            30,
            broke,
            broke + GRANT_TIMEOUT,
        );
        assert_eq!(act(&mut mobs, NO_ACTION, 0, true, broke, 0.0).grant, None);
        // Summons die for good.
        let Some(Err(_)) = mobs.damage(1, summoned[0], 100, later) else {
            panic!("the summon should die");
        };
        assert!(mobs.respawn(later + RESPAWN_DELAY * 10).is_empty());
    }

    #[test]
    fn area_skills_reach_mobs_the_way_the_caster_faces() {
        let now = Instant::now();
        let mut mobs = MobSet::new([stumpy(), snail_spawn(140), snail_spawn(40)]);
        let spawned = mobs.respawn(now);
        let (caster, ahead, behind) = (spawned[0], spawned[1], spawned[2]);
        let heal = Cast {
            object_id: caster,
            skill: 114,
            level: 1,
            effect: stumpy_levels(114, 1).unwrap(),
            at: now,
        };
        // Facing left (stance 5), lt.x -100 to rb.x 50 covers 0..150.
        assert_eq!(mobs.reached_by(&heal), [caster, ahead, behind]);
        // Facing right it is mirrored to 50..200.
        mobs.live.get_mut(&caster).unwrap().stance = 4;
        assert_eq!(mobs.reached_by(&heal), [caster, ahead]);

        mobs.live.get_mut(&caster).unwrap().hp = 600;
        assert_eq!(mobs.heal(caster, 700, 3), Some((400, 1000, 1000)));
        assert_eq!(mobs.get(caster).unwrap().mp, 12);
    }

    #[test]
    fn statuses_wear_off_and_spare_bosses() {
        let now = Instant::now();
        let mut mobs = MobSet::new([snail_spawn(100)]);
        let snail = mobs.respawn(now)[0];
        let until = now + Duration::from_secs(10);
        let seal = [(MobStat::Seal, 1), (MobStat::Speed, -20)];
        assert_eq!(
            mobs.add_statuses(snail, &seal, 2111004, 1, now, until),
            [MobStat::Seal, MobStat::Speed]
        );
        let views = mobs.get(snail).unwrap().status_views(now);
        assert_eq!(views.len(), 2);
        assert_eq!(views[0].left, Duration::from_secs(10));
        assert_eq!(
            mobs.remove_statuses(snail, &[MobStat::Speed, MobStat::Doom]),
            [MobStat::Speed]
        );
        assert!(mobs.expire_statuses(now).is_empty());
        assert_eq!(mobs.expire_statuses(until), [(snail, vec![MobStat::Seal])]);
        assert!(mobs.get(snail).unwrap().statuses.is_empty());

        mobs.live.get_mut(&snail).unwrap().info.boss = true;
        assert_eq!(
            mobs.add_statuses(snail, &seal, 2111004, 1, now, until),
            [MobStat::Speed]
        );
    }

    #[test]
    fn a_taunted_mob_gives_more_exp_until_taunt_wears_off() {
        let now = Instant::now();
        let mut mobs = MobSet::new([snail_spawn(100), snail_spawn(200)]);
        let snails = mobs.respawn(now);
        let until = now + Duration::from_secs(120);
        let taunt = [(MobStat::Showdown, 100), (MobStat::Pdr, 100)];
        for &snail in &snails {
            mobs.add_statuses(snail, &taunt, 4121003, 1, now, until);
        }
        let Some(Err(kill)) = mobs.damage(1, snails[0], 8, now) else {
            panic!("the snail should die");
        };
        assert_eq!((kill.exp, kill.showdown), (6, 100));
        let Some(Err(kill)) = mobs.damage(1, snails[1], 8, until) else {
            panic!("the snail should die");
        };
        assert_eq!((kill.exp, kill.showdown), (3, 0));
    }

    #[test]
    fn a_dead_mob_lets_its_revive_out_once_done_dying() {
        let now = Instant::now();
        let (template, mut info) = snail_spawn(100);
        info.revive = vec![100101, 100101];
        info.dying_ms = 600;
        let mut mobs = MobSet::new([(template, info)]);
        let parent = mobs.respawn(now)[0];
        mobs.live.get_mut(&parent).unwrap().controller = Some(1);
        let blue = |mob_id| {
            Some(MobInfo {
                mob_id,
                max_hp: 15,
                ..MobInfo::default()
            })
        };
        assert!(mobs.damage(1, parent, 8, now).unwrap().is_err());
        // Its controller leaves while it dies: the next one gets them.
        mobs.release(1, Some(2));
        assert!(mobs
            .revive(now + Duration::from_millis(599), blue)
            .is_empty());
        let revived = mobs.revive(now + Duration::from_millis(600), blue);
        assert_eq!(revived.len(), 2);
        for object_id in revived {
            let mob = mobs.get(object_id).unwrap();
            assert_eq!(
                (mob.info.mob_id, (mob.x, mob.y), mob.spawn, mob.controller),
                (100101, (100, -25), None, Some(2))
            );
        }
        assert!(mobs.revive(now + Duration::from_secs(5), blue).is_empty());
    }

    #[test]
    fn a_mob_with_remove_after_leaves_on_time_and_comes_back() {
        let now = Instant::now();
        let (template, mut info) = snail_spawn(100);
        info.remove_after = 10;
        info.revive = vec![100101];
        let mut mobs = MobSet::new([(template, info)]);
        let mob = mobs.respawn(now)[0];
        let due = now + Duration::from_secs(10);
        assert!(mobs.leave(due - Duration::from_millis(1)).is_empty());
        assert_eq!(mobs.leave(due), [mob]);
        assert!(mobs.get(mob).is_none());
        // Nothing comes out of it, and its spawn point fills again.
        assert!(mobs
            .revive(due + Duration::from_secs(5), |_| Some(MobInfo::default()))
            .is_empty());
        assert!(mobs
            .respawn(due + RESPAWN_DELAY - Duration::from_millis(1))
            .is_empty());
        assert_eq!(mobs.respawn(due + RESPAWN_DELAY).len(), 1);
    }

    #[test]
    fn counters_hurt_their_value_give_or_take_half_the_spread() {
        let now = Instant::now();
        let mut mobs = MobSet::new([snail_spawn(100)]);
        let snail = mobs.respawn(now)[0];
        let until = now + Duration::from_secs(10);
        assert_eq!(mobs.counter(snail, MobStat::PCounter, 7), 0);
        let counters = [(MobStat::PCounter, 220), (MobStat::MCounter, 220)];
        mobs.add_statuses(snail, &counters, 145 | 5 << 16, 1, now, until);
        assert_eq!(mobs.counter(snail, MobStat::MCounter, 7), 220);
        mobs.spread_counters(snail, &[MobStat::PCounter], 98);
        assert_eq!(mobs.counter(snail, MobStat::PCounter, 0), 220 - 49);
        assert_eq!(mobs.counter(snail, MobStat::PCounter, 97 + 98), 220 + 48);
        assert_eq!(mobs.counter(snail, MobStat::MCounter, 97), 220);
        let view = mobs.get(snail).unwrap().status_views(now);
        let spread = |stat| view.iter().find(|v| v.stat == stat).unwrap().spread;
        assert_eq!(
            (spread(MobStat::PCounter), spread(MobStat::MCounter)),
            (98, 0)
        );
    }

    #[test]
    fn poison_hurts_each_second_but_never_kills() {
        let now = Instant::now();
        let mut mobs = MobSet::new([snail_spawn(100)]);
        let snail = mobs.respawn(now)[0];
        let until = now + Duration::from_secs(10);
        let poison = [(MobStat::Poison, 3), (MobStat::Freeze, 1)];

        // A mob immune to poison and ice gets neither.
        mobs.live.get_mut(&snail).unwrap().info.element = "S1I1".into();
        assert!(mobs
            .add_statuses(snail, &poison, 2101005, 7, now, until)
            .is_empty());
        mobs.live.get_mut(&snail).unwrap().info.element = "S2".into();
        assert_eq!(
            mobs.add_statuses(snail, &poison, 2101005, 7, now, until),
            [MobStat::Poison, MobStat::Freeze]
        );

        assert!(mobs
            .hurt_by_statuses(now + Duration::from_millis(999))
            .is_empty());
        let second = now + HURT_INTERVAL;
        assert_eq!(
            mobs.hurt_by_statuses(second),
            [Hurt {
                object_id: snail,
                from: 7,
                percent: 63,
            }]
        );
        assert!(mobs.hurt_by_statuses(second).is_empty());
        // 8 HP: 3, then 3, then only down to 1.
        mobs.hurt_by_statuses(second + HURT_INTERVAL);
        mobs.hurt_by_statuses(second + HURT_INTERVAL * 2);
        let snail_mob = mobs.get(snail).unwrap();
        assert_eq!(snail_mob.hp, 1);
        assert_eq!(snail_mob.damage_by[&7], 7);
        // At 1 there's nothing to show.
        assert!(mobs.hurt_by_statuses(second + HURT_INTERVAL * 3).is_empty());

        // A mob with fixedDamage loses that instead.
        mobs.live.get_mut(&snail).unwrap().hp = 8;
        mobs.live.get_mut(&snail).unwrap().info.fixed_damage = 1;
        mobs.hurt_by_statuses(second + HURT_INTERVAL * 4);
        assert_eq!(mobs.get(snail).unwrap().hp, 7);
    }

    #[test]
    fn spawn_control_kill_and_respawn() {
        let now = Instant::now();
        let mut mobs = MobSet::new([snail_spawn(100), snail_spawn(200)]);
        let spawned = mobs.respawn(now);
        assert_eq!(spawned.len(), 2);
        assert_eq!(mobs.assign_uncontrolled(1), spawned);
        assert!(mobs.assign_uncontrolled(2).is_empty());
        let snail = spawned[0];

        // Only the controller may move it.
        let movement = [1, 0, 0x96, 0, 0xE7, 0xFF, 0, 0, 0, 0, 0, 0, 2, 0, 0];
        assert!(!mobs.move_mob(2, snail, (150, -25), &movement));
        assert!(mobs.move_mob(1, snail, (150, -25), &movement));
        assert_eq!(
            (mobs.get(snail).unwrap().x, mobs.get(snail).unwrap().y),
            (150, -25)
        );
        // Walking onto another platform: an absolute fragment with
        // foothold 52, which a relative one keeps.
        let walk = [
            2, 0, 0x96, 0, 0x2D, 0, 0, 0, 0, 0, 52, 0, 2, 100, 0, //
            1, 0x0A, 0, 0, 0, 2, 100, 0,
        ];
        assert!(mobs.move_mob(1, snail, (150, -25), &walk));
        let view = mobs.get(snail).unwrap().view(false);
        assert_eq!((view.foothold, view.home_foothold), (52, 45));
        // An absolute fragment without a foothold leaves it airborne.
        let fall = [1, 0, 0x96, 0, 0x30, 0, 0, 0, 0, 0, 0, 0, 2, 100, 0];
        assert!(mobs.move_mob(1, snail, (150, 45), &fall));
        assert_eq!(mobs.get(snail).unwrap().view(false).foothold, 0);

        assert_eq!(mobs.damage(1, snail, 3, now), Some(Ok(63)));
        let Some(Err(kill)) = mobs.damage(2, snail, 10, now) else {
            panic!("the snail should die");
        };
        assert_eq!(kill.damage_by, [(2, 5), (1, 3)]);
        // 3 of 8 damage vs 5 of 8: 1 and 2 EXP of 3.
        let solo = |client| Sharer {
            client,
            level: 1,
            party: None,
        };
        let shares = split_exp(kill.exp, kill.level, &kill.damage_by, &[solo(1), solo(2)]);
        let exp: Vec<_> = shares.iter().map(|s| (s.client, s.exp)).collect();
        assert_eq!(exp, [(2, 2), (1, 1)]);
        // A mob worth no EXP still counts for its attackers.
        let shares = split_exp(0, kill.level, &kill.damage_by, &[solo(1), solo(2)]);
        let exp: Vec<_> = shares.iter().map(|s| (s.client, s.exp)).collect();
        assert_eq!(exp, [(2, 0), (1, 0)]);
        assert!(mobs.get(snail).is_none());
        assert!(
            mobs.damage(1, snail, 1, now).is_none(),
            "dead mobs take no damage"
        );

        // Comes back after the respawn delay with a new object id.
        assert!(mobs.respawn(now + Duration::from_secs(1)).is_empty());
        let back = mobs.respawn(now + RESPAWN_DELAY);
        assert_eq!(back.len(), 1);
        assert_ne!(back[0], snail);
    }

    #[test]
    fn leaving_hands_mobs_to_someone_else() {
        let mut mobs = MobSet::new([snail_spawn(100)]);
        mobs.respawn(Instant::now());
        mobs.assign_uncontrolled(1);
        assert_eq!(mobs.release(1, Some(2)).len(), 1);
        assert_eq!(mobs.live().next().unwrap().controller, Some(2));
        mobs.release(2, None);
        assert_eq!(mobs.live().next().unwrap().controller, None);
    }

    #[test]
    fn a_hit_takes_control_with_aggro() {
        let now = Instant::now();
        let mut mobs = MobSet::new([snail_spawn(100)]);
        let snail = mobs.respawn(now)[0];
        mobs.assign_uncontrolled(1);
        assert_eq!(mobs.provoke(2, snail, now), Some(Some(1)));
        assert_eq!(mobs.get(snail).unwrap().controller, Some(2));
        // Further hits while the chase lasts change nothing...
        assert_eq!(mobs.provoke(2, snail, now + Duration::from_secs(60)), None);
        // ...but renew it once it may have ended.
        assert_eq!(mobs.provoke(2, snail, now + AGGRO_DURATION), Some(Some(2)));
        // Handing the mob on drops the aggro.
        mobs.release(2, Some(1));
        assert_eq!(mobs.provoke(1, snail, now), Some(Some(1)));
        assert!(mobs.provoke(1, 12345, now).is_none());
    }

    #[test]
    fn the_closest_player_controls_a_first_attack_mob() {
        let now = Instant::now();
        let (template, mut info) = snail_spawn(100);
        info.first_attack = true;
        let mut mobs = MobSet::new([(template, info), snail_spawn(200)]);
        let spawned = mobs.respawn(now);
        let (mob, snail) = (spawned[0], spawned[1]);
        mobs.assign_uncontrolled(1);

        // Until the controller reports, anyone else is closer.
        assert_eq!(mobs.report_distance(2, mob, 50, now), Some(Some(1)));
        assert_eq!(mobs.report_distance(1, mob, 50, now), None);
        assert_eq!(mobs.report_distance(1, mob, 20, now), Some(Some(2)));
        assert_eq!(mobs.report_distance(2, mob, 30, now), None);
        // The controller walking away lets the other take over.
        assert_eq!(mobs.report_distance(1, mob, 40, now), None);
        assert_eq!(mobs.report_distance(2, mob, 30, now), Some(Some(1)));

        // A chase keeps its player in control.
        assert_eq!(mobs.provoke(1, mob, now), Some(Some(2)));
        assert_eq!(mobs.report_distance(2, mob, 0, now), None);
        assert_eq!(
            mobs.report_distance(2, mob, 0, now + AGGRO_DURATION),
            Some(Some(1))
        );

        // Other mobs ignore reports.
        assert_eq!(mobs.report_distance(2, snail, 0, now), None);
        assert_eq!(mobs.get(snail).unwrap().controller, Some(1));
        assert!(mobs.report_distance(2, 12345, 0, now).is_none());
    }

    #[test]
    fn hidden_and_never_respawning_points_stay_empty() {
        let (mut hidden, info) = snail_spawn(0);
        hidden.hide = true;
        let (mut never, info2) = snail_spawn(0);
        never.mob_time = -1;
        let mut mobs = MobSet::new([(hidden, info), (never, info2)]);
        assert!(mobs.respawn(Instant::now()).is_empty());
    }

    #[test]
    fn a_party_splits_its_exp_by_level_with_a_bonus() {
        let sharer = |client, level, party| Sharer {
            client,
            level,
            party,
        };
        let on_field = [
            sharer(1, 10, Some(7)),
            sharer(2, 10, Some(7)),
            // Too far above the mob and the attacker to share.
            sharer(3, 30, Some(7)),
            sharer(4, 10, None),
            sharer(5, 10, Some(8)),
        ];
        let shares = split_exp(100, 10, &[(1, 80), (5, 20)], &on_field);
        // Party 7 earned 80 EXP: 40% each by level, 20% more to the attacker,
        // and 10% bonus for two sharers. Party 8 is one member: no bonus.
        assert_eq!(
            shares,
            [
                ExpShare {
                    client: 1,
                    exp: 48,
                    party_bonus: 5,
                },
                ExpShare {
                    client: 2,
                    exp: 32,
                    party_bonus: 3,
                },
                ExpShare {
                    client: 5,
                    exp: 20,
                    party_bonus: 0,
                },
            ]
        );
    }
}
