//! The mobs of one field: spawn points, live mobs, who controls each, and
//! the damage dealt to them. Pure state; `FieldActor` sends the packets.

use crate::actor::object_ids::{self, ObjectIds};
use game_data::mobs::MobInfo;
use game_data::MapMobTemplate;
use net::packet::build::world::field::parse_movement_state;
use net::packet::build::world::mob::MobView;
use net::ClientId;
use std::collections::{BTreeMap, HashMap};
use tokio::time::{Duration, Instant};

/// The soonest a killed mob comes back, when its spawn point has no
/// `mobTime` of its own.
pub const RESPAWN_DELAY: Duration = Duration::from_secs(7);

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
    pub hp: i64,
    pub controller: Option<ClientId>,
    /// Damage per player, for sharing EXP.
    pub damage_by: HashMap<ClientId, i64>,
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
            fade_in,
            controlled: self.controller.is_some(),
        }
    }

    pub fn hp_percent(&self) -> u8 {
        let max = i64::from(self.info.max_hp.max(1));
        ((self.hp.max(0) * 100 + max - 1) / max).clamp(0, 100) as u8
    }
}

/// A mob killed by an attack and who gets EXP for it.
#[derive(Debug, PartialEq, Eq)]
pub struct Kill {
    pub object_id: i32,
    pub mob_id: i32,
    pub x: i16,
    pub y: i16,
    /// EXP per player, shared by damage dealt.
    pub exp: Vec<(ClientId, i32)>,
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
                    hp: i64::from(spawn.info.max_hp),
                    controller: None,
                    damage_by: HashMap::new(),
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
                mob.damage_by.remove(&leaving);
                mob.object_id
            })
            .collect()
    }

    /// Apply the controller's movement. `false` if `from` doesn't control it.
    pub fn move_mob(&mut self, from: ClientId, object_id: i32, movement: &[u8]) -> bool {
        let Some(mob) = self.live.get_mut(&object_id) else {
            return false;
        };
        if mob.controller != Some(from) {
            return false;
        }
        if let Some((x, y, stance)) = parse_movement_state(movement, (mob.x, mob.y, mob.stance)) {
            mob.x = x;
            mob.y = y;
            mob.stance = stance;
        }
        true
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
            exp: share_exp(mob.info.exp, &mob.damage_by),
        }))
    }
}

/// Split a mob's EXP by damage dealt; the top damage dealer gets any
/// rounding remainder.
fn share_exp(exp: i32, damage_by: &HashMap<ClientId, i64>) -> Vec<(ClientId, i32)> {
    let total: i64 = damage_by.values().sum();
    if total <= 0 || exp <= 0 {
        return Vec::new();
    }
    let mut shares: Vec<(ClientId, i32)> = damage_by
        .iter()
        .filter(|(_, damage)| **damage > 0)
        .map(|(&client, &damage)| (client, (i64::from(exp) * damage / total) as i32))
        .collect();
    shares.sort_by_key(|(client, _)| std::cmp::Reverse(damage_by[client]));
    let given: i32 = shares.iter().map(|(_, e)| e).sum();
    if let Some(top) = shares.first_mut() {
        top.1 += exp - given;
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
        assert!(!mobs.move_mob(2, snail, &movement));
        assert!(mobs.move_mob(1, snail, &movement));
        assert_eq!(
            (mobs.get(snail).unwrap().x, mobs.get(snail).unwrap().y),
            (150, -25)
        );

        assert_eq!(mobs.damage(1, snail, 3, now), Some(Ok(63)));
        let Some(Err(kill)) = mobs.damage(2, snail, 10, now) else {
            panic!("the snail should die");
        };
        // 3 of 8 damage vs 5 of 8: 1 and 2 EXP of 3.
        let mut exp = kill.exp.clone();
        exp.sort();
        assert_eq!(exp, [(1, 1), (2, 2)]);
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
    fn hidden_and_never_respawning_points_stay_empty() {
        let (mut hidden, info) = snail_spawn(0);
        hidden.hide = true;
        let (mut never, info2) = snail_spawn(0);
        never.mob_time = -1;
        let mut mobs = MobSet::new([(hidden, info), (never, info2)]);
        assert!(mobs.respawn(Instant::now()).is_empty());
    }
}
