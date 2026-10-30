//! Wedding reservations (report b066): Nicole and Bonnie book an engaged
//! couple's wedding for a ticket while both stand beside them. The partner
//! checks their own side through the directory; the booker then takes the
//! ticket and books, and each partner takes their own invitations. A
//! partner then sends an invitation to a guest by name: the guest takes it
//! if online with room, or finds it at Duey with a note.

use super::ClientActor;
use crate::message::{DirectoryMessage, WeddingNews};
use crate::{db::spawn_db, error::RuntimeError};
use db::wedding::{Booking, Invite};
use game::marriage::ENGAGED;
use game::wedding::{
    self, Invitation, PartnerRefusal, Reservation, Venue, EXTRA_INVITATIONS, INVITATIONS,
    INVITATION_TICKET,
};
use game::{InventoryType, Player};
use game_data::server_content::keys;
use net::npc_scripts::{self, NpcScript, WeddingView};
use net::packet::build::world::inventory;
use net::packet::build::world::marriage::{build_invitation, build_wedding_notice, WeddingNotice};
use net::packet::build::world::messaging::{build_broadcast, Broadcast};
use net::packet::build::world::note::build_note_arrived;
use net::packet::build::world::stat::build_stat_update;
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

/// Put `quantity` of the couple's invitations for `reservation` in
/// `next`, expiring as the wedding starts, with the packets that show
/// them.
fn give_invitations(
    next: &mut Player,
    reservation: &Reservation,
    quantity: i16,
    packets: &mut Vec<Packet>,
) -> Result<bool, RuntimeError> {
    let item_id = reservation.venue.invitation();
    let until = Some(reservation.starts_at);
    let Ok(changes) = net::items::give_until(&mut next.inventory, item_id, quantity, until) else {
        return Ok(false);
    };
    packets.push(inventory::build_inventory_operation(false, &changes)?);
    packets.push(inventory::build_item_gain_in_chat(
        item_id,
        i32::from(quantity),
    )?);
    Ok(true)
}

/// `key`'s text for the guest of `invitation`: the couple's names, with
/// the wedding's minutes left and channel.
fn invitation_text(key: &str, invitation: &Invitation) -> String {
    let reservation = &invitation.reservation;
    let minutes = reservation.minutes_left(SystemTime::now()).to_string();
    let channel = (u16::from(reservation.channel) + 1).to_string();
    net::content::text().get(
        key,
        &[
            ("groom", invitation.groom.as_str()),
            ("bride", invitation.bride.as_str()),
            ("minutes", minutes.as_str()),
            ("channel", channel.as_str()),
        ],
    )
}

/// The packets both partners get for a booking: the client's notice and a
/// line saying when.
fn booked_packets(reservation: &Reservation) -> Result<Vec<Packet>, RuntimeError> {
    let line = npc_scripts::wedding_text(keys::WEDDING_DONE_CHAT, reservation, SystemTime::now());
    // The priest asks for wedding clothes (`game::wedding::WEDDING_CLOTHES`).
    let dress = net::content::text().get(keys::WEDDING_DRESS_CHAT, &[]);
    Ok(vec![
        build_wedding_notice(WeddingNotice::Reserved)?,
        build_broadcast(Broadcast::Pink, &line)?,
        build_broadcast(Broadcast::Pink, &dress)?,
    ])
}

impl ClientActor {
    /// Start a wedding assistant's script, which needs the couple's
    /// booking. Whether `npc_id` was one, here.
    pub(super) async fn talk_to_wedding_assistant(
        &mut self,
        npc_id: i32,
    ) -> Result<bool, RuntimeError> {
        let Some(venue) = npc_scripts::wedding_venue(npc_id) else {
            return Ok(false);
        };
        let now = SystemTime::now();
        let player = &self.in_game()?.player;
        let couple = engagement(player).map(|(marriage_id, _)| marriage_id);
        let cid = player.stats.id;
        let (reservation, guest_of) = spawn_db(move || {
            let reservation = match couple {
                Some(marriage_id) => db::wedding::current(marriage_id, now)?,
                None => None,
            };
            let guest_of = db::wedding::guest_of(cid, venue, now)?;
            Ok::<_, db::Error>((reservation, guest_of.map(|(wedding, _)| wedding)))
        })
        .await?;
        let view = WeddingView {
            reservation: reservation.as_ref(),
            guest_of: guest_of.as_ref(),
            channel: self.channel_id,
            now,
        };
        let player = &self.in_game()?.player;
        match NpcScript::wedding_assistant(npc_id, player, view) {
            Some(script) => self.start_script(script).await.map(|()| true),
            None => Ok(false),
        }
    }

    pub(super) async fn assistant_says(
        &mut self,
        venue: Venue,
        text: String,
    ) -> Result<(), RuntimeError> {
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
            WeddingNews::Invited(invitation) => self.invited(invitation).await,
            WeddingNews::Returned(invitation) => self.invitation_returned(invitation).await,
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
                give_invitations(&mut next, &reservation, INVITATIONS, &mut packets)?
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
        if !give_invitations(&mut next, &reservation, INVITATIONS, &mut packets)? {
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

    /// The player sends the invitation in Etc `slot` to the character
    /// named `guest`, for the wedding of their couple `marriage_id`. The
    /// invitation goes only once the guest is on the list.
    pub(super) async fn invite(
        &mut self,
        guest: String,
        marriage_id: i32,
        slot: i16,
    ) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        if self.in_cash_shop {
            return Ok(());
        }
        let player = &self.in_game()?.player;
        let record = player
            .marriage
            .clone()
            .filter(|record| record.id == marriage_id && record.status == ENGAGED);
        let venue = player
            .inventory
            .get(InventoryType::Etc, slot)
            .and_then(|item| Venue::of_invitation(item.item_id));
        let (Some(record), Some(venue)) = (record, venue) else {
            warn!(
                cid,
                marriage_id, slot, "Wedding invitation without its couple or item"
            );
            return Ok(());
        };
        let name = guest.clone();
        let Some((guest_id, guest)) = spawn_db(move || db::note::character(&name)).await? else {
            return self
                .pink(keys::WEDDING_INVITE_NOT_FOUND, &[("name", &guest)])
                .await;
        };
        if guest_id == record.groom_id || guest_id == record.bride_id {
            return self.pink(keys::WEDDING_INVITE_COUPLE, &[]).await;
        }
        let now = SystemTime::now();
        let invited =
            spawn_db(move || db::wedding::invite(marriage_id, venue, guest_id, now)).await;
        let reservation = match invited {
            Ok(Invite::Added(reservation)) => reservation,
            Ok(Invite::AlreadyInvited) => {
                return self
                    .pink(keys::WEDDING_INVITE_ALREADY, &[("name", &guest)])
                    .await;
            }
            Ok(Invite::Full) => return self.pink(keys::WEDDING_INVITE_FULL, &[]).await,
            Ok(Invite::NotBooked) => return self.pink(keys::WEDDING_INVITE_NOT_BOOKED, &[]).await,
            Ok(Invite::Started) => return self.pink(keys::WEDDING_INVITE_STARTED, &[]).await,
            Err(error) => {
                warn!(cid, %error, "Wedding guest not added");
                return self.pink(keys::WEDDING_INVITE_ERROR, &[]).await;
            }
        };
        let mut next = self.in_game()?.player.clone();
        let mut packets = Vec::new();
        match next.inventory.take(InventoryType::Etc, slot, 1) {
            Ok(changes) => {
                packets.push(inventory::build_inventory_operation(true, &changes)?);
                if !self.commit_player(next, None, &mut packets).await? {
                    packets.clear();
                }
            }
            Err(error) => warn!(cid, slot, %error, "The invitation sent went meanwhile"),
        }
        let text = net::content::text().get(keys::WEDDING_INVITE_SENT, &[("name", &guest)]);
        packets.push(build_broadcast(Broadcast::Pink, &text)?);
        self.send_all(&mut packets).await?;
        let from = self.in_game()?.player.stats.name.clone();
        self.send_to_directory(DirectoryMessage::DeliverInvitation {
            invitation: Invitation {
                guest: guest_id,
                guest_name: guest,
                reservation,
                groom: record.groom_name,
                bride: record.bride_name,
                sender: cid,
                from,
            },
        })
        .await?;
        self.save_if_dirty().await
    }

    /// The player is invited to a wedding: take the invitation, which
    /// expires a slot after the wedding starts, or find it among their notes
    /// without room for it. In the Cash Shop, the player sees it on leaving.
    async fn invited(&mut self, invitation: Invitation) -> Result<(), RuntimeError> {
        let reservation = &invitation.reservation;
        let item_id = reservation.venue.received_invitation();
        let until = Some(reservation.invitation_expiry());
        let mut next = self.in_game()?.player.clone();
        let mut packets = Vec::new();
        let taken = match net::items::give_until(&mut next.inventory, item_id, 1, until) {
            Ok(changes) => {
                packets.push(inventory::build_inventory_operation(false, &changes)?);
                packets.push(inventory::build_item_gain_in_chat(item_id, 1)?);
                self.commit_player(next, None, &mut packets).await?
            }
            Err(_) => false,
        };
        let key = if taken {
            keys::WEDDING_INVITED
        } else if leave_invitation_note(&invitation).await {
            packets.clear();
            packets.push(build_note_arrived()?);
            keys::WEDDING_INVITED_NOTE
        } else {
            return self
                .send_to_directory(DirectoryMessage::ReturnInvitation { invitation })
                .await;
        };
        let text = invitation_text(key, &invitation);
        packets.push(build_broadcast(Broadcast::Pink, &text)?);
        if !self.in_cash_shop {
            self.send_all(&mut packets).await?;
        }
        self.save_if_dirty().await
    }

    /// The invitation the player sent reached nobody: take it back while
    /// the wedding has not started, and say so.
    async fn invitation_returned(&mut self, invitation: Invitation) -> Result<(), RuntimeError> {
        let reservation = &invitation.reservation;
        let mut next = self.in_game()?.player.clone();
        let mut packets = Vec::new();
        if reservation.inviting(SystemTime::now())
            && give_invitations(&mut next, reservation, 1, &mut packets)?
            && !self.commit_player(next, None, &mut packets).await?
        {
            packets.clear();
        }
        let text = net::content::text().get(
            keys::WEDDING_INVITE_RETURNED,
            &[("name", &invitation.guest_name)],
        );
        packets.push(build_broadcast(Broadcast::Pink, &text)?);
        if !self.in_cash_shop {
            self.send_all(&mut packets).await?;
        }
        self.save_if_dirty().await
    }

    /// The player opens the guest's invitation `item_id` in Etc `slot`: the
    /// invitation window for the soonest wedding at its venue they are
    /// invited to, or the client's "not valid" without one.
    pub(super) async fn open_invitation(
        &mut self,
        slot: i16,
        item_id: i32,
    ) -> Result<(), RuntimeError> {
        if self.in_cash_shop {
            return Ok(());
        }
        let player = &self.in_game()?.player;
        let held = player
            .inventory
            .get(InventoryType::Etc, slot)
            .is_some_and(|item| item.item_id == item_id);
        let venue = Venue::of_received_invitation(item_id).filter(|_| held);
        let Some(venue) = venue else {
            return self.send(&mut build_stat_update(true, &[])?).await;
        };
        let (cid, now) = (player.stats.id, SystemTime::now());
        let mut packet = match spawn_db(move || db::wedding::guest_of(cid, venue, now)).await? {
            Some((_, record)) => build_invitation(&record.groom_name, &record.bride_name, venue)?,
            None => build_wedding_notice(WeddingNotice::InvalidInvitation)?,
        };
        self.send(&mut packet).await
    }

    /// The player trades a Wedding Invitation Ticket at the assistant of
    /// `venue` for more invitations to their wedding there.
    pub(super) async fn extra_invitations(&mut self, venue: Venue) -> Result<(), RuntimeError> {
        let player = &self.in_game()?.player;
        let error = || net::content::text().get(keys::WEDDING_ERROR, &[]);
        let Some((marriage_id, _)) = engagement(player) else {
            return self.assistant_says(venue, error()).await;
        };
        let now = SystemTime::now();
        let reservation = spawn_db(move || db::wedding::current(marriage_id, now))
            .await?
            .filter(|reservation| reservation.venue == venue && reservation.inviting(now));
        let Some(reservation) = reservation else {
            let text = net::content::text().get(keys::WEDDING_EXTRA_NOT_BOOKED, &[]);
            return self.assistant_says(venue, text).await;
        };
        let mut next = self.in_game()?.player.clone();
        let mut packets = Vec::new();
        let given = match next.inventory.remove(INVITATION_TICKET, 1) {
            Ok(changes) => {
                packets.push(inventory::build_inventory_operation(false, &changes)?);
                packets.push(inventory::build_item_gain_in_chat(INVITATION_TICKET, -1)?);
                give_invitations(&mut next, &reservation, EXTRA_INVITATIONS, &mut packets)?
            }
            Err(_) => false,
        };
        if !given || !self.commit_player(next, None, &mut packets).await? {
            return self.assistant_says(venue, error()).await;
        }
        self.send_all(&mut packets).await?;
        self.save_if_dirty().await?;
        let quantity = EXTRA_INVITATIONS.to_string();
        let text = net::content::text().get(keys::WEDDING_EXTRA_DONE, &[("quantity", &quantity)]);
        self.assistant_says(venue, text).await
    }
}

/// Leave `invitation` among its guest's notes, as v83's invitation note
/// (flag 3), which brings the invitation when read. Failing that (their
/// inbox full), the guest goes off the list again, so that the couple may
/// send another; false then.
pub(crate) async fn leave_invitation_note(invitation: &Invitation) -> bool {
    let (guest, marriage_id) = (invitation.guest, invitation.reservation.marriage_id);
    let text = invitation_text(keys::WEDDING_INVITED_NOTE, invitation);
    let from = invitation.from.clone();
    let left = spawn_db(move || {
        db::note::invite(guest, &from, &text, marriage_id, super::notes::INBOX_LIMIT)
    })
    .await;
    match left {
        Ok(true) => return true,
        Ok(false) => warn!(guest, "No room among the notes for an invitation"),
        Err(error) => warn!(guest, %error, "Invitation note not left"),
    }
    if let Err(error) = spawn_db(move || db::wedding::uninvite(marriage_id, guest)).await {
        warn!(guest, marriage_id, %error, "A guest without an invitation stays invited");
    }
    false
}
