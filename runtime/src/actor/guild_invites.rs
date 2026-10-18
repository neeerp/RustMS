//! Guild invitations waiting for an answer. The guilds themselves live in
//! the database; the world directory keeps only who was asked, by whom, and
//! when.

use net::ClientId;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// As long as the client shows the invitation.
pub const INVITE_TIMEOUT: Duration = Duration::from_secs(180);

struct Invite {
    guild_id: i32,
    inviter: ClientId,
    sent: Instant,
}

#[derive(Default)]
pub struct GuildInvites {
    /// By receiver.
    invites: HashMap<ClientId, Invite>,
}

impl GuildInvites {
    /// Record that `inviter` asked `invitee` to join `guild_id`. False if
    /// `invitee` is still considering another guild's invitation.
    pub fn invite(
        &mut self,
        guild_id: i32,
        inviter: ClientId,
        invitee: ClientId,
        now: Instant,
    ) -> bool {
        if let Some(pending) = self.invites.get(&invitee) {
            if pending.guild_id != guild_id && now < pending.sent + INVITE_TIMEOUT {
                return false;
            }
        }
        self.invites.insert(
            invitee,
            Invite {
                guild_id,
                inviter,
                sent: now,
            },
        );
        true
    }

    /// Take up `invitee`'s invitation to `guild_id`. Returns whether there
    /// was one, recent enough.
    pub fn accept(&mut self, invitee: ClientId, guild_id: i32, now: Instant) -> bool {
        match self.invites.remove(&invitee) {
            Some(invite) if invite.guild_id == guild_id && now < invite.sent + INVITE_TIMEOUT => {
                true
            }
            Some(invite) => {
                // Another guild's invitation still stands.
                self.invites.insert(invitee, invite);
                false
            }
            None => false,
        }
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
    pub fn forget(&mut self, invitee: ClientId) {
        self.invites.remove(&invitee);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_guild_at_a_time_until_it_expires() {
        let mut invites = GuildInvites::default();
        let now = Instant::now();
        assert!(invites.invite(4, 1, 2, now));
        // The same guild may ask again; another must wait.
        assert!(invites.invite(4, 3, 2, now));
        assert!(!invites.invite(5, 6, 2, now));
        assert!(!invites.accept(2, 5, now));
        assert!(invites.accept(2, 4, now));
        assert!(!invites.accept(2, 4, now));

        assert!(invites.invite(4, 1, 2, now));
        let later = now + INVITE_TIMEOUT;
        assert!(invites.invite(5, 6, 2, later));
        assert!(!invites.accept(2, 4, later));
        assert!(!invites.accept(2, 5, later + INVITE_TIMEOUT));
    }

    #[test]
    fn declines_reach_the_inviter_once() {
        let mut invites = GuildInvites::default();
        let now = Instant::now();
        invites.invite(4, 1, 2, now);
        assert!(!invites.decline(2, 3));
        assert!(invites.is_invited(2, 1));
        assert!(invites.decline(2, 1));
        assert!(!invites.decline(2, 1));

        invites.invite(4, 1, 2, now);
        invites.forget(2);
        assert!(!invites.accept(2, 4, now));
    }
}
