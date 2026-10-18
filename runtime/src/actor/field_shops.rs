//! Personal shops on one field. Pure state, like `field_trades`: this module
//! decides who gets which message, and `FieldActor` delivers them.
//!
//! The owner's client actor holds the stock; the room keeps a copy of the
//! listings to show visitors and to quote a price. A purchase goes from the
//! buyer, already charged, through the room to the owner, who sells only
//! if the stock is still there and the buyer is still waiting for the
//! oneshot; otherwise the buyer is refunded.

use crate::actor::field_trades::Outbox;
use crate::message::ServerMessage;
use game::{AvatarLook, Item, MiniRoomBox};
use net::error::NetworkError;
use net::packet::build::world::mini_room::{
    build_enter_error, build_room_chat, build_room_leave, build_room_visit, build_seat_left,
    build_shop_items, build_shop_room, enter_error, leave_reason, PERSONAL_SHOP, SHOP_USERS,
};
use net::personal_shop::{Listing, VISITORS};
use net::ClientId;
use packet::Packet;
use std::collections::HashMap;
use tokio::sync::oneshot;
use tracing::error;

/// What a client asks of a personal shop on its field. Visits come as
/// `RoomRequest::Visit`, the same as for trading rooms.
#[derive(Debug)]
pub enum ShopRoomRequest {
    /// Open shop `room`, the sender its owner. Visitors may enter once it
    /// opens.
    Create {
        room: i32,
        title: String,
        skin: u8,
    },
    /// The owner's listings changed.
    Listings(Vec<Listing>),
    Open,
    Chat {
        text: String,
    },
    /// What listing `index` is now.
    Quote {
        index: u8,
        reply: oneshot::Sender<Option<Listing>>,
    },
    /// A visitor, already charged, buys from the owner.
    Purchase(Sale),
    /// The owner turns out the visitor in `slot`, named `name`.
    Ban {
        slot: u8,
        name: String,
        overstayed: bool,
    },
    /// The owner's blacklist: these may not enter.
    Blacklist(Vec<String>),
    BlacklistAdd(String),
    BlacklistRemove(String),
    /// The owner sold everything: the shop closes for all.
    SoldOut,
    Leave,
}

/// A purchase on its way to the owner.
#[derive(Debug)]
pub struct Sale {
    pub index: u8,
    pub bundles: i16,
    /// The item the buyer was quoted.
    pub item_id: i32,
    /// What the buyer paid.
    pub paid: i32,
    pub buyer: String,
    /// The items, or the `buy_result` that says why not. Dropped unanswered
    /// if the shop is gone.
    pub reply: oneshot::Sender<Result<Item, u8>>,
}

/// What a shop tells a member's client actor.
#[derive(Debug)]
pub enum ShopEvent {
    /// The player sits in `slot` of the shop now.
    Entered { slot: u8 },
    /// The owner: a visitor buys.
    Sale(Sale),
    /// The player is out of the shop; send the packet, if any.
    Left(Option<Packet>),
}

struct Member {
    client_id: ClientId,
    name: String,
    look: AvatarLook,
}

struct Shop {
    owner: Member,
    visitors: [Option<Member>; VISITORS as usize],
    title: String,
    skin: u8,
    open: bool,
    listings: Vec<Listing>,
    blacklist: Vec<String>,
    /// Turned out this time the shop is open.
    banned: Vec<String>,
}

impl Shop {
    /// Everyone in the shop with their slot.
    fn members(&self) -> impl Iterator<Item = (u8, &Member)> {
        std::iter::once((0, &self.owner)).chain(
            self.visitors
                .iter()
                .enumerate()
                .filter_map(|(i, seat)| Some((i as u8 + 1, seat.as_ref()?))),
        )
    }

    fn barred(&self, name: &str) -> bool {
        let same = |barred: &String| barred.eq_ignore_ascii_case(name);
        self.blacklist.iter().any(same) || self.banned.iter().any(same)
    }

    fn balloon(&self, room: i32) -> MiniRoomBox {
        MiniRoomBox {
            kind: PERSONAL_SHOP,
            sn: room,
            title: self.title.clone(),
            skin: self.skin,
            users: self.members().count() as u8,
            max_users: SHOP_USERS,
        }
    }
}

#[derive(Default)]
pub struct Shops {
    rooms: HashMap<i32, Shop>,
    member_of: HashMap<ClientId, i32>,
    /// Owners whose balloon changed (`None`: it went), for the field to
    /// show.
    balloons: Vec<(ClientId, Option<MiniRoomBox>)>,
}

fn packet(out: &mut Outbox, to: ClientId, packet: Result<Packet, NetworkError>) {
    match packet {
        Ok(packet) => out.push((to, ServerMessage::SendPacket(packet))),
        Err(e) => error!(to, error = %e, "Failed to build a shop packet"),
    }
}

fn event(out: &mut Outbox, to: ClientId, room: i32, event: ShopEvent) {
    out.push((to, ServerMessage::PersonalShop { room, event }));
}

/// A packet to send with an event, if it builds.
fn leave_packet(packet: Result<Packet, NetworkError>) -> Option<Packet> {
    packet
        .map_err(|e| error!(error = %e, "Failed to build a shop packet"))
        .ok()
}

impl Shops {
    pub fn contains(&self, room: i32) -> bool {
        self.rooms.contains_key(&room)
    }

    pub fn is_member(&self, client_id: ClientId) -> bool {
        self.member_of.contains_key(&client_id)
    }

    /// The balloons that changed since last asked.
    pub fn take_balloons(&mut self) -> Vec<(ClientId, Option<MiniRoomBox>)> {
        std::mem::take(&mut self.balloons)
    }

    /// The shop `client_id` is in, with their slot.
    fn shop_of(&mut self, client_id: ClientId) -> Option<(i32, &mut Shop, u8)> {
        let id = *self.member_of.get(&client_id)?;
        let shop = self.rooms.get_mut(&id)?;
        let (slot, _) = shop.members().find(|(_, m)| m.client_id == client_id)?;
        Some((id, shop, slot))
    }

    /// The shop `client_id` owns.
    fn owned(&mut self, client_id: ClientId) -> Option<(i32, &mut Shop)> {
        self.shop_of(client_id)
            .filter(|(_, _, slot)| *slot == 0)
            .map(|(id, shop, _)| (id, shop))
    }

    /// `busy`: the owner is in another kind of room.
    pub fn create(
        &mut self,
        room: i32,
        owner: ClientId,
        name: &str,
        look: &AvatarLook,
        title: String,
        skin: u8,
        busy: bool,
    ) -> Outbox {
        let mut out = Outbox::new();
        if busy || self.member_of.contains_key(&owner) || self.rooms.contains_key(&room) {
            event(&mut out, owner, room, ShopEvent::Left(None));
            return out;
        }
        packet(
            &mut out,
            owner,
            build_shop_room(0, &[(0, look, name)], &title, &[]),
        );
        self.rooms.insert(
            room,
            Shop {
                owner: Member {
                    client_id: owner,
                    name: name.to_string(),
                    look: look.clone(),
                },
                visitors: Default::default(),
                title,
                skin,
                open: false,
                listings: Vec::new(),
                blacklist: Vec::new(),
                banned: Vec::new(),
            },
        );
        self.member_of.insert(owner, room);
        out
    }

    /// Show the owner's listings to everyone in the shop.
    pub fn listings(&mut self, from: ClientId, listings: Vec<Listing>) -> Outbox {
        let mut out = Outbox::new();
        let Some((_, shop)) = self.owned(from) else {
            return out;
        };
        for (_, member) in shop.members() {
            packet(&mut out, member.client_id, build_shop_items(&listings));
        }
        shop.listings = listings;
        out
    }

    pub fn open(&mut self, from: ClientId) {
        let Some((id, shop)) = self.owned(from).filter(|(_, shop)| !shop.open) else {
            return;
        };
        shop.open = true;
        let balloon = shop.balloon(id);
        self.balloons.push((from, Some(balloon)));
    }

    /// `busy`: the visitor is in another kind of room.
    pub fn visit(
        &mut self,
        from: ClientId,
        room: i32,
        name: &str,
        look: &AvatarLook,
        busy: bool,
    ) -> Outbox {
        let mut out = Outbox::new();
        let refusal = match self.rooms.get(&room) {
            _ if busy || self.member_of.contains_key(&from) => Some(enter_error::CLOSED),
            Some(shop) if shop.open && shop.barred(name) => Some(enter_error::BANNED),
            Some(shop) if shop.open => shop
                .visitors
                .iter()
                .all(Option::is_some)
                .then_some(enter_error::FULL),
            _ => Some(enter_error::CLOSED),
        };
        if let Some(refusal) = refusal {
            packet(&mut out, from, build_enter_error(refusal));
            return out;
        }
        let shop = self.rooms.get_mut(&room).expect("checked above");
        let free = shop
            .visitors
            .iter()
            .position(Option::is_none)
            .expect("checked above");
        let slot = free as u8 + 1;
        shop.visitors[free] = Some(Member {
            client_id: from,
            name: name.to_string(),
            look: look.clone(),
        });
        // Before the packets, so a purchase made on seeing them finds the
        // player in the shop.
        event(&mut out, from, room, ShopEvent::Entered { slot });
        let members: Vec<_> = shop
            .members()
            .map(|(slot, m)| (slot, &m.look, m.name.as_str()))
            .collect();
        packet(
            &mut out,
            from,
            build_shop_room(slot, &members, &shop.title, &shop.listings),
        );
        for (_, member) in shop.members().filter(|(s, _)| *s != slot) {
            packet(
                &mut out,
                member.client_id,
                build_room_visit(slot, look, name),
            );
        }
        self.member_of.insert(from, room);
        let balloon = shop.balloon(room);
        self.balloons.push((shop.owner.client_id, Some(balloon)));
        out
    }

    pub fn chat(&mut self, from: ClientId, text: &str) -> Outbox {
        let mut out = Outbox::new();
        let Some((_, shop, slot)) = self.shop_of(from) else {
            return out;
        };
        let name = &shop
            .members()
            .find(|(s, _)| *s == slot)
            .expect("a member")
            .1
            .name;
        let line = format!("{name} : {text}");
        for (_, member) in shop.members() {
            packet(&mut out, member.client_id, build_room_chat(slot, &line));
        }
        out
    }

    /// Answer a visitor's question about a listing; anyone else's reply is
    /// dropped.
    pub fn quote(&mut self, from: ClientId, index: u8, reply: oneshot::Sender<Option<Listing>>) {
        if let Some((_, shop, slot)) = self.shop_of(from) {
            if slot != 0 && shop.open {
                let _ = reply.send(shop.listings.get(usize::from(index)).cloned());
            }
        }
    }

    /// Pass a visitor's purchase on to the owner; anyone else's is dropped,
    /// which refunds them.
    pub fn purchase(&mut self, from: ClientId, sale: Sale) -> Outbox {
        let mut out = Outbox::new();
        if let Some((id, shop, slot)) = self.shop_of(from) {
            if slot != 0 && shop.open {
                event(&mut out, shop.owner.client_id, id, ShopEvent::Sale(sale));
            }
        }
        out
    }

    /// The owner turns out the visitor in `slot`, if it is still `name`,
    /// and keeps them out while the shop is open.
    pub fn ban(&mut self, from: ClientId, slot: u8, name: &str, overstayed: bool) -> Outbox {
        let mut out = Outbox::new();
        let Some((id, shop)) = self.owned(from) else {
            return out;
        };
        let seat = usize::from(slot).wrapping_sub(1);
        let Some(visitor) = shop
            .visitors
            .get_mut(seat)
            .filter(|seat| seat.as_ref().is_some_and(|v| v.name == name))
            .and_then(Option::take)
        else {
            return out;
        };
        shop.banned.push(visitor.name);
        let reason = if overstayed {
            leave_reason::OVERSTAYED
        } else {
            leave_reason::BANNED
        };
        let leave = leave_packet(build_room_leave(slot, reason));
        event(&mut out, visitor.client_id, id, ShopEvent::Left(leave));
        for (_, member) in shop.members() {
            packet(&mut out, member.client_id, build_seat_left(slot));
        }
        let balloon = shop.balloon(id);
        self.member_of.remove(&visitor.client_id);
        self.balloons.push((from, Some(balloon)));
        out
    }

    pub fn blacklist(&mut self, from: ClientId, names: Vec<String>) {
        if let Some((_, shop)) = self.owned(from) {
            shop.blacklist = names;
        }
    }

    pub fn blacklist_add(&mut self, from: ClientId, name: String) {
        if let Some((_, shop)) = self.owned(from) {
            shop.blacklist.push(name);
        }
    }

    pub fn blacklist_remove(&mut self, from: ClientId, name: &str) {
        if let Some((_, shop)) = self.owned(from) {
            shop.blacklist
                .retain(|barred| !barred.eq_ignore_ascii_case(name));
        }
    }

    /// The owner sold everything: everyone, the owner too, is told the
    /// items are out of stock.
    pub fn sold_out(&mut self, from: ClientId) -> Outbox {
        let Some((id, _)) = self.owned(from) else {
            return Outbox::new();
        };
        let mut out = self.close(id, leave_reason::OUT_OF_STOCK);
        packet(
            &mut out,
            from,
            build_room_leave(0, leave_reason::OUT_OF_STOCK),
        );
        out
    }

    /// `client_id` left the shop or the field. An owner leaving closes the
    /// shop; the owner's client actor already took the stock back.
    pub fn leave(&mut self, client_id: ClientId) -> Outbox {
        let mut out = Outbox::new();
        let Some((id, shop, slot)) = self.shop_of(client_id) else {
            return out;
        };
        if slot == 0 {
            return self.close(id, leave_reason::SHOP_CLOSED);
        }
        shop.visitors[usize::from(slot) - 1] = None;
        for (_, member) in shop.members() {
            packet(&mut out, member.client_id, build_seat_left(slot));
        }
        let balloon = shop.balloon(id);
        let owner = shop.owner.client_id;
        self.member_of.remove(&client_id);
        if self.rooms[&id].open {
            self.balloons.push((owner, Some(balloon)));
        }
        out
    }

    /// Close a shop: the visitors get the LEAVE with `reason`, and the
    /// balloon goes.
    fn close(&mut self, id: i32, reason: u8) -> Outbox {
        let mut out = Outbox::new();
        let Some(shop) = self.rooms.remove(&id) else {
            return out;
        };
        for (slot, member) in shop.members() {
            self.member_of.remove(&member.client_id);
            if slot != 0 {
                let leave = leave_packet(build_room_leave(slot, reason));
                event(&mut out, member.client_id, id, ShopEvent::Left(leave));
            }
        }
        if shop.open {
            self.balloons.push((shop.owner.client_id, None));
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
        }
    }

    fn potions(bundles: i16) -> Listing {
        Listing {
            item: Item::stack(2000000, 10),
            bundles,
            price: 500,
        }
    }

    /// The mode byte of a packet (and the slot and reason of a LEAVE), or
    /// what kind of event it is.
    fn summary(out: &Outbox) -> Vec<(ClientId, String)> {
        out.iter()
            .map(|(to, message)| {
                let what = match message {
                    ServerMessage::SendPacket(packet) => describe(packet),
                    ServerMessage::PersonalShop { event, .. } => match event {
                        ShopEvent::Entered { slot } => format!("entered {slot}"),
                        ShopEvent::Sale(sale) => format!("sale {}", sale.paid),
                        ShopEvent::Left(Some(packet)) => format!("left {}", describe(packet)),
                        ShopEvent::Left(None) => "left".to_string(),
                    },
                    other => format!("{other:?}"),
                };
                (*to, what)
            })
            .collect()
    }

    fn describe(packet: &Packet) -> String {
        match packet.bytes[2] {
            0x0a => format!("0xa {:?}", &packet.bytes[3..]),
            mode => format!("{mode:#x}"),
        }
    }

    fn s(to: ClientId, what: &str) -> (ClientId, String) {
        (to, what.to_string())
    }

    fn open_shop() -> Shops {
        let mut shops = Shops::default();
        shops.create(ROOM, ALICE, "Alice", &look(), "Pots".into(), 2, false);
        shops.listings(ALICE, vec![potions(3)]);
        shops.open(ALICE);
        shops.take_balloons();
        shops
    }

    #[test]
    fn an_owner_sets_up_then_opens_with_a_balloon() {
        let mut shops = Shops::default();
        let out = shops.create(ROOM, ALICE, "Alice", &look(), "Pots".into(), 2, false);
        assert_eq!(summary(&out), vec![s(ALICE, "0x5")]);
        // Nobody may come in yet.
        let out = shops.visit(BOB, ROOM, "Bob", &look(), false);
        assert_eq!(summary(&out), vec![s(BOB, "0x5")]);
        let out = shops.listings(ALICE, vec![potions(3)]);
        assert_eq!(summary(&out), vec![s(ALICE, "0x19")]);
        assert!(shops.take_balloons().is_empty());
        shops.open(ALICE);
        let balloons = shops.take_balloons();
        assert_eq!(balloons.len(), 1);
        let (owner, balloon) = &balloons[0];
        let balloon = balloon.as_ref().unwrap();
        assert_eq!(*owner, ALICE);
        assert_eq!(
            (
                balloon.sn,
                balloon.title.as_str(),
                balloon.skin,
                balloon.users
            ),
            (ROOM, "Pots", 2, 1)
        );
        // Someone busy, or in a room already, can't open one.
        let out = shops.create(ROOM + 1, ALICE, "Alice", &look(), "x".into(), 0, false);
        assert_eq!(summary(&out), vec![s(ALICE, "left")]);
        let out = shops.create(ROOM + 1, BOB, "Bob", &look(), "x".into(), 0, true);
        assert_eq!(summary(&out), vec![s(BOB, "left")]);
    }

    #[test]
    fn visitors_take_free_seats_up_to_three() {
        let mut shops = open_shop();
        let out = shops.visit(BOB, ROOM, "Bob", &look(), false);
        assert_eq!(
            summary(&out),
            vec![s(BOB, "entered 1"), s(BOB, "0x5"), s(ALICE, "0x4")]
        );
        assert_eq!(shops.take_balloons()[0].1.as_ref().unwrap().users, 2);
        shops.visit(CAROL, ROOM, "Carol", &look(), false);
        shops.visit(4, ROOM, "Dave", &look(), false);
        let out = shops.visit(5, ROOM, "Erin", &look(), false);
        assert_eq!(summary(&out), vec![s(5, "0x5")]);
        assert_eq!(out_bytes(&out)[4], enter_error::FULL);

        // Bob's seat frees up for the next.
        let out = shops.leave(BOB);
        assert_eq!(
            summary(&out),
            vec![s(ALICE, "0xa [1]"), s(CAROL, "0xa [1]"), s(4, "0xa [1]")]
        );
        let out = shops.visit(5, ROOM, "Erin", &look(), false);
        assert_eq!(summary(&out)[0], s(5, "entered 1"));
    }

    fn out_bytes(out: &Outbox) -> Vec<u8> {
        match &out[0].1 {
            ServerMessage::SendPacket(packet) => packet.bytes.clone(),
            other => panic!("not a packet: {other:?}"),
        }
    }

    #[test]
    fn chat_goes_to_everyone_in_the_shop() {
        let mut shops = open_shop();
        shops.visit(BOB, ROOM, "Bob", &look(), false);
        let out = shops.chat(BOB, "hi");
        assert_eq!(summary(&out), vec![s(ALICE, "0x6"), s(BOB, "0x6")]);
        assert!(shops.chat(CAROL, "hi").is_empty());
    }

    #[test]
    fn purchases_reach_the_owner_and_listings_reach_everyone() {
        let mut shops = open_shop();
        shops.visit(BOB, ROOM, "Bob", &look(), false);
        let (reply, mut quote) = oneshot::channel();
        shops.quote(BOB, 0, reply);
        assert_eq!(quote.try_recv(), Ok(Some(potions(3))));
        let (reply, mut quote) = oneshot::channel();
        shops.quote(BOB, 1, reply);
        assert_eq!(quote.try_recv(), Ok(None));
        // Only visitors are quoted.
        let (reply, mut quote) = oneshot::channel();
        shops.quote(CAROL, 0, reply);
        assert!(quote.try_recv().is_err());

        let sale = |paid| {
            let (reply, rx) = oneshot::channel();
            let sale = Sale {
                index: 0,
                bundles: 2,
                item_id: 2000000,
                paid,
                buyer: "Bob".into(),
                reply,
            };
            (sale, rx)
        };
        let (purchase, _rx) = sale(1000);
        let out = shops.purchase(BOB, purchase);
        assert_eq!(summary(&out), vec![s(ALICE, "sale 1000")]);
        let (purchase, mut rx) = sale(1000);
        assert!(shops.purchase(CAROL, purchase).is_empty());
        assert!(rx.try_recv().is_err());

        let out = shops.listings(ALICE, vec![potions(1)]);
        assert_eq!(summary(&out), vec![s(ALICE, "0x19"), s(BOB, "0x19")]);
    }

    #[test]
    fn banned_and_blacklisted_names_stay_out() {
        let mut shops = open_shop();
        shops.blacklist(ALICE, vec!["carol".into()]);
        let out = shops.visit(CAROL, ROOM, "Carol", &look(), false);
        assert_eq!(out_bytes(&out)[4], enter_error::BANNED);

        shops.visit(BOB, ROOM, "Bob", &look(), false);
        // The slot must still hold the name.
        assert!(shops.ban(ALICE, 1, "Carol", false).is_empty());
        let out = shops.ban(ALICE, 1, "Bob", false);
        assert_eq!(
            summary(&out),
            vec![s(BOB, "left 0xa [1, 5]"), s(ALICE, "0xa [1]")]
        );
        let out = shops.visit(BOB, ROOM, "Bob", &look(), false);
        assert_eq!(out_bytes(&out)[4], enter_error::BANNED);

        shops.blacklist_remove(ALICE, "Carol");
        let out = shops.visit(CAROL, ROOM, "Carol", &look(), false);
        assert_eq!(summary(&out)[0], s(CAROL, "entered 1"));
        let out = shops.ban(ALICE, 1, "Carol", true);
        assert_eq!(summary(&out)[0], s(CAROL, "left 0xa [1, 15]"));
    }

    #[test]
    fn the_owner_leaving_or_selling_out_closes_the_shop() {
        let mut shops = open_shop();
        shops.visit(BOB, ROOM, "Bob", &look(), false);
        shops.take_balloons();
        let out = shops.leave(ALICE);
        assert_eq!(summary(&out), vec![s(BOB, "left 0xa [1, 3]")]);
        assert_eq!(shops.take_balloons(), vec![(ALICE, None)]);
        assert!(shops.rooms.is_empty() && shops.member_of.is_empty());

        let mut shops = open_shop();
        shops.visit(BOB, ROOM, "Bob", &look(), false);
        let out = shops.sold_out(ALICE);
        assert_eq!(
            summary(&out),
            vec![s(BOB, "left 0xa [1, 14]"), s(ALICE, "0xa [0, 14]")]
        );
        // Only the owner can.
        let mut shops = open_shop();
        shops.visit(BOB, ROOM, "Bob", &look(), false);
        assert!(shops.sold_out(BOB).is_empty());
        assert!(shops.contains(ROOM));
    }
}
