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
    build_couple_ended, build_engaged, build_marriage_notice, build_marriage_refusal,
    build_proposal,
};
use net::{ClientId, RingRequest};
use packet::Packet;
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
            RingRequest::Break { item_id } => self.break_off(item_id).await,
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
            // The shop can't answer.
            _ if self.in_cash_shop => Some(Refusal::NotSameMap),
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

    /// The player said yes to `proposer`: record the engagement and take
    /// the ring, then let the proposer trade their ring box. Every failure
    /// gives the proposer a result, which ends their wait.
    pub(super) async fn proposal_accepted(
        &mut self,
        proposer: ClientId,
        item_id: i32,
    ) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        let player = &self.in_game()?.player;
        if etc_full(player) {
            self.send(&mut build_marriage_refusal(Refusal::EtcFull)?)
                .await?;
            return self
                .refuse_proposer(proposer, Refusal::PartnerEtcFull)
                .await;
        }
        let (groom, bride) = if player.stats.gender == 0 {
            (cid, proposer)
        } else {
            (proposer, cid)
        };
        let record =
            match spawn_db(move || db::marriage::engage(proposer, groom, bride, item_id)).await {
                Ok(Ok(record)) => record,
                Ok(Err(refusal)) => return self.refuse_proposer(proposer, refusal).await,
                Err(error) => {
                    // Two who proposed to each other and both said yes: one
                    // of the two engagements fails on the unique index.
                    warn!(cid, proposer, %error, "Failed to record an engagement");
                    return self.refuse_proposer(proposer, Refusal::Declined).await;
                }
            };
        let (_, ring) = marriage::tokens(item_id);
        let mut next = self.in_game()?.player.clone();
        let mut packets = Vec::new();
        match net::items::give(&mut next.inventory, ring, 1) {
            Ok(changes) => {
                packets.push(inventory::build_inventory_operation(false, &changes)?);
                packets.push(inventory::build_item_gain_in_chat(ring, 1)?);
            }
            Err(error) => warn!(cid, ring, %error, "No room for the engagement ring"),
        }
        next.marriage = Some(record.clone());
        packets.push(build_engaged(&record)?);
        self.send_to_directory(DirectoryMessage::Engaged { proposer, record })
            .await?;
        self.commit_engagement(next, packets).await
    }

    /// The proposal fell through on the partner's side; tell the proposer.
    async fn refuse_proposer(
        &mut self,
        proposer: ClientId,
        refusal: Refusal,
    ) -> Result<(), RuntimeError> {
        self.send_to_directory(DirectoryMessage::Deliver {
            to: vec![proposer],
            packet: build_marriage_refusal(refusal)?,
        })
        .await
    }

    /// The partner recorded the engagement: trade the ring box for its empty
    /// box. A box gone meanwhile leaves the player without the empty one.
    pub(super) async fn engaged(&mut self, record: MarriageRecord) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        let (empty_box, _) = marriage::tokens(record.item_id);
        let mut next = self.in_game()?.player.clone();
        let mut packets = Vec::new();
        match next.inventory.remove(record.item_id, 1) {
            Ok(mut changes) => match net::items::give(&mut next.inventory, empty_box, 1) {
                Ok(given) => {
                    changes.extend(given);
                    packets.push(inventory::build_inventory_operation(false, &changes)?);
                    packets.push(inventory::build_item_gain_in_chat(empty_box, 1)?);
                }
                Err(error) => {
                    warn!(cid, empty_box, %error, "No room for the empty box");
                    packets.push(inventory::build_inventory_operation(false, &changes)?);
                }
            },
            Err(error) => {
                warn!(cid, item_id = record.item_id, %error, "The ring box went before the answer")
            }
        }
        next.marriage = Some(record.clone());
        packets.push(build_engaged(&record)?);
        self.commit_engagement(next, packets).await
    }

    /// Keep `next`, and show it unless the player is in the Cash Shop, which
    /// shows the whole inventory again on the way out. The couple's row is
    /// already written, so the tokens are saved now too.
    async fn commit_engagement(
        &mut self,
        next: Player,
        mut packets: Vec<Packet>,
    ) -> Result<(), RuntimeError> {
        if self.commit_player(next, None, &mut packets).await? && !self.in_cash_shop {
            self.send_all(&mut packets).await?;
        }
        self.save_if_dirty().await
    }

    /// The player dropped the token `item_id` and said yes to breaking off
    /// (report b054). The client keeps the token until told otherwise. A
    /// token left from a couple already over just goes.
    async fn break_off(&mut self, item_id: i32) -> Result<(), RuntimeError> {
        let cid = self.client_id;
        let player = &self.in_game()?.player;
        if !marriage::is_token(item_id) || player.inventory.count(item_id) < 1 {
            warn!(cid, item_id, "Breaking off without a token");
            return Ok(());
        }
        let mut next = player.clone();
        let mut packets = Vec::new();
        if let Some(record) = player.marriage.clone() {
            let id = record.id;
            if !spawn_db(move || db::marriage::end(id)).await? {
                warn!(cid, id, "The couple was over already");
            }
            next.marriage = None;
            packets.push(build_couple_ended(&record)?);
            let partner = if record.groom_id == cid {
                record.bride_id
            } else {
                record.groom_id
            };
            self.send_to_directory(DirectoryMessage::CoupleEnded { partner, record })
                .await?;
        }
        take_tokens(&mut next, &mut packets)?;
        self.commit_engagement(next, packets).await
    }

    /// The partner broke off the couple `record`. A partner offline then
    /// finds no record on their next login, and the token stays until they
    /// drop it.
    pub(super) async fn couple_ended(
        &mut self,
        record: MarriageRecord,
    ) -> Result<(), RuntimeError> {
        let player = &self.in_game()?.player;
        if player.marriage.as_ref().map(|own| own.id) != Some(record.id) {
            return Ok(());
        }
        let mut next = player.clone();
        next.marriage = None;
        let mut packets = vec![build_couple_ended(&record)?];
        take_tokens(&mut next, &mut packets)?;
        self.commit_engagement(next, packets).await
    }
}

/// Take every token the player holds, which mean nothing without a couple.
fn take_tokens(player: &mut Player, packets: &mut Vec<Packet>) -> Result<(), RuntimeError> {
    let mut changes = Vec::new();
    for token in 4031357..=4031364 {
        let count = player.inventory.count(token);
        if count > 0 {
            match player
                .inventory
                .remove(token, i16::try_from(count).unwrap_or(i16::MAX))
            {
                Ok(removed) => changes.extend(removed),
                Err(error) => warn!(token, %error, "Failed to take a token"),
            }
        }
    }
    if !changes.is_empty() {
        packets.push(inventory::build_inventory_operation(false, &changes)?);
    }
    Ok(())
}
