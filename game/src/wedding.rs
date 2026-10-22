//! Wedding reservations (report b066): an engaged couple books the next
//! free hour of a venue on their channel, with a ticket from the Cash Shop,
//! and each partner gets invitations for their guests.

use crate::marriage::is_wedding_ring;
use crate::{InventoryType, Player};
use std::time::{Duration, SystemTime};

/// Where the wedding is held.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Venue {
    /// The Chapel, booked with Bonnie.
    Chapel = 0,
    /// The Cathedral, booked with Nicole.
    Cathedral = 1,
}

impl Venue {
    pub fn from_i16(value: i16) -> Option<Self> {
        match value {
            0 => Some(Self::Chapel),
            1 => Some(Self::Cathedral),
            _ => None,
        }
    }

    /// The Cash Shop ticket that books it, premium or not.
    pub fn ticket(self, premium: bool) -> i32 {
        match (self, premium) {
            (Self::Cathedral, false) => 5251000,
            (Self::Chapel, false) => 5251001,
            (Self::Chapel, true) => 5251002,
            (Self::Cathedral, true) => 5251003,
        }
    }

    /// The invitations the couple sends to guests.
    pub fn invitation(self) -> i32 {
        match self {
            Self::Chapel => 4031377,
            Self::Cathedral => 4031395,
        }
    }

    /// The invitation a guest receives.
    pub fn received_invitation(self) -> i32 {
        match self {
            Self::Chapel => 4031406,
            Self::Cathedral => 4031407,
        }
    }

    /// The venue of the couple's invitation `item_id`.
    pub fn of_invitation(item_id: i32) -> Option<Self> {
        [Self::Chapel, Self::Cathedral]
            .into_iter()
            .find(|venue| venue.invitation() == item_id)
    }

    /// The venue of the guest's invitation `item_id`.
    pub fn of_received_invitation(item_id: i32) -> Option<Self> {
        [Self::Chapel, Self::Cathedral]
            .into_iter()
            .find(|venue| venue.received_invitation() == item_id)
    }

    /// What the client's invitation window draws for it: 2 the
    /// Cathedral's, anything else the Chapel's.
    pub fn window(self) -> u16 {
        match self {
            Self::Chapel => 1,
            Self::Cathedral => 2,
        }
    }
}

/// Invitations each partner gets with a booking.
pub const INVITATIONS: i16 = 15;
/// Invitations a Wedding Invitation Ticket makes.
pub const EXTRA_INVITATIONS: i16 = 3;
/// The Cash Shop's Wedding Invitation Ticket.
pub const INVITATION_TICKET: i32 = 5251100;
/// The most guests a wedding may have.
pub const MAX_GUESTS: i64 = 60;
/// The length of a wedding's slot; weddings start on the hour.
pub const SLOT: Duration = Duration::from_secs(60 * 60);
/// The soonest a booking may start, unless configured otherwise.
pub const LEAD: Duration = Duration::from_secs(120 * 60);
/// How long after its start a wedding the couple never began is kept.
pub const NO_SHOW: Duration = Duration::from_secs(10 * 60);

/// A couple's booked wedding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reservation {
    /// The couple's marriage record.
    pub marriage_id: i32,
    pub venue: Venue,
    pub premium: bool,
    /// The channel it was booked on, where it takes place.
    pub channel: u8,
    pub starts_at: SystemTime,
}

impl Reservation {
    /// The ticket that booked it.
    pub fn ticket(&self) -> i32 {
        self.venue.ticket(self.premium)
    }

    /// Whether the couple let it pass without beginning it.
    pub fn over(&self, now: SystemTime) -> bool {
        now >= self.starts_at + NO_SHOW
    }

    /// Whether guests may still be invited at `now`.
    pub fn inviting(&self, now: SystemTime) -> bool {
        now < self.starts_at
    }

    /// When a guest's invitation to it expires: a slot after the start.
    pub fn invitation_expiry(&self) -> SystemTime {
        self.starts_at + SLOT
    }

    /// Whole minutes until it starts, rounded up; 0 once it has.
    pub fn minutes_left(&self, now: SystemTime) -> u64 {
        let left = self.starts_at.duration_since(now).unwrap_or_default();
        left.as_secs().div_ceil(60)
    }
}

/// A guest's invitation to a couple's wedding, on its way to them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invitation {
    pub guest: i32,
    pub guest_name: String,
    pub reservation: Reservation,
    pub groom: String,
    pub bride: String,
    /// The partner who sent it.
    pub sender: i32,
    pub from: String,
}

/// Why the partner's side can't take part in a booking.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartnerRefusal {
    Offline,
    /// Not on the booker's map and channel.
    Elsewhere,
    HasRing,
    /// No room in the Etc tab for the invitations.
    NoRoom,
}

/// Whether the player holds or wears a wedding ring, and so is married
/// already as far as a booking goes.
pub fn has_wedding_ring(player: &Player) -> bool {
    [InventoryType::Equip, InventoryType::Equipped]
        .into_iter()
        .flat_map(|kind| player.inventory.items(kind))
        .any(|(_, item)| is_wedding_ring(item.item_id))
}

/// The start of the slot a booking made at `now` gets on a venue whose
/// latest booking starts at `last`: the first top of a `slot` at least
/// `lead` away, and after the latest booking.
pub fn next_slot(
    now: SystemTime,
    lead: Duration,
    slot: Duration,
    last: Option<SystemTime>,
) -> SystemTime {
    let earliest = match last {
        Some(last) => (now + lead).max(last + slot),
        None => now + lead,
    };
    let since = earliest
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let slot = slot.as_secs().max(1);
    SystemTime::UNIX_EPOCH + Duration::from_secs(since.div_ceil(slot) * slot)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(minutes: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(minutes * 60)
    }

    #[test]
    fn a_free_venue_books_the_first_hour_past_the_lead() {
        assert_eq!(next_slot(at(600), LEAD, SLOT, None), at(720));
        assert_eq!(next_slot(at(601), LEAD, SLOT, None), at(780));
        assert_eq!(next_slot(at(659), LEAD, SLOT, None), at(780));
    }

    #[test]
    fn a_booked_venue_books_the_hour_after_its_latest() {
        assert_eq!(next_slot(at(600), LEAD, SLOT, Some(at(720))), at(780));
        assert_eq!(next_slot(at(600), LEAD, SLOT, Some(at(900))), at(960));
        // One long past changes nothing.
        assert_eq!(next_slot(at(600), LEAD, SLOT, Some(at(60))), at(720));
    }

    #[test]
    fn a_short_lead_books_within_the_hour() {
        let lead = Duration::from_secs(60);
        assert_eq!(
            next_slot(at(600), lead, Duration::from_secs(120), None),
            at(602)
        );
    }

    #[test]
    fn each_venue_has_its_tickets_and_invitations() {
        assert_eq!(Venue::Cathedral.ticket(false), 5251000);
        assert_eq!(Venue::Cathedral.ticket(true), 5251003);
        assert_eq!(Venue::Chapel.ticket(false), 5251001);
        assert_eq!(Venue::Chapel.ticket(true), 5251002);
        assert_eq!(Venue::Chapel.invitation(), 4031377);
        assert_eq!(Venue::Cathedral.invitation(), 4031395);
        assert_eq!(Venue::Chapel.received_invitation(), 4031406);
        assert_eq!(Venue::Cathedral.received_invitation(), 4031407);
        for venue in [Venue::Chapel, Venue::Cathedral] {
            assert_eq!(Venue::from_i16(venue as i16), Some(venue));
            assert_eq!(Venue::of_invitation(venue.invitation()), Some(venue));
            let received = venue.received_invitation();
            assert_eq!(Venue::of_received_invitation(received), Some(venue));
        }
        assert_eq!(Venue::of_invitation(4031406), None);
        assert_eq!(Venue::of_received_invitation(4031377), None);
        assert_eq!(Venue::Cathedral.window(), 2);
    }

    #[test]
    fn a_reservation_is_over_ten_minutes_after_its_start() {
        let reservation = Reservation {
            marriage_id: 1,
            venue: Venue::Chapel,
            premium: false,
            channel: 0,
            starts_at: at(720),
        };
        assert_eq!(reservation.minutes_left(at(600)), 120);
        assert_eq!(
            reservation.minutes_left(SystemTime::UNIX_EPOCH + Duration::from_secs(600 * 60 + 1)),
            120
        );
        assert_eq!(reservation.minutes_left(at(725)), 0);
        assert!(reservation.inviting(at(719)));
        assert!(!reservation.inviting(at(720)));
        assert_eq!(reservation.invitation_expiry(), at(780));
        assert!(!reservation.over(at(729)));
        assert!(reservation.over(at(730)));
    }
}
