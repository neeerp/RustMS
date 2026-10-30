//! The directory's part in engagements (report b052): a proposal waits
//! here for the partner's answer, and goes when either leaves. A wedding
//! booking (report b066) passes through here to the partner and back, and
//! an invitation to its guest.

use super::WorldDirectory;
use crate::actor::client::leave_invitation_note;
use crate::actor::wedding::{WeddingMessage, WeddingStart, WeddingTicket, WEDDING_QUEUE_CAPACITY};
use crate::db::spawn_db;
use crate::message::{DirectoryMessage, ServerMessage, WeddingNews};
use game::marriage::{MarriageRecord, Refusal, MIN_LEVEL};
use game::wedding::{GuestRefusal, Invitation, PartnerRefusal, StartRefusal, Venue};
use game_data::server_content::keys;
use net::packet::build::world::marriage::{
    build_marriage_notice, build_marriage_refusal, build_married_notice, build_wedding_notice,
    WeddingNotice,
};
use net::ClientId;
use std::time::SystemTime;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::{mpsc, oneshot};
use tracing::{info, warn};

/// A wedding under way, which guests reach through here.
pub(super) struct Wedding {
    pub(super) id: u32,
    marriage_id: i32,
    tx: mpsc::Sender<WeddingMessage>,
}

/// A proposal waiting for the partner's answer.
pub(super) struct Proposal {
    proposer: ClientId,
    item_id: i32,
}

impl WorldDirectory {
    /// `from` proposes to `partner` with `item_id`. The database's checks
    /// have passed; these need both online.
    pub(super) fn propose(&mut self, from: ClientId, partner: ClientId, item_id: i32) {
        let Some(proposer) = self.clients.get(&from) else {
            return;
        };
        let refusal = if self.proposals.values().any(|p| p.proposer == from) {
            Some(Refusal::Proposing)
        } else {
            match self.clients.get(&partner) {
                None => Some(Refusal::WrongName),
                Some(entry) if !entry.shares_map_with(proposer) => Some(Refusal::NotSameMap),
                Some(_) if self.proposals.contains_key(&partner) => Some(Refusal::BeingAsked),
                Some(entry) if entry.presence.level < MIN_LEVEL => {
                    let level = MIN_LEVEL.to_string();
                    let text = net::content::text().get(keys::MARRIAGE_LEVEL, &[("level", &level)]);
                    return self.send_packet(from, build_marriage_notice(&text));
                }
                Some(_) => None,
            }
        };
        if let Some(refusal) = refusal {
            return self.send_packet(from, build_marriage_refusal(refusal));
        }
        let name = proposer.presence.name.clone();
        self.proposals.insert(
            partner,
            Proposal {
                proposer: from,
                item_id,
            },
        );
        self.tell(partner, ServerMessage::Proposed { from, name });
    }

    /// `from` withdrew their proposal: tell the partner.
    pub(super) fn cancel_proposal(&mut self, from: ClientId) {
        let Some(partner) = self.partner_of(from) else {
            return;
        };
        self.proposals.remove(&partner);
        self.send_packet(partner, build_marriage_refusal(Refusal::Withdrew));
    }

    /// `from` answered `proposer`. A yes goes back to `from`'s actor, which
    /// records it and takes the ring while it is surely there; a no is passed
    /// on.
    pub(super) fn answer_proposal(&mut self, from: ClientId, proposer: ClientId, accept: bool) {
        if self.proposals.get(&from).map(|p| p.proposer) != Some(proposer) {
            return;
        }
        let Some(proposal) = self.proposals.remove(&from) else {
            return;
        };
        if !accept {
            return self.send_packet(proposer, build_marriage_refusal(Refusal::Declined));
        }
        // Either may have moved since the question came up.
        let together = match (self.clients.get(&from), self.clients.get(&proposer)) {
            (Some(partner), Some(proposer)) => partner.shares_map_with(proposer),
            _ => false,
        };
        if !together {
            return self.send_packet(proposer, build_marriage_refusal(Refusal::NotSameMap));
        }
        self.tell(
            from,
            ServerMessage::ProposalAccepted {
                proposer,
                item_id: proposal.item_id,
            },
        );
    }

    /// `from` can't be asked after all, or can't take the ring.
    pub(super) fn refuse_proposal(&mut self, from: ClientId, refusal: Refusal) {
        if let Some(proposal) = self.proposals.remove(&from) {
            self.send_packet(proposal.proposer, build_marriage_refusal(refusal));
        }
    }

    /// The partner recorded the engagement; the proposer trades the ring box
    /// for its empty box. A proposer gone by now keeps the box.
    pub(super) fn engaged(&self, proposer: ClientId, record: MarriageRecord) {
        self.tell(proposer, ServerMessage::Engaged(record));
    }

    /// `client_id` married: their guild online and their relatives online
    /// read so, and so do they (report b079).
    pub(super) fn tell_married(&self, client_id: ClientId) {
        let Some(presence) = self.clients.get(&client_id).map(|entry| &entry.presence) else {
            return;
        };
        if let Some(guild_id) = presence.guild_id {
            for (id, _) in self.guild_online(guild_id) {
                self.send_packet(id, build_married_notice(false, &presence.name));
            }
        }
        let relatives = &presence.family;
        if relatives.senior.is_some() || !relatives.juniors.is_empty() {
            for id in relatives.ids().chain([client_id]) {
                self.send_packet(id, build_married_notice(true, &presence.name));
            }
        }
    }

    /// A player left, logged in again or went to the Cash Shop: a proposal
    /// to them is declined, and one of theirs withdrawn.
    pub(super) fn forget_proposals(&mut self, client_id: ClientId) {
        if let Some(proposal) = self.proposals.remove(&client_id) {
            self.send_packet(proposal.proposer, build_marriage_refusal(Refusal::Declined));
        }
        self.cancel_proposal(client_id);
    }

    /// `from` books their wedding with `partner` (report b066): the partner
    /// checks their side if they are beside `from`.
    pub(super) fn reserve_wedding(
        &self,
        from: ClientId,
        partner: ClientId,
        venue: Venue,
        premium: bool,
    ) {
        let Some(booker) = self.clients.get(&from) else {
            return;
        };
        let refusal = match self.clients.get(&partner) {
            None => PartnerRefusal::Offline,
            Some(entry) if !entry.shares_map_with(booker) => PartnerRefusal::Elsewhere,
            Some(_) => {
                let news = WeddingNews::Check {
                    booker: from,
                    venue,
                    premium,
                };
                return self.tell(partner, ServerMessage::Wedding(news));
            }
        };
        let news = WeddingNews::Checked {
            venue,
            premium,
            refusal: Some(refusal),
        };
        self.tell(from, ServerMessage::Wedding(news));
    }

    /// Reserve `venue` on `from`'s channel for the wedding of `from` and
    /// `partner`, who must stand beside them.
    pub(super) fn start_wedding(
        &mut self,
        from: ClientId,
        partner: ClientId,
        venue: Venue,
        marriage_id: i32,
    ) -> Result<WeddingStart, StartRefusal> {
        let starter = self.clients.get(&from).ok_or(StartRefusal::PartnerAway)?;
        let partner = self
            .clients
            .get(&partner)
            .filter(|entry| entry.shares_map_with(starter))
            .ok_or(StartRefusal::PartnerAway)?;
        let channel_id = starter.channel_id;
        if self.weddings.contains_key(&(channel_id, venue)) {
            return Err(StartRefusal::Busy);
        }
        let couple = vec![starter.handle.clone(), partner.handle.clone()];
        // Weddings and party events share ids, so their copies don't meet.
        let id = self.next_event;
        self.next_event = self.next_event.wrapping_add(1).max(1);
        let (tx, rx) = mpsc::channel(WEDDING_QUEUE_CAPACITY);
        let wedding = Wedding {
            id,
            marriage_id,
            tx: tx.clone(),
        };
        self.weddings.insert((channel_id, venue), wedding);
        info!(?venue, id, channel_id, marriage_id, "Wedding reserved");
        Ok(WeddingStart {
            id,
            venue,
            channel_id,
            marriage_id,
            couple,
            tx,
            rx,
        })
    }

    /// Drop the bookings of weddings nobody began, in the background, and
    /// have the couples told. Only a directory that can hear back does so.
    pub(super) fn lapse_weddings(&self) {
        let Some((_, directory)) = &self.event_spawner else {
            return;
        };
        let directory = directory.clone();
        let running: Vec<i32> = self.weddings.values().map(|w| w.marriage_id).collect();
        tokio::spawn(async move {
            let now = SystemTime::now();
            match spawn_db(move || db::wedding::lapse(now, &running)).await {
                Ok(couples) if couples.is_empty() => {}
                Ok(couples) => {
                    if let Some(directory) = directory.upgrade() {
                        let _ = directory
                            .send(DirectoryMessage::WeddingsLapsed(couples))
                            .await;
                    }
                }
                Err(error) => warn!(%error, "Failed to drop the weddings nobody began"),
            }
        });
    }

    /// These couples let their wedding pass unstarted: each partner online
    /// reads that the booking is cancelled (MARRIAGE_RESULT 31), as for a
    /// wedding that failed. One begun meanwhile is left alone.
    pub(super) fn tell_lapsed(&self, couples: Vec<(i32, i32, i32)>) {
        let running: Vec<i32> = self.weddings.values().map(|w| w.marriage_id).collect();
        for (marriage_id, groom, bride) in couples {
            info!(marriage_id, "Wedding booking lapsed");
            if running.contains(&marriage_id) {
                continue;
            }
            for partner in [groom, bride] {
                if self.clients.get(&partner).is_some_and(|e| !e.in_cash_shop) {
                    self.send_packet(partner, build_wedding_notice(WeddingNotice::Cancelled));
                }
            }
        }
    }

    /// `from`, invited to the wedding of `marriage_id` at `venue`, asks to
    /// go in: the wedding answers, if it is under way on their channel.
    pub(super) fn join_wedding(
        &mut self,
        from: ClientId,
        venue: Venue,
        marriage_id: i32,
        reply: oneshot::Sender<Result<WeddingTicket, GuestRefusal>>,
    ) {
        let Some(guest) = self.clients.get(&from) else {
            return;
        };
        let wedding = self
            .weddings
            .get(&(guest.channel_id, venue))
            .filter(|wedding| wedding.marriage_id == marriage_id);
        let Some(wedding) = wedding else {
            let _ = reply.send(Err(GuestRefusal::NotStarted));
            return;
        };
        let join = WeddingMessage::Join {
            guest: guest.handle.clone(),
            reply,
        };
        // The wedding is never waited on; one that can't take the guest now
        // has no room for them.
        if let Err(error) = wedding.tx.try_send(join) {
            let refusal = match error {
                TrySendError::Full(_) => GuestRefusal::Full,
                TrySendError::Closed(_) => GuestRefusal::Late,
            };
            if let WeddingMessage::Join { reply, .. } = error.into_inner() {
                let _ = reply.send(Err(refusal));
            }
        }
    }

    /// Hand `invitation` to its guest if they are online. Otherwise leave
    /// it among their notes here, in the background, so that it gets there even if
    /// its sender leaves meanwhile.
    pub(super) fn deliver_invitation(&self, invitation: Invitation) {
        let guest = invitation.guest;
        if self.clients.contains_key(&guest) {
            self.tell(
                guest,
                ServerMessage::Wedding(WeddingNews::Invited(invitation)),
            );
            return;
        }
        let sender = self
            .clients
            .get(&invitation.sender)
            .map(|entry| entry.handle.clone());
        tokio::spawn(async move {
            if !leave_invitation_note(&invitation).await {
                if let Some(sender) = sender {
                    sender.send(ServerMessage::Wedding(WeddingNews::Returned(invitation)));
                }
            }
        });
    }

    /// Hand `invitation`, which reached nobody, back to its sender.
    pub(super) fn return_invitation(&self, invitation: Invitation) {
        self.tell(
            invitation.sender,
            ServerMessage::Wedding(WeddingNews::Returned(invitation)),
        );
    }

    fn partner_of(&self, proposer: ClientId) -> Option<ClientId> {
        self.proposals
            .iter()
            .find(|(_, p)| p.proposer == proposer)
            .map(|(&partner, _)| partner)
    }
}
