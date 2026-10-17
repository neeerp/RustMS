//! The mobs of one field: spawn points, live mobs, who controls each, and
//! the damage dealt to them. Pure state; `FieldActor` sends the packets.

use crate::actor::object_ids::{self, ObjectIds};
use game::MobStat;
use game_data::mobs::MobInfo;
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
    pub spawn: usize,
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
}

/// A mob's attackN is action 12 + N - 1, sent as action << 1 | facing left
/// (FUN_0066a517); its skills follow at 21.
pub const FIRST_ATTACK_ACTION: u8 = 12;
pub const FIRST_SKILL_ACTION: u8 = 21;
/// No action.
pub const NO_ACTION: u8 = 0xFF;

/// What a controller's move of a mob let it do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MobAct {
    /// The action to show the others.
    pub action: u8,
    /// Whether it may attack on its next move.
    pub next_attack: bool,
    pub mp: i16,
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
    pub exp: i32,
    pub level: i16,
    /// Damage per player still on the field, most first.
    pub damage_by: Vec<(ClientId, i64)>,
}

pub struct MobSet {
    spawns: Vec<SpawnPoint>,
    live: BTreeMap<i32, LiveMob>,
    object_ids: ObjectIds,
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
            self.live.insert(
                object_id,
                LiveMob {
                    object_id,
                    spawn: index,
                    info: spawn.info.clone(),
                    x: spawn.template.x,
                    y: spawn.template.y,
                    // Mob stances are 2 (moving) or 5 (standing), +1 facing left.
                    stance: if spawn.template.flip { 4 } else { 5 },
                    foothold: spawn.template.foothold,
                    home_foothold: spawn.template.foothold,
                    hp: i64::from(spawn.info.max_hp),
                    mp: spawn.info.max_mp,
                    controller: None,
                    aggro_until: None,
                    controller_distance: None,
                    damage_by: HashMap::new(),
                    statuses: BTreeMap::new(),
                },
            );
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
                mob.object_id
            })
            .collect()
    }

    /// `leaving` stopped controlling its mobs; hand them to `next` if any.
    /// Returns the mobs that changed hands.
    pub fn release(&mut self, leaving: ClientId, next: Option<ClientId>) -> Vec<i32> {
        self.live
            .values_mut()
            .filter(|mob| mob.controller == Some(leaving))
            .map(|mob| {
                mob.controller = next;
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
    /// [`NO_ACTION`]) as it moved, and its client wants it to attack next
    /// (`want_attack`). An attack costs the mob its MP; one it lacks or
    /// can't pay for is shown as no action, as are skills, which the server
    /// hasn't granted.
    pub fn act(&mut self, object_id: i32, action: u8, want_attack: bool) -> Option<MobAct> {
        let mob = self.live.get_mut(&object_id)?;
        let mut shown = action;
        if action != NO_ACTION && action >> 1 >= FIRST_SKILL_ACTION {
            shown = NO_ACTION;
        } else if action != NO_ACTION && action >> 1 >= FIRST_ATTACK_ACTION {
            let index = usize::from((action >> 1) - FIRST_ATTACK_ACTION);
            match mob.info.attacks.get(index) {
                Some(attack) if attack.mp_cost <= mob.mp => mob.mp -= attack.mp_cost,
                _ => shown = NO_ACTION,
            }
        }
        Some(MobAct {
            action: shown,
            next_attack: want_attack && !mob.info.not_attack && !mob.info.attacks.is_empty(),
            mp: i16::try_from(mob.mp).unwrap_or(i16::MAX),
        })
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
                };
                mob.statuses.insert(stat, status);
                stat
            })
            .collect()
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
        let spawn = &mut self.spawns[mob.spawn];
        spawn.alive = None;
        let delay = if spawn.template.mob_time > 0 {
            Duration::from_secs(spawn.template.mob_time as u64).max(RESPAWN_DELAY)
        } else {
            RESPAWN_DELAY
        };
        spawn.next_spawn = Some(now + delay);
        Some(Err(Kill {
            object_id,
            mob_id: mob.info.mob_id,
            x: mob.x,
            y: mob.y,
            exp: mob.info.exp,
            level: mob.info.level,
            damage_by: ranked_damage(&mob.damage_by),
        }))
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
    /// Whether they hit the mob, and so roll its loot.
    pub attacked: bool,
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
    if total <= 0 || exp <= 0 {
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
                attacked: true,
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
                attacked: attackers.iter().any(|(a, _)| a.client == member.client),
            });
        }
    }
    shares
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let walk = mobs.act(mob, NO_ACTION, true).unwrap();
        assert_eq!(
            (walk.action, walk.next_attack, walk.mp),
            (NO_ACTION, true, 3)
        );
        let first = mobs.act(mob, attack1_left, false).unwrap();
        assert_eq!(
            (first.action, first.next_attack, first.mp),
            (attack1_left, false, 1)
        );
        // Out of MP, and with no attack2: shown as no action.
        assert_eq!(mobs.act(mob, attack1_left, true).unwrap().action, NO_ACTION);
        assert_eq!(
            mobs.act(mob, (FIRST_ATTACK_ACTION + 1) << 1, true)
                .unwrap()
                .action,
            NO_ACTION
        );
        // Skills await the server's grant.
        assert_eq!(
            mobs.act(mob, FIRST_SKILL_ACTION << 1, true).unwrap().action,
            NO_ACTION
        );
        // A hit (action 6) is shown as it is.
        assert_eq!(mobs.act(mob, 6 << 1, true).unwrap().action, 6 << 1);
        // A mob without attacks never may.
        let mut snails = MobSet::new([snail_spawn(100)]);
        let snail = snails.respawn(now)[0];
        assert!(!snails.act(snail, NO_ACTION, true).unwrap().next_attack);
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
                    attacked: true,
                },
                ExpShare {
                    client: 2,
                    exp: 32,
                    party_bonus: 3,
                    attacked: false,
                },
                ExpShare {
                    client: 5,
                    exp: 20,
                    party_bonus: 0,
                    attacked: true,
                },
            ]
        );
    }
}
