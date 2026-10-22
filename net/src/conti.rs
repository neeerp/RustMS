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
    /// Between departures; one leaves `offset` after the hour.
    pub term: i64,
    /// Boarding opens this long before a departure...
    pub wait: i64,
    /// ...and closes this long before it.
    pub closed: i64,
    /// From departure to arrival.
    pub ride: i64,
    /// How long after the hour a departure is: 0 but for the Helios
    /// Tower elevator's way up.
    pub offset: i64,
}

/// A portal riders are put at: by name, the first of that name (none
/// is the map's first spawn point), or by id where a map's spawn points
/// share their name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Portal {
    Named(&'static str),
    Id(u8),
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
    pub arrival: (i32, Portal),
    /// Where the ship docks at the other end.
    pub arrival_platform: i32,
    /// The ticket the platform takes; 0 for the elevator, which is free.
    pub ticket: i32,
    pub price: i32,
    pub timetable: Timetable,
    /// Whether the platforms show a ship docking and leaving. The subway's
    /// maps have none (no `shipObj`), so they get no CONTI packets.
    pub ship: bool,
    /// The name of the platforms' reactor that shows the car docked or
    /// gone, for the Helios Tower elevator.
    pub car: Option<&'static str>,
}

/// Ellinia and Orbis: one leaves each way every 15 minutes and sails for
/// 10, boarding the 5 before (Continent.img; the minute's closing is from
/// the boarding NPC's text).
const ELLINIA_ORBIS: Timetable = Timetable {
    term: 15,
    wait: 5,
    closed: 1,
    ride: 10,
    offset: 0,
};

pub const TO_ORBIS: Route = Route {
    platform: 101000300,
    waiting_room: 101000301,
    deck: 200090010,
    cabin: Some(200090011),
    balrogs: Some((339, 148)),
    arrival: (200000100, Portal::Named("come00")),
    arrival_platform: 200000111,
    ticket: 4031045,
    price: 5000,
    timetable: ELLINIA_ORBIS,
    ship: true,
    car: None,
};

pub const TO_ELLINIA: Route = Route {
    platform: 200000111,
    waiting_room: 200000112,
    deck: 200090000,
    cabin: Some(200090001),
    balrogs: Some((-538, 143)),
    arrival: (101000300, Portal::Named("come00")),
    arrival_platform: 101000300,
    ticket: 4031047,
    price: 5000,
    timetable: ELLINIA_ORBIS,
    ship: true,
    car: None,
};

/// Orbis and Ludibrium, Leafre and Ariant: one leaves each way every 10
/// minutes and sails for 5, boarding the 5 before (Continent.img and
/// Cosmic; Ariant from Cosmic alone, report 183-routes).
const ORBIS_BOATS: Timetable = Timetable {
    term: 10,
    wait: 5,
    closed: 1,
    ride: 5,
    offset: 0,
};

pub const TO_LUDIBRIUM: Route = Route {
    platform: 200000121,
    waiting_room: 200000122,
    deck: 200090100,
    cabin: None,
    balrogs: None,
    arrival: (220000100, Portal::Named("sp")),
    arrival_platform: 220000110,
    ticket: 4031074,
    price: 6000,
    timetable: ORBIS_BOATS,
    ship: true,
    car: None,
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
    arrival: (200000100, Portal::Named("ludi00")),
    arrival_platform: 200000121,
    ticket: 4031045,
    price: 6000,
    timetable: ORBIS_BOATS,
    ship: true,
    car: None,
};

pub const TO_LEAFRE: Route = Route {
    platform: 200000131,
    waiting_room: 200000132,
    deck: 200090200,
    cabin: None,
    balrogs: None,
    arrival: (240000100, Portal::Named("sp")),
    arrival_platform: 240000110,
    ticket: 4031331,
    price: 30000,
    timetable: ORBIS_BOATS,
    ship: true,
    car: None,
};

pub const LEAFRE_TO_ORBIS: Route = Route {
    platform: 240000110,
    waiting_room: 240000111,
    deck: 200090210,
    cabin: None,
    balrogs: None,
    arrival: (200000100, Portal::Named("sp")),
    arrival_platform: 200000131,
    ticket: 4031045,
    price: 30000,
    timetable: ORBIS_BOATS,
    ship: true,
    car: None,
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
    arrival: (260000100, Portal::Named("sp")),
    arrival_platform: 260000100,
    ticket: 4031576,
    price: 6000,
    timetable: ORBIS_BOATS,
    ship: true,
    car: None,
};

pub const ARIANT_TO_ORBIS: Route = Route {
    platform: 260000100,
    waiting_room: 260000110,
    deck: 200090410,
    cabin: None,
    balrogs: None,
    arrival: (200000100, Portal::Named("sp")),
    arrival_platform: 200000151,
    ticket: 4031045,
    price: 6000,
    timetable: ORBIS_BOATS,
    ship: true,
    car: None,
};

/// The subway between Kerning City and New Leaf City: one leaves each way
/// every 5 minutes and rides for 1, boarding the 4 before (Edelstein's
/// Continent 8 and 9). Riders land on the first spawn point.
const SUBWAY: Timetable = Timetable {
    term: 5,
    wait: 4,
    closed: 1,
    ride: 1,
    offset: 0,
};

/// The fare is Cosmic's.
pub const TO_NLC: Route = Route {
    platform: 103000100,
    waiting_room: 600010004,
    deck: 600010005,
    cabin: None,
    balrogs: None,
    arrival: (600010001, Portal::Named("sp")),
    arrival_platform: 600010001,
    ticket: 4031711,
    price: 5000,
    timetable: SUBWAY,
    ship: false,
    car: None,
};

pub const TO_KERNING_CITY: Route = Route {
    platform: 600010001,
    waiting_room: 600010002,
    deck: 600010003,
    cabin: None,
    balrogs: None,
    arrival: (103000100, Portal::Named("sp")),
    arrival_platform: 103000100,
    ticket: 4031713,
    price: 5000,
    timetable: SUBWAY,
    ship: false,
    car: None,
};

/// Kerning City and Changi: one plane leaves each way every 6 minutes and
/// flies for 1, boarding the 4 after it lands (Cosmic's AirPlane event).
const PLANE: Timetable = Timetable {
    term: 6,
    wait: 5,
    closed: 1,
    ride: 1,
    offset: 0,
};

/// The fare is Cosmic's.
pub const TO_CHANGI: Route = Route {
    platform: 103000000,
    waiting_room: 540010100,
    deck: 540010101,
    cabin: None,
    balrogs: None,
    arrival: (540010000, Portal::Named("sp")),
    arrival_platform: 540010000,
    ticket: 4031731,
    price: 5000,
    timetable: PLANE,
    ship: false,
    car: None,
};

/// Kerning City's spawn points are all `sp`; riders land on the one by
/// Irene's booth (Cosmic).
pub const AIR_TO_KERNING_CITY: Route = Route {
    platform: 540010000,
    waiting_room: 540010001,
    deck: 540010002,
    cabin: None,
    balrogs: None,
    arrival: (103000000, Portal::Id(7)),
    arrival_platform: 103000000,
    ticket: 4031732,
    price: 5000,
    timetable: PLANE,
    ship: false,
    car: None,
};

/// The Helios Tower elevator in Ludibrium, between the 2nd floor and the
/// 99th: the car docks on a floor for a minute, rides for one, and docks
/// on the other floor two minutes after it left (Edelstein's Continent 4
/// and 5: term 4, wait 1, ride 1, the way up 2 behind; Cosmic's
/// Elevator.js). It is free; riders board through the floor's `in00`.
const ELEVATOR_DOWN_TIMES: Timetable = Timetable {
    term: 4,
    wait: 1,
    closed: 0,
    ride: 1,
    offset: 0,
};

const ELEVATOR_UP_TIMES: Timetable = Timetable {
    offset: 2,
    ..ELEVATOR_DOWN_TIMES
};

/// The elevator's reactor (2208004) on both floors: state 0 is the car
/// with its door open, 1 the door shut (Reactor.nx).
pub const CAR_HERE: u8 = 0;
pub const CAR_AWAY: u8 = 1;

/// Riders land on the 99th floor's first spawn point (Cosmic).
pub const ELEVATOR_UP: Route = Route {
    platform: 222020100,
    waiting_room: 222020110,
    deck: 222020111,
    cabin: None,
    balrogs: None,
    arrival: (222020200, Portal::Id(0)),
    arrival_platform: 222020200,
    ticket: 0,
    price: 0,
    timetable: ELEVATOR_UP_TIMES,
    ship: false,
    car: Some("elevator"),
};

pub const ELEVATOR_DOWN: Route = Route {
    platform: 222020200,
    waiting_room: 222020210,
    deck: 222020211,
    cabin: None,
    balrogs: None,
    arrival: (222020100, Portal::Named("st00")),
    arrival_platform: 222020100,
    ticket: 0,
    price: 0,
    timetable: ELEVATOR_DOWN_TIMES,
    ship: false,
    car: Some("elevator"),
};

const ROUTES: [&Route; 14] = [
    &TO_ORBIS,
    &TO_ELLINIA,
    &TO_LUDIBRIUM,
    &LUDIBRIUM_TO_ORBIS,
    &TO_LEAFRE,
    &LEAFRE_TO_ORBIS,
    &TO_ARIANT,
    &ARIANT_TO_ORBIS,
    &TO_NLC,
    &TO_KERNING_CITY,
    &TO_CHANGI,
    &AIR_TO_KERNING_CITY,
    &ELEVATOR_UP,
    &ELEVATOR_DOWN,
];

const MINUTE: i64 = 60_000;

impl Timetable {
    /// The state at route time `t`.
    pub fn state(&self, t: i64) -> State {
        let since_departure = (t - self.offset * MINUTE).rem_euclid(self.term * MINUTE);
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
        let (from, to) = (from - self.offset * MINUTE, to - self.offset * MINUTE);
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

/// How many times as fast as the wall clock route time runs.
pub(crate) fn scale() -> i64 {
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

/// The route whose riders are on `map_id`, its deck, if any.
pub fn riding_on(map_id: i32) -> Option<&'static Route> {
    ROUTES.into_iter().find(|route| route.deck == map_id)
}

/// The state a client on `map_id` is told when it asks (send 0xDC): that of
/// the route whose ship it shows, if any, and on a deck whether the Crimson
/// Balrog's ship is alongside.
pub fn ship_state(map_id: i32, t: i64) -> Option<(State, bool)> {
    let route = ROUTES.into_iter().find(|route| {
        route.ship && [route.platform, route.arrival_platform, route.deck].contains(&map_id)
    })?;
    let state = route.timetable.state(t);
    let timetable = &route.timetable;
    let departure = t - (t - timetable.offset * MINUTE).rem_euclid(timetable.term * MINUTE);
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
    MoveAll { map_id: i32, portal: Portal },
    /// Bring `count` of `mob_id` aboard on the ground below `at`.
    Board {
        mob_id: i32,
        count: usize,
        at: (i16, i16),
    },
    /// The Balrog's ship sails off with every mob aboard.
    Retreat,
}

/// Whether trips ever change anything on `map_id`. Without a ship or the
/// elevator's car, the platforms see nothing.
pub fn has_trips(map_id: i32) -> bool {
    ROUTES.into_iter().any(|route| {
        [route.waiting_room, route.deck].contains(&map_id)
            || route.cabin == Some(map_id)
            || (route.ship || route.car.is_some())
                && [route.platform, route.arrival_platform].contains(&map_id)
    })
}

/// The state of the car's reactor on `map_id`, if it is a floor the
/// elevator docks at: there while boarding for the way from it is open.
/// The floor's field sets it from the clock on every reading rather than
/// on each change, so a reading that spans both the car's arrival and its
/// departure still leaves the right door.
pub fn car_state(map_id: i32, t: i64) -> Option<(&'static str, u8)> {
    let route = ROUTES
        .into_iter()
        .find(|route| route.platform == map_id && route.car.is_some())?;
    let state = match route.timetable.state(t) {
        State::Wait => CAR_HERE,
        _ => CAR_AWAY,
    };
    Some((route.car?, state))
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
                State::Start if map_id == route.platform && route.ship => {
                    Happening::Show(Target::Platform, State::Start)
                }
                State::Move if map_id == route.waiting_room => Happening::MoveAll {
                    map_id: route.deck,
                    portal: Portal::Named(""),
                },
                State::End if map_id == route.deck || route.cabin == Some(map_id) => {
                    let (map_id, portal) = route.arrival;
                    Happening::MoveAll { map_id, portal }
                }
                State::End if map_id == route.arrival_platform && route.ship => {
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
/// `from` up to `to`, oldest first; at most the last trip's. Its routes
/// leave on multiples of their term (`offset` 0).
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
                portal: Portal::Named("")
            }]
        );
        let arrival = departure + 10 * MINUTE;
        for ship in [200090010, 200090011] {
            assert_eq!(
                happens(ship, arrival),
                [Happening::MoveAll {
                    map_id: 200000100,
                    portal: Portal::Named("come00")
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
                portal: Portal::Named("")
            }]
        );
        let arrival = departure + 5 * MINUTE;
        assert_eq!(
            happens(200090200, arrival),
            [Happening::MoveAll {
                map_id: 240000100,
                portal: Portal::Named("sp")
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

    #[test]
    fn the_subway_runs_every_five_minutes_without_a_ship() {
        let departure = 7 * HOUR + 5 * MINUTE;
        let happens = |map_id, at: i64| happenings(map_id, at - 50, at + 50);
        assert_eq!(SUBWAY.state(departure - 4 * MINUTE), State::Wait);
        assert_eq!(SUBWAY.state(departure - MINUTE), State::Start);
        assert_eq!(SUBWAY.state(departure + 30_000), State::Move);
        assert_eq!(departing_from(103000100), Some(&TO_NLC));
        assert_eq!(waiting_in(600010002), Some(&TO_KERNING_CITY));
        // The waiting room boards the train and the train arrives...
        assert_eq!(
            happens(600010004, departure),
            [Happening::MoveAll {
                map_id: 600010005,
                portal: Portal::Named("")
            }]
        );
        assert_eq!(
            happens(600010003, departure + MINUTE),
            [Happening::MoveAll {
                map_id: 103000100,
                portal: Portal::Named("sp")
            }]
        );
        // ...but the stations show no ship, and don't answer for one.
        for station in [103000100, 600010001] {
            assert_eq!(happens(station, departure - MINUTE), []);
            assert_eq!(happens(station, departure + MINUTE), []);
            assert_eq!(ship_state(station, departure), None);
        }
    }

    #[test]
    fn the_helios_elevator_docks_a_minute_on_each_floor_in_turn() {
        // The car leaves the 99th floor on the hour and the 2nd floor two
        // minutes later.
        let down = 9 * HOUR + 8 * MINUTE;
        let up = down + 2 * MINUTE;
        let happens = |map_id, at: i64| happenings(map_id, at - 50, at + 50);
        let car = |map_id, at| car_state(map_id, at).map(|(_, state)| state);
        for minute in 0..4 {
            let at = down + minute * MINUTE + 30_000;
            // On one floor or the other, never both, boarding only there.
            let floors = [car(222020100, at), car(222020200, at)];
            assert!(
                floors == [Some(CAR_HERE), Some(CAR_AWAY)]
                    || floors == [Some(CAR_AWAY), Some(CAR_HERE)]
                    || floors == [Some(CAR_AWAY), Some(CAR_AWAY)],
                "minute {minute}: {floors:?}"
            );
            assert_eq!(
                ELEVATOR_UP.timetable.state(at) == State::Wait,
                floors[0] == Some(CAR_HERE)
            );
        }
        assert_eq!(car(222020100, up - 30_000), Some(CAR_HERE));
        assert_eq!(car(222020200, down - 30_000), Some(CAR_HERE));
        assert_eq!(car(222020100, up + 30_000), Some(CAR_AWAY));
        assert_eq!(car(222020110, up), None);

        // The floors see no change of their own (the field reads the car
        // from the clock); the waiting rooms and cars move.
        for at in [up, up + MINUTE, down, down + MINUTE] {
            assert_eq!(happens(222020100, at), []);
            assert_eq!(happens(222020200, at), []);
        }
        assert_eq!(
            happens(222020110, up),
            [Happening::MoveAll {
                map_id: 222020111,
                portal: Portal::Named("")
            }]
        );
        assert_eq!(
            happens(222020111, up + MINUTE),
            [Happening::MoveAll {
                map_id: 222020200,
                portal: Portal::Id(0)
            }]
        );
        assert_eq!(
            happens(222020211, down + MINUTE),
            [Happening::MoveAll {
                map_id: 222020100,
                portal: Portal::Named("st00")
            }]
        );
        assert!(has_trips(222020100) && has_trips(222020211));
        assert_eq!(ship_state(222020100, up), None);
    }

    #[test]
    fn the_plane_flies_every_six_minutes_and_lands_by_irene() {
        let departure = 7 * HOUR + 6 * MINUTE;
        let happens = |map_id, at: i64| happenings(map_id, at - 50, at + 50);
        assert_eq!(PLANE.state(departure - 5 * MINUTE), State::Wait);
        assert_eq!(PLANE.state(departure - MINUTE), State::Start);
        assert_eq!(PLANE.state(departure + 30_000), State::Move);
        assert_eq!(departing_from(103000000), Some(&TO_CHANGI));
        assert_eq!(waiting_in(540010001), Some(&AIR_TO_KERNING_CITY));
        assert_eq!(riding_on(540010101), Some(&TO_CHANGI));
        assert_eq!(riding_on(540010100), None);
        assert_eq!(
            happens(540010100, departure),
            [Happening::MoveAll {
                map_id: 540010101,
                portal: Portal::Named("")
            }]
        );
        assert_eq!(
            happens(540010002, departure + MINUTE),
            [Happening::MoveAll {
                map_id: 103000000,
                portal: Portal::Id(7)
            }]
        );
        for airport in [103000000, 540010000] {
            assert!(!has_trips(airport));
            assert_eq!(happens(airport, departure - MINUTE), []);
            assert_eq!(happens(airport, departure + MINUTE), []);
            assert_eq!(ship_state(airport, departure), None);
        }
    }
}
