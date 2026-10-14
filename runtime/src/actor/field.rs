use crate::actor::field_drops::{Drop, DropSet};
use crate::actor::field_mobs::{split_exp, MobSet, Sharer};
use crate::message::{ClientHandle, FieldKey, FieldMessage, ServerMessage};
use game::FieldAvatar;
use net::packet::build::world::drop::{build_drop, build_remove_drop, DropAppearance};
use net::packet::build::world::field::{
    build_player_enter_field, build_player_leave_field, parse_movement_state,
};
use net::packet::build::world::mob::{
    build_control_mob, build_kill_mob, build_mob_hp, build_move_mob, build_move_mob_response,
    build_spawn_mob,
};
use net::packet::build::world::npc::{build_spawn_npc, ForeignNpc};
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

/// One live copy of a map. It owns everything local to that map: who is on
/// it and where, its NPCs, and (later) mobs, drops, and reactors, which will
/// advance on `tick`.
pub struct FieldActor {
    key: FieldKey,
    rx: mpsc::Receiver<FieldMessage>,
    occupants: HashMap<ClientId, Occupant>,
    map_npcs: Vec<FieldMapEntityNpc>,
    mobs: MobSet,
    drops: DropSet,
    tick: Duration,
}

impl FieldActor {
    pub fn new(
        key: FieldKey,
        rx: mpsc::Receiver<FieldMessage>,
        map_npcs: Vec<FieldMapEntityNpc>,
        mut mobs: MobSet,
        drops: DropSet,
        tick: Duration,
    ) -> Self {
        mobs.respawn(Instant::now());
        Self {
            key,
            rx,
            occupants: HashMap::new(),
            map_npcs,
            mobs,
            drops,
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

    /// Advance time-driven field state: mob respawns (drop expiry later).
    fn on_tick(&mut self, now: Instant) {
        for object_id in self.drops.expire(now) {
            if let Ok(packet) = build_remove_drop(object_id, None) {
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
                if let Ok(packet) = build_control_mob(&mob.view(false)) {
                    occupant.handle.send_packet(packet);
                }
            }
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
                    // Keep the live position; take everything else.
                    let (x, y, stance) =
                        (occupant.avatar.x, occupant.avatar.y, occupant.avatar.stance);
                    occupant.avatar = FieldAvatar {
                        x,
                        y,
                        stance,
                        ..avatar
                    };
                }
            }
            FieldMessage::Chat { packet, .. } => self.broadcast(None, packet),
            FieldMessage::Move {
                from,
                packet,
                movement_bytes,
            } => {
                if let Some(occupant) = self.occupants.get_mut(&from) {
                    let avatar = &mut occupant.avatar;
                    if let Some((x, y, stance)) =
                        parse_movement_state(&movement_bytes, (avatar.x, avatar.y, avatar.stance))
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
                FieldRequest::Attack { broadcast, hits } => {
                    self.handle_attack(from, broadcast, hits)
                }
                FieldRequest::Broadcast(packet) => self.broadcast(Some(from), packet),
                FieldRequest::Pickup { object_id, x, y } => {
                    self.handle_pickup(from, object_id, x, y)
                }
                FieldRequest::Throw { loot, vanish } => self.handle_throw(from, loot, vanish),
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
        if !self.mobs.move_mob(from, object_id, movement) {
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

    fn handle_attack(&mut self, from: ClientId, broadcast: Packet, hits: Vec<(i32, i64)>) {
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

    fn handle_join(&mut self, handle: ClientHandle, avatar: FieldAvatar, party: Option<i32>) {
        let client_id = handle.client_id;
        // A rejoin replaces the previous entry without announcing a leave.
        self.occupants.remove(&client_id);

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
    }

    fn handle_leave(&mut self, client_id: ClientId) {
        if self.occupants.remove(&client_id).is_none() {
            return;
        }
        let next = self.occupants.keys().next().copied();
        let handed = self.mobs.release(client_id, next);
        if let Some(next) = next {
            self.send_control(next, &handed);
        }
        match build_player_leave_field(client_id) {
            Ok(packet) => self.broadcast(None, packet),
            Err(error) => warn!(client_id, %error, "Failed to build leave packet"),
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
            Duration::from_millis(50),
        );
        tokio::spawn(actor.run());
        tx
    }

    async fn next_opcode(rx: &mut mpsc::Receiver<ServerMessage>) -> i16 {
        match timeout(Duration::from_secs(1), rx.recv())
            .await
            .unwrap()
            .unwrap()
        {
            ServerMessage::SendPacket(packet) => packet.opcode(),
            other => panic!("expected a packet, got {other:?}"),
        }
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
}
