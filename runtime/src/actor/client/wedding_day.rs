//! The couple's side of their wedding (`crate::actor::wedding`, report
//! b071): Wayne starts it, the ticket puts them in the venue's own copy of
//! its hall and altar, the priest takes their vows, the marriage brings the
//! rings, and the staff or the clock see them out.

use super::marriage::take_tokens;
use super::ClientActor;
use crate::actor::wedding::WeddingTicket;
use crate::db::spawn_db;
use crate::error::RuntimeError;
use crate::message::DirectoryMessage;
use db::parcel::NewParcel;
use game::marriage::{self, MarriageRecord, ENGAGED};
use game::wedding::{self, StartRefusal, Venue, AMORIA, WEDDING_EXIT};
use game_data::server_content::keys;
use net::npc_scripts::{self, NpcScript};
use net::packet::build::world::conti::{build_countdown, build_stop_clock};
use net::packet::build::world::inventory;
use net::packet::build::world::marriage::{build_married, build_wedding_notice, WeddingNotice};
use std::time::SystemTime;
use tokio::sync::oneshot;
use tokio::time::Instant;
use tracing::{info, warn};

impl ClientActor {
    /// Start the script of a wedding's starter, priest or staff. Whether
    /// `npc_id` was one, here.
    pub(super) async fn talk_to_wedding_staff(
        &mut self,
        npc_id: i32,
    ) -> Result<bool, RuntimeError> {
        if let Some(venue) = npc_scripts::wedding_starter_venue(npc_id) {
            let player = &self.in_game()?.player;
            let couple = engaged(player);
            let now = SystemTime::now();
            let reservation = match couple {
                Some(marriage_id) => {
                    spawn_db(move || db::wedding::current(marriage_id, now)).await?
                }
                None => None,
            };
            let player = &self.in_game()?.player;
            let script = NpcScript::wedding_starter(
                npc_id,
                player,
                reservation.as_ref().filter(|booked| booked.venue == venue),
                self.channel_id,
                now,
            );
            return match script {
                Some(script) => self.start_script(script).await.map(|()| true),
                None => Ok(false),
            };
        }
        let stage = self.wedding_here().map(WeddingTicket::stage);
        let player = &self.in_game()?.player;
        let script = if let Some(venue) = npc_scripts::wedding_priest_venue(npc_id) {
            NpcScript::wedding_priest(venue, player, stage)
        } else if let Some(venue) = npc_scripts::wedding_staff_venue(npc_id) {
            let Some(stage) = stage else {
                return Ok(false);
            };
            NpcScript::wedding_staff(venue, stage)
        } else {
            return Ok(false);
        };
        self.start_script(script).await.map(|()| true)
    }

    /// The player's wedding ticket, if they are on one of its maps.
    fn wedding_here(&self) -> Option<&WeddingTicket> {
        let map_id = self.game.as_ref()?.player.stats.map_id;
        self.wedding.as_ref().filter(|ticket| ticket.holds(map_id))
    }

    async fn starter_says(&mut self, venue: Venue, key: &str) -> Result<(), RuntimeError> {
        let text = net::content::text().get(key, &[]);
        self.start_script(NpcScript::wedding_starter_says(venue, text))
            .await
    }

    /// The player said yes to Wayne: start their booked wedding if it is
    /// still theirs to start and the partner stands beside them.
    pub(super) async fn start_wedding(&mut self, venue: Venue) -> Result<(), RuntimeError> {
        let player = &self.in_game()?.player;
        let Some((marriage_id, partner)) = player
            .marriage
            .as_ref()
            .filter(|record| record.status == ENGAGED)
            .map(|record| (record.id, record.partner_of(player.stats.id)))
        else {
            return Ok(());
        };
        let clothes = wedding::has_clothes(player);
        let now = SystemTime::now();
        let reservation = spawn_db(move || db::wedding::current(marriage_id, now)).await?;
        let startable = reservation.as_ref().is_some_and(|booked| {
            booked.venue == venue && booked.channel == self.channel_id && !booked.inviting(now)
        });
        if !startable || !clothes || self.event.is_some() || self.wedding.is_some() {
            warn!(self.client_id, marriage_id, "Wedding not startable");
            return self.starter_says(venue, keys::WEDDING_STARTER_ERROR).await;
        }
        let (reply, start) = oneshot::channel();
        self.send_to_directory(DirectoryMessage::StartWedding {
            from: self.client_id,
            partner,
            venue,
            marriage_id,
            reply,
        })
        .await?;
        match start.await {
            Ok(Ok(start)) => {
                crate::actor::wedding::spawn(
                    start,
                    self.services.fields.clone(),
                    self.services.directory.clone(),
                );
                Ok(())
            }
            Ok(Err(StartRefusal::PartnerAway)) => {
                self.starter_says(venue, keys::WEDDING_STARTER_PARTNER)
                    .await
            }
            Ok(Err(StartRefusal::Busy)) => {
                self.starter_says(venue, keys::WEDDING_STARTER_BUSY).await
            }
            Err(_) => self.starter_says(venue, keys::WEDDING_STARTER_ERROR).await,
        }
    }

    /// The player said yes to the priest. The checks are made again, as
    /// they may have changed while the priest asked.
    pub(super) async fn take_vows(&mut self) -> Result<(), RuntimeError> {
        let Some(ticket) = self.wedding_here() else {
            return Ok(());
        };
        let venue = ticket.venue;
        let player = &self.in_game()?.player;
        let ready = player.marriage.as_ref().is_some_and(|record| {
            let (empty_box, ring) = marriage::tokens(record.item_id);
            let token = player.inventory.count(empty_box) + player.inventory.count(ring) > 0;
            token
                && wedding::dressed(player)
                && net::items::fits_until(&player.inventory, record.ring_id(), 1, None)
        });
        if !ready {
            // The priest says what is missing.
            let stage = Some(ticket.stage());
            let script = NpcScript::wedding_priest(venue, player, stage);
            return self.start_script(script).await;
        }
        let vows = ticket.vow().await;
        self.start_script(NpcScript::wedding_priest_says(venue, vows))
            .await
    }

    /// The married player asked the staff to end their wedding.
    pub(super) async fn end_wedding(&mut self) {
        if let Some(ticket) = self.wedding_here() {
            ticket.end().await;
        }
    }

    /// The couple's wedding began: the player takes their ticket to the
    /// hall, unless they have since left Amoria, or are in the Cash Shop or
    /// an event.
    pub(super) async fn wedding_started(
        &mut self,
        ticket: WeddingTicket,
    ) -> Result<(), RuntimeError> {
        let waiting = !self.in_cash_shop
            && self.event.is_none()
            && self.wedding.is_none()
            && self
                .game
                .as_ref()
                .is_some_and(|game| game.player.stats.map_id == AMORIA);
        if !waiting {
            ticket.decline();
            return Ok(());
        }
        let hall = ticket.venue.hall();
        self.wedding = Some(ticket);
        self.warp_within_event(hall).await
    }

    /// Wedding `id` takes the player on to `to`, where the time left is
    /// `clock`.
    pub(super) async fn wedding_moved(
        &mut self,
        id: u32,
        to: i32,
        clock: Option<Instant>,
    ) -> Result<(), RuntimeError> {
        if self.in_cash_shop || self.game.is_none() {
            return Ok(());
        }
        let map_id = self.in_game()?.player.stats.map_id;
        let Some(ticket) = self.wedding.as_mut().filter(|ticket| ticket.id == id) else {
            return Ok(());
        };
        if !ticket.holds(map_id) {
            return Ok(());
        }
        ticket.clock = clock;
        self.warp_within_event(to).await
    }

    /// Wedding `id` ended: out to the exit, if the player still holds its
    /// ticket. A wedding that ended before the vows lost its booking, and
    /// the client says so.
    pub(super) async fn wedding_ended(
        &mut self,
        id: u32,
        failed: bool,
    ) -> Result<(), RuntimeError> {
        let Some(ticket) = self.wedding.as_ref().filter(|ticket| ticket.id == id) else {
            return Ok(());
        };
        let here = self
            .game
            .as_ref()
            .is_some_and(|game| ticket.holds(game.player.stats.map_id));
        if self.in_cash_shop || !here {
            self.drop_wedding_ticket().await?;
        } else {
            // Changing field to the exit hands the ticket back.
            self.warp_within_event(WEDDING_EXIT).await?;
        }
        if failed && !self.in_cash_shop && self.game.is_some() {
            self.send(&mut build_wedding_notice(WeddingNotice::Cancelled)?)
                .await?;
        }
        Ok(())
    }

    /// Hand the wedding's ticket back, and with it its clock.
    pub(super) async fn drop_wedding_ticket(&mut self) -> Result<(), RuntimeError> {
        if self.wedding.take().is_none() || self.in_cash_shop || self.game.is_none() {
            return Ok(());
        }
        self.send(&mut build_stop_clock()?).await
    }

    /// Wedding `id` married the couple `record`: the player gives up their
    /// half of the engagement and takes the wedding ring, and their time
    /// there starts again.
    pub(super) async fn married(
        &mut self,
        id: u32,
        record: MarriageRecord,
        clock: Instant,
    ) -> Result<(), RuntimeError> {
        let Some(game) = self.game.as_ref() else {
            return Ok(());
        };
        let mut next = game.player.clone();
        let mut packets = Vec::new();
        take_tokens(&mut next, &mut packets)?;
        let ring = record.ring_id();
        match net::items::give(&mut next.inventory, ring, 1) {
            Ok(changes) => {
                packets.push(inventory::build_inventory_operation(false, &changes)?);
                packets.push(inventory::build_item_gain_in_chat(ring, 1)?);
            }
            Err(error) => {
                // Their equipment filled up while the partner vowed.
                warn!(self.client_id, ring, %error, "Wedding ring sent by Duey");
                let partner = record.partner_of(next.stats.id);
                let from = if partner == record.groom_id {
                    record.groom_name.clone()
                } else {
                    record.bride_name.clone()
                };
                ring_by_duey(next.stats.id, from, ring).await;
            }
        }
        packets.push(build_married(&record)?);
        info!(self.client_id, marriage_id = record.id, "Married");
        next.marriage = Some(record);
        let here = self.wedding_here().is_some_and(|ticket| ticket.id == id);
        if let Some(ticket) = self.wedding.as_mut().filter(|ticket| ticket.id == id) {
            ticket.clock = Some(clock);
        }
        if here {
            let left = clock.saturating_duration_since(Instant::now());
            packets.push(build_countdown(
                u32::try_from(left.as_secs()).unwrap_or(u32::MAX),
            )?);
        }
        self.commit_engagement(next, packets).await
    }
}

/// Leave the wedding ring `ring` at Duey for `receiver`, from their
/// partner `from`, however full their Duey inbox is.
async fn ring_by_duey(receiver: i32, from: String, ring: i32) {
    let Some(item) = net::items::create(ring, 1) else {
        return;
    };
    let left = spawn_db(move || {
        let parcel = NewParcel {
            receiver_id: receiver,
            sender: &from,
            mesos: 0,
            message: None,
            quick: false,
            sent_at: SystemTime::now(),
            item: Some(&item),
        };
        db::parcel::leave(&parcel, i64::MAX)
    })
    .await;
    if let Err(error) = left {
        warn!(receiver, ring, %error, "Wedding ring lost");
    }
}

/// The couple's id, if the player is engaged.
fn engaged(player: &game::Player) -> Option<i32> {
    let record = player.marriage.as_ref()?;
    (record.status == ENGAGED).then_some(record.id)
}
