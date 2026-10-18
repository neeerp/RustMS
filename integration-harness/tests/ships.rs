//! The ship from Ellinia to Orbis (CONTI_MOVE): the station's wall clock,
//! boarding with a ticket while the ship boards, the waiting room moved onto
//! the deck at departure, and the riders set down in Orbis on arrival. The
//! harness world runs the route clock 60 times as fast
//! (`RUSTMS_CONTI_SCALE`), so a trip takes 15 seconds.

use game_data::GameData;
use integration_harness::database::grant_item;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::conti::{self, State};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::path::Path;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::Duration;

/// `RUSTMS_CONTI_SCALE` in `docker-compose.test.yml`.
const SCALE: i64 = 60;
const MINUTE: i64 = 60_000;
const ELLINIA_STATION: i32 = 101_000_300;
const ORBIS_STATION: i32 = 200_000_100;
const CHERRY: i32 = 1032008;

fn maps() -> &'static GameData {
    static MAPS: OnceLock<GameData> = OnceLock::new();
    MAPS.get_or_init(|| {
        GameData::load_from_nx_map(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/game-data/Map.nx"),
        )
        .unwrap()
    })
}

fn talk(map: i32, npc: i32) -> Packet {
    let index = maps()
        .field(map)
        .unwrap()
        .map_npcs
        .iter()
        .position(|n| n.npc_id == npc)
        .unwrap();
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TalkToNpc as i16).unwrap();
    p.write_int(1_000_000_000 + index as i32).unwrap();
    p
}

/// What the client asks on entering a map with a ship.
fn ship_state(map: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ContiStateRequest as i16).unwrap();
    p.write_int(map).unwrap();
    p.write_byte(0).unwrap();
    p
}

/// The world's route time, if it runs on the same clock as this machine.
fn route_time() -> i64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    i64::try_from(millis).unwrap() * SCALE
}

#[tokio::test]
async fn a_ticket_takes_the_ship_from_ellinia_to_orbis() {
    let route = &conti::TO_ORBIS;
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    // Loading the map data takes seconds, longer than boarding stays open.
    let board = talk(ELLINIA_STATION, CHERRY);
    // Make the character, and let its logout save before giving it a ticket.
    let character_id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_item(character_id, 4, 1, route.ticket).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    // The station's wall clock is set on arrival.
    assert_eq!(
        warp(c, ELLINIA_STATION, "", &mut trace).await,
        ELLINIA_STATION
    );
    until(c, &mut trace, |t| !t.clocks.is_empty()).await;

    // Boarding opens as the ship from Orbis docks, 5 minutes before the
    // departure on the quarter hour.
    let term = route.timetable.term * MINUTE;
    let opens = (route.timetable.term - route.timetable.wait) * MINUTE;
    let wait = (opens - route_time().rem_euclid(term)).rem_euclid(term) / SCALE;
    tokio::time::sleep(Duration::from_millis((wait - 500).max(0) as u64)).await;
    until(c, &mut trace, |t| t.ship_moves.contains(&(12, 6))).await;
    let t = step(c, &m, ship_state(ELLINIA_STATION)).await;
    assert_eq!(t.ship_states, [State::Wait as u8]);

    // Cherry takes the ticket and shows the player to the waiting room.
    let t = step(c, &m, board).await;
    assert_eq!(t.prompts, [(CHERRY, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.warps, [route.waiting_room]);
    assert_eq!(t.items, [(route.ticket, -1)]);

    // At departure the waiting room is moved onto the deck...
    let mut trace = Trace::default();
    until(c, &mut trace, |t| t.warps.contains(&route.deck)).await;
    let t = step(c, &m, ship_state(route.deck)).await;
    assert_eq!(t.ship_states, [State::Move as u8]);

    // ...and on arrival the riders step off by the station's come00.
    let mut trace = Trace::default();
    until(c, &mut trace, |t| t.warps.contains(&ORBIS_STATION)).await;
    let come00 = maps()
        .field(ORBIS_STATION)
        .unwrap()
        .portals()
        .find(|portal| portal.name == "come00")
        .unwrap()
        .id;
    assert_eq!(trace.arrivals[0].1, u8::try_from(come00).unwrap());
}
