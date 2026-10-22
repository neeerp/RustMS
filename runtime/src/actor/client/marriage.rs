//! Engagements (report b052): proposing with a ring box, and the partner's
//! answer. The directory holds the proposal in between.

use super::ClientActor;
use crate::message::DirectoryMessage;
use crate::{db::spawn_db, error::RuntimeError};
use game::marriage::{self, MarriageRecord, Refusal, MIN_LEVEL};
use game::{InventoryType, Player};
use game_data::server_content::keys;
use net::packet::build::world::inventory;
use net::packet::build::world::marriage::{
    build_engaged, build_marriage_notice, build_marriage_refusal, build_proposal,
};
use net::{ClientId, RingRequest};
use tracing::warn;

/// Whether the player's ETC tab has no room for the token.
fn etc_full(player: &Player) -> bool {
    let inventory = &player.inventory;
    inventory.items(InventoryType::Etc).count()
        >= usize::from(inventory.slot_limit(InventoryType::Etc))
}

impl ClientActor {
    pub(super) async fn ring_request(&mut self, request: RingRequest) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        match request {
            RingRequest::Propose { name, item_id } => self.propose(name, item_id).await,
            RingRequest::Cancel => {
                self.send_to_directory(DirectoryMessage::ProposalCancel { from: cid })
                    .await
            }
            RingRequest::Answer { proposer, accept } => {
                self.send_to_directory(DirectoryMessage::ProposalAnswer {
                    from: cid,
                    proposer,
                    accept,
                })
                .await
            }
        }
    }

    /// Propose to the character named `name` with the ring box `item_id`.
    /// The directory checks the map, the partner's level, and proposals
    /// already waiting.
    async fn propose(&mut self, name: String, item_id: i32) -> Result<(), RuntimeError> {
        let player = &self.in_game()?.player;
        if !marriage::is_ring_box(item_id) || player.inventory.count(item_id) < 1 {
            warn!(self.client_id, item_id, "Proposal without a ring box");
            return Ok(());
        }
        let refusal = if let Some(record) = &player.marriage {
            Some(Refusal::taken(record, false))
        } else if etc_full(player) {
            Some(Refusal::EtcFull)
        } else if name.len() > marriage::MAX_NAME || name.eq_ignore_ascii_case(&player.stats.name) {
            Some(Refusal::WrongName)
        } else {
            None
        };
        if let Some(refusal) = refusal {
            return self.send(&mut build_marriage_refusal(refusal)?).await;
        }
        if player.stats.level < MIN_LEVEL {
            let level = MIN_LEVEL.to_string();
            let text = net::content::text().get(keys::MARRIAGE_LEVEL, &[("level", &level)]);
            return self.send(&mut build_marriage_notice(&text)?).await;
        }
        let gender = player.stats.gender;
        let partner = spawn_db(move || db::marriage::partner(&name)).await?;
        let refusal = match partner {
            None => Refusal::WrongName,
            Some(partner) => match partner.record {
                Some(record) => Refusal::taken(&record, true),
                None if partner.gender == gender => Refusal::SameGender,
                None => {
                    return self
                        .send_to_directory(DirectoryMessage::Propose {
                            from: self.client_id,
                            partner: partner.id,
                            item_id,
                        })
                        .await
                }
            },
        };
        self.send(&mut build_marriage_refusal(refusal)?).await
    }

    /// `from` (named `name`) proposes: ask the player, unless they are
    /// taken or have no room for the ring.
    pub(super) async fn proposed(
        &mut self,
        from: ClientId,
        name: String,
    ) -> Result<(), RuntimeError> {
        let player = &self.in_game()?.player;
        let refusal = match &player.marriage {
            Some(record) => Some(Refusal::taken(record, true)),
            None if etc_full(player) => Some(Refusal::PartnerEtcFull),
            None => None,
        };
        match refusal {
            Some(refusal) => {
                self.send_to_directory(DirectoryMessage::ProposalRefused {
                    from: self.client_id,
                    refusal,
                })
                .await
            }
            None => self.send(&mut build_proposal(&name, from)?).await,
        }
    }

    /// `partner` said yes: record the engagement, trade the ring box for
    /// its empty box, and send the partner the ring.
    pub(super) async fn proposal_accepted(
        &mut self,
        partner: ClientId,
        item_id: i32,
    ) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        let player = &self.in_game()?.player;
        let refusal = if player.inventory.count(item_id) < 1 {
            warn!(cid, item_id, "The ring box went before the answer");
            Some(Refusal::Withdrew)
        } else if etc_full(player) {
            Some(Refusal::EtcFull)
        } else {
            None
        };
        if let Some(refusal) = refusal {
            if refusal == Refusal::EtcFull {
                self.send(&mut build_marriage_refusal(refusal)?).await?;
            }
            return self.withdraw_from(partner).await;
        }
        let (groom, bride) = if player.stats.gender == 0 {
            (cid, partner)
        } else {
            (partner, cid)
        };
        let record = match spawn_db(move || db::marriage::engage(cid, groom, bride, item_id)).await
        {
            Ok(Ok(record)) => record,
            Ok(Err(refusal)) => {
                self.send(&mut build_marriage_refusal(refusal)?).await?;
                return self.withdraw_from(partner).await;
            }
            Err(error) => {
                warn!(cid, partner, %error, "Failed to record an engagement");
                return self.withdraw_from(partner).await;
            }
        };
        let (empty_box, _) = marriage::tokens(item_id);
        let mut next = self.in_game()?.player.clone();
        let mut packets = Vec::new();
        let mut changes = next.inventory.remove(item_id, 1).unwrap_or_default();
        match net::items::give(&mut next.inventory, empty_box, 1) {
            Ok(given) => {
                changes.extend(given);
                packets.push(inventory::build_inventory_operation(false, &changes)?);
                packets.push(inventory::build_item_gain_in_chat(empty_box, 1)?);
            }
            Err(error) => warn!(cid, empty_box, %error, "No room for the empty box"),
        }
        next.marriage = Some(record.clone());
        packets.push(build_engaged(&record)?);
        if self.commit_player(next, None, &mut packets).await? {
            self.send_all(&mut packets).await?;
        }
        self.send_to_directory(DirectoryMessage::Engaged { partner, record })
            .await
    }

    /// The proposal fell through on the proposer's side; the partner hears
    /// it was withdrawn.
    async fn withdraw_from(&mut self, partner: ClientId) -> Result<(), RuntimeError> {
        self.send_to_directory(DirectoryMessage::Deliver {
            to: vec![partner],
            packet: build_marriage_refusal(Refusal::Withdrew)?,
        })
        .await
    }

    /// The proposer recorded the engagement: take the ring.
    pub(super) async fn engaged(&mut self, record: MarriageRecord) -> Result<(), RuntimeError> {
        let (_, ring) = marriage::tokens(record.item_id);
        let mut next = self.in_game()?.player.clone();
        let mut packets = Vec::new();
        match net::items::give(&mut next.inventory, ring, 1) {
            Ok(changes) => {
                packets.push(inventory::build_inventory_operation(false, &changes)?);
                packets.push(inventory::build_item_gain_in_chat(ring, 1)?);
            }
            Err(error) => warn!(self.client_id, ring, %error, "No room for the engagement ring"),
        }
        next.marriage = Some(record.clone());
        packets.push(build_engaged(&record)?);
        if self.commit_player(next, None, &mut packets).await? {
            self.send_all(&mut packets).await?;
        }
        Ok(())
    }
}
