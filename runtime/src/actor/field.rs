use crate::actor::field_drops::{Drop, DropSet};
use crate::actor::field_mists::MistSet;
use crate::actor::field_mobs::{split_exp, MobSet, Sharer};
use crate::actor::field_reactors::{Hit, ReactorSet};
use crate::actor::field_summons::SummonSet;
use crate::message::{ClientHandle, FieldKey, FieldMessage, ServerMessage};
use game::FieldAvatar;
use net::packet::build::world::drop::{build_drop, build_remove_drop, DropAppearance};
use net::packet::build::world::field::{
    build_player_enter_field, build_player_leave_field, build_show_chair, parse_movement_state,
};
use net::packet::build::world::mist::{build_remove_mist, build_spawn_mist, MistKind, MistView};
use net::packet::build::world::mob::{
    build_control_mob, build_kill_mob, build_mob_hp, build_mob_stat_reset, build_mob_stat_set,
    build_move_mob, build_move_mob_response, build_show_magnet, build_spawn_mob,
    build_stop_controlling_mob,
};
use net::packet::build::world::npc::{
    build_control_npc, build_npc_action, build_spawn_npc, ForeignNpc,
};
use net::packet::build::world::reactor::{
    build_hit_reactor, build_remove_reactor, build_spawn_reactor,
};
use net::packet::build::world::summon::{
    build_damage_summon, build_move_summon, build_remove_summon, build_spawn_summon,
    build_summon_attack, Gone, SummonView,
};
use net::skills::{self, Inflict};
use net::summons::{self, Limit, SummonAttack, SummonDamage};
use net::{ClientId, FieldRequest};
use packet::Packet;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{interval, Instant, MissedTickBehavior};
use tracing::{info, warn};

struct Occupant {
    handle: ClientHandle,
    avatar: FieldAvatar,
    party: Option<i32>,
}

#[derive(Clone, Debug)]
pub struct FieldMapEntityNpc {
    pub object_id: i32,
    pub npc_id: i32,
    pub x: i16,
    pub y: i16,
    pub flip: bool,
    pub foothold: i16,
    pub rx0: i16,
    pub rx1: i16,
}

/// How far a player may be from an NPC and still start or complete its quest:
/// roughly the screen around the player, so the NPC must be on screen.
const QUEST_NPC_REACH_X: i32 = 1200;
const QUEST_NPC_REACH_Y: i32 = 800;

/// How long after a cast its mist starts, in 100 ms (OdinMS-family
/// servers; v83's client starts the mist's time then).
const MIST_DELAY: i16 = 8;

/// How far from its owner a client may put a summon: a Puppet goes 200 px
/// ahead, onto a foothold within 80 px up or down (FUN_0096b140).
const SUMMON_REACH_X: i32 = 250;
const SUMMON_REACH_Y: i32 = 150;

/// One live copy of a map. It owns everything local to that map: who is on
/// it and where, its NPCs, mobs, drops, and reactors, which advance on
/// `tick`, and its players' mists and summons.
pub struct FieldActor {
    key: FieldKey,
    rx: mpsc::Receiver<FieldMessage>,
    occupants: HashMap<ClientId, Occupant>,
    map_npcs: Vec<FieldMapEntityNpc>,
    /// Who picks the NPCs' idle actions; without one they only stand.
    npc_controller: Option<ClientId>,
    mobs: MobSet,
    drops: DropSet,
    reactors: ReactorSet,
    mists: MistSet,
    summons: SummonSet,
    tick: Duration,
}

impl FieldActor {
    pub fn new(
        key: FieldKey,
        rx: mpsc::Receiver<FieldMessage>,
        map_npcs: Vec<FieldMapEntityNpc>,
        mut mobs: MobSet,
        drops: DropSet,
        mut reactors: ReactorSet,
        tick: Duration,
    ) -> Self {
        mobs.respawn(Instant::now());
        reactors.respawn(Instant::now());
        Self {
            key,
            rx,
            occupants: HashMap::new(),
            map_npcs,
            npc_controller: None,
            mobs,
            drops,
            reactors,
            mists: MistSet::default(),
            summons: SummonSet::default(),
            tick,
        }
    }

    pub async fn run(mut self) {
        info!(field = ?self.key, "FieldActor started");
        let mut ticker = interval(self.tick);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                message = self.rx.recv() => match message {
                    Some(message) => self.handle_message(message),
                    None => break,
                },
                now = ticker.tick() => self.on_tick(now),
            }
        }
    }

    /// Advance time-driven field state: drop expiry and mob and reactor
    /// respawns.
    fn on_tick(&mut self, now: Instant) {
        for object_id in self.drops.expire(now) {
            if let Ok(packet) = build_remove_drop(object_id, None) {
                self.broadcast(None, packet);
            }
        }
        for object_id in self.reactors.respawn(now) {
            if let Some(reactor) = self.reactors.get(object_id) {
                if let Ok(packet) = build_spawn_reactor(&reactor.view()) {
                    self.broadcast(None, packet);
                }
            }
        }
        for object_id in self.mists.expire(now) {
            if let Ok(packet) = build_remove_mist(object_id) {
                self.broadcast(None, packet);
            }
        }
        self.reach_from_mists(now);
        for summon in self.summons.expire(now) {
            let view = summon.view;
            if let Ok(packet) = build_remove_summon(view.owner_id, view.object_id, Gone::Expired) {
                self.broadcast(None, packet);
            }
        }
        for hurt in self.mobs.hurt_by_statuses(now) {
            if let (Some(occupant), Ok(packet)) = (
                self.occupants.get(&hurt.from),
                build_mob_hp(hurt.object_id, hurt.percent),
            ) {
                occupant.handle.send_packet(packet);
            }
        }
        for (object_id, lost) in self.mobs.expire_statuses(now) {
            if let Ok(packet) = build_mob_stat_reset(object_id, &lost) {
                self.broadcast(None, packet);
            }
        }
        let spawned = self.mobs.respawn(now);
        if spawned.is_empty() {
            return;
        }
        let controller = self.occupants.keys().next().copied();
        for object_id in spawned {
            let Some(mob) = self.mobs.get(object_id) else {
                continue;
            };
            if let Ok(packet) = build_spawn_mob(&mob.view(true)) {
                self.broadcast(None, packet);
            }
        }
        if let Some(controller) = controller {
            let controlled = self.mobs.assign_uncontrolled(controller);
            self.send_control(controller, &controlled);
        }
    }

    /// Tell `client` it now moves these mobs.
    fn send_control(&self, client: ClientId, object_ids: &[i32]) {
        let Some(occupant) = self.occupants.get(&client) else {
            return;
        };
        for &object_id in object_ids {
            if let Some(mob) = self.mobs.get(object_id) {
                if let Ok(packet) = build_control_mob(&mob.view(false), false) {
                    occupant.handle.send_packet(packet);
                }
            }
        }
    }

    /// Tell `client` it now picks every NPC's idle actions.
    fn send_npc_control(&mut self, client: ClientId) {
        let Some(occupant) = self.occupants.get(&client) else {
            return;
        };
        self.npc_controller = Some(client);
        for map_npc in &self.map_npcs {
            if let Ok(packet) = build_control_npc(&to_foreign_npc(map_npc)) {
                occupant.handle.send_packet(packet);
            }
        }
    }

    /// Show everyone the action the NPCs' controller chose.
    fn handle_npc_action(&self, from: ClientId, object_id: i32, action: &[u8]) {
        if self.npc_controller != Some(from)
            || !self.map_npcs.iter().any(|npc| npc.object_id == object_id)
        {
            return;
        }
        if let Ok(packet) = build_npc_action(object_id, action) {
            self.broadcast(None, packet);
        }
    }

    /// Seat `from` on a portable chair (0 to stand) and show the others.
    fn handle_chair(&mut self, from: ClientId, item_id: i32) {
        let Some(occupant) = self.occupants.get_mut(&from) else {
            return;
        };
        if occupant.avatar.chair == item_id {
            return;
        }
        occupant.avatar.chair = item_id;
        if let Ok(packet) = build_show_chair(occupant.avatar.id, item_id) {
            self.broadcast(Some(from), packet);
        }
    }

    fn handle_message(&mut self, message: FieldMessage) {
        match message {
            FieldMessage::Join {
                handle,
                avatar,
                party,
            } => self.handle_join(handle, avatar, party),
            FieldMessage::Leave { client_id } => self.handle_leave(client_id),
            FieldMessage::SetParty { client_id, party } => {
                if let Some(occupant) = self.occupants.get_mut(&client_id) {
                    occupant.party = party;
                }
            }
            FieldMessage::UpdateAvatar {
                client_id,
                avatar,
                announce,
            } => {
                if let Some(packet) = announce {
                    self.broadcast(Some(client_id), packet);
                }
                if let Some(occupant) = self.occupants.get_mut(&client_id) {
                    // Keep the live position and chair; take everything else.
                    let live = &occupant.avatar;
                    occupant.avatar = FieldAvatar {
                        x: live.x,
                        y: live.y,
                        stance: live.stance,
                        chair: live.chair,
                        ..avatar
                    };
                }
            }
            FieldMessage::Chat { packet, .. } => self.broadcast(None, packet),
            FieldMessage::Move {
                from,
                packet,
                start,
                movement_bytes,
            } => {
                if let Some(occupant) = self.occupants.get_mut(&from) {
                    let avatar = &mut occupant.avatar;
                    if let Some((x, y, stance)) =
                        parse_movement_state(&movement_bytes, (start.0, start.1, avatar.stance))
                    {
                        avatar.x = x;
                        avatar.y = y;
                        avatar.stance = stance;
                    }
                }
                self.broadcast(Some(from), packet);
            }
            FieldMessage::NpcInteraction {
                from,
                object_id,
                request_id,
            } => self.handle_npc_interaction(from, object_id, request_id),
            FieldMessage::QuestNpc {
                from,
                npc_id,
                request_id,
            } => self.handle_quest_npc(from, npc_id, request_id),
            FieldMessage::FromClient { from, request } => match request {
                FieldRequest::MobMove {
                    object_id,
                    move_id,
                    start,
                    movement,
                } => self.handle_mob_move(from, object_id, move_id, start, &movement),
                FieldRequest::MobDistance {
                    object_id,
                    distance,
                } => self.handle_mob_distance(from, object_id, distance),
                FieldRequest::NpcAction { object_id, action } => {
                    self.handle_npc_action(from, object_id, &action)
                }
                FieldRequest::Attack {
                    broadcast,
                    hits,
                    inflicts,
                } => self.handle_attack(from, broadcast, hits, &inflicts),
                FieldRequest::Mist {
                    skill_id,
                    level,
                    left,
                    inflict,
                } => self.handle_mist(from, skill_id, level, left, inflict),
                FieldRequest::Broadcast(packet) => self.broadcast(Some(from), packet),
                FieldRequest::Magnet { effect, caught } => self.handle_magnet(from, effect, caught),
                FieldRequest::MobDebuff {
                    skill_id,
                    level,
                    origin,
                    mobs,
                    inflict,
                } => self.handle_mob_debuff(from, skill_id, level, origin, &mobs, &inflict),
                FieldRequest::Chair(item_id) => self.handle_chair(from, item_id),
                FieldRequest::Pickup { object_id, x, y } => {
                    self.handle_pickup(from, object_id, x, y)
                }
                FieldRequest::Throw { loot, vanish } => self.handle_throw(from, loot, vanish),
                FieldRequest::HitReactor {
                    object_id,
                    skill_id,
                    stance,
                } => self.handle_hit_reactor(from, object_id, skill_id, stance),
                FieldRequest::CharInfo { character_id } => {
                    self.handle_char_info(from, character_id)
                }
                FieldRequest::Fame {
                    target,
                    giver_name,
                    up,
                } => self.handle_fame(from, target, giver_name, up),
                FieldRequest::Summon {
                    skill_id,
                    level,
                    at,
                    left,
                    lasts,
                    appear,
                } => self.handle_summon(from, skill_id, level, at, left, lasts, appear),
                FieldRequest::Unsummon { skill_id, why } => {
                    self.handle_unsummon(from, skill_id, why)
                }
                FieldRequest::SummonMove {
                    object_id,
                    start,
                    movement,
                } => self.handle_summon_move(from, object_id, start, movement),
                FieldRequest::SummonAttack { attack, limits } => {
                    self.handle_summon_attack(from, attack, &limits)
                }
                FieldRequest::SummonDamage(damage) => self.handle_summon_damage(from, damage),
            },
            FieldMessage::SpawnLoot {
                owner,
                loot,
                x,
                y,
                dropper,
            } => self.spawn_loot(owner, loot, x, y, dropper),
            FieldMessage::ReturnDrop { drop } => {
                let now = Instant::now();
                if let Ok(packet) = build_drop(&drop.view(now), DropAppearance::Lying) {
                    self.send_drop_packet(&drop, packet);
                }
                self.drops.restore(drop);
            }
        }
    }

    fn spawn_loot(
        &mut self,
        owner: ClientId,
        loot: Vec<net::loot::Loot>,
        x: i16,
        y: i16,
        dropper: i32,
    ) {
        let now = Instant::now();
        let packets: Vec<(Drop, Packet)> = self
            .drops
            .spawn(loot, owner, x, y, dropper, now)
            .into_iter()
            .filter_map(|drop| {
                build_drop(&drop.view(now), DropAppearance::Falling { from: (x, y) })
                    .ok()
                    .map(|packet| (drop.clone(), packet))
            })
            .collect();
        for (drop, packet) in packets {
            self.send_drop_packet(&drop, packet);
        }
    }

    /// Have `character_id`, if they are here, answer `from`'s request for
    /// their info: their own actor holds the current level and fame.
    fn handle_char_info(&self, from: ClientId, character_id: i32) {
        let Some(requester) = self.occupants.get(&from) else {
            return;
        };
        if let Some(target) = self
            .occupants
            .values()
            .find(|occupant| occupant.avatar.id == character_id)
        {
            target.handle.send(ServerMessage::CharInfoRequested {
                requester: requester.handle.clone(),
            });
        } else if let Ok(packet) = net::packet::build::world::stat::build_stat_update(true, &[]) {
            // Unlock the client, which waits on CHAR_INFO otherwise.
            requester.handle.send_packet(packet);
        }
    }

    /// Hand `from`'s fame to `target`, if they are here.
    fn handle_fame(&self, from: ClientId, target: i32, giver_name: String, up: bool) {
        let Some(giver) = self.occupants.get(&from) else {
            return;
        };
        if let Some(occupant) = self
            .occupants
            .values()
            .find(|occupant| occupant.avatar.id == target)
        {
            occupant.handle.send(ServerMessage::Famed {
                giver: giver.handle.clone(),
                giver_name,
                up,
            });
        } else {
            giver.handle.send(ServerMessage::FameMissed { target });
        }
    }

    /// Put down what a player threw away where they stand, or show it
    /// falling and fading if it can't lie on the ground.
    fn handle_throw(&mut self, from: ClientId, loot: net::loot::Loot, vanish: bool) {
        let Some(occupant) = self.occupants.get(&from) else {
            return;
        };
        let (x, y) = (occupant.avatar.x, occupant.avatar.y);
        let now = Instant::now();
        let (drop, appearance) = if vanish {
            let drop = Drop {
                object_id: self.drops.next_object_id(),
                loot,
                owner: from,
                x,
                y,
                dropper: 0,
                dropped_at: now,
                player_drop: true,
            };
            (drop, DropAppearance::Vanishing { from: (x, y) })
        } else {
            let drop = self.drops.throw(loot, from, x, y, now).clone();
            (drop, DropAppearance::Falling { from: (x, y) })
        };
        if let Ok(packet) = build_drop(&drop.view(now), appearance) {
            self.send_drop_packet(&drop, packet);
        }
    }

    /// Send a drop packet to everyone who may see the drop.
    fn send_drop_packet(&self, drop: &Drop, packet: Packet) {
        for (&client, occupant) in &self.occupants {
            if drop.visible_to(client) {
                occupant.handle.send_packet(packet.clone());
            }
        }
    }

    fn handle_pickup(&mut self, from: ClientId, object_id: i32, x: i16, y: i16) {
        let Some(occupant) = self.occupants.get(&from) else {
            return;
        };
        let handle = occupant.handle.clone();
        match self.drops.take(from, object_id, x, y, Instant::now()) {
            Ok(drop) => {
                if let Ok(packet) = build_remove_drop(object_id, Some(from)) {
                    self.send_drop_packet(&drop, packet);
                }
                handle.send(ServerMessage::PickedUp { drop });
            }
            Err(_) => {
                // Unlock the client's pickup.
                if let Ok(packet) = net::packet::build::world::stat::build_stat_update(true, &[]) {
                    handle.send_packet(packet);
                }
            }
        }
    }

    fn handle_mob_move(
        &mut self,
        from: ClientId,
        object_id: i32,
        move_id: i16,
        start: (i16, i16),
        movement: &[u8],
    ) {
        if !self.mobs.move_mob(from, object_id, start, movement) {
            return;
        }
        if let Some(occupant) = self.occupants.get(&from) {
            if let Ok(packet) = build_move_mob_response(object_id, move_id, 0) {
                occupant.handle.send_packet(packet);
            }
        }
        if let Ok(packet) = build_move_mob(object_id, start, movement) {
            self.broadcast(Some(from), packet);
        }
    }

    fn handle_attack(
        &mut self,
        from: ClientId,
        broadcast: Packet,
        hits: Vec<(i32, i64)>,
        inflicts: &[Inflict],
    ) {
        let Some(attacker) = self.occupants.get(&from) else {
            return;
        };
        let attacker = attacker.handle.clone();
        self.broadcast(Some(from), broadcast);
        let now = Instant::now();
        for (object_id, damage) in hits {
            match self.mobs.damage(from, object_id, damage, now) {
                Some(Ok(percent)) => {
                    if let Ok(packet) = build_mob_hp(object_id, percent) {
                        attacker.send_packet(packet);
                    }
                    self.provoke(from, &attacker, object_id, now);
                    // A miss puts nothing on it.
                    for inflict in inflicts.iter().filter(|_| damage > 0) {
                        if inflict.takes(rand::random()) {
                            self.put_on(from, object_id, inflict, now);
                        }
                    }
                }
                Some(Err(kill)) => {
                    if let Ok(packet) = build_kill_mob(kill.object_id, 1) {
                        self.broadcast(None, packet);
                    }
                    let sharers: Vec<Sharer> = self
                        .occupants
                        .iter()
                        .map(|(&client, occupant)| Sharer {
                            client,
                            level: occupant.avatar.level,
                            party: occupant.party,
                        })
                        .collect();
                    for share in split_exp(kill.exp, kill.level, &kill.damage_by, &sharers) {
                        if let Some(occupant) = self.occupants.get(&share.client) {
                            occupant.handle.send(ServerMessage::MobKilled {
                                mob_id: kill.mob_id,
                                object_id: kill.object_id,
                                exp: share.exp,
                                party_bonus: share.party_bonus,
                                attacked: share.attacked,
                                x: kill.x,
                                y: kill.y,
                            });
                        }
                    }
                }
                None => {}
            }
        }
    }

    /// `from` used Monster Magnet. The others see it catch each live mob,
    /// or fail to; a mob caught (never a boss) chases `from`, whose client
    /// pulls it in.
    fn handle_magnet(&mut self, from: ClientId, effect: Packet, caught: Vec<(i32, bool)>) {
        let Some(user) = self.occupants.get(&from) else {
            return;
        };
        let user = user.handle.clone();
        let now = Instant::now();
        for (object_id, success) in caught {
            let Some(mob) = self.mobs.get(object_id) else {
                continue;
            };
            let success = success && !mob.info.boss;
            if let Ok(packet) = build_show_magnet(object_id, success) {
                self.broadcast(Some(from), packet);
            }
            if success {
                self.provoke(from, &user, object_id, now);
            }
        }
        self.broadcast(Some(from), effect);
    }

    /// `from` used a skill that reaches mobs on these, as their client saw
    /// them. Each in its area takes `inflict`, or loses the buffs the skill
    /// takes away, with the skill's chance (`prop`), and everyone sees.
    fn handle_mob_debuff(
        &mut self,
        from: ClientId,
        skill_id: i32,
        level: i32,
        origin: Option<(i16, i16)>,
        mobs: &[i32],
        inflict: &Inflict,
    ) {
        let Some(user) = self.occupants.get(&from) else {
            return;
        };
        let origin = origin.unwrap_or((user.avatar.x, user.avatar.y));
        let Some(skill) = skills::at_level(skill_id, level) else {
            return;
        };
        let cancels = skills::mob_cancels(skill_id);
        let effect = &skill.effect;
        let now = Instant::now();
        let mut reached: Vec<i32> = Vec::new();
        for &object_id in mobs {
            if reached.len() >= effect.mob_count.max(1) as usize || reached.contains(&object_id) {
                continue;
            }
            let Some(mob) = self.mobs.get(object_id) else {
                continue;
            };
            if mob.hp > 0 && skills::in_area(effect, origin, (mob.x, mob.y)) {
                reached.push(object_id);
            }
        }
        for object_id in reached {
            if !inflict.takes(rand::random()) {
                continue;
            }
            let lost = self.mobs.remove_statuses(object_id, cancels);
            if !lost.is_empty() {
                if let Ok(packet) = build_mob_stat_reset(object_id, &lost) {
                    self.broadcast(None, packet);
                }
            }
            self.put_on(from, object_id, inflict, now);
        }
    }

    /// `from`'s spell left a mist around them, which everyone sees.
    fn handle_mist(
        &mut self,
        from: ClientId,
        skill_id: i32,
        level: i32,
        left: bool,
        inflict: Inflict,
    ) {
        let Some(user) = self.occupants.get(&from) else {
            return;
        };
        let Some(skill) = skills::at_level(skill_id, level) else {
            return;
        };
        let view = MistView {
            object_id: 0,
            kind: MistKind::Poison,
            owner_id: user.avatar.id,
            skill_id,
            level: level as u8,
            delay: MIST_DELAY,
            area: skills::area_box(&skill.effect, (user.avatar.x, user.avatar.y), left),
        };
        let lasts = inflict.duration;
        let view = self.mists.add(view, from, inflict, lasts, Instant::now());
        if let Ok(packet) = build_spawn_mist(&view) {
            self.broadcast(None, packet);
        }
    }

    /// `from` called up a summon. It replaces their summon of that kind,
    /// which the others see dismissed.
    #[allow(clippy::too_many_arguments)]
    fn handle_summon(
        &mut self,
        from: ClientId,
        skill_id: i32,
        level: i32,
        at: (i16, i16),
        left: bool,
        lasts: Duration,
        appear: bool,
    ) {
        let Some(owner) = self.occupants.get(&from) else {
            return;
        };
        let (Some((moves, assist)), Some(skill)) =
            (summons::kind(skill_id), skills::at_level(skill_id, level))
        else {
            return;
        };
        let avatar = &owner.avatar;
        let near = (i32::from(at.0) - i32::from(avatar.x)).abs() <= SUMMON_REACH_X
            && (i32::from(at.1) - i32::from(avatar.y)).abs() <= SUMMON_REACH_Y;
        let (x, y) = if near { at } else { (avatar.x, avatar.y) };
        let view = SummonView {
            owner_id: avatar.id,
            object_id: 0,
            skill_id,
            owner_level: avatar.level,
            level: level as u8,
            x,
            y,
            stance: u8::from(left),
            moves,
            assist,
        };
        let hp = summons::hp(&skill);
        let (view, replaced) = self.summons.add(view, from, hp, lasts, Instant::now());
        if let Some(old) = replaced {
            self.remove_summon(&old.view, Gone::Dismissed);
        }
        if let Ok(packet) = build_spawn_summon(&view, appear) {
            self.broadcast(None, packet);
        }
    }

    fn handle_unsummon(&mut self, from: ClientId, skill_id: Option<i32>, why: Gone) {
        for summon in self.summons.remove_owned(from, skill_id) {
            self.remove_summon(&summon.view, why);
        }
    }

    fn remove_summon(&self, view: &SummonView, why: Gone) {
        if let Ok(packet) = build_remove_summon(view.owner_id, view.object_id, why) {
            self.broadcast(None, packet);
        }
    }

    fn handle_summon_move(
        &mut self,
        from: ClientId,
        object_id: i32,
        start: (i16, i16),
        movement: Vec<u8>,
    ) {
        let Some(summon) = self.summons.owned(from, object_id) else {
            return;
        };
        let view = &mut summon.view;
        if let Some((x, y, stance)) =
            parse_movement_state(&movement, (start.0, start.1, view.stance))
        {
            (view.x, view.y, view.stance) = (x, y, stance);
        }
        let owner_id = view.owner_id;
        if let Ok(packet) = build_move_summon(owner_id, object_id, start, &movement) {
            self.broadcast(Some(from), packet);
        }
    }

    /// `from`'s summon attacked. An attack beyond its skill's reach or
    /// damage cap, or one that hits a mob twice, counts for nothing.
    fn handle_summon_attack(&mut self, from: ClientId, attack: SummonAttack, limits: &[Limit]) {
        let Some(summon) = self.summons.owned(from, attack.object_id) else {
            return;
        };
        let view = summon.view;
        let Some(limit) = limits.iter().find(|limit| limit.skill_id == view.skill_id) else {
            return;
        };
        let hits = &attack.hits;
        let mut mobs: Vec<i32> = hits.iter().map(|hit| hit.object_id).collect();
        mobs.sort_unstable();
        mobs.dedup();
        if hits.len() > limit.mob_count as usize
            || mobs.len() != hits.len()
            || hits
                .iter()
                .any(|hit| !(0..=limit.max_damage).contains(&hit.damage))
        {
            warn!(
                from,
                skill_id = view.skill_id,
                ?attack,
                ?limit,
                "Summon attack refused"
            );
            return;
        }
        let Ok(broadcast) = build_summon_attack(&view, attack.action, hits) else {
            return;
        };
        let hits = hits
            .iter()
            .map(|hit| (hit.object_id, i64::from(hit.damage)))
            .collect();
        self.handle_attack(from, broadcast, hits, &[]);
    }

    /// A mob hurt `from`'s Puppet: the others see it, and it breaks at
    /// 0 HP, ending its owner's buff.
    fn handle_summon_damage(&mut self, from: ClientId, damage: SummonDamage) {
        let Some(summon) = self.summons.owned(from, damage.object_id) else {
            return;
        };
        let Some(hp) = summon.hp.as_mut() else {
            return;
        };
        *hp -= damage.damage;
        let (broken, view) = (*hp <= 0, summon.view);
        if let Ok(packet) = build_damage_summon(view.owner_id, &damage) {
            self.broadcast(Some(from), packet);
        }
        if !broken {
            return;
        }
        self.summons.remove(view.object_id);
        self.remove_summon(&view, Gone::Broken);
        if let Some(owner) = self.occupants.get(&from) {
            owner.handle.send(ServerMessage::SummonBroken {
                skill_id: view.skill_id,
            });
        }
    }

    /// Each mist due puts its statuses, with its chance, on the live mobs
    /// in it that lack them (OdinMS-family servers).
    fn reach_from_mists(&mut self, now: Instant) {
        let mut reached = Vec::new();
        for mist in self.mists.reaching(now) {
            for mob in self.mobs.live() {
                let lacks = mist
                    .inflict
                    .statuses
                    .iter()
                    .any(|(stat, _)| !mob.statuses.contains_key(stat));
                if mob.hp > 0 && lacks && mist.covers((mob.x, mob.y)) {
                    reached.push((mist.owner, mob.object_id, mist.inflict.clone()));
                }
            }
        }
        for (owner, object_id, inflict) in reached {
            if inflict.takes(rand::random()) {
                self.put_on(owner, object_id, &inflict, now);
            }
        }
    }

    /// Put `inflict`'s statuses on a mob for `from`, and show everyone the
    /// ones it got.
    fn put_on(&mut self, from: ClientId, object_id: i32, inflict: &Inflict, now: Instant) {
        let Some(mob) = self.mobs.get(object_id) else {
            return;
        };
        let statuses: Vec<_> = inflict
            .statuses
            .iter()
            .map(|&(stat, amount)| (stat, amount.against(mob.info.max_hp)))
            .collect();
        if statuses.is_empty() {
            return;
        }
        let until = now + inflict.duration;
        let given =
            self.mobs
                .add_statuses(object_id, &statuses, inflict.skill_id, from, now, until);
        let Some(mob) = self.mobs.get(object_id) else {
            return;
        };
        let views: Vec<_> = mob
            .status_views(now)
            .into_iter()
            .filter(|view| given.contains(&view.stat))
            .collect();
        if views.is_empty() {
            return;
        }
        if let Ok(packet) = build_mob_stat_set(object_id, &views) {
            self.broadcast(None, packet);
        }
    }

    /// Give the mob `from` hit to its client with aggro, taking it from
    /// whoever moved it before.
    fn provoke(&mut self, from: ClientId, attacker: &ClientHandle, object_id: i32, now: Instant) {
        let Some(previous) = self.mobs.provoke(from, object_id, now) else {
            return;
        };
        self.hand_over(object_id, previous, from, attacker, true);
    }

    /// `from` reported how far it is from a firstAttack mob; the closest
    /// player moves it.
    fn handle_mob_distance(&mut self, from: ClientId, object_id: i32, distance: i32) {
        let Some(occupant) = self.occupants.get(&from) else {
            return;
        };
        let handle = occupant.handle.clone();
        let Some(previous) = self
            .mobs
            .report_distance(from, object_id, distance, Instant::now())
        else {
            return;
        };
        self.hand_over(object_id, previous, from, &handle, false);
    }

    /// Tell `previous` it no longer moves the mob, and `to` that it does.
    fn hand_over(
        &self,
        object_id: i32,
        previous: Option<ClientId>,
        to: ClientId,
        handle: &ClientHandle,
        aggro: bool,
    ) {
        if let Some(previous) = previous.filter(|&previous| previous != to) {
            if let (Some(occupant), Ok(packet)) = (
                self.occupants.get(&previous),
                build_stop_controlling_mob(object_id),
            ) {
                occupant.handle.send_packet(packet);
            }
        }
        if let Some(mob) = self.mobs.get(object_id) {
            if let Ok(packet) = build_control_mob(&mob.view(false), aggro) {
                handle.send_packet(packet);
            }
        }
    }

    /// A player hit a reactor: show its new state, or break it and have the
    /// player's client actor roll its drops.
    fn handle_hit_reactor(&mut self, from: ClientId, object_id: i32, skill_id: i32, stance: i16) {
        let Some(occupant) = self.occupants.get(&from) else {
            return;
        };
        let handle = occupant.handle.clone();
        let position = (occupant.avatar.x, occupant.avatar.y);
        match self
            .reactors
            .hit(object_id, skill_id, position, Instant::now())
        {
            Some(Hit::Moved(view)) => {
                if let Ok(packet) = build_hit_reactor(&view, stance) {
                    self.broadcast(None, packet);
                }
            }
            Some(Hit::Broke(view)) => {
                // The client plays the state's last hit animation as it
                // removes the reactor.
                if let Ok(packet) = build_remove_reactor(&view) {
                    self.broadcast(None, packet);
                }
                handle.send(ServerMessage::ReactorBroken {
                    reactor_id: view.reactor_id,
                    object_id: view.object_id,
                    x: view.x,
                    y: view.y,
                });
            }
            None => {}
        }
    }

    fn handle_join(&mut self, handle: ClientHandle, avatar: FieldAvatar, party: Option<i32>) {
        let client_id = handle.client_id;
        // A rejoin replaces the previous entry without announcing a leave.
        self.occupants.remove(&client_id);
        // Its summons come back with it.
        for summon in self.summons.remove_owned(client_id, None) {
            self.remove_summon(&summon.view, Gone::Silently);
        }

        for map_npc in &self.map_npcs {
            match build_spawn_npc(&to_foreign_npc(map_npc)) {
                Ok(packet) => {
                    handle.send_packet(packet);
                }
                Err(error) => {
                    warn!(client_id, npc_id = map_npc.npc_id, %error, "Failed to build NPC spawn")
                }
            }
        }

        for occupant in self.occupants.values() {
            match build_player_enter_field(&occupant.avatar) {
                Ok(packet) => {
                    handle.send_packet(packet);
                }
                Err(error) => warn!(client_id, %error, "Failed to build occupant spawn"),
            }
        }

        match build_player_enter_field(&avatar) {
            Ok(packet) => self.broadcast(None, packet),
            Err(error) => warn!(client_id, %error, "Failed to build join spawn"),
        }

        for mob in self.mobs.live() {
            if let Ok(packet) = build_spawn_mob(&mob.view(false)) {
                handle.send_packet(packet);
            }
        }

        for reactor in self.reactors.live() {
            if let Ok(packet) = build_spawn_reactor(&reactor.view()) {
                handle.send_packet(packet);
            }
        }

        for mist in self.mists.views() {
            if let Ok(packet) = build_spawn_mist(mist) {
                handle.send_packet(packet);
            }
        }

        // After their owners, whose summons they are.
        for summon in self.summons.views() {
            if let Ok(packet) = build_spawn_summon(summon, false) {
                handle.send_packet(packet);
            }
        }

        let now = Instant::now();
        for drop in self.drops.all().filter(|drop| drop.visible_to(client_id)) {
            if let Ok(packet) = build_drop(&drop.view(now), DropAppearance::Lying) {
                handle.send_packet(packet);
            }
        }

        self.occupants.insert(
            client_id,
            Occupant {
                handle,
                avatar,
                party,
            },
        );
        let controlled = self.mobs.assign_uncontrolled(client_id);
        self.send_control(client_id, &controlled);
        // A rejoining controller has forgotten it controls them.
        if self.npc_controller.is_none() || self.npc_controller == Some(client_id) {
            self.send_npc_control(client_id);
        }
    }

    fn handle_leave(&mut self, client_id: ClientId) {
        if self.occupants.remove(&client_id).is_none() {
            return;
        }
        // The others drop a player's summons with the player.
        self.summons.remove_owned(client_id, None);
        // Everyone sees the player go before control passes on.
        match build_player_leave_field(client_id) {
            Ok(packet) => self.broadcast(None, packet),
            Err(error) => warn!(client_id, %error, "Failed to build leave packet"),
        }
        let next = self.occupants.keys().next().copied();
        let handed = self.mobs.release(client_id, next);
        if let Some(next) = next {
            self.send_control(next, &handed);
        }
        if self.npc_controller == Some(client_id) {
            self.npc_controller = None;
            if let Some(next) = next {
                self.send_npc_control(next);
            }
        }
    }

    fn handle_npc_interaction(&self, from: ClientId, object_id: i32, request_id: u64) {
        let Some(occupant) = self.occupants.get(&from) else {
            return;
        };
        let Some(npc) = self.map_npcs.iter().find(|npc| npc.object_id == object_id) else {
            return;
        };
        // The field validates map object identity; the client actor owns the
        // conversation and serializes quest persistence with character state.
        occupant.handle.send(ServerMessage::NpcConversation {
            npc_id: npc.npc_id,
            field: self.key,
            request_id,
        });
    }

    fn handle_quest_npc(&self, from: ClientId, npc_id: i32, request_id: u64) {
        let Some(occupant) = self.occupants.get(&from) else {
            return;
        };
        let (x, y) = (i32::from(occupant.avatar.x), i32::from(occupant.avatar.y));
        let nearby = self.map_npcs.iter().any(|npc| {
            npc.npc_id == npc_id
                && (i32::from(npc.x) - x).abs() <= QUEST_NPC_REACH_X
                && (i32::from(npc.y) - y).abs() <= QUEST_NPC_REACH_Y
        });
        occupant
            .handle
            .send(ServerMessage::QuestNpcChecked { request_id, nearby });
    }

    /// Send to every occupant, optionally skipping one. Never waits.
    fn broadcast(&self, except: Option<ClientId>, packet: Packet) {
        for (&client_id, occupant) in &self.occupants {
            if Some(client_id) != except {
                occupant.handle.send_packet(packet.clone());
            }
        }
    }
}

fn to_foreign_npc(npc: &FieldMapEntityNpc) -> ForeignNpc {
    ForeignNpc {
        object_id: npc.object_id,
        npc_id: npc.npc_id,
        x: npc.x,
        y: npc.y,
        flip: npc.flip,
        foothold: npc.foothold,
        rx0: npc.rx0,
        rx1: npc.rx1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::AvatarLook;
    use net::packet::op::SendOpcode;
    use std::collections::BTreeMap;
    use tokio::time::timeout;
    use tokio_util::sync::CancellationToken;

    fn avatar(id: i32, x: i16, y: i16) -> FieldAvatar {
        FieldAvatar {
            id,
            name: format!("p{id}"),
            level: 1,
            job: 0,
            look: AvatarLook {
                gender: 0,
                skin: 0,
                face: 20000,
                hair: 30000,
                visible: BTreeMap::from([(5, 1040002)]),
                masked: BTreeMap::new(),
                cash_weapon: None,
            },
            x,
            y,
            stance: 2,
            chair: 0,
            buffs: Vec::new(),
        }
    }

    fn client(id: i32) -> (ClientHandle, mpsc::Receiver<ServerMessage>) {
        let (tx, rx) = mpsc::channel(16);
        (
            ClientHandle::new(id, id as u64, tx, CancellationToken::new()),
            rx,
        )
    }

    fn spawn_field() -> mpsc::Sender<FieldMessage> {
        let (tx, rx) = mpsc::channel(16);
        let actor = FieldActor::new(
            FieldKey::new(0, 100000000),
            rx,
            Vec::new(),
            MobSet::default(),
            DropSet::default(),
            ReactorSet::default(),
            Duration::from_millis(50),
        );
        tokio::spawn(actor.run());
        tx
    }

    async fn next_packet(rx: &mut mpsc::Receiver<ServerMessage>) -> Packet {
        match timeout(Duration::from_secs(1), rx.recv())
            .await
            .unwrap()
            .unwrap()
        {
            ServerMessage::SendPacket(packet) => packet,
            other => panic!("expected a packet, got {other:?}"),
        }
    }

    async fn next_opcode(rx: &mut mpsc::Receiver<ServerMessage>) -> i16 {
        next_packet(rx).await.opcode()
    }

    #[tokio::test]
    async fn the_map_sees_who_sits_on_a_chair() {
        let field = spawn_field();
        let (first, _first_rx) = client(1);
        let (second, mut second_rx) = client(2);
        let relaxer = 3010000i32.to_le_bytes();

        field
            .send(FieldMessage::Join {
                handle: first,
                avatar: avatar(1, 10, 20),
                party: None,
            })
            .await
            .unwrap();
        field
            .send(FieldMessage::FromClient {
                from: 1,
                request: FieldRequest::Chair(3010000),
            })
            .await
            .unwrap();
        // Someone arriving later sees the player already seated.
        field
            .send(FieldMessage::Join {
                handle: second,
                avatar: avatar(2, 30, 40),
                party: None,
            })
            .await
            .unwrap();
        let spawn = next_packet(&mut second_rx).await;
        assert_eq!(spawn.opcode(), SendOpcode::SpawnPlayer as i16);
        assert!(spawn.bytes.windows(4).any(|bytes| bytes == relaxer));

        field
            .send(FieldMessage::FromClient {
                from: 1,
                request: FieldRequest::Chair(0),
            })
            .await
            .unwrap();
        let stood = next_packet(&mut second_rx).await;
        assert_eq!(stood.opcode(), SendOpcode::ShowChair as i16);
        assert_eq!(&stood.bytes[2..], &[1, 0, 0, 0, 0, 0, 0, 0]);
    }

    fn mob(x: i16, boss: bool) -> (game_data::MapMobTemplate, game_data::mobs::MobInfo) {
        (
            game_data::MapMobTemplate {
                mob_id: 100100,
                x,
                y: -25,
                foothold: 45,
                flip: false,
                mob_time: 0,
                hide: false,
            },
            game_data::mobs::MobInfo {
                mob_id: 100100,
                max_hp: 8,
                boss,
                ..Default::default()
            },
        )
    }

    /// The opcodes `rx` gets until it goes quiet.
    async fn opcodes(rx: &mut mpsc::Receiver<ServerMessage>) -> Vec<i16> {
        let mut opcodes = Vec::new();
        while let Ok(Some(message)) = timeout(Duration::from_millis(100), rx.recv()).await {
            if let ServerMessage::SendPacket(packet) = message {
                opcodes.push(packet.opcode());
            }
        }
        opcodes
    }

    #[tokio::test]
    async fn monster_magnet_shows_its_catches_and_the_caught_chase_the_user() {
        let mut mobs = MobSet::new([mob(100, false), mob(200, true)]);
        let spawned = mobs.respawn(Instant::now());
        let (tx, rx) = mpsc::channel(16);
        let actor = FieldActor::new(
            FieldKey::new(0, 100000000),
            rx,
            Vec::new(),
            mobs,
            DropSet::default(),
            ReactorSet::default(),
            Duration::from_secs(60),
        );
        tokio::spawn(actor.run());
        let (first, mut first_rx) = client(1);
        let (second, mut second_rx) = client(2);
        for (handle, id) in [(first, 1), (second, 2)] {
            tx.send(FieldMessage::Join {
                handle,
                avatar: avatar(id, 0, 0),
                party: None,
            })
            .await
            .unwrap();
        }
        opcodes(&mut first_rx).await;
        opcodes(&mut second_rx).await;

        let effect = Packet::new_empty();
        tx.send(FieldMessage::FromClient {
            from: 2,
            request: FieldRequest::Magnet {
                effect,
                caught: vec![(spawned[0], true), (spawned[1], true), (999, true)],
            },
        })
        .await
        .unwrap();

        // The others see the snail caught and the boss not, then the skill;
        // the snail now chases the user, whose client moves it.
        let magnet = SendOpcode::ShowMagnet as i16;
        let control = SendOpcode::SpawnMonsterControl as i16;
        let seen = opcodes(&mut first_rx).await;
        assert_eq!(seen.iter().filter(|&&op| op == magnet).count(), 2);
        assert_eq!(seen.iter().filter(|&&op| op == control).count(), 1);
        assert_eq!(opcodes(&mut second_rx).await, [control]);
    }

    #[tokio::test]
    async fn occupants_see_each_other_join_and_leave() {
        let field = spawn_field();
        let (first, mut first_rx) = client(1);
        let (second, mut second_rx) = client(2);

        field
            .send(FieldMessage::Join {
                handle: first,
                avatar: avatar(1, 10, 20),
                party: None,
            })
            .await
            .unwrap();
        field
            .send(FieldMessage::Join {
                handle: second,
                avatar: avatar(2, 30, 40),
                party: None,
            })
            .await
            .unwrap();

        // The newcomer sees the existing occupant; the occupant sees the newcomer.
        assert_eq!(
            next_opcode(&mut second_rx).await,
            SendOpcode::SpawnPlayer as i16
        );
        assert_eq!(
            next_opcode(&mut first_rx).await,
            SendOpcode::SpawnPlayer as i16
        );

        field
            .send(FieldMessage::Leave { client_id: 2 })
            .await
            .unwrap();
        assert_eq!(
            next_opcode(&mut first_rx).await,
            SendOpcode::RemovePlayerFromMap as i16
        );
    }

    #[tokio::test]
    async fn a_stalled_client_cannot_block_the_field() {
        let field = spawn_field();
        let (tx, _never_read) = mpsc::channel(1);
        let kill = CancellationToken::new();
        let stalled = ClientHandle::new(1, 1, tx, kill.clone());
        let (healthy, mut healthy_rx) = client(2);

        field
            .send(FieldMessage::Join {
                handle: stalled,
                avatar: avatar(1, 0, 0),
                party: None,
            })
            .await
            .unwrap();
        field
            .send(FieldMessage::Join {
                handle: healthy,
                avatar: avatar(2, 0, 0),
                party: None,
            })
            .await
            .unwrap();
        for _ in 0..4 {
            field
                .send(FieldMessage::Chat {
                    from: 2,
                    packet: Packet::new(&[0x00, 0x00]),
                })
                .await
                .unwrap();
        }

        // The healthy client still receives everything; the stalled one is kicked.
        assert_eq!(
            next_opcode(&mut healthy_rx).await,
            SendOpcode::SpawnPlayer as i16
        );
        for _ in 0..4 {
            next_opcode(&mut healthy_rx).await;
        }
        assert!(kill.is_cancelled());
    }

    #[tokio::test]
    async fn quest_npcs_must_be_on_the_map_and_within_reach() {
        let (tx, rx) = mpsc::channel(16);
        let heena = FieldMapEntityNpc {
            object_id: 1_000_000_000,
            npc_id: 2101,
            x: 130,
            y: 185,
            flip: false,
            foothold: 0,
            rx0: 80,
            rx1: 180,
        };
        let actor = FieldActor::new(
            FieldKey::new(0, 10000),
            rx,
            vec![heena],
            MobSet::default(),
            DropSet::default(),
            ReactorSet::default(),
            Duration::from_millis(50),
        );
        tokio::spawn(actor.run());
        let (near, mut near_rx) = client(1);
        let (far, mut far_rx) = client(2);
        tx.send(FieldMessage::Join {
            handle: near,
            avatar: avatar(1, 600, 185),
            party: None,
        })
        .await
        .unwrap();
        tx.send(FieldMessage::Join {
            handle: far,
            avatar: avatar(2, 1500, 185),
            party: None,
        })
        .await
        .unwrap();

        async fn checked(rx: &mut mpsc::Receiver<ServerMessage>) -> (u64, bool) {
            loop {
                match timeout(Duration::from_secs(1), rx.recv())
                    .await
                    .unwrap()
                    .unwrap()
                {
                    ServerMessage::QuestNpcChecked { request_id, nearby } => {
                        return (request_id, nearby)
                    }
                    ServerMessage::SendPacket(_) => continue,
                    other => panic!("unexpected {other:?}"),
                }
            }
        }

        tx.send(FieldMessage::QuestNpc {
            from: 1,
            npc_id: 2101,
            request_id: 7,
        })
        .await
        .unwrap();
        assert_eq!(checked(&mut near_rx).await, (7, true));
        // Sera is not on this copy of the map.
        tx.send(FieldMessage::QuestNpc {
            from: 1,
            npc_id: 2100,
            request_id: 8,
        })
        .await
        .unwrap();
        assert_eq!(checked(&mut near_rx).await, (8, false));
        tx.send(FieldMessage::QuestNpc {
            from: 2,
            npc_id: 2101,
            request_id: 9,
        })
        .await
        .unwrap();
        assert_eq!(checked(&mut far_rx).await, (9, false));
    }

    #[tokio::test]
    async fn one_player_picks_the_npcs_actions_for_everyone() {
        let (tx, rx) = mpsc::channel(16);
        let maria = FieldMapEntityNpc {
            object_id: 1_000_000_000,
            npc_id: 2100,
            x: 130,
            y: 185,
            flip: false,
            foothold: 0,
            rx0: 80,
            rx1: 180,
        };
        let actor = FieldActor::new(
            FieldKey::new(0, 50000),
            rx,
            vec![maria],
            MobSet::default(),
            DropSet::default(),
            ReactorSet::default(),
            Duration::from_millis(50),
        );
        tokio::spawn(actor.run());
        let (first, mut first_rx) = client(1);
        let (second, mut second_rx) = client(2);
        for (handle, id) in [(first, 1), (second, 2)] {
            tx.send(FieldMessage::Join {
                handle,
                avatar: avatar(id, 100, 185),
                party: None,
            })
            .await
            .unwrap();
        }
        let control = SendOpcode::SpawnNpcRequestController as i16;
        assert!(opcodes(&mut first_rx).await.contains(&control));
        assert!(!opcodes(&mut second_rx).await.contains(&control));

        let action = |from| FieldMessage::FromClient {
            from,
            request: FieldRequest::NpcAction {
                object_id: 1_000_000_000,
                action: vec![2, 0xff],
            },
        };
        tx.send(action(2)).await.unwrap();
        assert!(opcodes(&mut first_rx).await.is_empty());
        tx.send(action(1)).await.unwrap();
        for rx in [&mut first_rx, &mut second_rx] {
            let packet = next_packet(rx).await;
            assert_eq!(packet.opcode(), SendOpcode::NpcAction as i16);
            assert_eq!(&packet.bytes[2..], &[0x00, 0xca, 0x9a, 0x3b, 2, 0xff]);
        }

        tx.send(FieldMessage::Leave { client_id: 1 }).await.unwrap();
        assert!(opcodes(&mut second_rx).await.contains(&control));
    }

    #[tokio::test]
    async fn the_target_answers_a_char_info_request() {
        let field = spawn_field();
        let (first, mut first_rx) = client(1);
        let (second, mut second_rx) = client(2);
        for (handle, id) in [(first, 1), (second, 2)] {
            field
                .send(FieldMessage::Join {
                    handle,
                    avatar: avatar(id, 0, 0),
                    party: None,
                })
                .await
                .unwrap();
        }
        next_opcode(&mut first_rx).await;
        next_opcode(&mut second_rx).await;

        // Character 3 is not here, so only the request for 2 reaches anyone;
        // the requester is unlocked instead.
        for character_id in [3, 2] {
            field
                .send(FieldMessage::FromClient {
                    from: 1,
                    request: FieldRequest::CharInfo { character_id },
                })
                .await
                .unwrap();
        }
        match timeout(Duration::from_secs(1), second_rx.recv())
            .await
            .unwrap()
            .unwrap()
        {
            ServerMessage::CharInfoRequested { requester } => assert_eq!(requester.client_id, 1),
            other => panic!("expected a char info request, got {other:?}"),
        }
        assert_eq!(
            next_opcode(&mut first_rx).await,
            SendOpcode::StatChange as i16
        );
        assert!(first_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn fame_reaches_its_target_only_on_the_givers_map() {
        let field = spawn_field();
        let (first, mut first_rx) = client(1);
        let (second, mut second_rx) = client(2);
        for (handle, id) in [(first, 1), (second, 2)] {
            field
                .send(FieldMessage::Join {
                    handle,
                    avatar: avatar(id, 0, 0),
                    party: None,
                })
                .await
                .unwrap();
        }
        next_opcode(&mut first_rx).await;
        next_opcode(&mut second_rx).await;

        for target in [3, 2] {
            field
                .send(FieldMessage::FromClient {
                    from: 1,
                    request: FieldRequest::Fame {
                        target,
                        giver_name: "Al".into(),
                        up: true,
                    },
                })
                .await
                .unwrap();
        }
        match timeout(Duration::from_secs(1), second_rx.recv())
            .await
            .unwrap()
            .unwrap()
        {
            ServerMessage::Famed {
                giver,
                giver_name,
                up,
            } => {
                assert_eq!((giver.client_id, giver_name.as_str(), up), (1, "Al", true))
            }
            other => panic!("expected fame, got {other:?}"),
        }
        match timeout(Duration::from_secs(1), first_rx.recv())
            .await
            .unwrap()
            .unwrap()
        {
            ServerMessage::FameMissed { target } => assert_eq!(target, 3),
            other => panic!("expected a miss, got {other:?}"),
        }
        assert!(first_rx.try_recv().is_err());
    }

    /// Two players on a field with a long tick, everything they got so far
    /// read.
    async fn two_players() -> (
        mpsc::Sender<FieldMessage>,
        mpsc::Receiver<ServerMessage>,
        mpsc::Receiver<ServerMessage>,
    ) {
        let field = spawn_field();
        let (first, mut first_rx) = client(1);
        let (second, mut second_rx) = client(2);
        for (handle, id) in [(first, 1), (second, 2)] {
            field
                .send(FieldMessage::Join {
                    handle,
                    avatar: avatar(id, 0, 0),
                    party: None,
                })
                .await
                .unwrap();
        }
        opcodes(&mut first_rx).await;
        opcodes(&mut second_rx).await;
        (field, first_rx, second_rx)
    }

    fn summon(skill_id: i32, at: (i16, i16), lasts: Duration) -> FieldMessage {
        FieldMessage::FromClient {
            from: 1,
            request: FieldRequest::Summon {
                skill_id,
                level: 20,
                at,
                left: true,
                lasts,
                appear: true,
            },
        }
    }

    #[tokio::test]
    async fn a_puppet_shows_to_all_takes_its_owners_hits_and_breaks() {
        if skills::data().is_err() {
            return;
        }
        let (field, mut first_rx, mut second_rx) = two_players().await;
        let puppet = summons::PUPPETS[0];
        // Too far from its owner: it goes where they stand.
        field
            .send(summon(puppet, (900, 0), Duration::from_secs(60)))
            .await
            .unwrap();
        let spawned = next_packet(&mut second_rx).await;
        assert_eq!(spawned.opcode(), SendOpcode::SpawnSummon as i16);
        let object_id = i32::from_le_bytes(spawned.bytes[6..10].try_into().unwrap());
        assert_eq!(&spawned.bytes[16..20], [0, 0, 0, 0]);
        assert_eq!(
            next_opcode(&mut first_rx).await,
            SendOpcode::SpawnSummon as i16
        );

        // Someone arriving sees it after its owner.
        let (third, mut third_rx) = client(3);
        field
            .send(FieldMessage::Join {
                handle: third,
                avatar: avatar(3, 0, 0),
                party: None,
            })
            .await
            .unwrap();
        let seen = opcodes(&mut third_rx).await;
        let at = |op: SendOpcode| {
            let op = op as i16;
            seen.iter().position(|&seen| seen == op).unwrap()
        };
        assert!(at(SendOpcode::SpawnPlayer) < at(SendOpcode::SpawnSummon));
        opcodes(&mut first_rx).await;
        opcodes(&mut second_rx).await;

        let hit = |from, damage| FieldMessage::FromClient {
            from,
            request: FieldRequest::SummonDamage(SummonDamage {
                object_id,
                attack: -1,
                damage,
                mob: Some((100100, false)),
            }),
        };
        // Only its owner's client says what hurt it.
        field.send(hit(2, 1_000_000)).await.unwrap();
        field.send(hit(1, 10)).await.unwrap();
        assert_eq!(
            next_opcode(&mut second_rx).await,
            SendOpcode::DamageSummon as i16
        );
        let hp = summons::hp(&skills::at_level(puppet, 20).unwrap()).unwrap();
        field.send(hit(1, hp)).await.unwrap();
        assert_eq!(
            next_opcode(&mut second_rx).await,
            SendOpcode::DamageSummon as i16
        );
        let removed = next_packet(&mut second_rx).await;
        assert_eq!(removed.opcode(), SendOpcode::RemoveSummon as i16);
        assert_eq!(removed.bytes[10], Gone::Broken as u8);
        assert_eq!(
            next_opcode(&mut first_rx).await,
            SendOpcode::RemoveSummon as i16
        );
        match timeout(Duration::from_secs(1), first_rx.recv()).await {
            Ok(Some(ServerMessage::SummonBroken { skill_id })) => assert_eq!(skill_id, puppet),
            other => panic!("expected the puppet broken, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_summon_attacks_within_its_limits_and_goes_when_its_time_is_up() {
        if skills::data().is_err() {
            return;
        }
        let (field, mut first_rx, mut second_rx) = two_players().await;
        let hawk = 3111005;
        field
            .send(summon(hawk, (0, 0), Duration::from_millis(300)))
            .await
            .unwrap();
        let spawned = next_packet(&mut second_rx).await;
        let object_id = i32::from_le_bytes(spawned.bytes[6..10].try_into().unwrap());
        opcodes(&mut first_rx).await;

        let attack = |damage: &[i32]| FieldMessage::FromClient {
            from: 1,
            request: FieldRequest::SummonAttack {
                attack: SummonAttack {
                    object_id,
                    action: 0x84,
                    hits: damage
                        .iter()
                        .enumerate()
                        .map(|(i, &damage)| summons::SummonHit {
                            object_id: 2_000_000_000 + i as i32,
                            action: 6,
                            damage,
                        })
                        .collect(),
                },
                limits: vec![Limit {
                    skill_id: hawk,
                    mob_count: 1,
                    max_damage: 100,
                }],
            },
        };
        field.send(attack(&[101])).await.unwrap();
        field.send(attack(&[50, 50])).await.unwrap();
        field.send(attack(&[100])).await.unwrap();
        // Only the last keeps to the limits; its owner already saw it.
        assert_eq!(
            opcodes(&mut second_rx).await,
            [SendOpcode::SummonAttack as i16]
        );
        assert!(first_rx.try_recv().is_err());

        let removed = next_packet(&mut second_rx).await;
        assert_eq!(removed.opcode(), SendOpcode::RemoveSummon as i16);
        assert_eq!(removed.bytes[10], Gone::Expired as u8);
    }
}
