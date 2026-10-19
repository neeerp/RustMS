//! A map with a time limit (`info/timeLimit`): the client is shown the
//! countdown on arrival (CLOCK kind 2), and when it runs out the player is
//! sent to the map's `forcedReturn`.

use integration_harness::login_to_world_session;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{barrier, read_for, until, warp, Trace};

const HENESYS: i32 = 100_000_000;
/// One of Gold Richie's treasure maps: 60 seconds, then back to his map.
const TREASURE: i32 = 390_000_000;
const GOLDRICH: i32 = 390_009_999;

#[tokio::test]
async fn time_runs_out_on_a_timed_map() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;

    // A map without a limit has no countdown.
    let mut t = Trace::default();
    warp(c, HENESYS, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    assert!(t.countdowns.is_empty());

    let mut t = Trace::default();
    warp(c, TREASURE, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    assert_eq!(t.countdowns, [60]);

    // The player stays the minute, then goes to Gold Richie's first portal.
    read_for(c, &mut t, 57_000).await;
    assert!(t.warps == [TREASURE], "left early: {:?}", t.warps);
    until(c, &mut t, |t| t.warps.contains(&GOLDRICH)).await;
    assert_eq!(t.arrivals.last(), Some(&(GOLDRICH, 0, None)));
}
