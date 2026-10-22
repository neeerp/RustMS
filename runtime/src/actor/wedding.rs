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
//! As with a party event (`crate::actor::event`), the client actors do the
//! moving, and a ticket hands itself back when dropped.

use crate::actor::registry::FieldRegistry;
use crate::db::spawn_db;
use crate::message::{ClientHandle, DirectoryMessage, ServerMessage};
use crate::supervisor::spawn_supervised;
use game::wedding::{
    GuestRefusal, Venue, Vows, WeddingStage, AFTER_VOWS, CEREMONY, GATHERING, MAX_GUESTS,
};
use net::events::scaled;
use net::ClientId;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep, sleep_until, Instant};
use tracing::{info, warn};

/// How long the couple have to get out after the wedding ends before its
/// fields close.
const GRACE: Duration = Duration::from_secs(2);
/// Room for each of the couple and the guests to leave while another
/// message of theirs waits, so a dropped ticket is never turned away.
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
    /// `client_id` asks the staff to end the wedding.
    End { client_id: ClientId },
    /// An invited guest asks to come in.
    Join {
        guest: ClientHandle,
        reply: oneshot::Sender<Result<WeddingTicket, GuestRefusal>>,
    },
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
    tx: mpsc::Sender<WeddingMessage>,
    /// The wedding's stage, shared with it; only it writes.
    stage: Arc<AtomicU8>,
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

    /// Ask for the wedding to end.
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

/// Start the wedding's actor, which hands out the tickets.
pub fn spawn(
    start: WeddingStart,
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
        ends: Instant::now() + scaled(GATHERING),
        tx: start.tx,
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
    /// When the current stage's time runs out.
    ends: Instant,
    /// For the tickets; the directory keeps another for the guests.
    tx: mpsc::Sender<WeddingMessage>,
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
                        over = self.members.contains_key(&client_id)
                            && self.stage() == WeddingStage::Married;
                    }
                    Some(WeddingMessage::Join { guest, reply }) => {
                        // A ticket the guest can't take hands itself back.
                        let _ = reply.send(self.join(guest));
                    }
                    None => over = true,
                },
                _ = sleep_until(ends) => over = self.time_up(),
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
            tx: self.tx.clone(),
            stage: self.stage.clone(),
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

    fn set_stage(&self, stage: WeddingStage) {
        self.stage.store(stage as u8, Ordering::Release);
    }

    /// A partner left: before the vows that ends the wedding; after them,
    /// it goes on while one of the couple stays. A guest only goes.
    fn left(&mut self, client_id: ClientId) -> bool {
        if self.guests().remove(&client_id).is_some() {
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
        // The guests' time starts again with the couple's.
        for member in self.members.values().chain(self.guests().values()) {
            member.send(ServerMessage::Married {
                id: self.id,
                record: record.clone(),
                clock: self.ends,
            });
        }
        Vows::Married
    }
}
