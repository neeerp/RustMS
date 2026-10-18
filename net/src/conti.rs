//! Continent ships (v83 `ContiMove`): boats between continents that sail on
//! a fixed timetable, the first departure on the hour. Every channel runs
//! its own boats on the same clock, so the state of a trip is a function of
//! the time alone and any field or client actor can work it out.
//!
//! A trip: boarding opens (`Wait`), closes a minute before departure as the
//! ship leaves the platform (`Start`), the waiting room is moved onto the
//! ship (`Move`), and on arrival (`End`) the riders step off and the ship
//! docks at the other end, where it boards for the way back.

use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// A trip's state, as CONTI_STATE (149) sends it and CONTI_MOVE (148) names
/// the change (v83 FUN_0054dd08, FUN_0054dca3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum State {
    /// Between an arrival and the next boarding.
    Dormant = 0,
    /// Boarding.
    Wait = 1,
    /// Boarding has closed and the ship leaves the platform.
    Start = 2,
    /// Under way.
    Move = 3,
    /// Arrived.
    End = 6,
}

/// Which of a route's maps a CONTI_MOVE is for. The client only checks that
/// its own map has a docking ship.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Target {
    /// The departure platform (`startShipMoveField`).
    Platform = 8,
    /// The arrival platform (`endShipMoveField`).
    ArrivalPlatform = 12,
}

/// When a route's ships sail, in minutes.
#[derive(Debug, PartialEq, Eq)]
pub struct Timetable {
    /// Between departures; one leaves on the hour.
    pub term: i64,
    /// Boarding opens this long before a departure...
    pub wait: i64,
    /// ...and closes this long before it.
    pub closed: i64,
    /// From departure to arrival.
    pub ride: i64,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Route {
    /// Where the ship docks and passengers board.
    pub platform: i32,
    pub waiting_room: i32,
    /// The deck, where the waiting room is moved at departure.
    pub deck: i32,
    pub cabin: i32,
    /// Where riders step off, and by which portal.
    pub arrival: (i32, &'static str),
    /// Where the ship docks at the other end.
    pub arrival_platform: i32,
    /// The ticket the platform takes.
    pub ticket: i32,
    pub price: i32,
    pub timetable: Timetable,
}

/// Ellinia and Orbis: one leaves each way every 15 minutes and sails for
/// 10, boarding the 5 before (Continent.img; the minute's closing is from
/// the boarding NPC's text).
const ELLINIA_ORBIS: Timetable = Timetable {
    term: 15,
    wait: 5,
    closed: 1,
    ride: 10,
};

pub const TO_ORBIS: Route = Route {
    platform: 101000300,
    waiting_room: 101000301,
    deck: 200090010,
    cabin: 200090011,
    arrival: (200000100, "come00"),
    arrival_platform: 200000111,
    ticket: 4031045,
    price: 5000,
    timetable: ELLINIA_ORBIS,
};

pub const TO_ELLINIA: Route = Route {
    platform: 200000111,
    waiting_room: 200000112,
    deck: 200090000,
    cabin: 200090001,
    arrival: (101000300, "come00"),
    arrival_platform: 101000300,
    ticket: 4031047,
    price: 5000,
    timetable: ELLINIA_ORBIS,
};

const ROUTES: [&Route; 2] = [&TO_ORBIS, &TO_ELLINIA];

const MINUTE: i64 = 60_000;

impl Timetable {
    /// The state at route time `t`.
    pub fn state(&self, t: i64) -> State {
        let since_departure = t.rem_euclid(self.term * MINUTE);
        if since_departure < self.ride * MINUTE {
            State::Move
        } else if since_departure < (self.term - self.wait) * MINUTE {
            State::Dormant
        } else if since_departure < (self.term - self.closed) * MINUTE {
            State::Wait
        } else {
            State::Start
        }
    }

    /// The changes after route time `from` up to `to`, each once however
    /// many trips passed, oldest first.
    pub fn changes(&self, from: i64, to: i64) -> Vec<State> {
        let term = self.term * MINUTE;
        // Each change happens at its offset from a departure.
        let at = [
            (State::Start, -self.closed * MINUTE),
            (State::Move, 0),
            (State::End, self.ride * MINUTE),
        ];
        let mut changes: Vec<_> = at
            .into_iter()
            .filter(|(_, offset)| (to - offset).div_euclid(term) > (from - offset).div_euclid(term))
            .map(|(state, offset)| ((to - offset).rem_euclid(term), state))
            .collect();
        changes.sort_by_key(|(ago, _)| std::cmp::Reverse(*ago));
        changes.into_iter().map(|(_, state)| state).collect()
    }
}

/// Route time in milliseconds: the wall clock, sped up by
/// `RUSTMS_CONTI_SCALE` (a whole number, default 1) for testing.
pub fn now() -> i64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as i64);
    millis * scale()
}

fn scale() -> i64 {
    static SCALE: OnceLock<i64> = OnceLock::new();
    *SCALE.get_or_init(|| {
        std::env::var("RUSTMS_CONTI_SCALE")
            .ok()
            .and_then(|scale| scale.parse().ok())
            .filter(|scale| *scale >= 1)
            .unwrap_or(1)
    })
}

/// Route time `t` as a time of day (UTC), for the stations' wall clocks.
pub fn time_of_day(t: i64) -> (u8, u8, u8) {
    let seconds = (t / 1000).rem_euclid(24 * 60 * 60);
    let part = |n: i64| u8::try_from(n).unwrap_or(0);
    (
        part(seconds / 3600),
        part(seconds / 60 % 60),
        part(seconds % 60),
    )
}

/// The route whose ship the boarding NPC on `platform` takes passengers to.
pub fn departing_from(platform: i32) -> Option<&'static Route> {
    ROUTES.into_iter().find(|route| route.platform == platform)
}

/// The state a client on `map_id` is told when it asks (send 0xDC): that of
/// the route whose ship it shows, if any.
pub fn ship_state(map_id: i32, t: i64) -> Option<State> {
    ROUTES
        .into_iter()
        .find(|route| [route.platform, route.arrival_platform, route.deck].contains(&map_id))
        .map(|route| route.timetable.state(t))
}

/// What a field does when a trip changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Happening {
    /// Show the ship leaving or docking (CONTI_MOVE).
    Show(Target, State),
    /// Move everyone on the map there.
    MoveAll { map_id: i32, portal: &'static str },
}

/// Whether trips ever change anything on `map_id`.
pub fn has_trips(map_id: i32) -> bool {
    ROUTES.into_iter().any(|route| {
        [
            route.platform,
            route.waiting_room,
            route.deck,
            route.cabin,
            route.arrival_platform,
        ]
        .contains(&map_id)
    })
}

/// What `map_id` does for the trips that changed after route time `from`
/// up to `to`.
pub fn happenings(map_id: i32, from: i64, to: i64) -> Vec<Happening> {
    let mut happenings = Vec::new();
    for route in ROUTES {
        for change in route.timetable.changes(from, to) {
            let happening = match change {
                State::Start if map_id == route.platform => {
                    Happening::Show(Target::Platform, State::Start)
                }
                State::Move if map_id == route.waiting_room => Happening::MoveAll {
                    map_id: route.deck,
                    portal: "",
                },
                State::End if map_id == route.deck || map_id == route.cabin => {
                    let (map_id, portal) = route.arrival;
                    Happening::MoveAll { map_id, portal }
                }
                State::End if map_id == route.arrival_platform => {
                    Happening::Show(Target::ArrivalPlatform, State::End)
                }
                _ => continue,
            };
            happenings.push(happening);
        }
    }
    happenings
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOUR: i64 = 60 * MINUTE;

    #[test]
    fn ships_leave_every_quarter_hour_after_four_minutes_of_boarding() {
        let at = |minutes: i64, seconds: i64| {
            ELLINIA_ORBIS.state(7 * HOUR + minutes * MINUTE + seconds * 1000)
        };
        assert_eq!(at(0, 0), State::Move);
        assert_eq!(at(9, 59), State::Move);
        assert_eq!(at(10, 0), State::Wait);
        assert_eq!(at(13, 59), State::Wait);
        assert_eq!(at(14, 0), State::Start);
        assert_eq!(at(14, 59), State::Start);
        assert_eq!(at(15, 0), State::Move);
        assert_eq!(at(55, 30), State::Wait);
    }

    #[test]
    fn a_change_shows_once_between_the_readings_that_span_it() {
        let t = 7 * HOUR + 14 * MINUTE;
        assert_eq!(ELLINIA_ORBIS.changes(t - 100, t - 1), []);
        assert_eq!(ELLINIA_ORBIS.changes(t - 1, t), [State::Start]);
        assert_eq!(ELLINIA_ORBIS.changes(t, t + 100), []);
        // A long gap still gives each change once, in the order they last
        // happened: the arrival at 9:10, boarding closing at 9:14, and the
        // departure at 9:15.
        assert_eq!(
            ELLINIA_ORBIS.changes(t, t + 2 * HOUR + 3 * MINUTE),
            [State::End, State::Start, State::Move]
        );
    }

    #[test]
    fn the_route_maps_follow_a_trip_from_boarding_to_arrival() {
        let departure = 7 * HOUR;
        let happens = |map_id, at: i64| happenings(map_id, at - 50, at + 50);
        assert_eq!(
            happens(101000300, departure - MINUTE),
            [Happening::Show(Target::Platform, State::Start)]
        );
        assert_eq!(
            happens(101000301, departure),
            [Happening::MoveAll {
                map_id: 200090010,
                portal: ""
            }]
        );
        let arrival = departure + 10 * MINUTE;
        for ship in [200090010, 200090011] {
            assert_eq!(
                happens(ship, arrival),
                [Happening::MoveAll {
                    map_id: 200000100,
                    portal: "come00"
                }]
            );
        }
        assert_eq!(
            happens(200000111, arrival),
            [Happening::Show(Target::ArrivalPlatform, State::End)]
        );
        // Ellinia sees the other ship dock as its own boards.
        assert_eq!(
            happens(101000300, arrival),
            [Happening::Show(Target::ArrivalPlatform, State::End)]
        );
        assert_eq!(happens(101000000, arrival), []);
        assert!(!has_trips(101000000));
    }

    #[test]
    fn the_wall_clock_reads_the_route_time_of_day() {
        assert_eq!(time_of_day(7 * HOUR + 14 * MINUTE + 9_500), (7, 14, 9));
        assert_eq!(time_of_day(3 * 24 * HOUR + 23 * HOUR), (23, 0, 0));
    }

    #[test]
    fn clients_are_told_the_state_of_the_ship_their_map_shows() {
        let boarding = 7 * HOUR + 12 * MINUTE;
        assert_eq!(ship_state(101000300, boarding), Some(State::Wait));
        assert_eq!(ship_state(200000111, boarding), Some(State::Wait));
        assert_eq!(ship_state(200090000, 7 * HOUR), Some(State::Move));
        assert_eq!(ship_state(200000100, boarding), None);
    }
}
