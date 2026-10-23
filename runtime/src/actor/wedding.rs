//! One couple's wedding in its own copy of the venue's hall and altar
//! (report b071). The directory checks the couple may start and that the
//! venue is free on the channel; this actor gives each partner a ticket into
//! the copy, keeps the time, marries them when both have taken their vows,
//! and when the wedding ends sends those still in it to the exit, closes the
//! copy's fields and frees the venue.
//!
//! The hall gathers the couple for a while, then the altar holds the
//! ceremony on a clock of its own. A partner who leaves before the vows
//! ends the wedding, and the booking with it; after them the couple may
//! stay until the staff see them out or the time runs out.
//!
//! Invited guests come in through the venue's assistant while the couple
//! gather, up to `MAX_GUESTS`, and go on to the altar with them; a guest
//! may leave at any time without ending anything.
//!
//! Once married, the couple of a premium wedding may take everyone on to
//! the party room for the rest of their time (report b080); the couple of
//! any other may only end it.
//!
//! The priest speaks to everyone at the altar (report b075): his welcome as
//! the ceremony begins, the pronouncement once the couple are married, and
//! then the hearts between them and his farewell. He goes on from a line
//! when one of the couple closes it, or after `LINE_WAIT`. At the Cathedral
//! his welcome leads, before the vows, to his call for blessings and then
//! to the blessing, when each guest may bless the couple once until they
//! are married.
//!
//! As with a party event (`crate::actor::event`), the client actors do the
//! moving, and a ticket hands itself back when dropped.

use crate::actor::registry::FieldRegistry;
use crate::db::spawn_db;
use crate::message::{ClientHandle, DirectoryMessage, ServerMessage};
use crate::supervisor::spawn_supervised;
use game::wedding::{
    GuestRefusal, Registry, Venue, Vows, WeddingStage, AFTER_VOWS, CEREMONY, GATHERING, MAX_GUESTS,
    PARTY_ROOM, PRIEST_WELCOME,
};
use net::events::scaled;
use net::ClientId;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep, sleep_until, Instant};
use tracing::{info, warn};

/// How long the couple have to get out after the wedding ends before its
/// fields close.
const GRACE: Duration = Duration::from_secs(2);
/// How long each of the priest's lines that leads on waits for the couple
/// to close it before he goes on anyway.
const LINE_WAIT: Duration = Duration::from_secs(10);
/// Room for each of the couple and the guests to leave while another
/// message of theirs waits, so a dropped ticket is never turned away. Each
/// of the couple may also have up to five closes queued (one per step of
/// the priest's), which the slack covers.
pub const WEDDING_QUEUE_CAPACITY: usize = 2 * (MAX_GUESTS as usize + 2) + 16;

#[derive(Debug)]
pub enum WeddingMessage {
    /// `client_id` left the wedding's maps or the game.
    Left { client_id: ClientId },
    /// `client_id` could not take their ticket: the wedding never began.
    Declined { client_id: ClientId },
    /// `client_id` takes their vows.
    Vow {
        client_id: ClientId,
        reply: oneshot::Sender<Vows>,
    },
    /// `client_id` asks the staff to end the wedding, or to take a premium
    /// one on to its party room.
    End { client_id: ClientId },
    /// An invited guest asks to come in.
    Join {
        guest: ClientHandle,
        reply: oneshot::Sender<Result<WeddingTicket, GuestRefusal>>,
    },
    /// `client_id` closed the priest's line `step`.
    Talk { client_id: ClientId, step: u8 },
    /// The guest `client_id` blesses the couple: whether it counted.
    Bless {
        client_id: ClientId,
        reply: oneshot::Sender<bool>,
    },
}

/// What the priest shows everyone at the altar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PriestLine {
    /// His line `step`, for the couple `groom` and `bride`.
    Says { step: u8, groom: i32, bride: i32 },
    /// The hearts between the married couple.
    Hearts,
}

/// What one of the couple or a guest holds while in the wedding: the
/// wedding's own fields for the venue's maps, and when its stage ends.
/// Dropping it leaves the wedding.
#[derive(Debug)]
pub struct WeddingTicket {
    pub id: u32,
    pub venue: Venue,
    /// When the time shown on the wedding's maps runs out.
    pub clock: Option<Instant>,
    member: ClientId,
    /// Held by a guest rather than one of the couple.
    pub guest: bool,
    /// The wedding was booked with a premium ticket.
    pub premium: bool,
    /// The groom's and the bride's character ids.
    pub couple: (i32, i32),
    tx: mpsc::Sender<WeddingMessage>,
    /// The wishlists and gifts, shared with the other tickets.
    registry: Arc<Mutex<Registry>>,
    /// The wedding's stage, shared with it; only it writes.
    stage: Arc<AtomicU8>,
    /// How many of the priest's lines have had their close passed on.
    talked: AtomicU8,
    /// Whether the guest's blessing counted already.
    blessed: AtomicBool,
}

impl Drop for WeddingTicket {
    fn drop(&mut self) {
        // The queue has room for everyone in it to leave.
        let _ = self.tx.try_send(WeddingMessage::Left {
            client_id: self.member,
        });
    }
}

impl WeddingTicket {
    /// Whether `map_id` is one of the wedding's maps, held in its copy.
    pub fn holds(&self, map_id: i32) -> bool {
        self.venue.holds(map_id)
    }

    pub fn stage(&self) -> WeddingStage {
        WeddingStage::from_u8(self.stage.load(Ordering::Acquire))
    }

    /// Hand back a ticket the partner can't take, which ends the wedding
    /// before it began, keeping the booking.
    pub fn decline(self) {
        let _ = self.tx.try_send(WeddingMessage::Declined {
            client_id: self.member,
        });
    }

    /// Take the partner's vows: where the couple stand once they are taken.
    pub async fn vow(&self) -> Vows {
        let (reply, vows) = oneshot::channel();
        let message = WeddingMessage::Vow {
            client_id: self.member,
            reply,
        };
        if self.tx.send(message).await.is_err() {
            return Vows::Refused;
        }
        vows.await.unwrap_or(Vows::Refused)
    }

    /// Say one of the couple closed the priest's line `step`. Only the
    /// first close of each line, in order, is passed on, so the queue
    /// keeps its room for leaving.
    pub fn talk(&self, step: u8) {
        if self.guest || step > self.venue.farewell() {
            return;
        }
        if self.talked.fetch_max(step + 1, Ordering::AcqRel) > step {
            return;
        }
        let _ = self.tx.try_send(WeddingMessage::Talk {
            client_id: self.member,
            step,
        });
    }

    /// Bless the couple, as a guest while the priest lets them: whether it
    /// counted. Each guest's blessing counts once.
    pub async fn bless(&self) -> bool {
        let open = self.venue.blessing().is_some() && self.stage() == WeddingStage::Ceremony;
        if !self.guest || !open || self.blessed.load(Ordering::Acquire) {
            return false;
        }
        let (reply, counted) = oneshot::channel();
        let message = WeddingMessage::Bless {
            client_id: self.member,
            reply,
        };
        if self.tx.send(message).await.is_err() {
            return false;
        }
        let counted = counted.await.unwrap_or(false);
        if counted {
            self.blessed.store(true, Ordering::Release);
        }
        counted
    }

    /// The wedding's wishlists and gifts.
    pub fn registry(&self) -> MutexGuard<'_, Registry> {
        self.registry.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Ask for the wedding to end, or to go on to its party room.
    pub async fn end(&self) {
        let _ = self
            .tx
            .send(WeddingMessage::End {
                client_id: self.member,
            })
            .await;
    }
}

/// A wedding the directory let a couple start: `id` at `venue` on
/// `channel_id`, for the couple `marriage_id`. Guests reach it by `tx`,
/// which the directory keeps.
#[derive(Debug)]
pub struct WeddingStart {
    pub id: u32,
    pub venue: Venue,
    pub channel_id: u8,
    pub marriage_id: i32,
    pub couple: Vec<ClientHandle>,
    pub tx: mpsc::Sender<WeddingMessage>,
    pub rx: mpsc::Receiver<WeddingMessage>,
}

/// Start the wedding's actor, which hands out the tickets, for the couple
/// `groom` and `bride`, booked with a premium ticket or not.
pub fn spawn(
    start: WeddingStart,
    (groom, bride): (i32, i32),
    premium: bool,
    fields: Arc<FieldRegistry>,
    directory: mpsc::Sender<DirectoryMessage>,
) {
    let (id, venue, channel_id) = (start.id, start.venue, start.channel_id);
    fields.open_instance(channel_id, id);
    let couple = start.couple.clone();
    let guests = Arc::new(Mutex::new(HashMap::new()));
    let inside = guests.clone();
    let actor = WeddingActor {
        id,
        venue,
        marriage_id: start.marriage_id,
        groom,
        bride,
        premium,
        partying: false,
        ends: Instant::now() + scaled(GATHERING),
        line: None,
        next_line: None,
        blessers: HashSet::new(),
        tx: start.tx,
        registry: Arc::default(),
        rx: start.rx,
        guests,
        members: start
            .couple
            .into_iter()
            .map(|member| (member.client_id, member))
            .collect(),
        vowed: HashSet::new(),
        stage: Arc::new(AtomicU8::new(WeddingStage::Gathering as u8)),
        failed: false,
    };
    // However it ends, even by a panic, the couple and guests are sent
    // out, the copy closes and the venue is free again.
    spawn_supervised(format!("wedding {venue:?} {id}"), actor.run(), move || {
        let guests = inside
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        for member in couple.iter().chain(guests.values()) {
            member.send(ServerMessage::WeddingEnded { id, failed: false });
        }
        fields.close_instance(channel_id, id);
        tokio::spawn(async move {
            let _ = directory
                .send(DirectoryMessage::WeddingClosed {
                    channel_id,
                    venue,
                    id,
                })
                .await;
        });
    });
}

struct WeddingActor {
    id: u32,
    venue: Venue,
    marriage_id: i32,
    groom: i32,
    bride: i32,
    premium: bool,
    /// Everyone went on to the party room.
    partying: bool,
    /// When the current stage's time runs out.
    ends: Instant,
    /// The priest's last line at the altar.
    line: Option<u8>,
    /// When the priest goes on from it if neither of the couple closes it
    /// first, if he does.
    next_line: Option<Instant>,
    /// The guests still in the wedding who blessed the couple.
    blessers: HashSet<ClientId>,
    /// For the tickets; the directory keeps another for the guests.
    tx: mpsc::Sender<WeddingMessage>,
    registry: Arc<Mutex<Registry>>,
    rx: mpsc::Receiver<WeddingMessage>,
    /// The partners still in the wedding.
    members: HashMap<ClientId, ClientHandle>,
    /// The guests in it, shared with whoever sees them out after a panic.
    guests: Arc<Mutex<HashMap<ClientId, ClientHandle>>>,
    vowed: HashSet<ClientId>,
    stage: Arc<AtomicU8>,
    /// The wedding ended before the couple married.
    failed: bool,
}

impl WeddingActor {
    async fn run(mut self) {
        let (id, venue) = (self.id, self.venue);
        let couple: Vec<_> = self.members.keys().copied().collect();
        info!(
            ?venue,
            id,
            marriage_id = self.marriage_id,
            ?couple,
            "Wedding started"
        );
        for member in self.members.values() {
            // A ticket that doesn't arrive hands itself back.
            let ticket = self.ticket(member.client_id, false);
            member.send(ServerMessage::WeddingStarted(ticket));
        }
        let mut over = false;
        while !over {
            let ends = self.ends;
            let next_line = self.next_line;
            tokio::select! {
                message = self.rx.recv() => match message {
                    Some(WeddingMessage::Left { client_id }) => over = self.left(client_id),
                    Some(WeddingMessage::Declined { client_id }) => {
                        info!(?venue, id, client_id, "Wedding declined");
                        self.members.remove(&client_id);
                        over = true;
                    }
                    Some(WeddingMessage::Vow { client_id, reply }) => {
                        let vows = self.vow(client_id).await;
                        let _ = reply.send(vows);
                    }
                    Some(WeddingMessage::End { client_id }) => {
                        let married = self.members.contains_key(&client_id)
                            && self.stage() == WeddingStage::Married;
                        over = married && self.after_party(client_id);
                    }
                    Some(WeddingMessage::Join { guest, reply }) => {
                        // A ticket the guest can't take hands itself back.
                        let _ = reply.send(self.join(guest));
                    }
                    Some(WeddingMessage::Talk { client_id, step }) => {
                        if self.members.contains_key(&client_id)
                            && self.line == Some(step)
                            && self.next_line.is_some()
                        {
                            self.go_on();
                        }
                    }
                    Some(WeddingMessage::Bless { client_id, reply }) => {
                        let _ = reply.send(self.bless(client_id));
                    }
                    None => over = true,
                },
                _ = sleep_until(ends) => over = self.time_up(),
                _ = sleep_until(next_line.unwrap_or(ends)), if next_line.is_some() => self.go_on(),
            }
        }
        if self.failed {
            info!(?venue, id, marriage_id = self.marriage_id, "Wedding failed");
            // No-shows get nothing back (report b071 Q9).
            let marriage_id = self.marriage_id;
            if let Err(error) = spawn_db(move || db::wedding::cancel(marriage_id)).await {
                warn!(id, %error, "Failed wedding's booking not cancelled");
            }
        } else {
            info!(?venue, id, "Wedding ended");
        }
        // Nothing is answered from here on.
        drop(self.rx);
        let failed = self.failed;
        let ended = || ServerMessage::WeddingEnded { id, failed };
        for member in self.members.values() {
            member.send(ended());
        }
        for guest in self
            .guests
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
        {
            guest.send(ended());
        }
        // Give them time to get out before the copy closes.
        sleep(GRACE).await;
    }

    fn stage(&self) -> WeddingStage {
        WeddingStage::from_u8(self.stage.load(Ordering::Acquire))
    }

    fn guests(&self) -> MutexGuard<'_, HashMap<ClientId, ClientHandle>> {
        self.guests.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn ticket(&self, member: ClientId, guest: bool) -> WeddingTicket {
        WeddingTicket {
            id: self.id,
            venue: self.venue,
            clock: Some(self.ends),
            member,
            guest,
            premium: self.premium,
            couple: (self.groom, self.bride),
            tx: self.tx.clone(),
            registry: self.registry.clone(),
            stage: self.stage.clone(),
            talked: AtomicU8::new(0),
            blessed: AtomicBool::new(false),
        }
    }

    /// Let an invited guest in while the couple gather and there is room.
    fn join(&mut self, guest: ClientHandle) -> Result<WeddingTicket, GuestRefusal> {
        if self.stage() != WeddingStage::Gathering {
            return Err(GuestRefusal::Late);
        }
        let client_id = guest.client_id;
        let mut guests = self.guests();
        if guests.len() >= MAX_GUESTS as usize && !guests.contains_key(&client_id) {
            return Err(GuestRefusal::Full);
        }
        guests.insert(client_id, guest);
        drop(guests);
        info!(venue = ?self.venue, id = self.id, client_id, "Wedding guest came in");
        Ok(self.ticket(client_id, true))
    }

    /// Show `line` to everyone in the wedding; the client actors keep it
    /// to those at the altar.
    fn priest(&self, line: PriestLine) {
        for member in self.members.values().chain(self.guests().values()) {
            member.send(ServerMessage::WeddingPriest { id: self.id, line });
        }
    }

    /// The priest says his line `step`, and waits on it if he goes on
    /// from it.
    fn say(&mut self, step: u8) {
        self.priest(PriestLine::Says {
            step,
            groom: self.groom,
            bride: self.bride,
        });
        self.line = Some(step);
        let leads_on = step == self.venue.pronouncing() || self.venue.before_vows(step).is_some();
        self.next_line = leads_on.then(|| Instant::now() + LINE_WAIT);
    }

    /// The priest goes on from his last line: after the pronouncement, the
    /// hearts between the married couple and his farewell; before the
    /// vows, his next line, if any.
    fn go_on(&mut self) {
        self.next_line = None;
        let Some(line) = self.line else {
            return;
        };
        if line == self.venue.pronouncing() {
            self.priest(PriestLine::Hearts);
            self.say(self.venue.farewell());
        } else if let Some(next) = self.venue.before_vows(line) {
            self.say(next);
        }
    }

    /// The guest `client_id` blesses the couple: it counts once, while the
    /// priest asks for blessings and the couple have yet to marry.
    fn bless(&mut self, client_id: ClientId) -> bool {
        let asking = self
            .venue
            .blessing()
            .is_some_and(|step| self.line == Some(step));
        let counted = asking
            && self.stage() == WeddingStage::Ceremony
            && self.guests().contains_key(&client_id)
            && self.blessers.insert(client_id);
        if counted {
            info!(venue = ?self.venue, id = self.id, client_id, "Wedding guest blessed the couple");
        }
        counted
    }

    /// The married couple asked the staff to see everyone out: a premium
    /// wedding goes on to its party room first, with everyone still in it,
    /// for the rest of the couple's time. Whether the wedding ends.
    fn after_party(&mut self, client_id: ClientId) -> bool {
        if !self.premium {
            return true;
        }
        let (id, ends) = (self.id, self.ends);
        let moved = || ServerMessage::WeddingMoved {
            id,
            to: PARTY_ROOM,
            clock: Some(ends),
        };
        if self.partying {
            // One who missed the move (in the Cash Shop) asks again.
            if let Some(member) = self.members.get(&client_id) {
                member.send(moved());
            }
            return false;
        }
        self.partying = true;
        for member in self.members.values().chain(self.guests().values()) {
            member.send(moved());
        }
        info!(venue = ?self.venue, id = self.id, "Wedding party started");
        false
    }

    fn set_stage(&self, stage: WeddingStage) {
        self.stage.store(stage as u8, Ordering::Release);
    }

    /// A partner left: before the vows that ends the wedding; after them,
    /// it goes on while one of the couple stays. A guest only goes.
    fn left(&mut self, client_id: ClientId) -> bool {
        if self.guests().remove(&client_id).is_some() {
            // A blessing counts only from those who stay for the vows.
            self.blessers.remove(&client_id);
            return false;
        }
        if self.members.remove(&client_id).is_none() {
            return false;
        }
        if self.stage() != WeddingStage::Married {
            self.failed = true;
            return true;
        }
        self.members.is_empty()
    }

    /// The stage's time ran out: the gathering moves on to the altar; the
    /// ceremony without vows fails; the couple's time after them ends.
    fn time_up(&mut self) -> bool {
        match self.stage() {
            WeddingStage::Gathering => {
                self.set_stage(WeddingStage::Ceremony);
                self.ends = Instant::now() + scaled(CEREMONY);
                // The guests take their seats at the altar too.
                for member in self.members.values().chain(self.guests().values()) {
                    member.send(ServerMessage::WeddingMoved {
                        id: self.id,
                        to: self.venue.altar(),
                        clock: Some(self.ends),
                    });
                }
                self.say(PRIEST_WELCOME);
                info!(venue = ?self.venue, id = self.id, "Ceremony started");
                false
            }
            WeddingStage::Ceremony => {
                self.failed = true;
                true
            }
            WeddingStage::Married => true,
        }
    }

    /// Take `client_id`'s vows; once both partners have, marry them.
    async fn vow(&mut self, client_id: ClientId) -> Vows {
        if !self.members.contains_key(&client_id) {
            return Vows::Refused;
        }
        match self.stage() {
            WeddingStage::Married => return Vows::Married,
            WeddingStage::Gathering => return Vows::Refused,
            WeddingStage::Ceremony => {}
        }
        self.vowed.insert(client_id);
        if !self
            .members
            .keys()
            .all(|member| self.vowed.contains(member))
        {
            return Vows::Waiting;
        }
        let marriage_id = self.marriage_id;
        let record = match spawn_db(move || db::marriage::marry(marriage_id)).await {
            Ok(Some(record)) => record,
            Ok(None) => {
                warn!(id = self.id, marriage_id, "Couple to marry is not engaged");
                return Vows::Refused;
            }
            Err(error) => {
                warn!(id = self.id, %error, "Couple not married");
                return Vows::Refused;
            }
        };
        self.set_stage(WeddingStage::Married);
        self.ends = Instant::now() + scaled(AFTER_VOWS);
        info!(venue = ?self.venue, id = self.id, marriage_id, "Couple married");
        let blessings = u32::try_from(self.blessers.len()).unwrap_or(u32::MAX);
        // The guests' time starts again with the couple's.
        for member in self.members.values().chain(self.guests().values()) {
            member.send(ServerMessage::Married {
                id: self.id,
                record: record.clone(),
                blessings,
                clock: self.ends,
            });
        }
        self.say(self.venue.pronouncing());
        Vows::Married
    }
}
