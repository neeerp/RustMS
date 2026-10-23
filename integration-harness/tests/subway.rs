//! The subway from Kerning City to New Leaf City: Bell sells the ticket,
//! the gate takes it while the train boards, and the waiting room rides to
//! New Leaf City, all without a ship on the platforms. Then back the same
//! way, leaving the waiting room once by Bell, who keeps the ticket that
//! was given up. Then the NLC Taxi to
//! the Haunted House's grounds and its gate (`halloween_enter`). The
//! harness world runs the route clock 60 times as fast
//! (`RUSTMS_CONTI_SCALE`), so a train leaves every 5 seconds.

use game_data::GameData;
use integration_harness::database::set_meso;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{send, stat, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::conti;
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
const BELL: i32 = 9201057;
const KERNING_CITY_GATE: i32 = 1052007;
const NLC_GATE: i32 = 9201068;
const NLC: i32 = 600_000_000;
const NLC_TAXI: i32 = 9201056;
const HAUNTED_HOUSE_GROUNDS: i32 = 682_000_000;
const HAUNTED_HOUSE: i32 = 682_000_100;

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

/// CHANGE_MAP_SPECIAL: walk into the scripted portal `portal`.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

fn route_time() -> i64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    i64::try_from(millis).unwrap() * SCALE
}

/// Wait for `route`'s next boarding: it opens 4 minutes before each
/// departure on the 5 minutes.
async fn until_boarding(route: &conti::Route) {
    let term = route.timetable.term * MINUTE;
    let opens = (route.timetable.term - route.timetable.wait) * MINUTE;
    let wait = (opens - route_time().rem_euclid(term)).rem_euclid(term) / SCALE;
    tokio::time::sleep(Duration::from_millis((wait + 200) as u64)).await;
}

#[tokio::test]
async fn the_subway_takes_a_ticket_holder_to_new_leaf_city() {
    let route = &conti::TO_NLC;
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    // Load the map data before boarding opens.
    let (buy, board) = (
        talk(route.platform, BELL),
        talk(route.platform, KERNING_CITY_GATE),
    );
    let taxi = talk(NLC, NLC_TAXI);
    let character_id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_meso(character_id, 30000).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(
        warp(c, route.platform, "", &mut trace).await,
        route.platform
    );

    // Bell sells the ticket to New Leaf City.
    let t = step(c, &m, buy).await;
    assert_eq!(t.prompts, [(BELL, 4)]);
    let t = step(c, &m, answer(4, 1, Some(0))).await;
    assert_eq!(t.prompts, [(BELL, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.items, [(route.ticket, 1)]);
    assert_eq!(t.stat(stat::MESO), Some(25000));

    // The platform has no ship to ask about.
    let t = step(c, &m, ship_state(route.platform)).await;
    assert!(t.ship_states.is_empty());

    until_boarding(route).await;
    let t = step(c, &m, board).await;
    assert_eq!(t.prompts, [(KERNING_CITY_GATE, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.warps, [route.waiting_room]);
    assert_eq!(t.items, [(route.ticket, -1)]);

    // The train leaves and arrives, and no ship is shown on the way.
    let mut trace = Trace::default();
    until(c, &mut trace, |t| t.warps.contains(&route.deck)).await;
    until(c, &mut trace, |t| t.warps.contains(&route.arrival.0)).await;
    assert!(trace.ship_moves.is_empty());

    // Bell at New Leaf City's station sells the ticket back.
    let back = &conti::TO_KERNING_CITY;
    assert_eq!(route.arrival.0, back.platform);
    let buy_back = || {
        (
            talk(back.platform, BELL),
            answer(4, 1, Some(0)),
            answer(1, 1, None),
        )
    };
    let (menu, choice, yes) = buy_back();
    assert_eq!(step(c, &m, menu).await.prompts, [(BELL, 4)]);
    assert_eq!(step(c, &m, choice).await.prompts, [(BELL, 1)]);
    let t = step(c, &m, yes).await;
    assert_eq!(t.items, [(back.ticket, 1)]);
    assert_eq!(t.stat(stat::MESO), Some(20000));

    // Its gate takes it; Bell in the waiting room lets the player out
    // again, without it.
    until_boarding(back).await;
    let t = step(c, &m, talk(back.platform, NLC_GATE)).await;
    assert_eq!(t.prompts, [(NLC_GATE, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.warps, [back.waiting_room]);
    assert_eq!(t.items, [(back.ticket, -1)]);
    let t = step(c, &m, talk(back.waiting_room, BELL)).await;
    assert_eq!(t.prompts, [(BELL, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.warps, [back.platform]);
    assert!(t.items.is_empty());

    // With another ticket the train takes them home to Kerning City.
    let (menu, choice, yes) = buy_back();
    step(c, &m, menu).await;
    step(c, &m, choice).await;
    let t = step(c, &m, yes).await;
    assert_eq!(t.stat(stat::MESO), Some(15000));
    until_boarding(back).await;
    step(c, &m, talk(back.platform, NLC_GATE)).await;
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.warps, [back.waiting_room]);
    let mut trace = Trace::default();
    until(c, &mut trace, |t| t.warps.contains(&back.deck)).await;
    until(c, &mut trace, |t| t.warps.contains(&back.arrival.0)).await;
    assert!(trace.ship_moves.is_empty());

    // The NLC Taxi to the Haunted House's grounds, and in by its gate.
    let mut trace = Trace::default();
    assert_eq!(warp(c, NLC, "", &mut trace).await, NLC);
    let t = step(c, &m, taxi).await;
    assert_eq!(t.prompts, [(NLC_TAXI, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.warps, [HAUNTED_HOUSE_GROUNDS]);
    assert_eq!(t.stat(stat::MESO), Some(0));
    let mut trace = Trace::default();
    send(c, enter("st01")).await;
    until(c, &mut trace, |t| !t.arrivals.is_empty()).await;
    assert_eq!(trace.arrivals[0].0, HAUNTED_HOUSE);
}
