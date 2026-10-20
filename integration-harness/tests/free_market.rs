//! The Free Market: a town's entrance remembers the town, and the Free
//! Market's exit goes back there, across a relogin.

use integration_harness::connection::MapleTestConnection;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;

const HENESYS: i32 = 100_000_000;
const HENESYS_MARKET: i32 = 100_000_100;
const FREE_MARKET: i32 = 910_000_000;
/// The Free Market's exit, "out00".
const EXIT: u8 = 34;
/// Henesys Market's Free Market entrance.
const ENTRANCE: u8 = 21;

/// CHANGE_MAP_SPECIAL: walk into the scripted portal `portal`.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

/// Enter `portal` and return where the player arrived and whether the
/// portal sound played.
async fn go(c: &mut MapleTestConnection, portal: &str) -> (i32, u8, bool) {
    let mut t = Trace::default();
    send(c, enter(portal)).await;
    until(c, &mut t, |t| !t.arrivals.is_empty()).await;
    let (map, spawn_point, _) = t.arrivals[0];
    (map, spawn_point, t.portal_sounds == 1)
}

#[tokio::test]
async fn the_free_market_returns_players_to_their_town() {
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config).await.unwrap();
    let mut t = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, HENESYS_MARKET, "", &mut t).await, HENESYS_MARKET);
    assert_eq!(go(c, "market00").await, (FREE_MARKET, EXIT, true));
    drop(session);

    // The town is kept across a relogin, and forgotten on the way out.
    let mut session = relogin(&config).await;
    assert_eq!(session.map_id, FREE_MARKET);
    let c = &mut session.connection;
    assert_eq!(go(c, "out00").await, (HENESYS_MARKET, ENTRANCE, true));
    assert_eq!(go(c, "market00").await, (FREE_MARKET, EXIT, true));
    assert_eq!(go(c, "out00").await, (HENESYS_MARKET, ENTRANCE, true));

    // Without a town, the exit goes to Henesys.
    assert_eq!(warp(c, FREE_MARKET, "", &mut t).await, FREE_MARKET);
    assert_eq!(go(c, "out00").await, (HENESYS, 0, true));
}
