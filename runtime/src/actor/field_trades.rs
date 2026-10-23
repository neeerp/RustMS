//! Trading rooms on one field. Pure state: this module decides who gets
//! which message, and `FieldActor` delivers them. Each member's client actor
//! holds what its player put up, out of their inventory; the room keeps
//! copies to show the partner and to hand over for the final check.
//!
//! When both members pressed Trade, each gets the partner's offer to check
//! it can take it, and answers with a oneshot for the decision. Once both
//! answered, the room resolves both oneshots (go ahead only if both can) and
//! closes; a member leaving first resolves any it holds with `false`.

use crate::message::ServerMessage;
use game::{AvatarLook, Item};
use net::error::NetworkError;
use net::packet::build::world::mini_room::{
    build_enter_error, build_invite_not_found, build_invite_result, build_room_chat,
    build_room_leave, build_room_visit, build_trade_confirmed, build_trade_invite,
    build_trade_item, build_trade_meso, build_trade_room, enter_error, invite_result, leave_reason,
};
use net::trade::{Offer, TradeFailure, TRADE_SLOTS};
use net::ClientId;
use packet::Packet;
use std::collections::HashMap;
use tokio::sync::oneshot;
use tracing::{debug, error};

/// What a client asks of its trading room.
#[derive(Debug)]
pub enum RoomRequest {
    /// Open room `room`, the sender in slot 0.
    Create {
        room: i32,
    },
    /// Invite the character with this id, who must be on the field.
    Invite {
        target: ClientId,
    },
    /// Turn down `room`'s invitation; `reason` is an `invite_result`.
    Decline {
        room: i32,
        reason: u8,
    },
    /// Accept `room`'s invitation, or go into a shop or game; a game
    /// may want `password`, and shows the visitor's `records` there.
    Visit {
        room: i32,
        password: Option<String>,
        records: [game::GameRecord; 2],
    },
    Chat {
        text: String,
    },
    /// The sender put `item` up in trade slot `slot`.
    PutItem {
        slot: u8,
        item: Item,
    },
    /// The sender's mesos up are now `total`.
    Meso {
        total: i32,
    },
    /// The sender pressed Trade.
    Confirm,
    /// Whether the sender can take the partner's offer, and where the
    /// room's decision goes.
    Answer {
        result: Result<(), TradeFailure>,
        decision: oneshot::Sender<bool>,
    },
    Leave,
}

/// What a trading room tells a member's client actor.
#[derive(Debug)]
pub enum TradeEvent {
    /// The partner is in: offers may start.
    Started,
    /// Both pressed Trade: check the partner's offer can be taken, and
    /// answer.
    Check(Offer),
    /// The room closed. Finish the trade (its oneshot, if any, is
    /// resolved), then send the packet.
    Ended(Option<Packet>),
}

/// Messages to deliver, in order.
pub type Outbox = Vec<(ClientId, ServerMessage)>;

struct Member {
    client_id: ClientId,
    name: String,
    look: AvatarLook,
    /// A copy of what the member put up.
    offer: Offer,
    confirmed: bool,
    /// Its answer to the final check, and where the decision goes.
    answer: Option<(Result<(), TradeFailure>, oneshot::Sender<bool>)>,
}

impl Member {
    fn new(client_id: ClientId, name: &str, look: &AvatarLook) -> Self {
        Self {
            client_id,
            name: name.to_string(),
            look: look.clone(),
            offer: Offer::default(),
            confirmed: false,
            answer: None,
        }
    }
}

/// A trading room: the creator in slot 0, then the one they invited.
struct Room {
    members: Vec<Member>,
    invitee: Option<ClientId>,
}

#[derive(Default)]
pub struct Trades {
    rooms: HashMap<i32, Room>,
    member_of: HashMap<ClientId, i32>,
}

fn packet(out: &mut Outbox, to: ClientId, packet: Result<Packet, NetworkError>) {
    match packet {
        Ok(packet) => out.push((to, ServerMessage::SendPacket(packet))),
        Err(e) => error!(to, error = %e, "Failed to build a trade packet"),
    }
}

fn event(out: &mut Outbox, to: ClientId, room: i32, event: TradeEvent) {
    out.push((to, ServerMessage::Trade { room, event }));
}

impl Trades {
    /// The room `client_id` is in, with their slot.
    fn room_of(&mut self, client_id: ClientId) -> Option<(i32, &mut Room, usize)> {
        let id = *self.member_of.get(&client_id)?;
        let room = self.rooms.get_mut(&id)?;
        let slot = room.members.iter().position(|m| m.client_id == client_id)?;
        Some((id, room, slot))
    }

    pub fn is_member(&self, client_id: ClientId) -> bool {
        self.member_of.contains_key(&client_id)
    }

    /// `busy`: the creator is in another kind of room.
    pub fn create(
        &mut self,
        room: i32,
        client_id: ClientId,
        name: &str,
        look: &AvatarLook,
        busy: bool,
    ) -> Outbox {
        let mut out = Outbox::new();
        if busy || self.member_of.contains_key(&client_id) || self.rooms.contains_key(&room) {
            event(&mut out, client_id, room, TradeEvent::Ended(None));
            return out;
        }
        packet(&mut out, client_id, build_trade_room(0, &[(0, look, name)]));
        self.rooms.insert(
            room,
            Room {
                members: vec![Member::new(client_id, name, look)],
                invitee: None,
            },
        );
        self.member_of.insert(client_id, room);
        out
    }

    /// Invite `target`, named `target_name` if they are on the field. A new
    /// invitation replaces the room's last one.
    pub fn invite(
        &mut self,
        from: ClientId,
        target: ClientId,
        target_name: Option<&str>,
    ) -> Outbox {
        let mut out = Outbox::new();
        let busy = self.member_of.contains_key(&target)
            || self.rooms.values().any(|room| room.invitee == Some(target));
        let Some((id, room, 0)) = self.room_of(from) else {
            return out;
        };
        if room.members.len() > 1 {
            return out;
        }
        let Some(target_name) = target_name else {
            packet(&mut out, from, build_invite_not_found());
            return out;
        };
        if busy {
            packet(
                &mut out,
                from,
                build_invite_result(invite_result::BUSY, target_name),
            );
            return out;
        }
        room.invitee = Some(target);
        packet(
            &mut out,
            target,
            build_trade_invite(&room.members[0].name, id),
        );
        out
    }

    /// `from`, named `name`, turned down `room`'s invitation.
    pub fn decline(&mut self, from: ClientId, room: i32, reason: u8, name: &str) -> Outbox {
        let mut out = Outbox::new();
        let Some(room) = self
            .rooms
            .get_mut(&room)
            .filter(|room| room.invitee == Some(from))
        else {
            return out;
        };
        room.invitee = None;
        let reason = match reason {
            invite_result::NOT_ACCEPTING => reason,
            _ => invite_result::DECLINED,
        };
        packet(
            &mut out,
            room.members[0].client_id,
            build_invite_result(reason, name),
        );
        out
    }

    /// `from` accepts `room`'s invitation; the room must still wait for them.
    /// `busy`: they are in another kind of room.
    pub fn visit(
        &mut self,
        from: ClientId,
        room: i32,
        name: &str,
        look: &AvatarLook,
        busy: bool,
    ) -> Outbox {
        let mut out = Outbox::new();
        let waiting = !busy
            && !self.member_of.contains_key(&from)
            && self
                .rooms
                .get(&room)
                .is_some_and(|r| r.invitee == Some(from) && r.members.len() == 1);
        if !waiting {
            packet(&mut out, from, build_enter_error(enter_error::CLOSED));
            event(&mut out, from, room, TradeEvent::Ended(None));
            return out;
        }
        let id = room;
        let room = self.rooms.get_mut(&id).expect("checked above");
        room.invitee = None;
        room.members.push(Member::new(from, name, look));
        // Before the packets, so an offer the player makes on seeing them
        // finds the trade started.
        for member in &room.members {
            event(&mut out, member.client_id, id, TradeEvent::Started);
        }
        let host = &room.members[0];
        packet(
            &mut out,
            from,
            build_trade_room(1, &[(0, &host.look, &host.name), (1, look, name)]),
        );
        packet(&mut out, host.client_id, build_room_visit(1, look, name));
        self.member_of.insert(from, id);
        out
    }

    pub fn chat(&mut self, from: ClientId, text: &str) -> Outbox {
        let mut out = Outbox::new();
        let Some((_, room, slot)) = self.room_of(from) else {
            return out;
        };
        let line = format!("{} : {}", room.members[slot].name, text);
        for member in &room.members {
            packet(
                &mut out,
                member.client_id,
                build_room_chat(slot as u8, &line),
            );
        }
        out
    }

    /// Whether the member in `slot` may still change their offer.
    fn open_offer(room: &Room, slot: usize) -> bool {
        room.members.len() == 2 && !room.members[slot].confirmed
    }

    pub fn put_item(&mut self, from: ClientId, trade_slot: u8, item: Item) -> Outbox {
        let mut out = Outbox::new();
        let Some((id, room, slot)) = self.room_of(from) else {
            return out;
        };
        let free = (1..=TRADE_SLOTS).contains(&trade_slot)
            && !room.members[slot].offer.items.contains_key(&trade_slot);
        if !Self::open_offer(room, slot) || !free {
            // The client already took the item; the room's copy must match.
            debug!(from, trade_slot, "Trade item out of turn; closing the room");
            return self.close(id, leave_reason::FAILED);
        }
        for (other, member) in room.members.iter().enumerate() {
            packet(
                &mut out,
                member.client_id,
                build_trade_item(other != slot, trade_slot, &item),
            );
        }
        room.members[slot].offer.items.insert(trade_slot, item);
        out
    }

    pub fn meso(&mut self, from: ClientId, total: i32) -> Outbox {
        let mut out = Outbox::new();
        let Some((id, room, slot)) = self.room_of(from) else {
            return out;
        };
        if !Self::open_offer(room, slot) {
            debug!(from, total, "Trade mesos out of turn; closing the room");
            return self.close(id, leave_reason::FAILED);
        }
        room.members[slot].offer.meso = total;
        for (other, member) in room.members.iter().enumerate() {
            packet(
                &mut out,
                member.client_id,
                build_trade_meso(other != slot, total),
            );
        }
        out
    }

    /// `from` pressed Trade; once both have, each checks the other's offer.
    pub fn confirm(&mut self, from: ClientId) -> Outbox {
        let mut out = Outbox::new();
        let Some((id, room, slot)) = self.room_of(from) else {
            return out;
        };
        if !Self::open_offer(room, slot) {
            return out;
        }
        room.members[slot].confirmed = true;
        let partner = &room.members[1 - slot];
        packet(&mut out, partner.client_id, build_trade_confirmed());
        if !partner.confirmed {
            return out;
        }
        for (slot, member) in room.members.iter().enumerate() {
            let incoming = room.members[1 - slot].offer.clone();
            event(&mut out, member.client_id, id, TradeEvent::Check(incoming));
        }
        out
    }

    /// `from`'s answer to the final check. Once both are in, the trade goes
    /// through only if both can take what they get.
    pub fn answer(
        &mut self,
        from: ClientId,
        result: Result<(), TradeFailure>,
        decision: oneshot::Sender<bool>,
    ) -> Outbox {
        let Some((id, room, slot)) = self.room_of(from) else {
            // Dropping `decision` tells the client the trade is off.
            return Outbox::new();
        };
        let checking = room.members.len() == 2 && room.members.iter().all(|m| m.confirmed);
        if !checking || room.members[slot].answer.is_some() {
            return Outbox::new();
        }
        room.members[slot].answer = Some((result, decision));
        if room.members.iter().any(|m| m.answer.is_none()) {
            return Outbox::new();
        }
        let failure = room
            .members
            .iter()
            .find_map(|m| m.answer.as_ref().and_then(|(result, _)| result.err()));
        for member in &mut room.members {
            if let Some((_, decision)) = member.answer.take() {
                let _ = decision.send(failure.is_none());
            }
        }
        self.close(id, failure.map_or(leave_reason::DONE, TradeFailure::reason))
    }

    /// `client_id` left the room or the field. Their partner, if any, is
    /// told the trade was cancelled.
    pub fn leave(&mut self, client_id: ClientId) -> Outbox {
        for room in self.rooms.values_mut() {
            if room.invitee == Some(client_id) {
                room.invitee = None;
            }
        }
        let Some(id) = self.member_of.get(&client_id).copied() else {
            return Outbox::new();
        };
        let mut out = self.close(id, leave_reason::CANCELLED);
        out.retain(|(to, _)| *to != client_id);
        out
    }

    /// Close a room: any decision still held is `false`, and every member
    /// gets the LEAVE with `reason` for their slot.
    fn close(&mut self, id: i32, reason: u8) -> Outbox {
        let mut out = Outbox::new();
        let Some(room) = self.rooms.remove(&id) else {
            return out;
        };
        for (slot, member) in room.members.into_iter().enumerate() {
            if let Some((_, decision)) = member.answer {
                let _ = decision.send(false);
            }
            self.member_of.remove(&member.client_id);
            let leave = build_room_leave(slot as u8, reason)
                .map_err(|e| error!(error = %e, "Failed to build a trade packet"))
                .ok();
            event(&mut out, member.client_id, id, TradeEvent::Ended(leave));
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
    const ROOM: i32 = 77;

    fn look() -> AvatarLook {
        AvatarLook {
            gender: 0,
            skin: 0,
            face: 20000,
            hair: 30000,
            visible: BTreeMap::new(),
            masked: BTreeMap::new(),
            cash_weapon: None,
            pets: [0; 3],
        }
    }

    /// The mode byte of a packet, or what kind of event it is.
    fn summary(out: &Outbox) -> Vec<(ClientId, String)> {
        out.iter()
            .map(|(to, message)| {
                let what = match message {
                    ServerMessage::SendPacket(packet) => format!("{:#x}", packet.bytes[2]),
                    ServerMessage::Trade { event, .. } => match event {
                        TradeEvent::Started => "started".to_string(),
                        TradeEvent::Check(offer) => format!("check {}", offer.meso),
                        TradeEvent::Ended(Some(packet)) => format!("ended {}", packet.bytes[4]),
                        TradeEvent::Ended(None) => "ended".to_string(),
                    },
                    other => format!("{other:?}"),
                };
                (*to, what)
            })
            .collect()
    }

    fn s(to: ClientId, what: &str) -> (ClientId, String) {
        (to, what.to_string())
    }

    fn open_room() -> Trades {
        let mut trades = Trades::default();
        trades.create(ROOM, ALICE, "Alice", &look(), false);
        trades.invite(ALICE, BOB, Some("Bob"));
        trades.visit(BOB, ROOM, "Bob", &look(), false);
        trades
    }

    #[test]
    fn invite_and_visit_fill_the_room() {
        let mut trades = Trades::default();
        let out = trades.create(ROOM, ALICE, "Alice", &look(), false);
        assert_eq!(summary(&out), vec![s(ALICE, "0x5")]);
        let out = trades.invite(ALICE, CAROL, None);
        assert_eq!(summary(&out), vec![s(ALICE, "0x3")]);
        let out = trades.invite(ALICE, BOB, Some("Bob"));
        assert_eq!(summary(&out), vec![s(BOB, "0x2")]);
        // Someone not invited finds it closed.
        let out = trades.visit(CAROL, ROOM, "Carol", &look(), false);
        assert_eq!(summary(&out), vec![s(CAROL, "0x5"), s(CAROL, "ended")]);
        let out = trades.visit(BOB, ROOM, "Bob", &look(), false);
        assert_eq!(
            summary(&out),
            vec![
                s(ALICE, "started"),
                s(BOB, "started"),
                s(BOB, "0x5"),
                s(ALICE, "0x4")
            ]
        );
        // Bob is busy now.
        trades.create(ROOM + 1, CAROL, "Carol", &look(), false);
        let out = trades.invite(CAROL, BOB, Some("Bob"));
        assert_eq!(summary(&out), vec![s(CAROL, "0x3")]);
        assert_eq!(out_bytes(&out)[3], 2);
    }

    fn out_bytes(out: &Outbox) -> Vec<u8> {
        match &out[0].1 {
            ServerMessage::SendPacket(packet) => packet.bytes.clone(),
            other => panic!("not a packet: {other:?}"),
        }
    }

    #[test]
    fn a_decline_reaches_the_inviter_and_keeps_the_room() {
        let mut trades = Trades::default();
        trades.create(ROOM, ALICE, "Alice", &look(), false);
        trades.invite(ALICE, BOB, Some("Bob"));
        let out = trades.decline(BOB, ROOM, 3, "Bob");
        assert_eq!(summary(&out), vec![s(ALICE, "0x3")]);
        assert_eq!(out_bytes(&out)[3], 3);
        // The invitation is spent.
        let out = trades.visit(BOB, ROOM, "Bob", &look(), false);
        assert_eq!(summary(&out), vec![s(BOB, "0x5"), s(BOB, "ended")]);
        let out = trades.invite(ALICE, BOB, Some("Bob"));
        assert_eq!(summary(&out), vec![s(BOB, "0x2")]);
    }

    #[test]
    fn offers_show_on_both_sides_until_confirmed() {
        let mut trades = open_room();
        let out = trades.put_item(ALICE, 1, Item::stack(4000000, 3));
        assert_eq!(summary(&out), vec![s(ALICE, "0xf"), s(BOB, "0xf")]);
        let out = trades.meso(BOB, 500);
        assert_eq!(summary(&out), vec![s(ALICE, "0x10"), s(BOB, "0x10")]);
        let out = trades.chat(BOB, "hi");
        assert_eq!(summary(&out), vec![s(ALICE, "0x6"), s(BOB, "0x6")]);

        let out = trades.confirm(ALICE);
        assert_eq!(summary(&out), vec![s(BOB, "0x11")]);
        let out = trades.confirm(BOB);
        assert_eq!(
            summary(&out),
            vec![s(ALICE, "0x11"), s(ALICE, "check 500"), s(BOB, "check 0")]
        );
    }

    #[test]
    fn an_item_out_of_turn_closes_the_room() {
        let mut trades = open_room();
        trades.put_item(ALICE, 1, Item::stack(4000000, 3));
        let out = trades.put_item(ALICE, 1, Item::stack(4000000, 3));
        assert_eq!(summary(&out), vec![s(ALICE, "ended 8"), s(BOB, "ended 8")]);
        assert!(trades.rooms.is_empty() && trades.member_of.is_empty());
    }

    fn answer(
        trades: &mut Trades,
        from: ClientId,
        result: Result<(), TradeFailure>,
    ) -> (Outbox, oneshot::Receiver<bool>) {
        let (decision, rx) = oneshot::channel();
        (trades.answer(from, result, decision), rx)
    }

    #[test]
    fn both_answers_decide_the_trade() {
        let mut trades = open_room();
        trades.confirm(ALICE);
        trades.confirm(BOB);
        let (out, mut alice) = answer(&mut trades, ALICE, Ok(()));
        assert!(out.is_empty());
        assert!(alice.try_recv().is_err());
        let (out, mut bob) = answer(&mut trades, BOB, Ok(()));
        assert_eq!(summary(&out), vec![s(ALICE, "ended 7"), s(BOB, "ended 7")]);
        assert_eq!((alice.try_recv(), bob.try_recv()), (Ok(true), Ok(true)));
        assert!(trades.rooms.is_empty());

        let mut trades = open_room();
        trades.confirm(ALICE);
        trades.confirm(BOB);
        let (_, mut alice) = answer(&mut trades, ALICE, Ok(()));
        let (out, mut bob) = answer(&mut trades, BOB, Err(TradeFailure::OnlyOne));
        assert_eq!(summary(&out), vec![s(ALICE, "ended 9"), s(BOB, "ended 9")]);
        assert_eq!((alice.try_recv(), bob.try_recv()), (Ok(false), Ok(false)));
    }

    #[test]
    fn leaving_cancels_for_the_partner_and_any_held_decision() {
        let mut trades = open_room();
        trades.confirm(ALICE);
        trades.confirm(BOB);
        let (_, mut alice) = answer(&mut trades, ALICE, Ok(()));
        let out = trades.leave(BOB);
        assert_eq!(summary(&out), vec![s(ALICE, "ended 2")]);
        assert_eq!(alice.try_recv(), Ok(false));
        // A late answer finds no room and its decision is dropped.
        let (out, mut bob) = answer(&mut trades, BOB, Ok(()));
        assert!(out.is_empty());
        assert!(matches!(
            bob.try_recv(),
            Err(oneshot::error::TryRecvError::Closed)
        ));
    }

    #[test]
    fn a_room_alone_just_closes_and_its_invitation_goes_stale() {
        let mut trades = Trades::default();
        trades.create(ROOM, ALICE, "Alice", &look(), false);
        trades.invite(ALICE, BOB, Some("Bob"));
        assert!(trades.leave(ALICE).is_empty());
        let out = trades.visit(BOB, ROOM, "Bob", &look(), false);
        assert_eq!(summary(&out), vec![s(BOB, "0x5"), s(BOB, "ended")]);
    }
}
