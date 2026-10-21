//! Rides with no timetable: the Ereve and Rien ferries and the crane
//! between Orbis and Mu Lung. A player boards when they like and rides
//! alone: the ride map shows a countdown, and when it runs out the player
//! arrives (Cosmic's `MapleMap` rides and `Hak` event). Leaving the map
//! early, by falling off or logging out, ends where its `forcedReturn`
//! says, the dock the ride began at.

use crate::conti;

/// A ride map's trip: how long it takes and where it arrives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ride {
    /// The ride map.
    pub map: i32,
    /// How long the ride takes, in seconds of real time.
    pub seconds: u32,
    /// The map and spawn point it arrives at.
    pub to: (i32, u8),
}

impl Ride {
    /// The ride's length on the route clock, which `RUSTMS_CONTI_SCALE`
    /// speeds up for testing; never under a second.
    pub fn countdown(&self) -> u32 {
        let scale = u32::try_from(conti::scale()).unwrap_or(1);
        (self.seconds / scale).max(1)
    }
}

const fn ride(map: i32, seconds: u32, to: (i32, u8)) -> Ride {
    Ride { map, seconds, to }
}

/// Orbis Station to Ereve's Sky Ferry dock, and back.
pub const ORBIS_TO_EREVE: Ride = ride(200090020, 8 * 60, (130000210, 0));
pub const EREVE_TO_ORBIS: Ride = ride(200090021, 8 * 60, (200000161, 0));
/// Ellinia's Sky Ferry dock to Ereve's, and back.
pub const ELLINIA_TO_EREVE: Ride = ride(200090030, 2 * 60, (130000210, 0));
pub const EREVE_TO_ELLINIA: Ride = ride(200090031, 2 * 60, (101000400, 0));
/// Lith Harbor to Rien's dock, and back to Lith Harbor's spawn point 3, by
/// the pier.
pub const LITH_TO_RIEN: Ride = ride(200090060, 60, (140020300, 0));
pub const RIEN_TO_LITH: Ride = ride(200090070, 60, (104000000, 3));
/// The crane from Orbis Station to Mu Lung Temple, and back.
pub const ORBIS_TO_MU_LUNG: Ride = ride(200090300, 60, (250000100, 0));
pub const MU_LUNG_TO_ORBIS: Ride = ride(200090310, 60, (200000141, 0));

const RIDES: [Ride; 8] = [
    ORBIS_TO_EREVE,
    EREVE_TO_ORBIS,
    ELLINIA_TO_EREVE,
    EREVE_TO_ELLINIA,
    LITH_TO_RIEN,
    RIEN_TO_LITH,
    ORBIS_TO_MU_LUNG,
    MU_LUNG_TO_ORBIS,
];

/// The ride `map_id` is, if any.
pub fn ride_on(map_id: i32) -> Option<Ride> {
    RIDES.iter().copied().find(|ride| ride.map == map_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_ride_map_arrives_somewhere_else_and_has_its_own_row() {
        for (i, ride) in RIDES.iter().enumerate() {
            assert_ne!(ride.to.0, ride.map);
            assert_eq!(ride_on(ride.map), Some(*ride));
            assert!(RIDES[i + 1..].iter().all(|other| other.map != ride.map));
            assert!(ride.countdown() >= 1 && ride.countdown() <= ride.seconds);
        }
        assert_eq!(ride_on(RIEN_TO_LITH.map).unwrap().to, (104000000, 3));
        assert_eq!(ride_on(104000000), None);
    }
}
