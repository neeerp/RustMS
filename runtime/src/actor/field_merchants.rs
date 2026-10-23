//! Hired merchants on one field (row 165, plan c007). Pure state, like
//! `field_shops`: this module decides who gets which message, and
//! `FieldActor` delivers them and does the database work it asks for.
//!
//! The database holds every merchant (`db::merchant`); the field keeps a
//! copy to show and to sell from. A purchase is reserved here, written by
//! the buyer's client actor (`db::merchant::sell`), and settled here once
//! written: only then are the mesos credited. The owner comes in for
//! maintenance only when no purchase is outstanding, so what they work
//! on is what the database holds.

use crate::actor::field_trades::Outbox;
use crate::message::ServerMessage;
use game::hired_merchant::{HiredMerchant, Sale, Stand, Stock};
use game::mini_room::Listing;
use game::AvatarLook;
use net::error::NetworkError;
use net::hired_merchant::leave_reason;
use net::owl::{go_result, ShopHit};
use net::packet::build::world::hired_merchant::{
    build_blacklist, build_destroy_merchant, build_merchant_items, build_merchant_room,
    build_spawn_merchant, build_update_merchant, build_visitor_list, OwnerView,
};
use net::packet::build::world::mini_room::{
    build_enter_error, build_room_chat, build_room_leave, build_room_visit, build_seat_left,
    enter_error,
};
use net::personal_shop::tax;
use net::ClientId;
use packet::Packet;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use tokio::sync::oneshot;
use tokio::time::Instant;
use tracing::error;

/// Merchants' rooms are numbered apart from trading rooms and personal
/// shops, which count up from 1.
const SN_BASE: i32 = 0x4000_0000;
/// Visitors a merchant seats.
const VISITORS: usize = 3;
/// How long a purchase may stay reserved before the field stops waiting
/// for its buyer and reads the merchant again.
const RESERVATION_TIMEOUT: Duration = Duration::from_secs(30);
/// The lines of chat a merchant keeps for its owner, and the visits.
const CHAT_KEPT: usize = 50;
const VISITS_KEPT: usize = 50;
/// The most names a blacklist holds (0x519611).
const BLACKLIST_MAX: usize = 20;
/// How close to another merchant or a personal shop a merchant may be
/// set up: a server rule, as the client doesn't check (report 165 §5).
pub const SPACING: i32 = 150;

/// The room of `owner_id`'s merchant.
pub fn sn(owner_id: i32) -> i32 {
    SN_BASE | owner_id
}

/// The owner of the merchant whose room is `sn`, if it is a merchant's.
pub fn owner_of(sn: i32) -> Option<i32> {
    (sn & SN_BASE != 0).then_some(sn & !SN_BASE)
}

/// What a client asks of a hired merchant on its field. Visits come as
/// `RoomRequest::Visit`, the same as for other rooms.
#[derive(Debug)]
pub enum MerchantRequest {
    /// Where the sender would set up a merchant: on the ground under them,
    /// if this map has merchants and none stands or sells too close.
    Stand {
        reply: oneshot::Sender<Option<Stand>>,
        /// Keep the spot for the sender while they set up, until they
        /// place the merchant or leave.
        hold: bool,
    },
    /// The sender opened their merchant, already saved as open: it stands
    /// here now, keeping out `blacklist`.
    Place {
        merchant: Box<HiredMerchant>,
        blacklist: Vec<String>,
    },
    Chat {
        text: String,
    },
    /// A visitor wants `bundles` of row `index`.
    Buy {
        index: u8,
        bundles: i16,
        reply: oneshot::Sender<Result<Reservation, u8>>,
    },
    /// The buyer's client actor wrote the purchase it reserved, the sale,
    /// or found it couldn't. Taken from anyone: the buyer may have left.
    /// `refused`: the database wouldn't have it, so the copy here is out
    /// of date.
    Settle {
        owner: i32,
        id: u64,
        sale: Option<Sale>,
        refused: bool,
    },
    /// The owner, in for maintenance, changed what the merchant holds.
    Restock {
        stock: Vec<Stock>,
        meso: i32,
    },
    /// The owner asks who visited, or for their blacklist.
    Visitors,
    Blacklist,
    BlacklistAdd(String),
    BlacklistRemove(String),
    /// The owner is done with maintenance: the merchant opens again.
    Reopen,
    /// The owner closes the merchant from maintenance: it goes from the
    /// map, and the reply has what it held and its blacklist.
    Close {
        reply: oneshot::Sender<(Box<HiredMerchant>, Vec<String>)>,
    },
    Leave,
    /// Out of `owner`'s merchant only: the player didn't want the seat.
    Unseat {
        owner: i32,
    },
}

/// A purchase held for a buyer while they write it.
#[derive(Debug)]
pub struct Reservation {
    pub id: u64,
    pub owner: i32,
    pub stock: i64,
    pub index: u8,
    pub bundles: i16,
    /// The row as it was offered.
    pub listing: Listing,
}

/// What a merchant tells a client actor. The room's packet comes with
/// the news, for the client actor to send only if it still expects it.
#[derive(Debug)]
pub enum MerchantEvent {
    /// The player sits in `slot` of the merchant's room now.
    Entered { slot: u8, room: Option<Packet> },
    /// The owner is in for maintenance, with the merchant as it stands.
    Maintenance(Box<HiredMerchant>, Option<Packet>),
    /// The merchant as the database has it now, for the owner in for
    /// maintenance, whose copy is out of date.
    Refreshed(Box<HiredMerchant>),
    /// The player is out of the room; send the packet, if any.
    Left(Option<Packet>),
}

/// What everyone on the field sees change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shown {
    Spawn(i32),
    Balloon(i32),
    Destroy(i32),
}

struct Member {
    client_id: ClientId,
    name: String,
    look: AvatarLook,
    since: Instant,
}

struct Held {
    id: u64,
    stock: i64,
    bundles: i16,
    at: Instant,
}

struct Merchant {
    data: HiredMerchant,
    visitors: [Option<Member>; VISITORS],
    /// The owner, in for maintenance.
    owner: Option<Member>,
    /// The owner, waiting to come in until the purchases settle.
    waiting: Option<Member>,
    held: Vec<Held>,
    /// Reads of the database asked for and not yet back. Until they are,
    /// nothing sells and the owner waits.
    rereading: u32,
    chat: Vec<(String, u8)>,
    blacklist: Vec<String>,
    visits: Vec<(String, Duration)>,
}

impl Merchant {
    fn new(data: HiredMerchant, blacklist: Vec<String>) -> Self {
        Self {
            data,
            visitors: Default::default(),
            owner: None,
            waiting: None,
            held: Vec::new(),
            rereading: 0,
            chat: Vec::new(),
            blacklist,
            visits: Vec::new(),
        }
    }

    /// Open to visitors: not in maintenance.
    fn open(&self) -> bool {
        self.owner.is_none() && self.waiting.is_none()
    }

    /// Nothing held or being read again: the numbers here are the
    /// database's.
    fn settled(&self) -> bool {
        self.held.is_empty() && self.rereading == 0
    }

    /// Let the waiting owner in, once the numbers are settled.
    fn admit_waiting(&mut self, out: &mut Outbox, now: SystemTime) {
        if self.settled() {
            if let Some(waiting) = self.waiting.take() {
                self.admit(waiting, out, now);
            }
        }
    }

    fn visitor_count(&self) -> u8 {
        self.visitors.iter().flatten().count() as u8
    }

    /// Everyone in the room with their slot; the owner sits in 0.
    fn members(&self) -> impl Iterator<Item = (u8, &Member)> {
        self.owner.iter().map(|owner| (0, owner)).chain(
            self.visitors
                .iter()
                .enumerate()
                .filter_map(|(i, seat)| Some((i as u8 + 1, seat.as_ref()?))),
        )
    }

    fn barred(&self, name: &str) -> bool {
        self.blacklist
            .iter()
            .any(|barred| barred.eq_ignore_ascii_case(name))
    }

    fn listings(&self) -> Vec<&Listing> {
        self.data.stock.iter().map(|stock| &stock.listing).collect()
    }

    fn items_packet(&self) -> Result<Packet, NetworkError> {
        build_merchant_items(self.data.meso, &self.listings())
    }

    fn log_visit(&mut self, member: &Member, now: Instant) {
        self.visits
            .push((member.name.clone(), now.duration_since(member.since)));
        let over = self.visits.len().saturating_sub(VISITS_KEPT);
        self.visits.drain(..over);
    }

    /// The owner, in for maintenance, sees the merchant as it stands.
    fn admit(&mut self, owner: Member, out: &mut Outbox, now: SystemTime) {
        let view = OwnerView {
            open_for: self.data.open_for(now),
            setup: false,
        };
        let room = built(build_merchant_room(
            &self.data,
            0,
            Some(view),
            &[],
            &self.chat,
        ));
        event(
            out,
            owner.client_id,
            self.data.owner_id,
            MerchantEvent::Maintenance(Box::new(self.data.clone()), room),
        );
        self.owner = Some(owner);
    }
}

#[derive(Default)]
pub struct Merchants {
    merchants: HashMap<i32, Merchant>,
    member_of: HashMap<ClientId, i32>,
    next_reservation: u64,
    shown: Vec<Shown>,
    rereads: Vec<i32>,
    /// The spots kept for players setting up a merchant, which stands
    /// nowhere yet.
    held_stands: HashMap<ClientId, (i16, i16)>,
}

fn packet(out: &mut Outbox, to: ClientId, packet: Result<Packet, NetworkError>) {
    match packet {
        Ok(packet) => out.push((to, ServerMessage::SendPacket(packet))),
        Err(e) => error!(to, error = %e, "Failed to build a merchant packet"),
    }
}

fn event(out: &mut Outbox, to: ClientId, owner: i32, event: MerchantEvent) {
    out.push((to, ServerMessage::HiredMerchant { owner, event }));
}

fn built(packet: Result<Packet, NetworkError>) -> Option<Packet> {
    packet
        .map_err(|e| error!(error = %e, "Failed to build a merchant packet"))
        .ok()
}

impl Merchants {
    /// Whether room `sn` is a merchant's here.
    pub fn contains(&self, sn: i32) -> bool {
        self.by_sn(sn).is_some()
    }

    fn by_sn(&self, sn: i32) -> Option<i32> {
        owner_of(sn).filter(|owner| self.merchants.contains_key(owner))
    }

    pub fn is_member(&self, client_id: ClientId) -> bool {
        self.member_of.contains_key(&client_id)
    }

    /// What everyone on the field must see change, since last asked.
    pub fn take_shown(&mut self) -> Vec<Shown> {
        std::mem::take(&mut self.shown)
    }

    /// The merchants to read from the database again, since last asked;
    /// each read comes back through `reload`.
    pub fn take_rereads(&mut self) -> Vec<i32> {
        std::mem::take(&mut self.rereads)
    }

    /// What `owner`'s merchant shows may be out of date: let go of what it
    /// holds, the buyers' answers no longer count, and read it again.
    fn reread(&mut self, owner: i32) {
        if let Some(merchant) = self.merchants.get_mut(&owner) {
            merchant.held.clear();
            merchant.rereading += 1;
            self.rereads.push(owner);
        }
    }

    /// The packet that shows `owner`'s merchant as it stands, or takes it
    /// away.
    pub fn show(&self, shown: Shown) -> Option<Result<Packet, NetworkError>> {
        let packet = match shown {
            Shown::Destroy(owner) => return Some(build_destroy_merchant(owner)),
            Shown::Spawn(owner) => {
                let m = self.merchants.get(&owner)?;
                build_spawn_merchant(&m.data, sn(owner), m.open(), m.visitor_count())
            }
            Shown::Balloon(owner) => {
                let m = self.merchants.get(&owner)?;
                build_update_merchant(&m.data, sn(owner), m.open(), m.visitor_count())
            }
        };
        Some(packet)
    }

    /// Every merchant here, for a player arriving.
    pub fn spawns(&self) -> Vec<Result<Packet, NetworkError>> {
        self.merchants
            .keys()
            .filter_map(|&owner| self.show(Shown::Spawn(owner)))
            .collect()
    }

    /// Where the merchants stand, and those being set up but `from`'s, to
    /// keep new ones apart.
    fn stands(&self, from: ClientId) -> impl Iterator<Item = (i16, i16)> + '_ {
        let held = self
            .held_stands
            .iter()
            .filter(move |(&id, _)| id != from)
            .map(|(_, &at)| at);
        self.merchants
            .values()
            .map(|m| (m.data.stand.x, m.data.stand.y))
            .chain(held)
    }

    /// Whether a merchant `from` sets up at (x, y) would stand clear of
    /// the merchants here and of the shops at `shops`.
    pub fn clear_of(
        &self,
        from: ClientId,
        x: i16,
        y: i16,
        shops: impl IntoIterator<Item = (i16, i16)>,
    ) -> bool {
        let near = |(other_x, other_y): (i16, i16)| {
            let dx = i32::from(x) - i32::from(other_x);
            let dy = i32::from(y) - i32::from(other_y);
            dx * dx + dy * dy < SPACING * SPACING
        };
        !self.stands(from).chain(shops).any(near)
    }

    /// Keep (x, y) for `from`, who sets up a merchant there.
    pub fn hold_stand(&mut self, from: ClientId, x: i16, y: i16) {
        self.held_stands.insert(from, (x, y));
    }

    /// The merchants read from the database when the field started. One
    /// already here, as when its owner placed it meanwhile, stays.
    pub fn load(&mut self, merchants: Vec<HiredMerchant>) {
        for data in merchants {
            let owner = data.owner_id;
            if !self.merchants.contains_key(&owner) {
                self.merchants
                    .insert(owner, Merchant::new(data, Vec::new()));
                self.shown.push(Shown::Spawn(owner));
            }
        }
    }

    /// `merchant`'s owner, `from`, opened it; it stands here from now on.
    pub fn place(&mut self, from: ClientId, merchant: HiredMerchant, names: Vec<String>) {
        self.held_stands.remove(&from);
        let owner = merchant.owner_id;
        if self.merchants.contains_key(&owner) {
            return;
        }
        let mut blacklist: Vec<String> = Vec::new();
        for name in names {
            if blacklist.len() < BLACKLIST_MAX
                && !blacklist.iter().any(|b| b.eq_ignore_ascii_case(&name))
            {
                blacklist.push(name);
            }
        }
        self.merchants
            .insert(owner, Merchant::new(merchant, blacklist));
        self.shown.push(Shown::Spawn(owner));
    }

    /// `character_id` enters merchant `sn`: the owner for maintenance, once
    /// the purchases settle, anyone else as a visitor. `busy`: they are in
    /// another kind of room.
    #[allow(clippy::too_many_arguments)]
    pub fn visit(
        &mut self,
        from: ClientId,
        character_id: i32,
        sn: i32,
        name: &str,
        look: &AvatarLook,
        busy: bool,
        now: (Instant, SystemTime),
    ) -> Outbox {
        let mut out = Outbox::new();
        if self
            .by_sn(sn)
            .is_some_and(|owner| self.member_of.get(&from) == Some(&owner))
        {
            // A second click while waiting to come in.
            return out;
        }
        let Some(owner_id) = self.by_sn(sn).filter(|_| !busy && !self.is_member(from)) else {
            packet(&mut out, from, build_enter_error(enter_error::CLOSED));
            return out;
        };
        let merchant = self.merchants.get_mut(&owner_id).expect("found by sn");
        let member = Member {
            client_id: from,
            name: name.to_string(),
            look: look.clone(),
            since: now.0,
        };
        if character_id == owner_id {
            if !merchant.open() {
                packet(&mut out, from, build_enter_error(enter_error::CLOSED));
                return out;
            }
            // Visitors make way for maintenance.
            let mut out_of_the_way = Vec::new();
            for (seat, visitor) in merchant.visitors.iter_mut().enumerate() {
                let Some(visitor) = visitor.take() else {
                    continue;
                };
                let slot = seat as u8 + 1;
                let leave = built(build_room_leave(slot, leave_reason::MAINTENANCE));
                self.member_of.remove(&visitor.client_id);
                event(
                    &mut out,
                    visitor.client_id,
                    owner_id,
                    MerchantEvent::Left(leave),
                );
                out_of_the_way.push(visitor);
            }
            for visitor in &out_of_the_way {
                merchant.log_visit(visitor, now.0);
            }
            self.member_of.insert(from, owner_id);
            merchant.waiting = Some(member);
            merchant.admit_waiting(&mut out, now.1);
            self.shown.push(Shown::Balloon(owner_id));
            return out;
        }
        let refusal = if !merchant.open() {
            Some(enter_error::MAINTENANCE)
        } else if merchant.barred(name) {
            Some(enter_error::BANNED)
        } else if merchant.visitors.iter().all(Option::is_some) {
            Some(enter_error::FULL)
        } else {
            None
        };
        if let Some(refusal) = refusal {
            packet(&mut out, from, build_enter_error(refusal));
            return out;
        }
        let free = merchant
            .visitors
            .iter()
            .position(Option::is_none)
            .expect("checked above");
        let slot = free as u8 + 1;
        merchant.visitors[free] = Some(member);
        let visitors: Vec<_> = merchant
            .members()
            .map(|(slot, m)| (slot, &m.look, m.name.as_str()))
            .collect();
        let room = built(build_merchant_room(
            &merchant.data,
            slot,
            None,
            &visitors,
            &merchant.chat,
        ));
        event(
            &mut out,
            from,
            owner_id,
            MerchantEvent::Entered { slot, room },
        );
        for (_, member) in merchant.members().filter(|(s, _)| *s != slot) {
            packet(
                &mut out,
                member.client_id,
                build_room_visit(slot, look, name),
            );
        }
        self.member_of.insert(from, owner_id);
        self.shown.push(Shown::Balloon(owner_id));
        out
    }

    /// The merchant `client_id` is in, with their slot.
    fn room_of(&mut self, client_id: ClientId) -> Option<(&mut Merchant, u8)> {
        let owner = *self.member_of.get(&client_id)?;
        let merchant = self.merchants.get_mut(&owner)?;
        let (slot, _) = merchant.members().find(|(_, m)| m.client_id == client_id)?;
        Some((merchant, slot))
    }

    /// The merchant `client_id` owns and is in for maintenance.
    fn maintained(&mut self, client_id: ClientId) -> Option<&mut Merchant> {
        self.room_of(client_id)
            .filter(|(_, slot)| *slot == 0)
            .map(|(merchant, _)| merchant)
    }

    pub fn chat(&mut self, from: ClientId, text: &str) -> Outbox {
        let mut out = Outbox::new();
        let Some((merchant, slot)) = self.room_of(from) else {
            return out;
        };
        let name = &merchant
            .members()
            .find(|(s, _)| *s == slot)
            .expect("a member")
            .1
            .name;
        let line = format!("{name} : {text}");
        for (_, member) in merchant.members() {
            packet(&mut out, member.client_id, build_room_chat(slot, &line));
        }
        merchant.chat.push((line, slot));
        let over = merchant.chat.len().saturating_sub(CHAT_KEPT);
        merchant.chat.drain(..over);
        out
    }

    /// Hold `bundles` of row `index` for a visitor, or say why not
    /// (`buy_result`). Anyone else's request is dropped.
    pub fn buy(
        &mut self,
        from: ClientId,
        index: u8,
        bundles: i16,
        reply: oneshot::Sender<Result<Reservation, u8>>,
        now: Instant,
    ) -> Outbox {
        use net::packet::build::world::mini_room::buy_result;
        let id = self.next_reservation;
        self.next_reservation += 1;
        let Some((merchant, slot)) = self.room_of(from) else {
            return Outbox::new();
        };
        if slot == 0 || !merchant.open() {
            return Outbox::new();
        }
        if merchant.rereading > 0 {
            let _ = reply.send(Err(buy_result::NO_STOCK));
            return Outbox::new();
        }
        let owner = merchant.data.owner_id;
        let Some(stock) = merchant
            .data
            .stock
            .get_mut(usize::from(index))
            .filter(|stock| bundles > 0 && stock.listing.bundles >= bundles)
        else {
            let _ = reply.send(Err(buy_result::NO_STOCK));
            return Outbox::new();
        };
        let reservation = Reservation {
            id,
            owner,
            stock: stock.id,
            index,
            bundles,
            listing: stock.listing.clone(),
        };
        if reply.send(Ok(reservation)).is_err() {
            return Outbox::new();
        }
        stock.listing.bundles -= bundles;
        merchant.held.push(Held {
            id,
            stock: stock.id,
            bundles,
            at: now,
        });
        // The room sees the purchase once it is settled.
        Outbox::new()
    }

    /// The buyer wrote reservation `id` of `owner`'s merchant, or didn't:
    /// credit the mesos or put the bundles back. One the merchant let go
    /// of, as after a timeout, is in the database alone; if it sold, the
    /// merchant is read again, as it is when the database `refused` the
    /// purchase.
    pub fn settle(
        &mut self,
        owner: i32,
        id: u64,
        sold: Option<Sale>,
        refused: bool,
        now: SystemTime,
    ) -> Outbox {
        let mut out = Outbox::new();
        let Some(merchant) = self.merchants.get_mut(&owner) else {
            return out;
        };
        let Some(at) = merchant.held.iter().position(|held| held.id == id) else {
            if sold.is_some() || refused {
                self.reread(owner);
            }
            return out;
        };
        if refused {
            self.reread(owner);
            return out;
        }
        let held = merchant.held.remove(at);
        let row = merchant
            .data
            .stock
            .iter_mut()
            .find(|stock| stock.id == held.stock);
        match (sold, row) {
            (Some(sale), _) => {
                merchant.data.meso += sale.mesos - tax(sale.mesos);
                merchant.data.sales.push(sale);
            }
            (None, Some(row)) => row.listing.bundles += held.bundles,
            (None, None) => {}
        }
        for (_, member) in merchant.members() {
            packet(&mut out, member.client_id, merchant.items_packet());
        }
        merchant.admit_waiting(&mut out, now);
        out
    }

    /// Let go of the merchants' purchases held too long, and read those
    /// merchants again. The owners of those merchants returned.
    pub fn stale(&mut self, now: Instant) -> Vec<i32> {
        let stale: Vec<i32> = self
            .merchants
            .iter()
            .filter(|(_, m)| {
                m.held
                    .iter()
                    .any(|held| now.duration_since(held.at) >= RESERVATION_TIMEOUT)
            })
            .map(|(&owner, _)| owner)
            .collect();
        for &owner in &stale {
            self.reread(owner);
        }
        stale
    }

    /// `owner`'s merchant as the database has it now; one no longer open
    /// goes. The owner comes in once the last read is back.
    pub fn reload(&mut self, owner: i32, data: Option<HiredMerchant>, now: SystemTime) -> Outbox {
        let Some(merchant) = self.merchants.get_mut(&owner) else {
            return Outbox::new();
        };
        merchant.rereading = merchant.rereading.saturating_sub(1);
        match data {
            Some(data) if data.state == game::hired_merchant::MerchantState::Open => {
                merchant.data = data;
                let mut out = Outbox::new();
                if let Some(owner) = &merchant.owner {
                    let data = Box::new(merchant.data.clone());
                    event(
                        &mut out,
                        owner.client_id,
                        merchant.data.owner_id,
                        MerchantEvent::Refreshed(data),
                    );
                }
                for (_, member) in merchant.members() {
                    packet(&mut out, member.client_id, merchant.items_packet());
                }
                merchant.admit_waiting(&mut out, now);
                out
            }
            _ => self.remove(owner, leave_reason::CLOSED),
        }
    }

    /// The owner, in for maintenance, put in or took out items or mesos.
    pub fn restock(&mut self, from: ClientId, stock: Vec<Stock>, meso: i32) {
        if let Some(merchant) = self.maintained(from) {
            merchant.data.stock = stock;
            merchant.data.meso = meso;
        }
    }

    pub fn visitors(&mut self, from: ClientId) -> Outbox {
        let mut out = Outbox::new();
        if let Some(merchant) = self.maintained(from) {
            packet(&mut out, from, build_visitor_list(&merchant.visits));
        }
        out
    }

    pub fn blacklist(&mut self, from: ClientId) -> Outbox {
        let mut out = Outbox::new();
        if let Some(merchant) = self.maintained(from) {
            packet(&mut out, from, build_blacklist(&merchant.blacklist));
        }
        out
    }

    pub fn blacklist_add(&mut self, from: ClientId, name: String) {
        if let Some(merchant) = self.maintained(from) {
            if !merchant.barred(&name) && merchant.blacklist.len() < BLACKLIST_MAX {
                merchant.blacklist.push(name);
            }
        }
    }

    pub fn blacklist_remove(&mut self, from: ClientId, name: &str) {
        if let Some(merchant) = self.maintained(from) {
            merchant
                .blacklist
                .retain(|barred| !barred.eq_ignore_ascii_case(name));
        }
    }

    /// The owner is out of maintenance, by choice or by leaving the field:
    /// the merchant opens again.
    fn reopen(&mut self, client_id: ClientId) {
        let Some(owner) = self.member_of.remove(&client_id) else {
            return;
        };
        let Some(merchant) = self.merchants.get_mut(&owner) else {
            return;
        };
        merchant.owner = None;
        merchant.waiting = None;
        self.shown.push(Shown::Balloon(owner));
    }

    /// The owner, in for maintenance, closes the merchant: it goes, and
    /// they get what it held.
    pub fn close(
        &mut self,
        from: ClientId,
        reply: oneshot::Sender<(Box<HiredMerchant>, Vec<String>)>,
    ) -> Outbox {
        let Some(owner) = self
            .maintained(from)
            .filter(|m| m.rereading == 0)
            .map(|m| m.data.owner_id)
        else {
            return Outbox::new();
        };
        let merchant = self.merchants.remove(&owner).expect("maintained");
        self.member_of.remove(&from);
        self.shown.push(Shown::Destroy(owner));
        let _ = reply.send((Box::new(merchant.data), merchant.blacklist));
        Outbox::new()
    }

    /// `client_id` left the room or the field. An owner leaving
    /// maintenance opens the merchant again.
    pub fn leave(&mut self, client_id: ClientId, now: Instant) -> Outbox {
        self.held_stands.remove(&client_id);
        self.leave_room(client_id, now)
    }

    /// `client_id` is out of `owner`'s room, if they are in it, and keeps
    /// any spot they hold.
    pub fn unseat(&mut self, client_id: ClientId, owner: i32, now: Instant) -> Outbox {
        if self.member_of.get(&client_id) != Some(&owner) {
            return Outbox::new();
        }
        self.leave_room(client_id, now)
    }

    fn leave_room(&mut self, client_id: ClientId, now: Instant) -> Outbox {
        let mut out = Outbox::new();
        let Some(&owner) = self.member_of.get(&client_id) else {
            return out;
        };
        let Some(merchant) = self.merchants.get_mut(&owner) else {
            self.member_of.remove(&client_id);
            return out;
        };
        let in_maintenance = merchant.owner.as_ref().map(|m| m.client_id) == Some(client_id)
            || merchant.waiting.as_ref().map(|m| m.client_id) == Some(client_id);
        if in_maintenance {
            self.reopen(client_id);
            return out;
        }
        let Some(seat) = merchant
            .visitors
            .iter()
            .position(|seat| seat.as_ref().is_some_and(|v| v.client_id == client_id))
        else {
            self.member_of.remove(&client_id);
            return out;
        };
        let visitor = merchant.visitors[seat].take().expect("found");
        merchant.log_visit(&visitor, now);
        let slot = seat as u8 + 1;
        for (_, member) in merchant.members() {
            packet(&mut out, member.client_id, build_seat_left(slot));
        }
        self.member_of.remove(&client_id);
        self.shown.push(Shown::Balloon(owner));
        out
    }

    /// The open merchants whose time is up at `now`, with nobody in for
    /// maintenance and nothing held: they go, and the field closes them
    /// in the database.
    pub fn expire(&mut self, now: SystemTime) -> (Outbox, Vec<i32>) {
        let due: Vec<i32> = self
            .merchants
            .iter()
            .filter(|(_, m)| m.open() && m.settled() && m.data.expired(now))
            .map(|(&owner, _)| owner)
            .collect();
        let mut out = Outbox::new();
        for &owner in &due {
            out.extend(self.remove(owner, leave_reason::CLOSED));
        }
        (out, due)
    }

    /// Take `owner`'s merchant off the map, turning out its visitors with
    /// `reason`.
    fn remove(&mut self, owner: i32, reason: u8) -> Outbox {
        let mut out = Outbox::new();
        let Some(merchant) = self.merchants.remove(&owner) else {
            return out;
        };
        for (slot, member) in merchant.members() {
            self.member_of.remove(&member.client_id);
            let leave = built(build_room_leave(slot, reason));
            event(
                &mut out,
                member.client_id,
                owner,
                MerchantEvent::Left(leave),
            );
        }
        if let Some(waiting) = &merchant.waiting {
            self.member_of.remove(&waiting.client_id);
            event(
                &mut out,
                waiting.client_id,
                owner,
                MerchantEvent::Left(None),
            );
        }
        self.shown.push(Shown::Destroy(owner));
        out
    }

    /// The open merchants' rows of `item_id` with bundles left, for the
    /// Owl of Minerva.
    pub fn search(&self, item_id: i32, map_id: i32, channel_id: u8) -> Vec<ShopHit> {
        self.merchants
            .values()
            .filter(|m| m.open())
            .flat_map(|m| {
                m.data
                    .stock
                    .iter()
                    .map(|stock| &stock.listing)
                    .filter(move |l| l.item.item_id == item_id && l.bundles > 0)
                    .map(move |l| ShopHit {
                        owner: m.data.owner_name.clone(),
                        title: m.data.title.clone(),
                        map_id,
                        room: sn(m.data.owner_id),
                        channel_id,
                        item: l.item.clone(),
                        bundles: l.bundles,
                        price: l.price,
                    })
            })
            .collect()
    }

    /// Whether `name` would get into merchant `sn` now, as the Owl's GO
    /// asks.
    pub fn go_result(&self, sn: i32, name: &str) -> u8 {
        match self.by_sn(sn).and_then(|owner| self.merchants.get(&owner)) {
            Some(m) if m.open() && m.barred(name) => go_result::BARRED,
            Some(m) if m.open() && m.visitors.iter().all(Option::is_some) => go_result::FULL,
            Some(m) if m.open() => go_result::GO,
            Some(_) => go_result::MAINTENANCE,
            None => go_result::CLOSED,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::hired_merchant::{MerchantState, Stand};
    use game::Item;
    use net::packet::build::world::mini_room::buy_result;
    use std::collections::BTreeMap;

    const OWNER: i32 = 42;
    const ALICE: ClientId = 1;
    const BOB: ClientId = 2;
    const CAROL: ClientId = 3;
    const DAVE: ClientId = 4;
    const ERIN: ClientId = 5;

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

    fn merchant(opened: SystemTime) -> HiredMerchant {
        HiredMerchant {
            owner_id: OWNER,
            owner_name: "Owner".to_string(),
            stand: Stand {
                channel_id: 0,
                map_id: 910000001,
                x: 100,
                y: 34,
                foothold: 5,
            },
            item_id: 5030000,
            title: "Potions".to_string(),
            state: MerchantState::Open,
            opened_at: Some(opened),
            closed_at: None,
            meso: 0,
            stock: vec![Stock {
                id: 7,
                listing: Listing {
                    item: Item::stack(2000000, 10),
                    bundles: 5,
                    price: 1_000_000,
                },
            }],
            sales: Vec::new(),
        }
    }

    fn now() -> (Instant, SystemTime) {
        (Instant::now(), SystemTime::now())
    }

    fn placed() -> Merchants {
        let mut merchants = Merchants::default();
        merchants.place(
            ALICE,
            merchant(SystemTime::now()),
            vec!["Mallory".to_string()],
        );
        assert_eq!(merchants.take_shown(), vec![Shown::Spawn(OWNER)]);
        merchants
    }

    fn visit(merchants: &mut Merchants, from: ClientId, id: i32, name: &str) -> Outbox {
        merchants.visit(from, id, sn(OWNER), name, &look(), false, now())
    }

    /// The mode byte of a packet (and the slot and reason of a LEAVE), or
    /// what kind of event it is.
    /// An event's room packet follows it.
    fn summary(out: &Outbox) -> Vec<(ClientId, String)> {
        out.iter()
            .flat_map(|(to, message)| {
                let (what, room) = match message {
                    ServerMessage::SendPacket(packet) => (describe(packet), None),
                    ServerMessage::HiredMerchant { event, .. } => match event {
                        MerchantEvent::Entered { slot, room } => {
                            (format!("entered {slot}"), room.as_ref())
                        }
                        MerchantEvent::Maintenance(m, room) => {
                            (format!("maintenance {}", m.meso), room.as_ref())
                        }
                        MerchantEvent::Left(Some(packet)) => {
                            (format!("left {}", describe(packet)), None)
                        }
                        MerchantEvent::Left(None) => ("left".to_string(), None),
                        MerchantEvent::Refreshed(m) => (format!("refreshed {}", m.meso), None),
                    },
                    other => (format!("{other:?}"), None),
                };
                std::iter::once((*to, what)).chain(room.map(|room| (*to, describe(room))))
            })
            .collect()
    }

    fn describe(packet: &Packet) -> String {
        match packet.bytes[2] {
            0x05 => format!("0x5 {:?}", &packet.bytes[3..5]),
            0x0a => format!("0xa {:?}", &packet.bytes[3..]),
            mode => format!("{mode:#x}"),
        }
    }

    fn reserve(merchants: &mut Merchants, from: ClientId, bundles: i16) -> Result<Reservation, u8> {
        let (reply, mut answer) = oneshot::channel();
        merchants.buy(from, 0, bundles, reply, Instant::now());
        answer.try_recv().expect("answered")
    }

    fn sale(mesos: i32) -> Sale {
        Sale {
            item_id: 2000000,
            quantity: 20,
            mesos,
            buyer: "Bob".to_string(),
        }
    }

    #[test]
    fn rooms_are_numbered_apart_from_other_rooms() {
        let merchants = placed();
        assert!(merchants.contains(sn(OWNER)));
        assert!(!merchants.contains(OWNER), "a trading room's number");
        assert!(!merchants.contains(sn(OWNER + 1)));
    }

    #[test]
    fn visitors_sit_in_three_seats_and_the_balloon_counts_them() {
        let mut merchants = placed();
        let out = visit(&mut merchants, BOB, 2, "Bob");
        assert_eq!(
            summary(&out),
            vec![(BOB, "entered 1".into()), (BOB, "0x5 [5, 4]".into())]
        );
        assert_eq!(merchants.take_shown(), vec![Shown::Balloon(OWNER)]);
        let out = visit(&mut merchants, CAROL, 3, "Carol");
        assert_eq!(summary(&out)[2], (BOB, "0x4".into()), "Bob sees Carol come");
        visit(&mut merchants, DAVE, 4, "Dave");
        let out = visit(&mut merchants, ERIN, 5, "Erin");
        assert_eq!(summary(&out), vec![(ERIN, "0x5 [0, 2]".into())], "full");
        let out = merchants.leave(CAROL, Instant::now());
        assert_eq!(
            summary(&out),
            vec![(BOB, "0xa [2]".into()), (DAVE, "0xa [2]".into())]
        );
        assert!(!merchants.is_member(CAROL));
        assert_eq!(merchants.go_result(sn(OWNER), "Erin"), go_result::GO);
    }

    #[test]
    fn the_blacklist_keeps_names_out() {
        let mut merchants = Merchants::default();
        let names = ["Mallory", "MALLORY"].map(String::from).into_iter();
        let names = names.chain((0..30).map(|i| format!("Name{i}")));
        merchants.place(ALICE, merchant(SystemTime::now()), names.collect());
        assert_eq!(merchants.merchants[&OWNER].blacklist.len(), BLACKLIST_MAX);
        let out = visit(&mut merchants, BOB, 2, "mallory");
        assert_eq!(summary(&out), vec![(BOB, "0x5 [0, 17]".into())]);
        assert_eq!(merchants.go_result(sn(OWNER), "Mallory"), go_result::BARRED);
    }

    #[test]
    fn a_purchase_is_held_then_credited_net_of_tax() {
        let mut merchants = placed();
        visit(&mut merchants, BOB, 2, "Bob");
        let held = reserve(&mut merchants, BOB, 2).expect("in stock");
        assert_eq!((held.owner, held.stock, held.bundles), (OWNER, 7, 2));
        assert_eq!(held.listing.bundles, 5, "as offered");
        assert_eq!(
            reserve(&mut merchants, BOB, 4).unwrap_err(),
            buy_result::NO_STOCK,
            "two of the five are held"
        );
        let out = merchants.settle(
            OWNER,
            held.id,
            Some(sale(2_000_000)),
            false,
            SystemTime::now(),
        );
        assert_eq!(summary(&out), vec![(BOB, "0x19".into())]);
        assert!(merchants.take_rereads().is_empty());
        let m = &merchants.merchants[&OWNER];
        assert_eq!(m.data.meso, 2_000_000 - tax(2_000_000));
        assert_eq!(
            (m.data.stock[0].listing.bundles, m.data.sales.len()),
            (3, 1)
        );
    }

    #[test]
    fn a_purchase_not_written_puts_the_bundles_back() {
        let mut merchants = placed();
        visit(&mut merchants, BOB, 2, "Bob");
        let held = reserve(&mut merchants, BOB, 5).expect("in stock");
        merchants.leave(BOB, Instant::now());
        // The buyer settles after leaving.
        merchants.settle(OWNER, held.id, None, false, SystemTime::now());
        let m = &merchants.merchants[&OWNER];
        assert_eq!((m.data.stock[0].listing.bundles, m.data.meso), (5, 0));
        merchants.settle(OWNER, held.id, None, false, SystemTime::now());
        let m = &merchants.merchants[&OWNER];
        assert_eq!(m.data.stock[0].listing.bundles, 5, "settled once");
        assert!(merchants.take_rereads().is_empty());
    }

    #[test]
    fn the_owner_turns_visitors_out_and_waits_for_purchases() {
        let mut merchants = placed();
        visit(&mut merchants, BOB, 2, "Bob");
        visit(&mut merchants, CAROL, 3, "Carol");
        let held = reserve(&mut merchants, CAROL, 1).expect("in stock");
        merchants.take_shown();
        let out = visit(&mut merchants, ALICE, OWNER, "Owner");
        assert_eq!(
            summary(&out),
            vec![
                (BOB, "left 0xa [1, 17]".into()),
                (CAROL, "left 0xa [2, 17]".into()),
            ],
            "the owner waits for Carol's purchase"
        );
        assert_eq!(merchants.take_shown(), vec![Shown::Balloon(OWNER)]);
        assert_eq!(
            merchants.go_result(sn(OWNER), "Bob"),
            go_result::MAINTENANCE
        );
        let out = visit(&mut merchants, ALICE, OWNER, "Owner");
        assert!(out.is_empty(), "a second click");
        let out = visit(&mut merchants, BOB, 2, "Bob");
        assert_eq!(
            summary(&out),
            vec![(BOB, "0x5 [0, 18]".into())],
            "maintenance"
        );
        let out = merchants.settle(
            OWNER,
            held.id,
            Some(sale(1_000_000)),
            false,
            SystemTime::now(),
        );
        assert_eq!(
            summary(&out),
            vec![
                (ALICE, format!("maintenance {}", 1_000_000 - tax(1_000_000))),
                (ALICE, "0x5 [5, 4]".into()),
            ]
        );
        assert_eq!(merchants.merchants[&OWNER].visits.len(), 2);
        // Done: it opens again.
        merchants.leave(ALICE, Instant::now());
        assert_eq!(merchants.go_result(sn(OWNER), "Bob"), go_result::GO);
    }

    #[test]
    fn the_owner_closes_from_maintenance() {
        let mut merchants = placed();
        visit(&mut merchants, ALICE, OWNER, "Owner");
        merchants.take_shown();
        merchants.restock(ALICE, Vec::new(), 300);
        let (reply, mut closed) = oneshot::channel();
        merchants.close(ALICE, reply);
        let (closed, blacklist) = closed.try_recv().expect("closed");
        assert_eq!((closed.meso, closed.stock.len()), (300, 0));
        assert_eq!(blacklist, vec!["Mallory".to_string()]);
        assert_eq!(merchants.take_shown(), vec![Shown::Destroy(OWNER)]);
        assert!(!merchants.contains(sn(OWNER)) && !merchants.is_member(ALICE));
    }

    #[test]
    fn only_the_owner_in_maintenance_restocks_or_closes() {
        let mut merchants = placed();
        visit(&mut merchants, BOB, 2, "Bob");
        merchants.restock(BOB, Vec::new(), 1);
        let (reply, mut closed) = oneshot::channel();
        merchants.close(BOB, reply);
        assert!(closed.try_recv().is_err());
        assert_eq!(merchants.merchants[&OWNER].data.meso, 0);
    }

    #[test]
    fn purchases_held_too_long_are_let_go_and_the_merchant_read_again() {
        let mut merchants = placed();
        visit(&mut merchants, BOB, 2, "Bob");
        let held = reserve(&mut merchants, BOB, 1).expect("in stock");
        visit(&mut merchants, ALICE, OWNER, "Owner");
        let later = Instant::now() + RESERVATION_TIMEOUT;
        assert_eq!(merchants.stale(later), vec![OWNER]);
        assert_eq!(merchants.take_rereads(), vec![OWNER]);
        // The sale lands meanwhile: read once more.
        merchants.settle(OWNER, held.id, Some(sale(1)), false, SystemTime::now());
        assert_eq!(merchants.take_rereads(), vec![OWNER]);
        assert_eq!(merchants.merchants[&OWNER].data.meso, 0, "not counted here");
        let mut sold = merchant(SystemTime::now());
        sold.stock[0].listing.bundles = 4;
        sold.meso = 1;
        let out = merchants.reload(OWNER, Some(sold.clone()), SystemTime::now());
        assert!(
            !summary(&out).iter().any(|(to, _)| *to == ALICE),
            "the owner waits for the second read"
        );
        let out = merchants.reload(OWNER, Some(sold), SystemTime::now());
        assert_eq!(summary(&out)[0], (ALICE, "maintenance 1".into()), "let in");
    }

    #[test]
    fn the_owner_in_maintenance_hears_of_a_late_sale() {
        let mut merchants = placed();
        visit(&mut merchants, BOB, 2, "Bob");
        let held = reserve(&mut merchants, BOB, 1).expect("in stock");
        merchants.stale(Instant::now() + RESERVATION_TIMEOUT);
        merchants.take_rereads();
        merchants.reload(OWNER, Some(merchant(SystemTime::now())), SystemTime::now());
        let out = visit(&mut merchants, ALICE, OWNER, "Owner");
        assert!(summary(&out).contains(&(ALICE, "maintenance 0".into())));
        merchants.settle(OWNER, held.id, Some(sale(1)), false, SystemTime::now());
        assert_eq!(merchants.take_rereads(), vec![OWNER]);
        let mut sold = merchant(SystemTime::now());
        sold.meso = 1;
        let out = merchants.reload(OWNER, Some(sold), SystemTime::now());
        assert_eq!(summary(&out)[0], (ALICE, "refreshed 1".into()));
    }

    #[test]
    fn nothing_sells_while_the_merchant_is_read_again() {
        let mut merchants = placed();
        visit(&mut merchants, BOB, 2, "Bob");
        let first = reserve(&mut merchants, BOB, 1).expect("in stock");
        merchants.stale(Instant::now() + RESERVATION_TIMEOUT);
        assert_eq!(
            reserve(&mut merchants, BOB, 1).unwrap_err(),
            buy_result::NO_STOCK
        );
        // Let go of and not written: the database is right as it is.
        merchants.settle(OWNER, first.id, None, false, SystemTime::now());
        assert_eq!(merchants.take_rereads(), vec![OWNER]);
        merchants.reload(OWNER, Some(merchant(SystemTime::now())), SystemTime::now());
        assert!(reserve(&mut merchants, BOB, 1).is_ok());
    }

    #[test]
    fn a_merchant_whose_time_is_up_goes() {
        let mut merchants = Merchants::default();
        let opened = SystemTime::now() - game::hired_merchant::LIFETIME;
        merchants.load(vec![merchant(opened)]);
        visit(&mut merchants, BOB, 2, "Bob");
        merchants.take_shown();
        let (out, due) = merchants.expire(SystemTime::now());
        assert_eq!(due, vec![OWNER]);
        assert_eq!(summary(&out), vec![(BOB, "left 0xa [1, 18]".into())]);
        assert_eq!(merchants.take_shown(), vec![Shown::Destroy(OWNER)]);
    }

    #[test]
    fn a_reread_merchant_no_longer_open_goes() {
        let mut merchants = placed();
        let mut data = merchant(SystemTime::now());
        data.meso = 9;
        merchants.reload(OWNER, Some(data), SystemTime::now());
        assert_eq!(merchants.merchants[&OWNER].data.meso, 9);
        merchants.reload(OWNER, None, SystemTime::now());
        assert!(!merchants.contains(sn(OWNER)));
    }
    #[test]
    fn the_owl_finds_open_merchants_rows() {
        let mut merchants = placed();
        let hits = merchants.search(2000000, 910000001, 0);
        assert_eq!(hits.len(), 1);
        assert_eq!((hits[0].room, hits[0].bundles), (sn(OWNER), 5));
        visit(&mut merchants, ALICE, OWNER, "Owner");
        assert!(merchants.search(2000000, 910000001, 0).is_empty());
    }

    #[test]
    fn a_new_merchant_stands_clear_of_merchants_and_shops() {
        let merchants = placed();
        // The placed merchant stands at (100, 34).
        assert!(!merchants.clear_of(BOB, 100 + 149, 34, []));
        assert!(merchants.clear_of(BOB, 100 + 150, 34, []));
        assert!(!merchants.clear_of(BOB, 100 + 106, 34 + 106, []));
        assert!(!merchants.clear_of(BOB, -500, 34, [(-400, 34)]));
        assert!(merchants.clear_of(BOB, -500, 34, [(-200, 34)]));
    }

    #[test]
    fn a_spot_is_kept_while_its_merchant_is_set_up() {
        let mut merchants = placed();
        merchants.hold_stand(CAROL, -500, 34);
        assert!(!merchants.clear_of(BOB, -400, 34, []));
        assert!(merchants.clear_of(CAROL, -400, 34, []), "their own spot");
        merchants.leave(CAROL, Instant::now());
        assert!(merchants.clear_of(BOB, -400, 34, []));
        merchants.hold_stand(CAROL, -500, 34);
        let mut theirs = merchant(SystemTime::now());
        theirs.owner_id = OWNER + 1;
        theirs.stand.x = -500;
        merchants.place(CAROL, theirs, Vec::new());
        assert!(
            !merchants.clear_of(BOB, -400, 34, []),
            "placed where it was kept"
        );
        merchants.hold_stand(CAROL, 800, 34);
        merchants.place(CAROL, merchant(SystemTime::now()), Vec::new());
        assert!(merchants.clear_of(BOB, 800, 34, []), "let go once placed");
    }

    #[test]
    fn a_purchase_the_database_refused_reads_the_merchant_again() {
        let mut merchants = placed();
        visit(&mut merchants, BOB, 2, "Bob");
        let held = reserve(&mut merchants, BOB, 1).expect("reserved");
        let out = merchants.settle(OWNER, held.id, None, true, SystemTime::now());
        assert!(out.is_empty());
        assert_eq!(merchants.take_rereads(), vec![OWNER]);
        assert_eq!(
            reserve(&mut merchants, BOB, 1).unwrap_err(),
            buy_result::NO_STOCK
        );
    }

    #[test]
    fn an_unwanted_seat_is_given_up_only_in_its_own_room() {
        let mut merchants = placed();
        visit(&mut merchants, BOB, 2, "Bob");
        merchants.hold_stand(BOB, 800, 34);
        merchants.unseat(BOB, OWNER + 1, Instant::now());
        assert!(merchants.is_member(BOB), "another merchant's seat");
        merchants.unseat(BOB, OWNER, Instant::now());
        assert!(!merchants.is_member(BOB));
        assert!(!merchants.clear_of(CAROL, 800, 34, []), "the spot stays");
    }
}
