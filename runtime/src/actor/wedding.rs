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
//! As with a party event (`crate::actor::event`), the partners' client
//! actors do the moving, and a ticket hands itself back when dropped.

use crate::actor::registry::FieldRegistry;
use crate::db::spawn_db;
use crate::message::{ClientHandle, DirectoryMessage, ServerMessage};
use crate::supervisor::spawn_supervised;
use game::wedding::{Venue, Vows, WeddingStage, AFTER_VOWS, CEREMONY, GATHERING};
use net::events::scaled;
use net::ClientId;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep, sleep_until, Instant};
use tracing::{info, warn};

/// How long the couple have to get out after the wedding ends before its
/// fields close.
const GRACE: Duration = Duration::from_secs(2);
const WEDDING_QUEUE_CAPACITY: usize = 16;

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
}

/// What a partner holds while in the wedding: the wedding's own fields for
/// the venue's maps, and when its stage ends. Dropping it leaves the
/// wedding.
#[derive(Debug)]
pub struct WeddingTicket {
    pub id: u32,
    pub venue: Venue,
    /// When the time shown on the wedding's maps runs out.
    pub clock: Option<Instant>,
    member: ClientId,
    tx: mpsc::Sender<WeddingMessage>,
    /// The wedding's stage, shared with it; only it writes.
    stage: Arc<AtomicU8>,
}

impl Drop for WeddingTicket {
    fn drop(&mut self) {
        // Each partner has one ticket, so the queue can't fill.
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
/// `channel_id`, for the couple `marriage_id`.
#[derive(Debug)]
pub struct WeddingStart {
    pub id: u32,
    pub venue: Venue,
    pub channel_id: u8,
    pub marriage_id: i32,
    pub couple: Vec<ClientHandle>,
}

/// Start the wedding's actor, which hands out the tickets.
pub fn spawn(
    start: WeddingStart,
    fields: Arc<FieldRegistry>,
    directory: mpsc::Sender<DirectoryMessage>,
) {
    let (id, venue, channel_id) = (start.id, start.venue, start.channel_id);
    fields.open_instance(channel_id, id);
    let (tx, rx) = mpsc::channel(WEDDING_QUEUE_CAPACITY);
    let couple = start.couple.clone();
    let actor = WeddingActor {
        id,
        venue,
        marriage_id: start.marriage_id,
        ends: Instant::now() + scaled(GATHERING),
        tx: Some(tx),
        rx,
        members: start
            .couple
            .into_iter()
            .map(|member| (member.client_id, member))
            .collect(),
        vowed: HashSet::new(),
        stage: Arc::new(AtomicU8::new(WeddingStage::Gathering as u8)),
        failed: false,
    };
    // However it ends, even by a panic, the couple are sent out, the copy
    // closes and the venue is free again.
    spawn_supervised(format!("wedding {venue:?} {id}"), actor.run(), move || {
        for member in &couple {
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
    /// Ours until the tickets are out; then only they keep the queue open.
    tx: Option<mpsc::Sender<WeddingMessage>>,
    rx: mpsc::Receiver<WeddingMessage>,
    /// The partners still in the wedding.
    members: HashMap<ClientId, ClientHandle>,
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
        if let Some(tx) = self.tx.take() {
            for member in self.members.values() {
                // A ticket that doesn't arrive hands itself back.
                member.send(ServerMessage::WeddingStarted(WeddingTicket {
                    id,
                    venue,
                    clock: Some(self.ends),
                    member: member.client_id,
                    tx: tx.clone(),
                    stage: self.stage.clone(),
                }));
            }
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
        for member in self.members.values() {
            member.send(ServerMessage::WeddingEnded {
                id,
                failed: self.failed,
            });
        }
        // Give them time to get out before the copy closes.
        sleep(GRACE).await;
    }

    fn stage(&self) -> WeddingStage {
        WeddingStage::from_u8(self.stage.load(Ordering::Acquire))
    }

    fn set_stage(&self, stage: WeddingStage) {
        self.stage.store(stage as u8, Ordering::Release);
    }

    /// A partner left: before the vows that ends the wedding; after them,
    /// it goes on while one of the couple stays.
    fn left(&mut self, client_id: ClientId) -> bool {
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
                for member in self.members.values() {
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
        for member in self.members.values() {
            member.send(ServerMessage::Married {
                id: self.id,
                record: record.clone(),
                clock: self.ends,
            });
        }
        Vows::Married
    }
}
