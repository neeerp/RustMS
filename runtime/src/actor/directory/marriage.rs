//! The directory's part in engagements (report b052): a proposal waits
//! here for the partner's answer, and goes when either leaves.

use super::WorldDirectory;
use crate::message::ServerMessage;
use game::marriage::{MarriageRecord, Refusal, MIN_LEVEL};
use game_data::server_content::keys;
use net::packet::build::world::marriage::{build_marriage_notice, build_marriage_refusal};
use net::ClientId;

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

    /// `from` answered `proposer`. A yes goes to the proposer's actor, which
    /// records it; a no is passed on.
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
            proposer,
            ServerMessage::ProposalAccepted {
                partner: from,
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

    /// `from` recorded the engagement; the partner takes the ring.
    pub(super) fn engaged(&self, partner: ClientId, record: MarriageRecord) {
        self.tell(partner, ServerMessage::Engaged(record));
    }

    /// A player left: a proposal to them is declined, and one of theirs
    /// withdrawn.
    pub(super) fn forget_proposals(&mut self, client_id: ClientId) {
        if let Some(proposal) = self.proposals.remove(&client_id) {
            self.send_packet(proposal.proposer, build_marriage_refusal(Refusal::Declined));
        }
        self.cancel_proposal(client_id);
    }

    fn partner_of(&self, proposer: ClientId) -> Option<ClientId> {
        self.proposals
            .iter()
            .find(|(_, p)| p.proposer == proposer)
            .map(|(&partner, _)| partner)
    }
}
