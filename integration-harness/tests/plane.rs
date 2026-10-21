//! The plane between Kerning City and Singapore's Changi Airport: Irene
//! and Shalon sell the tickets and board their holders while the plane
//! boards, the waiting rooms fly without a ship on the airports, and the
//! pilots let passengers back out. The plane home lands at Kerning City's
//! portal 7, by Irene. The harness world runs the route clock 60 times as
//! fast (`RUSTMS_CONTI_SCALE`), so a plane leaves every 6 seconds.

use game_data::GameData;
use integration_harness::connection::MapleTestConnection;
use integration_harness::database::set_meso;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{stat, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::conti::{self, Route};
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
const IRENE: i32 = 9270041;
const SHALON: i32 = 9270038;
const XINGA: i32 = 9270017;
const KERNY: i32 = 9270018;

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

fn route_time() -> i64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    i64::try_from(millis).unwrap() * SCALE
}

/// Sleep until `minute` minutes into the plane's next 6-minute round, and
/// a little more.
async fn sleep_until(route: &Route, minute: i64) {
    let term = route.timetable.term * MINUTE;
    let wait = (minute * MINUTE - route_time().rem_euclid(term)).rem_euclid(term) / SCALE;
    tokio::time::sleep(Duration::from_millis((wait + 200) as u64)).await;
}

/// Boarding opens `wait` minutes before each departure.
async fn sleep_until_boarding(route: &Route) {
    sleep_until(route, route.timetable.term - route.timetable.wait).await;
}

/// Buy `route`'s ticket from `booth`, its first choice.
async fn buy(c: &mut MapleTestConnection, m: &str, booth: i32, talk: Packet) -> Trace {
    let t = step(c, m, talk).await;
    assert_eq!(t.prompts, [(booth, 4)]);
    let t = step(c, m, answer(4, 1, Some(0))).await;
    assert_eq!(t.prompts, [(booth, 1)]);
    step(c, m, answer(1, 1, None)).await
}

/// Board at `booth`, its second choice.
async fn board(c: &mut MapleTestConnection, m: &str, booth: i32, talk: Packet) -> Trace {
    let t = step(c, m, talk).await;
    assert_eq!(t.prompts, [(booth, 4)]);
    let t = step(c, m, answer(4, 1, Some(1))).await;
    if t.prompts != [(booth, 1)] {
        return t;
    }
    step(c, m, answer(1, 1, None)).await
}

#[tokio::test]
async fn the_plane_takes_a_ticket_holder_from_kerning_city_to_changi_and_back() {
    let (out, home) = (&conti::TO_CHANGI, &conti::AIR_TO_KERNING_CITY);
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    // Load the map data before boarding opens.
    let (irene, shalon) = (talk(out.platform, IRENE), talk(home.platform, SHALON));
    let character_id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_meso(character_id, 20000).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(warp(c, out.platform, "", &mut trace).await, out.platform);

    // Irene sells one ticket to Singapore, and not a second.
    let t = buy(c, &m, IRENE, irene.clone()).await;
    assert_eq!(t.items, [(out.ticket, 1)]);
    assert_eq!(t.stat(stat::MESO), Some(15000));
    let t = buy(c, &m, IRENE, irene.clone()).await;
    assert!(t.items.is_empty());
    assert_eq!(t.stat(stat::MESO), None);

    // The airport has no ship to ask about.
    let t = step(c, &m, ship_state(out.platform)).await;
    assert!(t.ship_states.is_empty());

    // She boards a holder while the plane boards.
    sleep_until_boarding(out).await;
    let t = board(c, &m, IRENE, irene).await;
    assert_eq!(t.warps, [out.waiting_room]);
    assert_eq!(t.items, [(out.ticket, -1)]);

    // The plane leaves and lands at Changi, and no ship is shown.
    let mut trace = Trace::default();
    until(c, &mut trace, |t| t.warps.contains(&out.deck)).await;
    until(c, &mut trace, |t| t.warps.contains(&home.platform)).await;
    assert!(trace.ship_moves.is_empty());
    assert_eq!(trace.arrivals.last().unwrap().0, home.platform);
    assert_eq!(trace.arrivals.last().unwrap().1, 0);

    // Shalon sells the way back, and the plane home lands by Irene.
    let t = buy(c, &m, SHALON, shalon.clone()).await;
    assert_eq!(t.items, [(home.ticket, 1)]);
    assert_eq!(t.stat(stat::MESO), Some(10000));
    sleep_until_boarding(home).await;
    let t = board(c, &m, SHALON, shalon).await;
    assert_eq!(t.warps, [home.waiting_room]);
    assert_eq!(t.items, [(home.ticket, -1)]);
    let mut trace = Trace::default();
    until(c, &mut trace, |t| t.warps.contains(&home.deck)).await;
    until(c, &mut trace, |t| t.warps.contains(&out.platform)).await;
    assert!(trace.ship_moves.is_empty());
    let landed = *trace.arrivals.last().unwrap();
    assert_eq!((landed.0, landed.1), (out.platform, 7));
}

#[tokio::test]
async fn a_passenger_can_leave_the_waiting_room_and_boarding_closes() {
    let (out, home) = (&conti::TO_CHANGI, &conti::AIR_TO_KERNING_CITY);
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let (irene, shalon) = (talk(out.platform, IRENE), talk(home.platform, SHALON));
    let (xinga, kerny) = (
        talk(out.waiting_room, XINGA),
        talk(home.waiting_room, KERNY),
    );
    let kerny_aboard = talk(out.deck, KERNY);
    let character_id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_meso(character_id, 20000).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(warp(c, out.platform, "", &mut trace).await, out.platform);

    // Xinga lets a passenger out of Kerning City's waiting room; the
    // ticket is gone.
    buy(c, &m, IRENE, irene.clone()).await;
    sleep_until_boarding(out).await;
    let t = board(c, &m, IRENE, irene.clone()).await;
    assert_eq!(t.warps, [out.waiting_room]);
    let t = step(c, &m, xinga).await;
    assert_eq!(t.prompts, [(XINGA, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.warps, [out.platform]);
    assert!(t.items.is_empty());

    // Boarding closes a minute before the plane leaves: the holder stays.
    buy(c, &m, IRENE, irene.clone()).await;
    sleep_until(out, out.timetable.term - out.timetable.closed).await;
    let t = board(c, &m, IRENE, irene.clone()).await;
    assert_eq!(t.prompts, [(IRENE, 0)]);
    assert!(t.warps.is_empty());
    assert!(t.items.is_empty());

    // Aboard, Kerny only says the plane is nearly there.
    sleep_until_boarding(out).await;
    let t = board(c, &m, IRENE, irene).await;
    assert_eq!(t.warps, [out.waiting_room]);
    let mut trace = Trace::default();
    until(c, &mut trace, |t| t.warps.contains(&out.deck)).await;
    let mut trace = step(c, &m, kerny_aboard).await;
    assert_eq!(trace.prompts, [(KERNY, 0)]);
    until(c, &mut trace, |t| t.warps.contains(&home.platform)).await;

    // Kerny lets a passenger out of Changi's waiting room.
    buy(c, &m, SHALON, shalon.clone()).await;
    sleep_until_boarding(home).await;
    let t = board(c, &m, SHALON, shalon).await;
    assert_eq!(t.warps, [home.waiting_room]);
    let t = step(c, &m, kerny).await;
    assert_eq!(t.prompts, [(KERNY, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.warps, [home.platform]);
}
