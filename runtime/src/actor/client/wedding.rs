//! Wedding reservations (report b066): Nicole and Bonnie book an engaged
//! couple's wedding for a ticket while both stand beside them. The partner
//! checks their own side through the directory; the booker then takes the
//! ticket and books, and each partner takes their own invitations.

use super::ClientActor;
use crate::message::{DirectoryMessage, WeddingNews};
use crate::{db::spawn_db, error::RuntimeError};
use db::wedding::Booking;
use game::marriage::ENGAGED;
use game::wedding::{self, PartnerRefusal, Reservation, Venue, INVITATIONS};
use game::Player;
use game_data::server_content::keys;
use net::npc_scripts::{self, NpcScript, WeddingView};
use net::packet::build::world::inventory;
use net::packet::build::world::marriage::{build_wedding_notice, WeddingNotice};
use net::packet::build::world::messaging::{build_broadcast, Broadcast};
use net::ClientId;
use packet::Packet;
use std::sync::OnceLock;
use std::time::{Duration, SystemTime};
use tracing::warn;

/// The soonest a booking may start: `RUSTMS_WEDDING_LEAD_MINUTES`, or
/// two hours.
fn lead() -> Duration {
    static LEAD: OnceLock<Duration> = OnceLock::new();
    *LEAD.get_or_init(|| {
        std::env::var("RUSTMS_WEDDING_LEAD_MINUTES")
            .ok()
            .and_then(|minutes| minutes.parse::<u64>().ok())
            .map_or(wedding::LEAD, |minutes| Duration::from_secs(minutes * 60))
    })
}

/// The couple's id and the partner's, if the player is engaged.
fn engagement(player: &Player) -> Option<(i32, ClientId)> {
    let record = player.marriage.as_ref()?;
    (record.status == ENGAGED).then(|| (record.id, record.partner_of(player.stats.id)))
}

/// Put the couple's invitations for `reservation` in `next`, expiring as
/// the wedding starts, with the packets that show them.
fn give_invitations(
    next: &mut Player,
    reservation: &Reservation,
    packets: &mut Vec<Packet>,
) -> Result<bool, RuntimeError> {
    let item_id = reservation.venue.invitation();
    let until = Some(reservation.starts_at);
    let Ok(changes) = net::items::give_until(&mut next.inventory, item_id, INVITATIONS, until)
    else {
        return Ok(false);
    };
    packets.push(inventory::build_inventory_operation(false, &changes)?);
    packets.push(inventory::build_item_gain_in_chat(
        item_id,
        i32::from(INVITATIONS),
    )?);
    Ok(true)
}

/// The packets both partners get for a booking: the client's notice and a
/// line saying when.
fn booked_packets(reservation: &Reservation) -> Result<Vec<Packet>, RuntimeError> {
    let line = npc_scripts::wedding_text(keys::WEDDING_DONE_CHAT, reservation, SystemTime::now());
    Ok(vec![
        build_wedding_notice(WeddingNotice::Reserved)?,
        build_broadcast(Broadcast::Pink, &line)?,
    ])
}

impl ClientActor {
    /// Start a wedding assistant's script, which needs the couple's
    /// booking. Whether `npc_id` was one, here.
    pub(super) async fn talk_to_wedding_assistant(
        &mut self,
        npc_id: i32,
    ) -> Result<bool, RuntimeError> {
        if npc_scripts::wedding_venue(npc_id).is_none() {
            return Ok(false);
        }
        let now = SystemTime::now();
        let reservation = match engagement(&self.in_game()?.player) {
            Some((marriage_id, _)) => {
                spawn_db(move || db::wedding::current(marriage_id, now)).await?
            }
            None => None,
        };
        let view = WeddingView {
            reservation: reservation.as_ref(),
            now,
        };
        let player = &self.in_game()?.player;
        match NpcScript::wedding_assistant(npc_id, player, view) {
            Some(script) => self.start_script(script).await.map(|()| true),
            None => Ok(false),
        }
    }

    async fn assistant_says(&mut self, venue: Venue, text: String) -> Result<(), RuntimeError> {
        self.start_script(NpcScript::wedding_assistant_says(venue, text))
            .await
    }

    /// The player asked the assistant of `venue` to book their wedding:
    /// ask the partner for their side.
    pub(super) async fn reserve_wedding(
        &mut self,
        venue: Venue,
        premium: bool,
    ) -> Result<(), RuntimeError> {
        let Some((_, partner)) = engagement(&self.in_game()?.player) else {
            let text = net::content::text().get(keys::WEDDING_ERROR, &[]);
            return self.assistant_says(venue, text).await;
        };
        self.send_to_directory(DirectoryMessage::ReserveWedding {
            from: self.client_id,
            partner,
            venue,
            premium,
        })
        .await
    }

    pub(super) async fn wedding_news(&mut self, news: WeddingNews) -> Result<(), RuntimeError> {
        match news {
            WeddingNews::Check {
                booker,
                venue,
                premium,
            } => self.check_wedding(booker, venue, premium).await,
            WeddingNews::Checked {
                venue,
                refusal: Some(refusal),
                ..
            } => {
                let text = npc_scripts::partner_refusal_text(refusal);
                self.assistant_says(venue, text).await
            }
            WeddingNews::Checked {
                venue,
                premium,
                refusal: None,
            } => self.book_wedding(venue, premium).await,
            WeddingNews::Booked(reservation) => self.wedding_booked(reservation).await,
        }
    }

    /// The partner `booker` books the couple's wedding: say whether the
    /// player can take part.
    async fn check_wedding(
        &mut self,
        booker: ClientId,
        venue: Venue,
        premium: bool,
    ) -> Result<(), RuntimeError> {
        let player = &self.in_game()?.player;
        if engagement(player).map(|(_, partner)| partner) != Some(booker) {
            warn!(self.client_id, booker, "Wedding check from someone else");
            return Ok(());
        }
        let refusal = if self.in_cash_shop {
            // The shop keeps the field's map and channel, but the player is
            // not beside the assistant.
            Some(PartnerRefusal::Elsewhere)
        } else if wedding::has_wedding_ring(player) {
            Some(PartnerRefusal::HasRing)
        } else if !npc_scripts::invitations_fit(player, venue) {
            Some(PartnerRefusal::NoRoom)
        } else {
            None
        };
        self.send_to_directory(DirectoryMessage::Wedding {
            to: booker,
            news: WeddingNews::Checked {
                venue,
                premium,
                refusal,
            },
        })
        .await
    }

    /// The partner can take part: check the player's side again, book, take
    /// the ticket, and give the invitations to both.
    async fn book_wedding(&mut self, venue: Venue, premium: bool) -> Result<(), RuntimeError> {
        if self.in_cash_shop {
            warn!(
                self.client_id,
                "Went to the Cash Shop while booking a wedding"
            );
            return Ok(());
        }
        let player = &self.in_game()?.player;
        let ticket = venue.ticket(premium);
        let ready = !wedding::has_wedding_ring(player)
            && player.inventory.count(ticket) > 0
            && npc_scripts::invitations_fit(player, venue);
        let error = || net::content::text().get(keys::WEDDING_ERROR, &[]);
        let Some((marriage_id, partner)) = engagement(player).filter(|_| ready) else {
            return self.assistant_says(venue, error()).await;
        };
        let channel = self.channel_id;
        let now = SystemTime::now();
        let booked =
            spawn_db(move || db::wedding::book(marriage_id, venue, premium, channel, now, lead()))
                .await;
        let reservation = match booked {
            Ok(Booking::Booked(reservation)) => reservation,
            Ok(Booking::AlreadyBooked(reservation)) => {
                let text = npc_scripts::wedding_text(keys::WEDDING_BOOKED, &reservation, now);
                return self.assistant_says(venue, text).await;
            }
            Ok(Booking::NotEngaged) => return self.assistant_says(venue, error()).await,
            Err(error_) => {
                warn!(self.client_id, %error_, "Wedding not booked");
                return self.assistant_says(venue, error()).await;
            }
        };

        let mut next = self.in_game()?.player.clone();
        let mut packets = Vec::new();
        let taken = next.inventory.remove(ticket, 1);
        let given = match taken {
            Ok(changes) => {
                packets.push(inventory::build_inventory_operation(false, &changes)?);
                packets.push(inventory::build_item_gain_in_chat(ticket, -1)?);
                give_invitations(&mut next, &reservation, &mut packets)?
            }
            Err(_) => false,
        };
        if !given || !self.commit_player(next, None, &mut packets).await? {
            if let Err(error_) = spawn_db(move || db::wedding::cancel(marriage_id)).await {
                warn!(self.client_id, %error_, "Booked wedding not cancelled");
            }
            return self.assistant_says(venue, error()).await;
        }
        packets.extend(booked_packets(&reservation)?);
        self.send_all(&mut packets).await?;
        self.save_if_dirty().await?;
        self.send_to_directory(DirectoryMessage::Wedding {
            to: partner,
            news: WeddingNews::Booked(reservation.clone()),
        })
        .await?;
        let text = npc_scripts::wedding_text(keys::WEDDING_DONE, &reservation, now);
        self.assistant_says(venue, text).await
    }

    /// The partner booked the couple's wedding: take the invitations. They
    /// had room a moment ago; without it now, or offline by now, the
    /// wedding stands and the partner's invitations serve both. In the
    /// Cash Shop, the player sees them on leaving.
    async fn wedding_booked(&mut self, reservation: Reservation) -> Result<(), RuntimeError> {
        let player = &self.in_game()?.player;
        if engagement(player).map(|(id, _)| id) != Some(reservation.marriage_id) {
            warn!(self.client_id, "Wedding booked for another couple");
            return Ok(());
        }
        let mut next = player.clone();
        let mut packets = Vec::new();
        if !give_invitations(&mut next, &reservation, &mut packets)? {
            warn!(self.client_id, "No room for the wedding invitations");
            packets.clear();
        } else if !self.commit_player(next, None, &mut packets).await? {
            packets.clear();
        }
        packets.extend(booked_packets(&reservation)?);
        if !self.in_cash_shop {
            self.send_all(&mut packets).await?;
        }
        self.save_if_dirty().await
    }
}
