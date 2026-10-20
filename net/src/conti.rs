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
    /// The Crimson Balrog's ship comes alongside a deck (`MobGen`).
    MobGen = 4,
    /// It sails off (`MobDestroy`).
    MobDestroy = 5,
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
    /// A deck, which the Crimson Balrog's ship comes alongside.
    Deck = 10,
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
    /// Only Ellinia's ships have one.
    pub cabin: Option<i32>,
    /// Where the Crimson Balrogs board the deck, on a route whose ship they
    /// attack (Cosmic's Boats.js).
    pub balrogs: Option<(i16, i16)>,
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
    cabin: Some(200090011),
    balrogs: Some((339, 148)),
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
    cabin: Some(200090001),
    balrogs: Some((-538, 143)),
    arrival: (101000300, "come00"),
    arrival_platform: 101000300,
    ticket: 4031047,
    price: 5000,
    timetable: ELLINIA_ORBIS,
};

/// Orbis and Ludibrium, Leafre and Ariant: one leaves each way every 10
/// minutes and sails for 5, boarding the 5 before (Continent.img and
/// Cosmic; Ariant from Cosmic alone, report 183-routes).
const ORBIS_BOATS: Timetable = Timetable {
    term: 10,
    wait: 5,
    closed: 1,
    ride: 5,
};

pub const TO_LUDIBRIUM: Route = Route {
    platform: 200000121,
    waiting_room: 200000122,
    deck: 200090100,
    cabin: None,
    balrogs: None,
    arrival: (220000100, "sp"),
    arrival_platform: 220000110,
    ticket: 4031074,
    price: 6000,
    timetable: ORBIS_BOATS,
};

/// Riders land on Orbis's `ludi00`, the spawn point named for this train (an
/// inference; Cosmic uses the first). Leafre's and Ariant's land on the
/// first, as Cosmic's do.
pub const LUDIBRIUM_TO_ORBIS: Route = Route {
    platform: 220000110,
    waiting_room: 220000111,
    deck: 200090110,
    cabin: None,
    balrogs: None,
    arrival: (200000100, "ludi00"),
    arrival_platform: 200000121,
    ticket: 4031045,
    price: 6000,
    timetable: ORBIS_BOATS,
};

pub const TO_LEAFRE: Route = Route {
    platform: 200000131,
    waiting_room: 200000132,
    deck: 200090200,
    cabin: None,
    balrogs: None,
    arrival: (240000100, "sp"),
    arrival_platform: 240000110,
    ticket: 4031331,
    price: 30000,
    timetable: ORBIS_BOATS,
};

pub const LEAFRE_TO_ORBIS: Route = Route {
    platform: 240000110,
    waiting_room: 240000111,
    deck: 200090210,
    cabin: None,
    balrogs: None,
    arrival: (200000100, "sp"),
    arrival_platform: 200000131,
    ticket: 4031045,
    price: 30000,
    timetable: ORBIS_BOATS,
};

/// Ariant's platform is also the town's entrance. Both its spawn points are
/// named `sp`; riders get the first, at the west end (Cosmic uses the
/// second).
pub const TO_ARIANT: Route = Route {
    platform: 200000151,
    waiting_room: 200000152,
    deck: 200090400,
    cabin: None,
    balrogs: None,
    arrival: (260000100, "sp"),
    arrival_platform: 260000100,
    ticket: 4031576,
    price: 6000,
    timetable: ORBIS_BOATS,
};

pub const ARIANT_TO_ORBIS: Route = Route {
    platform: 260000100,
    waiting_room: 260000110,
    deck: 200090410,
    cabin: None,
    balrogs: None,
    arrival: (200000100, "sp"),
    arrival_platform: 200000151,
    ticket: 4031045,
    price: 6000,
    timetable: ORBIS_BOATS,
};

const ROUTES: [&Route; 8] = [
    &TO_ORBIS,
    &TO_ELLINIA,
    &TO_LUDIBRIUM,
    &LUDIBRIUM_TO_ORBIS,
    &TO_LEAFRE,
    &LEAFRE_TO_ORBIS,
    &TO_ARIANT,
    &ARIANT_TO_ORBIS,
];

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

/// The Crimson Balrog, and how many board a deck: Continent.img's `genMob`
/// is a sack (2100009) of two.
pub const BALROG: i32 = 8150000;
pub const BALROGS: usize = 2;

/// When the Crimson Balrog's ship attacks a trip, in route time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Invasion {
    /// The ship comes alongside.
    pub appear: i64,
    /// The Balrogs board, 5 seconds later.
    pub board: i64,
    /// The ship sails off, taking them with it, a minute before arrival
    /// (`tEventEnd`, 9 minutes, in a later version's Continent.img).
    pub end: i64,
}

/// The attack on the trip that departs at route time `departure`, if it is
/// attacked: one roll for both decks and every channel, as in Cosmic, which
/// has 42% of trips attacked 3 to 4 minutes after departure.
/// `RUSTMS_CONTI_BALROG` sets the percentage.
pub fn invasion(departure: i64) -> Option<Invasion> {
    let roll = splitmix64(departure as u64);
    if roll % 100 >= balrog_odds() {
        return None;
    }
    let appear = departure + 3 * MINUTE + ((roll >> 8) % MINUTE as u64) as i64;
    Some(Invasion {
        appear,
        board: appear + 5_000,
        end: departure + 9 * MINUTE,
    })
}

fn balrog_odds() -> u64 {
    static ODDS: OnceLock<u64> = OnceLock::new();
    *ODDS.get_or_init(|| {
        std::env::var("RUSTMS_CONTI_BALROG")
            .ok()
            .and_then(|odds| odds.parse().ok())
            .filter(|odds| *odds <= 100)
            .unwrap_or(42)
    })
}

fn splitmix64(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
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

/// The route whose waiting room is `map_id`.
pub fn waiting_in(map_id: i32) -> Option<&'static Route> {
    ROUTES
        .into_iter()
        .find(|route| route.waiting_room == map_id)
}

/// The state a client on `map_id` is told when it asks (send 0xDC): that of
/// the route whose ship it shows, if any, and on a deck whether the Crimson
/// Balrog's ship is alongside.
pub fn ship_state(map_id: i32, t: i64) -> Option<(State, bool)> {
    let route = ROUTES
        .into_iter()
        .find(|route| [route.platform, route.arrival_platform, route.deck].contains(&map_id))?;
    let state = route.timetable.state(t);
    let departure = t - t.rem_euclid(route.timetable.term * MINUTE);
    let alongside = map_id == route.deck
        && route.balrogs.is_some()
        && state == State::Move
        && invasion(departure).is_some_and(|attack| attack.appear <= t && t < attack.end);
    Some((state, alongside))
}

/// What a field does when a trip changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Happening {
    /// Show the ship leaving or docking (CONTI_MOVE).
    Show(Target, State),
    /// Move everyone on the map there.
    MoveAll { map_id: i32, portal: &'static str },
    /// Bring `count` of `mob_id` aboard on the ground below `at`.
    Board {
        mob_id: i32,
        count: usize,
        at: (i16, i16),
    },
    /// The Balrog's ship sails off with every mob aboard.
    Retreat,
}

/// Whether trips ever change anything on `map_id`.
pub fn has_trips(map_id: i32) -> bool {
    ROUTES.into_iter().any(|route| {
        [
            route.platform,
            route.waiting_room,
            route.deck,
            route.arrival_platform,
        ]
        .contains(&map_id)
            || route.cabin == Some(map_id)
    })
}

/// What `map_id` does for the trips that changed after route time `from`
/// up to `to`.
pub fn happenings(map_id: i32, from: i64, to: i64) -> Vec<Happening> {
    let mut happenings = Vec::new();
    for route in ROUTES {
        if map_id == route.deck {
            happenings.extend(attacks(route, from, to));
        }
        for change in route.timetable.changes(from, to) {
            let happening = match change {
                State::Start if map_id == route.platform => {
                    Happening::Show(Target::Platform, State::Start)
                }
                State::Move if map_id == route.waiting_room => Happening::MoveAll {
                    map_id: route.deck,
                    portal: "",
                },
                State::End if map_id == route.deck || route.cabin == Some(map_id) => {
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

/// What the Crimson Balrog's attacks do on `route`'s deck after route time
/// `from` up to `to`, oldest first; at most the last trip's.
fn attacks(route: &Route, from: i64, to: i64) -> Vec<Happening> {
    let Some(at) = route.balrogs else {
        return Vec::new();
    };
    let term = route.timetable.term * MINUTE;
    let from = from.max(to - term);
    let mut timed = Vec::new();
    for trip in (from.div_euclid(term) - 1)..=to.div_euclid(term) {
        let Some(attack) = invasion(trip * term) else {
            continue;
        };
        timed.extend([
            (attack.appear, Happening::Show(Target::Deck, State::MobGen)),
            (
                attack.board,
                Happening::Board {
                    mob_id: BALROG,
                    count: BALROGS,
                    at,
                },
            ),
            (attack.end, Happening::Retreat),
        ]);
    }
    timed.retain(|(t, _)| from < *t && *t <= to);
    timed.sort_by_key(|(t, _)| *t);
    timed.into_iter().map(|(_, happening)| happening).collect()
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
    fn orbis_boats_leave_every_ten_minutes_for_ludibrium_leafre_and_ariant() {
        let departure = 7 * HOUR + 20 * MINUTE;
        let happens = |map_id, at: i64| happenings(map_id, at - 50, at + 50);
        assert_eq!(ORBIS_BOATS.state(departure - 5 * MINUTE), State::Wait);
        assert_eq!(ORBIS_BOATS.state(departure - MINUTE), State::Start);
        assert_eq!(
            happens(200000132, departure),
            [Happening::MoveAll {
                map_id: 200090200,
                portal: ""
            }]
        );
        let arrival = departure + 5 * MINUTE;
        assert_eq!(
            happens(200090200, arrival),
            [Happening::MoveAll {
                map_id: 240000100,
                portal: "sp"
            }]
        );
        // Ariant's platform sees its own ship leave and the other dock.
        assert_eq!(
            happens(260000100, departure - MINUTE),
            [Happening::Show(Target::Platform, State::Start)]
        );
        assert_eq!(
            happens(260000100, arrival),
            [Happening::Show(Target::ArrivalPlatform, State::End)]
        );
        assert_eq!(waiting_in(220000111), Some(&LUDIBRIUM_TO_ORBIS));
        assert_eq!(
            ship_state(220000110, departure + MINUTE),
            Some((State::Move, false))
        );
    }

    #[test]
    fn the_wall_clock_reads_the_route_time_of_day() {
        assert_eq!(time_of_day(7 * HOUR + 14 * MINUTE + 9_500), (7, 14, 9));
        assert_eq!(time_of_day(3 * 24 * HOUR + 23 * HOUR), (23, 0, 0));
    }

    #[test]
    fn clients_are_told_the_state_of_the_ship_their_map_shows() {
        let boarding = 7 * HOUR + 12 * MINUTE;
        assert_eq!(ship_state(101000300, boarding), Some((State::Wait, false)));
        assert_eq!(ship_state(200000111, boarding), Some((State::Wait, false)));
        assert_eq!(ship_state(200090000, 7 * HOUR), Some((State::Move, false)));
        assert_eq!(ship_state(200000100, boarding), None);
    }

    /// The first `count` Ellinia trips from 7:00 that are attacked.
    fn attacked(count: usize) -> Vec<(i64, Invasion)> {
        (0..)
            .map(|trip| 7 * HOUR + trip * 15 * MINUTE)
            .filter_map(|departure| Some((departure, invasion(departure)?)))
            .take(count)
            .collect()
    }

    #[test]
    fn some_trips_meet_the_balrog_between_three_and_four_minutes_out() {
        let trips = 1000;
        let hit = (0..trips)
            .filter(|trip| invasion(trip * 15 * MINUTE).is_some())
            .count();
        assert!((350..490).contains(&hit), "{hit} of {trips}");
        for (departure, attack) in attacked(50) {
            assert!((3 * MINUTE..4 * MINUTE).contains(&(attack.appear - departure)));
            assert_eq!(attack.board, attack.appear + 5_000);
            assert_eq!(attack.end, departure + 9 * MINUTE);
            assert_eq!(invasion(departure), Some(attack));
        }
    }

    #[test]
    fn the_balrog_comes_aboard_both_ellinia_decks_and_leaves_before_arrival() {
        let (departure, attack) = attacked(1)[0];
        let happens = |map_id, at: i64| happenings(map_id, at - 1, at);
        for (deck, at) in [(200090010, (339, 148)), (200090000, (-538, 143))] {
            assert_eq!(
                happens(deck, attack.appear),
                [Happening::Show(Target::Deck, State::MobGen)]
            );
            assert_eq!(
                happens(deck, attack.board),
                [Happening::Board {
                    mob_id: BALROG,
                    count: 2,
                    at
                }]
            );
            assert_eq!(happens(deck, attack.end), [Happening::Retreat]);
            assert_eq!(
                ship_state(deck, attack.appear - 1),
                Some((State::Move, false))
            );
            assert_eq!(ship_state(deck, attack.appear), Some((State::Move, true)));
            assert_eq!(ship_state(deck, attack.end - 1), Some((State::Move, true)));
            assert_eq!(ship_state(deck, attack.end), Some((State::Move, false)));
        }
        // Not the cabins, nor the other routes' decks.
        assert_eq!(happens(200090011, attack.appear), []);
        assert_eq!(happens(200090200, departure + 3 * MINUTE + 30_000), []);
        // A field that missed the whole attack still sees it, in order.
        assert_eq!(
            happenings(200090010, departure, attack.end),
            [
                Happening::Show(Target::Deck, State::MobGen),
                Happening::Board {
                    mob_id: BALROG,
                    count: 2,
                    at: (339, 148)
                },
                Happening::Retreat
            ]
        );
    }
}
