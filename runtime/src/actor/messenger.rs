//! Maple Messenger rooms. The world directory owns them because members may
//! sit on different channels; this module only decides who gets which
//! packet, and the directory delivers them.

use game::AvatarLook;
use net::packet::build::world::messenger::{
    build_messenger_add, build_messenger_chat, build_messenger_declined, build_messenger_invite,
    build_messenger_invite_result, build_messenger_join, build_messenger_remove,
    build_messenger_update, MESSENGER_SLOTS,
};
use net::{error::NetworkError, ClientId};
use packet::Packet;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use tracing::error;

/// How long a member changing channel keeps their seat.
pub const CHANNEL_CHANGE_GRACE: Duration = Duration::from_secs(30);

/// Packets to deliver, in order.
pub type Outbox = Vec<(ClientId, Packet)>;

struct Member {
    client_id: ClientId,
    name: String,
    look: AvatarLook,
    /// Zero-based channel.
    channel: u8,
    /// Set while the member is between channels.
    away_since: Option<Instant>,
}

#[derive(Default)]
struct Room {
    slots: [Option<Member>; MESSENGER_SLOTS],
}

impl Room {
    fn members(&self) -> impl Iterator<Item = (u8, &Member)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(slot, member)| member.as_ref().map(|m| (slot as u8, m)))
    }

    fn slot_of(&self, client_id: ClientId) -> Option<usize> {
        self.slots
            .iter()
            .position(|m| m.as_ref().is_some_and(|m| m.client_id == client_id))
    }

    fn has_room(&self) -> bool {
        self.slots.iter().any(Option::is_none)
    }

    fn is_empty(&self) -> bool {
        self.slots.iter().all(Option::is_none)
    }
}

#[derive(Default)]
pub struct Messengers {
    rooms: HashMap<i32, Room>,
    member_of: HashMap<ClientId, i32>,
    /// Pending invitations: (invited client, messenger id).
    invites: HashSet<(ClientId, i32)>,
    next_id: i32,
}

fn push(out: &mut Outbox, to: ClientId, packet: Result<Packet, NetworkError>) {
    match packet {
        Ok(packet) => out.push((to, packet)),
        Err(e) => error!(to, error = %e, "Failed to build a messenger packet"),
    }
}

impl Messengers {
    /// Open a new messenger (`messenger_id` 0) or accept an invitation.
    pub fn open(
        &mut self,
        client_id: ClientId,
        name: &str,
        channel: u8,
        look: AvatarLook,
        messenger_id: i32,
    ) -> Outbox {
        let mut out = Outbox::new();
        if self.member_of.contains_key(&client_id) {
            return out;
        }
        let member = Member {
            client_id,
            name: name.to_string(),
            look,
            channel,
            away_since: None,
        };

        if messenger_id == 0 {
            self.next_id += 1;
            let id = self.next_id;
            let mut room = Room::default();
            room.slots[0] = Some(member);
            self.rooms.insert(id, room);
            self.member_of.insert(client_id, id);
            push(&mut out, client_id, build_messenger_join(0));
            return out;
        }

        if !self.invites.remove(&(client_id, messenger_id)) {
            return out;
        }
        let Some(room) = self.rooms.get_mut(&messenger_id) else {
            return out;
        };
        let Some(slot) = room.slots.iter().position(Option::is_none) else {
            return out;
        };
        let slot = slot as u8;
        push(&mut out, client_id, build_messenger_join(slot));
        for (other_slot, other) in room.members() {
            push(
                &mut out,
                other.client_id,
                build_messenger_add(slot, &member.look, &member.name, member.channel),
            );
            push(
                &mut out,
                client_id,
                build_messenger_add(other_slot, &other.look, &other.name, other.channel),
            );
        }
        room.slots[slot as usize] = Some(member);
        self.member_of.insert(client_id, messenger_id);
        out
    }

    pub fn leave(&mut self, client_id: ClientId) -> Outbox {
        let mut out = Outbox::new();
        let Some(id) = self.member_of.remove(&client_id) else {
            return out;
        };
        let Some(room) = self.rooms.get_mut(&id) else {
            return out;
        };
        let Some(slot) = room.slot_of(client_id) else {
            return out;
        };
        room.slots[slot] = None;
        for (_, other) in room.members() {
            push(
                &mut out,
                other.client_id,
                build_messenger_remove(slot as u8),
            );
        }
        if room.is_empty() {
            self.rooms.remove(&id);
            self.invites.retain(|&(_, room_id)| room_id != id);
        }
        out
    }

    /// Invite `target` (`None` when nobody by that name is online).
    pub fn invite(
        &mut self,
        client_id: ClientId,
        target: Option<ClientId>,
        target_name: &str,
    ) -> Outbox {
        let mut out = Outbox::new();
        let Some(&id) = self.member_of.get(&client_id) else {
            return out;
        };
        let room = &self.rooms[&id];
        let inviter = room.members().find(|(_, m)| m.client_id == client_id);
        let invitable = target.filter(|t| !self.member_of.contains_key(t) && room.has_room());
        match (inviter, invitable) {
            (Some((_, inviter)), Some(target)) => {
                push(&mut out, target, build_messenger_invite(&inviter.name, id));
                push(
                    &mut out,
                    client_id,
                    build_messenger_invite_result(target_name, true),
                );
                self.invites.insert((target, id));
            }
            _ => push(
                &mut out,
                client_id,
                build_messenger_invite_result(target_name, false),
            ),
        }
        out
    }

    /// `client_id` (named `name`) turned down `inviter`'s invitation.
    pub fn decline(&mut self, client_id: ClientId, name: &str, inviter: ClientId) -> Outbox {
        let mut out = Outbox::new();
        let Some(&id) = self.member_of.get(&inviter) else {
            return out;
        };
        if self.invites.remove(&(client_id, id)) {
            push(&mut out, inviter, build_messenger_declined(name));
        }
        out
    }

    pub fn chat(&self, client_id: ClientId, line: &str) -> Outbox {
        let mut out = Outbox::new();
        let Some(room) = self
            .member_of
            .get(&client_id)
            .and_then(|id| self.rooms.get(id))
        else {
            return out;
        };
        for (_, other) in room.members() {
            if other.client_id != client_id {
                push(&mut out, other.client_id, build_messenger_chat(line));
            }
        }
        out
    }

    pub fn look_changed(&mut self, client_id: ClientId, look: AvatarLook) -> Outbox {
        self.update_member(client_id, |member| member.look = look)
    }

    /// The client went offline. A member changing channel keeps their seat
    /// for a while; anyone else leaves.
    pub fn disconnected(
        &mut self,
        client_id: ClientId,
        changing_channel: bool,
        now: Instant,
    ) -> Outbox {
        self.invites.retain(|&(invited, _)| invited != client_id);
        if !changing_channel {
            return self.leave(client_id);
        }
        if let Some(member) = self.member_mut(client_id) {
            member.away_since = Some(now);
        }
        Outbox::new()
    }

    /// The client came online on `channel`; if they kept a seat, show them
    /// the room again and tell the others where they are now.
    pub fn connected(&mut self, client_id: ClientId, channel: u8) -> Outbox {
        let Some(member) = self.member_mut(client_id) else {
            return Outbox::new();
        };
        member.away_since = None;
        let mut out = Outbox::new();
        let room = &self.rooms[&self.member_of[&client_id]];
        if let Some(own_slot) = room.slot_of(client_id) {
            push(&mut out, client_id, build_messenger_join(own_slot as u8));
        }
        for (slot, other) in room.members() {
            if other.client_id != client_id {
                push(
                    &mut out,
                    client_id,
                    build_messenger_add(slot, &other.look, &other.name, other.channel),
                );
            }
        }
        out.extend(self.update_member(client_id, |member| member.channel = channel));
        out
    }

    /// Drop members whose channel change never finished.
    pub fn purge(&mut self, now: Instant) -> Outbox {
        let stale: Vec<ClientId> = self
            .rooms
            .values()
            .flat_map(Room::members)
            .filter(|(_, m)| {
                m.away_since
                    .is_some_and(|since| now.duration_since(since) >= CHANNEL_CHANGE_GRACE)
            })
            .map(|(_, m)| m.client_id)
            .collect();
        stale
            .into_iter()
            .flat_map(|client_id| self.leave(client_id))
            .collect()
    }

    fn member_mut(&mut self, client_id: ClientId) -> Option<&mut Member> {
        let room = self.rooms.get_mut(self.member_of.get(&client_id)?)?;
        let slot = room.slot_of(client_id)?;
        room.slots[slot].as_mut()
    }

    /// Change a member, then show the others their new state.
    fn update_member(&mut self, client_id: ClientId, change: impl FnOnce(&mut Member)) -> Outbox {
        let mut out = Outbox::new();
        let Some(member) = self.member_mut(client_id) else {
            return out;
        };
        change(member);
        let room = &self.rooms[&self.member_of[&client_id]];
        let Some(slot) = room.slot_of(client_id) else {
            return out;
        };
        let member = room.slots[slot].as_ref().expect("slot_of found the member");
        for (_, other) in room.members() {
            if other.client_id != client_id {
                push(
                    &mut out,
                    other.client_id,
                    build_messenger_update(slot as u8, &member.look, &member.name, member.channel),
                );
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    const ALICE: ClientId = 1;
    const BOB: ClientId = 2;
    const CAROL: ClientId = 3;
    const DAN: ClientId = 4;

    fn look(hair: i32) -> AvatarLook {
        AvatarLook {
            gender: 0,
            skin: 0,
            face: 20000,
            hair,
            visible: BTreeMap::new(),
            masked: BTreeMap::new(),
            cash_weapon: None,
            pets: [0; 3],
        }
    }

    /// (recipient, mode, the byte after the mode) for each packet.
    fn summary(out: &Outbox) -> Vec<(ClientId, u8, u8)> {
        out.iter()
            .map(|(to, packet)| (*to, packet.bytes[2], packet.bytes[3]))
            .collect()
    }

    /// Alice opens messenger 1 and Bob joins it.
    fn alice_and_bob() -> Messengers {
        let mut messengers = Messengers::default();
        messengers.open(ALICE, "Alice", 0, look(30000), 0);
        messengers.invite(ALICE, Some(BOB), "Bob");
        messengers.open(BOB, "Bob", 1, look(30010), 1);
        messengers
    }

    #[test]
    fn opening_seats_the_player_in_the_first_slot() {
        let mut messengers = Messengers::default();
        let out = messengers.open(ALICE, "Alice", 0, look(30000), 0);
        assert_eq!(summary(&out), vec![(ALICE, 0x01, 0)]);
        // A second open while seated does nothing.
        assert!(messengers
            .open(ALICE, "Alice", 0, look(30000), 0)
            .is_empty());
    }

    #[test]
    fn an_invited_player_joins_and_both_sides_see_each_other() {
        let mut messengers = Messengers::default();
        messengers.open(ALICE, "Alice", 0, look(30000), 0);

        let invite = messengers.invite(ALICE, Some(BOB), "Bob");
        assert_eq!(
            invite
                .iter()
                .map(|(to, p)| (*to, p.bytes[2]))
                .collect::<Vec<_>>(),
            vec![(BOB, 0x03), (ALICE, 0x04)]
        );
        assert_eq!(*invite[1].1.bytes.last().unwrap(), 1, "invite sent");

        let join = messengers.open(BOB, "Bob", 1, look(30010), 1);
        assert_eq!(
            summary(&join),
            vec![(BOB, 0x01, 1), (ALICE, 0x00, 1), (BOB, 0x00, 0)]
        );
    }

    #[test]
    fn joining_needs_an_invitation() {
        let mut messengers = Messengers::default();
        messengers.open(ALICE, "Alice", 0, look(30000), 0);
        assert!(messengers.open(BOB, "Bob", 0, look(30010), 1).is_empty());

        // An invitation works once.
        let mut messengers = alice_and_bob();
        messengers.leave(BOB);
        assert!(messengers.open(BOB, "Bob", 1, look(30010), 1).is_empty());
    }

    #[test]
    fn invitations_fail_for_missing_busy_or_full_targets() {
        let mut messengers = alice_and_bob();
        let failed = |out: Outbox| {
            assert_eq!(out.len(), 1);
            assert_eq!(out[0].1.bytes[2], 0x04);
            assert_eq!(*out[0].1.bytes.last().unwrap(), 0);
        };
        failed(messengers.invite(ALICE, None, "Nobody"));
        failed(messengers.invite(ALICE, Some(BOB), "Bob"));

        messengers.invite(ALICE, Some(CAROL), "Carol");
        messengers.open(CAROL, "Carol", 0, look(30020), 1);
        failed(messengers.invite(ALICE, Some(DAN), "Dan"));
    }

    #[test]
    fn declining_tells_the_inviter_and_voids_the_invitation() {
        let mut messengers = Messengers::default();
        messengers.open(ALICE, "Alice", 0, look(30000), 0);
        messengers.invite(ALICE, Some(BOB), "Bob");

        let out = messengers.decline(BOB, "Bob", ALICE);
        assert_eq!(summary(&out), vec![(ALICE, 0x05, 3)]);
        assert!(messengers.decline(BOB, "Bob", ALICE).is_empty());
        assert!(messengers.open(BOB, "Bob", 0, look(30010), 1).is_empty());
    }

    #[test]
    fn chat_reaches_the_other_members_only() {
        let messengers = alice_and_bob();
        let out = messengers.chat(ALICE, "Alice : hi");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0, BOB);
        assert!(messengers.chat(CAROL, "Carol : hi").is_empty());
    }

    #[test]
    fn leaving_frees_the_slot_and_the_last_member_closes_the_room() {
        let mut messengers = alice_and_bob();
        assert_eq!(summary(&messengers.leave(ALICE)), vec![(BOB, 0x02, 0)]);
        assert!(messengers.leave(BOB).is_empty());
        assert!(messengers.rooms.is_empty());
        assert!(messengers.member_of.is_empty());
    }

    #[test]
    fn a_look_change_updates_the_others() {
        let mut messengers = alice_and_bob();
        let out = messengers.look_changed(BOB, look(30030));
        assert_eq!(summary(&out), vec![(ALICE, 0x07, 1)]);
    }

    #[test]
    fn changing_channel_keeps_the_seat_and_reannounces_it() {
        let mut messengers = alice_and_bob();
        let now = Instant::now();
        assert!(messengers.disconnected(BOB, true, now).is_empty());
        assert!(messengers.purge(now + Duration::from_secs(1)).is_empty());

        let out = messengers.connected(BOB, 2);
        assert_eq!(
            summary(&out),
            vec![(BOB, 0x01, 1), (BOB, 0x00, 0), (ALICE, 0x07, 1)]
        );
        // The update names Bob's new channel, then the trailing zero.
        let update = &out[2].1.bytes;
        assert_eq!(update[update.len() - 2..], [2, 0]);
    }

    #[test]
    fn an_unfinished_channel_change_gives_up_the_seat() {
        let mut messengers = alice_and_bob();
        let now = Instant::now();
        messengers.disconnected(BOB, true, now);
        let out = messengers.purge(now + CHANNEL_CHANGE_GRACE);
        assert_eq!(summary(&out), vec![(ALICE, 0x02, 1)]);
        assert!(messengers.connected(BOB, 0).is_empty());
    }

    #[test]
    fn logging_out_leaves_and_drops_pending_invitations() {
        let mut messengers = alice_and_bob();
        messengers.invite(ALICE, Some(CAROL), "Carol");
        messengers.disconnected(CAROL, false, Instant::now());
        assert!(messengers
            .open(CAROL, "Carol", 0, look(30020), 1)
            .is_empty());

        let out = messengers.disconnected(BOB, false, Instant::now());
        assert_eq!(summary(&out), vec![(ALICE, 0x02, 1)]);
    }
}
