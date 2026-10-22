//! Party membership and invitations. Pure state owned by the world
//! directory, which sends the packets. Parties live in memory: they survive
//! logouts and channel changes but not a server restart.

use net::packet::build::world::party::{PartyStatus, PARTY_CAPACITY};
use net::ClientId;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// How long an unanswered invitation keeps its receiver busy.
pub const INVITE_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member {
    pub id: ClientId,
    pub name: String,
    pub job: i16,
    pub level: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Party {
    pub id: i32,
    pub leader: ClientId,
    /// In joining order; the client lists them this way.
    pub members: Vec<Member>,
}

impl Party {
    fn is_full(&self) -> bool {
        self.members.len() >= PARTY_CAPACITY
    }
}

struct Invite {
    party_id: i32,
    inviter: ClientId,
    sent: Instant,
    /// From a party search. v83 answers those itself or drops them without
    /// a word, so they never make their receiver busy.
    search: bool,
}

/// What leaving did to a party.
#[derive(Debug, PartialEq, Eq)]
pub enum Departure {
    /// The leader left, so the party is gone; these were its members.
    Disbanded(Party),
    /// A member left; the party as it is now.
    Left(Party),
}

#[derive(Default)]
pub struct Parties {
    parties: HashMap<i32, Party>,
    party_of: HashMap<ClientId, i32>,
    /// Pending invitations by receiver.
    invites: HashMap<ClientId, Invite>,
    last_id: i32,
}

impl Parties {
    pub fn of(&self, client: ClientId) -> Option<&Party> {
        self.party_of
            .get(&client)
            .and_then(|id| self.parties.get(id))
    }

    pub fn create(&mut self, leader: Member) -> Result<&Party, PartyStatus> {
        if self.party_of.contains_key(&leader.id) {
            return Err(PartyStatus::AlreadyInParty);
        }
        self.last_id += 1;
        let id = self.last_id;
        self.party_of.insert(leader.id, id);
        self.invites.remove(&leader.id);
        let party = Party {
            id,
            leader: leader.id,
            members: vec![leader],
        };
        Ok(self.parties.entry(id).or_insert(party))
    }

    /// Record that `inviter` asked `invitee` to join `inviter`'s party.
    pub fn invite(
        &mut self,
        inviter: ClientId,
        invitee: ClientId,
        now: Instant,
    ) -> Result<(), PartyStatus> {
        self.send_invite(inviter, invitee, now, false)
    }

    /// Record a party search's invitation from `leader`. Someone already
    /// asked to join any party is left alone.
    pub fn search_invite(
        &mut self,
        leader: ClientId,
        invitee: ClientId,
        now: Instant,
    ) -> Result<(), PartyStatus> {
        if self.is_asked(invitee, now) {
            return Err(PartyStatus::BusyWithInvite);
        }
        self.send_invite(leader, invitee, now, true)
    }

    /// Whether `invitee` has an invitation still waiting.
    pub fn is_asked(&self, invitee: ClientId, now: Instant) -> bool {
        self.invites
            .get(&invitee)
            .is_some_and(|invite| now < invite.sent + INVITE_TIMEOUT)
    }

    fn send_invite(
        &mut self,
        inviter: ClientId,
        invitee: ClientId,
        now: Instant,
        search: bool,
    ) -> Result<(), PartyStatus> {
        let party = self.of(inviter).ok_or(PartyStatus::NotFound)?;
        let party_id = party.id;
        if self.party_of.contains_key(&invitee) {
            return Err(PartyStatus::AlreadyInParty);
        }
        if party.is_full() {
            return Err(PartyStatus::Full);
        }
        if let Some(pending) = self.invites.get(&invitee) {
            if pending.party_id != party_id
                && !pending.search
                && now < pending.sent + INVITE_TIMEOUT
            {
                return Err(PartyStatus::BusyWithInvite);
            }
        }
        self.invites.insert(
            invitee,
            Invite {
                party_id,
                inviter,
                sent: now,
                search,
            },
        );
        Ok(())
    }

    /// Join `party_id` if invited to it recently and there is room.
    pub fn accept(&mut self, member: Member, party_id: i32, now: Instant) -> Option<&Party> {
        let invite = self.invites.remove(&member.id)?;
        if invite.party_id != party_id
            || now >= invite.sent + INVITE_TIMEOUT
            || self.party_of.contains_key(&member.id)
        {
            return None;
        }
        let party = self.parties.get_mut(&party_id)?;
        if party.is_full() {
            return None;
        }
        self.party_of.insert(member.id, party_id);
        party.members.push(member);
        Some(party)
    }

    /// Whether `inviter`'s invitation to `invitee` is waiting.
    pub fn is_invited(&self, invitee: ClientId, inviter: ClientId) -> bool {
        self.invites
            .get(&invitee)
            .is_some_and(|invite| invite.inviter == inviter)
    }

    /// Turn down `inviter`'s invitation. Returns whether there was one.
    pub fn decline(&mut self, invitee: ClientId, inviter: ClientId) -> bool {
        let pending = self.is_invited(invitee, inviter);
        if pending {
            self.invites.remove(&invitee);
        }
        pending
    }

    /// Forget invitations to a character who went offline.
    pub fn forget_invites(&mut self, invitee: ClientId) {
        self.invites.remove(&invitee);
    }

    pub fn leave(&mut self, client: ClientId) -> Option<Departure> {
        let party_id = *self.party_of.get(&client)?;
        let party = self.parties.get_mut(&party_id)?;
        if party.leader == client {
            let party = self.parties.remove(&party_id)?;
            for member in &party.members {
                self.party_of.remove(&member.id);
            }
            self.invites.retain(|_, invite| invite.party_id != party_id);
            return Some(Departure::Disbanded(party));
        }
        party.members.retain(|member| member.id != client);
        self.party_of.remove(&client);
        Some(Departure::Left(party.clone()))
    }

    /// The leader removes another member. Returns the party without them,
    /// and their name.
    pub fn expel(&mut self, leader: ClientId, target: ClientId) -> Option<(Party, String)> {
        let party_id = *self.party_of.get(&leader)?;
        let party = self.parties.get_mut(&party_id)?;
        if party.leader != leader || target == leader {
            return None;
        }
        let index = party.members.iter().position(|m| m.id == target)?;
        let expelled = party.members.remove(index);
        self.party_of.remove(&target);
        Some((party.clone(), expelled.name))
    }

    /// The leader hands the lead to another member.
    pub fn change_leader(&mut self, leader: ClientId, target: ClientId) -> Option<&Party> {
        let party_id = *self.party_of.get(&leader)?;
        let party = self.parties.get_mut(&party_id)?;
        if party.leader != leader || !party.members.iter().any(|m| m.id == target) {
            return None;
        }
        party.leader = target;
        Some(party)
    }

    /// Refresh a member's name, job, and level. Returns their party if
    /// anything changed.
    pub fn update(&mut self, member: Member) -> Option<&Party> {
        let party_id = *self.party_of.get(&member.id)?;
        let party = self.parties.get_mut(&party_id)?;
        let slot = party.members.iter_mut().find(|m| m.id == member.id)?;
        if *slot == member {
            return None;
        }
        *slot = member;
        Some(party)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(id: ClientId) -> Member {
        Member {
            id,
            name: format!("m{id}"),
            job: 0,
            level: 10,
        }
    }

    fn ids(party: &Party) -> Vec<ClientId> {
        party.members.iter().map(|m| m.id).collect()
    }

    #[test]
    fn a_search_invitation_never_makes_anyone_busy() {
        let now = Instant::now();
        let mut parties = Parties::default();
        let first = parties.create(member(1)).unwrap().id;
        parties.create(member(2)).unwrap();
        parties.search_invite(1, 3, now).unwrap();
        assert!(parties.is_asked(3, now));
        // An ordinary invitation replaces it; a search's doesn't.
        parties.invite(2, 3, now).unwrap();
        assert!(parties.is_invited(3, 2));
        assert_eq!(
            parties.search_invite(1, 3, now).unwrap_err(),
            PartyStatus::BusyWithInvite
        );
        assert!(!parties.is_asked(3, now + INVITE_TIMEOUT));
        parties.search_invite(1, 3, now + INVITE_TIMEOUT).unwrap();
        assert!(parties.accept(member(3), first, now + INVITE_TIMEOUT).is_some());
    }

    #[test]
    fn invite_accept_and_leave() {
        let now = Instant::now();
        let mut parties = Parties::default();
        let id = parties.create(member(1)).unwrap().id;
        assert_eq!(
            parties.create(member(1)).unwrap_err(),
            PartyStatus::AlreadyInParty
        );

        // Joining needs an invitation to that party.
        assert!(parties.accept(member(2), id, now).is_none());
        parties.invite(1, 2, now).unwrap();
        assert!(parties.accept(member(2), id + 1, now).is_none());
        parties.invite(1, 2, now).unwrap();
        assert_eq!(ids(parties.accept(member(2), id, now).unwrap()), [1, 2]);
        assert_eq!(
            parties.invite(1, 2, now).unwrap_err(),
            PartyStatus::AlreadyInParty
        );

        // A member leaving keeps the party; the leader leaving ends it.
        parties.invite(1, 3, now).unwrap();
        parties.accept(member(3), id, now).unwrap();
        let Some(Departure::Left(party)) = parties.leave(2) else {
            panic!("member should leave");
        };
        assert_eq!(ids(&party), [1, 3]);
        assert!(parties.of(2).is_none());
        let Some(Departure::Disbanded(party)) = parties.leave(1) else {
            panic!("leader should disband");
        };
        assert_eq!(ids(&party), [1, 3]);
        assert!(parties.of(3).is_none());
    }

    #[test]
    fn invitations_expire_and_block_other_parties_meanwhile() {
        let now = Instant::now();
        let mut parties = Parties::default();
        let first = parties.create(member(1)).unwrap().id;
        parties.create(member(2)).unwrap();
        parties.invite(1, 3, now).unwrap();
        assert_eq!(
            parties.invite(2, 3, now).unwrap_err(),
            PartyStatus::BusyWithInvite
        );
        let later = now + INVITE_TIMEOUT;
        parties.invite(2, 3, later).unwrap();
        assert!(parties.accept(member(3), first, later).is_none());

        // Declining clears the invitation and tells only the right inviter.
        parties.invite(1, 4, now).unwrap();
        assert!(!parties.decline(4, 2));
        assert!(parties.decline(4, 1));
        assert!(parties.accept(member(4), first, now).is_none());
    }

    #[test]
    fn a_party_holds_six() {
        let now = Instant::now();
        let mut parties = Parties::default();
        let id = parties.create(member(1)).unwrap().id;
        for invitee in 2..=6 {
            parties.invite(1, invitee, now).unwrap();
            parties.accept(member(invitee), id, now).unwrap();
        }
        assert_eq!(parties.invite(1, 7, now).unwrap_err(), PartyStatus::Full);
    }

    #[test]
    fn only_the_leader_expels_or_passes_the_lead() {
        let now = Instant::now();
        let mut parties = Parties::default();
        let id = parties.create(member(1)).unwrap().id;
        for invitee in [2, 3] {
            parties.invite(1, invitee, now).unwrap();
            parties.accept(member(invitee), id, now).unwrap();
        }
        assert!(parties.expel(2, 3).is_none());
        assert!(parties.expel(1, 1).is_none());
        let (party, name) = parties.expel(1, 3).unwrap();
        assert_eq!((ids(&party), name.as_str()), (vec![1, 2], "m3"));

        assert!(parties.change_leader(2, 1).is_none());
        assert!(parties.change_leader(1, 3).is_none());
        assert_eq!(parties.change_leader(1, 2).unwrap().leader, 2);
        assert!(parties.update(member(2)).is_none());
        let levelled = Member {
            level: 11,
            ..member(2)
        };
        assert_eq!(parties.update(levelled).unwrap().members[1].level, 11);
    }
}
