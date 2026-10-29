//! Guilds being founded by a party, waiting for its members to agree
//! (`GUILD_OPERATION` mode 3, v83 report b337). The world directory keeps
//! who founds which name and who has answered.

use net::ClientId;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Longer than a member's client waits before it refuses for them; a
/// founding older than this has gone without an answer.
pub const AGREEMENT_TIMEOUT: Duration = Duration::from_secs(60);

struct Founding {
    name: String,
    /// The members still to answer.
    waiting: Vec<ClientId>,
    /// The members who agreed, in the order they did.
    agreed: Vec<ClientId>,
    started: Instant,
}

/// What asking the party came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asking {
    /// There is no one to ask: the founder founds it alone.
    Alone,
    /// The members were asked.
    Asked,
    /// A member is answering another founding.
    Busy,
}

/// What a founding came to.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Everyone agreed: found `name` with them.
    Agreed {
        founder: ClientId,
        name: String,
        cofounders: Vec<ClientId>,
    },
    /// A member said no.
    Refused { founder: ClientId },
    /// A member left the game before it ended.
    Broken { founder: ClientId },
}

#[derive(Default)]
pub struct GuildFoundings {
    /// By founder.
    foundings: HashMap<ClientId, Founding>,
}

impl GuildFoundings {
    /// `founder` asks `members` to found `name` with them; any founding
    /// they started before is dropped. False if a member is still answering
    /// another, recent one.
    pub fn start(
        &mut self,
        founder: ClientId,
        name: String,
        members: Vec<ClientId>,
        now: Instant,
    ) -> bool {
        self.foundings.remove(&founder);
        let busy = self.foundings.iter().any(|(other, founding)| {
            let asked =
                |id: &ClientId| founding.waiting.contains(id) || founding.agreed.contains(id);
            now < founding.started + AGREEMENT_TIMEOUT
                && (asked(&founder)
                    || members
                        .iter()
                        .any(|member| asked(member) || member == other))
        });
        if busy {
            return false;
        }
        self.foundings.retain(|_, founding| {
            !std::iter::once(&founder)
                .chain(&members)
                .any(|member| founding.waiting.contains(member) || founding.agreed.contains(member))
        });
        self.foundings.insert(
            founder,
            Founding {
                name,
                waiting: members,
                agreed: vec![],
                started: now,
            },
        );
        true
    }

    /// `founder` founds alone after all; any founding they started goes.
    pub fn withdraw(&mut self, founder: ClientId) {
        self.foundings.remove(&founder);
    }

    /// `member` agrees or not. None while others are still to answer, or if
    /// `member` was asked nothing (or too long ago).
    pub fn answer(&mut self, member: ClientId, agree: bool, now: Instant) -> Option<Outcome> {
        let (&founder, founding) = self
            .foundings
            .iter_mut()
            .find(|(_, founding)| founding.waiting.contains(&member))?;
        if now >= founding.started + AGREEMENT_TIMEOUT {
            self.foundings.remove(&founder);
            return None;
        }
        if !agree {
            self.foundings.remove(&founder);
            return Some(Outcome::Refused { founder });
        }
        founding.waiting.retain(|id| *id != member);
        founding.agreed.push(member);
        if !founding.waiting.is_empty() {
            return None;
        }
        let founding = self.foundings.remove(&founder)?;
        Some(Outcome::Agreed {
            founder,
            name: founding.name,
            cofounders: founding.agreed,
        })
    }

    /// `client_id` left the game (or the channel). Their own founding goes
    /// quietly; one they were asked to join is broken.
    pub fn forget(&mut self, client_id: ClientId) -> Option<Outcome> {
        self.foundings.remove(&client_id);
        let founder = self.foundings.iter().find_map(|(&founder, founding)| {
            (founding.waiting.contains(&client_id) || founding.agreed.contains(&client_id))
                .then_some(founder)
        })?;
        self.foundings.remove(&founder);
        Some(Outcome::Broken { founder })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everyone_agreeing_founds_the_guild() {
        let mut foundings = GuildFoundings::default();
        let now = Instant::now();
        assert!(foundings.start(1, "Maples".into(), vec![2, 3], now));
        assert_eq!(foundings.answer(3, true, now), None);
        assert_eq!(foundings.answer(4, true, now), None, "not asked");
        assert_eq!(
            foundings.answer(2, true, now),
            Some(Outcome::Agreed {
                founder: 1,
                name: "Maples".into(),
                cofounders: vec![3, 2],
            })
        );
        assert_eq!(foundings.answer(2, true, now), None, "over");
    }

    #[test]
    fn one_refusal_ends_it() {
        let mut foundings = GuildFoundings::default();
        let now = Instant::now();
        foundings.start(1, "Maples".into(), vec![2, 3], now);
        assert_eq!(foundings.answer(2, true, now), None);
        assert_eq!(
            foundings.answer(3, false, now),
            Some(Outcome::Refused { founder: 1 })
        );
        assert_eq!(foundings.forget(2), None, "over");
    }

    #[test]
    fn a_late_answer_counts_for_nothing() {
        let mut foundings = GuildFoundings::default();
        let now = Instant::now();
        foundings.start(1, "Maples".into(), vec![2], now);
        assert_eq!(foundings.answer(2, true, now + AGREEMENT_TIMEOUT), None);
        assert!(foundings.start(5, "Leaves".into(), vec![2], now), "free");
    }

    #[test]
    fn a_member_leaving_breaks_it_and_the_founder_leaving_drops_it() {
        let mut foundings = GuildFoundings::default();
        let now = Instant::now();
        foundings.start(1, "Maples".into(), vec![2, 3], now);
        foundings.answer(2, true, now);
        assert_eq!(foundings.forget(2), Some(Outcome::Broken { founder: 1 }));
        foundings.start(1, "Maples".into(), vec![2, 3], now);
        assert_eq!(foundings.forget(1), None);
        assert_eq!(foundings.answer(2, true, now), None, "gone");
    }

    #[test]
    fn a_member_answering_another_founding_is_busy() {
        let mut foundings = GuildFoundings::default();
        let now = Instant::now();
        foundings.start(1, "Maples".into(), vec![2, 3], now);
        assert!(!foundings.start(5, "Leaves".into(), vec![2], now));
        assert!(!foundings.start(5, "Leaves".into(), vec![1], now));
        assert!(!foundings.start(2, "Leaves".into(), vec![6], now), "asked");
        assert_eq!(foundings.answer(2, true, now), None);
        assert!(!foundings.start(5, "Leaves".into(), vec![2], now), "agreed");
        assert!(foundings.start(1, "Oaks".into(), vec![2], now), "again");
        assert_eq!(
            foundings.answer(2, true, now),
            Some(Outcome::Agreed {
                founder: 1,
                name: "Oaks".into(),
                cofounders: vec![2],
            })
        );
        foundings.start(1, "Elms".into(), vec![2], now);
        foundings.withdraw(1);
        assert_eq!(foundings.answer(2, true, now), None, "withdrawn");
    }
}
